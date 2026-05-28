#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol, Vec};

// ── Data models ──────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub owner: Address,
    pub timestamp: u64,
    pub authorized_actors: Vec<Address>,
    /// Unix timestamp (seconds) after which the product is considered expired.
    /// 0 means no expiration set.
    pub expiration_timestamp: u64,
    /// Whether the product has been marked as spoiled.
    pub spoiled: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
    pub product_id: String,
    pub location: String,
    pub actor: Address,
    pub timestamp: u64,
    pub event_type: String, // HARVEST | PROCESSING | SHIPPING | RETAIL | SPOILED | EXPIRED
    pub metadata: String,   // JSON string
}

/// A batch/lot grouping multiple product IDs together. (#405)
#[contracttype]
#[derive(Clone)]
pub struct Batch {
    pub id: String,
    pub name: String,
    pub owner: Address,
    pub product_ids: Vec<String>,
    pub timestamp: u64,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Product(String),
    Events(String),
    /// Batch entity keyed by batch ID. (#405)
    Batch(String),
    /// Aggregate events recorded at the batch level. (#405)
    BatchEvents(String),
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct SupplyLinkContract;

#[contractimpl]
impl SupplyLinkContract {
    // ── Product registration ──────────────────────────────────────────────────

    /// Register a new product on-chain.
    pub fn register_product(
        env: Env,
        id: String,
        name: String,
        origin: String,
        owner: Address,
    ) -> Product {
        owner.require_auth();
        let product = Product {
            id: id.clone(),
            name,
            origin,
            owner,
            timestamp: env.ledger().timestamp(),
            authorized_actors: Vec::new(&env),
            expiration_timestamp: 0,
            spoiled: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Product(id), &product);
        product
    }

    /// Add a tracking event for a product.
    pub fn add_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if product.spoiled {
            panic!("product is spoiled");
        }

        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            panic!("caller is not authorized");
        }
        caller.require_auth();

        let event = TrackingEvent {
            product_id: product_id.clone(),
            location,
            actor: caller,
            timestamp: env.ledger().timestamp(),
            event_type,
            metadata,
        };

        let mut events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        events.push_back(event.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id), &events);

        event
    }

    /// Get product details.
    pub fn get_product(env: Env, id: String) -> Product {
        env.storage()
            .persistent()
            .get(&DataKey::Product(id))
            .expect("product not found")
    }

    /// Get all tracking events for a product.
    pub fn get_tracking_events(env: Env, product_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Transfer product ownership.
    /// Panics if the product is spoiled — spoiled products cannot be transferred.
    pub fn transfer_ownership(env: Env, product_id: String, new_owner: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if product.spoiled {
            panic!("spoiled product cannot be transferred");
        }

        product.owner.require_auth();

        if product.owner == new_owner {
            panic!("new owner must differ from current owner");
        }

        product.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "ownership_transferred"), product_id),
            new_owner,
        );

        true
    }

    /// Authorize an actor to add events for a product.
    pub fn add_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
        product.authorized_actors.push_back(actor);
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
    }

    /// Remove an authorized actor from a product.
    pub fn remove_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        let mut new_actors = Vec::new(&env);
        let mut found = false;
        for i in 0..product.authorized_actors.len() {
            let a = product.authorized_actors.get(i).unwrap();
            if a == actor && !found {
                found = true;
            } else {
                new_actors.push_back(a);
            }
        }
        product.authorized_actors = new_actors;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        found
    }

    // ── #406: Expiration & spoilage ───────────────────────────────────────────

    /// Set or update the expiration timestamp for a product (owner-only).
    /// Pass 0 to clear the expiration.
    pub fn update_expiration(
        env: Env,
        product_id: String,
        expiration_timestamp: u64,
    ) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();
        product.expiration_timestamp = expiration_timestamp;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "expiration_updated"), product_id),
            expiration_timestamp,
        );
        true
    }

    /// Returns true if the product has an expiration set and the ledger
    /// timestamp has passed it.
    pub fn is_expired(env: Env, product_id: String) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id))
            .expect("product not found");
        product.expiration_timestamp > 0
            && env.ledger().timestamp() >= product.expiration_timestamp
    }

    /// Mark a product as spoiled (owner-only). Records a SPOILED event.
    /// Spoiled products cannot receive new tracking events or be transferred.
    pub fn mark_spoiled(
        env: Env,
        product_id: String,
        reason: String,
    ) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        if product.spoiled {
            return true; // idempotent
        }

        product.spoiled = true;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Record a SPOILED event in the event log
        let event = TrackingEvent {
            product_id: product_id.clone(),
            location: String::from_str(&env, "N/A"),
            actor: product.owner.clone(),
            timestamp: env.ledger().timestamp(),
            event_type: String::from_str(&env, "SPOILED"),
            metadata: reason.clone(),
        };
        let mut events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        events.push_back(event);
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id.clone()), &events);

        env.events().publish(
            (Symbol::new(&env, "product_spoiled"), product_id),
            reason,
        );
        true
    }

    // ── #408: Key rotation ────────────────────────────────────────────────────

    /// Rotate the owner key for a product.
    ///
    /// The current owner must sign (via `old_owner.require_auth()`).
    /// The new owner address replaces the old one atomically.
    /// This is semantically equivalent to `transfer_ownership` but is
    /// explicitly named for key-rotation workflows.
    pub fn rotate_owner_key(
        env: Env,
        product_id: String,
        old_owner: Address,
        new_owner: Address,
    ) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if product.owner != old_owner {
            panic!("old_owner does not match current owner");
        }
        if old_owner == new_owner {
            panic!("new_owner must differ from old_owner");
        }

        old_owner.require_auth();

        product.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "owner_key_rotated"), product_id),
            (old_owner, new_owner),
        );
        true
    }

    /// Rotate an authorized actor key for a product.
    ///
    /// The old actor must sign. The old address is removed from
    /// `authorized_actors` and the new address is appended atomically.
    pub fn rotate_authorized_actor_key(
        env: Env,
        product_id: String,
        old_actor: Address,
        new_actor: Address,
    ) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if old_actor == new_actor {
            panic!("new_actor must differ from old_actor");
        }

        // Verify old_actor is currently authorized
        if !product.authorized_actors.contains(&old_actor) {
            panic!("old_actor is not an authorized actor");
        }

        old_actor.require_auth();

        // Replace old_actor with new_actor
        let mut new_actors = Vec::new(&env);
        for i in 0..product.authorized_actors.len() {
            let a = product.authorized_actors.get(i).unwrap();
            if a == old_actor {
                new_actors.push_back(new_actor.clone());
            } else {
                new_actors.push_back(a);
            }
        }
        product.authorized_actors = new_actors;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "actor_key_rotated"), product_id),
            (old_actor, new_actor),
        );
        true
    }

    // ── #405: Batch / lot tracking ────────────────────────────────────────────

    /// Create a new batch/lot grouping.
    pub fn create_batch(
        env: Env,
        id: String,
        name: String,
        owner: Address,
    ) -> Batch {
        owner.require_auth();
        let batch = Batch {
            id: id.clone(),
            name,
            owner,
            product_ids: Vec::new(&env),
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Batch(id.clone()), &batch);

        env.events().publish(
            (Symbol::new(&env, "batch_created"), id),
            batch.clone(),
        );
        batch
    }

    /// Add a product to a batch (batch owner-only).
    pub fn add_product_to_batch(
        env: Env,
        batch_id: String,
        product_id: String,
    ) -> bool {
        let mut batch: Batch = env
            .storage()
            .persistent()
            .get(&DataKey::Batch(batch_id.clone()))
            .expect("batch not found");

        // Verify product exists
        if !env.storage().persistent().has(&DataKey::Product(product_id.clone())) {
            panic!("product not found");
        }

        batch.owner.require_auth();
        batch.product_ids.push_back(product_id.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Batch(batch_id.clone()), &batch);

        env.events().publish(
            (Symbol::new(&env, "product_added_to_batch"), batch_id),
            product_id,
        );
        true
    }

    /// Record an aggregate event against a batch (batch owner-only).
    /// The event is stored at the batch level and does NOT appear in
    /// individual product event logs.
    pub fn record_batch_event(
        env: Env,
        batch_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        let batch: Batch = env
            .storage()
            .persistent()
            .get(&DataKey::Batch(batch_id.clone()))
            .expect("batch not found");

        if batch.owner != caller {
            panic!("caller is not the batch owner");
        }
        caller.require_auth();

        let event = TrackingEvent {
            product_id: batch_id.clone(),
            location,
            actor: caller,
            timestamp: env.ledger().timestamp(),
            event_type,
            metadata,
        };

        let mut events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::BatchEvents(batch_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        events.push_back(event.clone());
        env.storage()
            .persistent()
            .set(&DataKey::BatchEvents(batch_id), &events);

        event
    }

    /// Get all events recorded at the batch level.
    pub fn get_batch_events(env: Env, batch_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::BatchEvents(batch_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get a batch by ID.
    pub fn get_batch(env: Env, id: String) -> Batch {
        env.storage()
            .persistent()
            .get(&DataKey::Batch(id))
            .expect("batch not found")
    }
}

mod tests;
