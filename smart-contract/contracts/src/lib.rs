#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, Symbol};

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
}

#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
    pub product_id: String,
    pub location: String,
    pub actor: Address,
    pub timestamp: u64,
    pub event_type: String, // HARVEST | PROCESSING | SHIPPING | RETAIL
    pub metadata: String,   // JSON string
    pub archived: bool,
    pub archived_at: u64,   // 0 if not archived
}

/// Certification record associated with a product.
#[contracttype]
#[derive(Clone)]
pub struct Certification {
    pub cert_id: String,
    pub product_id: String,
    pub issuer: Address,
    pub issued_at: u64,
    pub cert_type: String,  // e.g. "ORGANIC", "FAIR_TRADE", "ISO9001"
    pub reference: String,  // external registry reference / URL
    pub revoked: bool,
    pub revoked_at: u64,    // 0 if not revoked
}

// ── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Product(String),
    Events(String),
    ArchivedEvents(String),
    ProductCount,
    ProductIndex(u64),
    Certifications(String), // keyed by product_id
    CertById(String),       // keyed by cert_id for fast lookup
    Paused,                 // bool — emergency stop flag
    Guardians,              // Vec<Address> — addresses allowed to pause/unpause
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Panics with "contract is paused" when the emergency stop is active.
fn require_not_paused(env: &Env) {
    let paused: bool = env
        .storage()
        .persistent()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused {
        panic!("contract is paused");
    }
}

/// Panics unless `caller` is in the guardians list.
fn require_guardian(env: &Env, caller: &Address) {
    let guardians: Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::Guardians)
        .unwrap_or_else(|| Vec::new(env));
    if !guardians.contains(caller) {
        panic!("caller is not a guardian");
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct SupplyLinkContract;

#[contractimpl]
impl SupplyLinkContract {
    /// Register a new product on-chain.
    pub fn register_product(
        env: Env,
        id: String,
        name: String,
        origin: String,
        owner: Address,
    ) -> Product {
        require_not_paused(&env);
        owner.require_auth();
        let product = Product {
            id: id.clone(),
            name,
            origin,
            owner,
            timestamp: env.ledger().timestamp(),
            authorized_actors: Vec::new(&env),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Product(id.clone()), &product);

        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::ProductCount, &(count + 1));
        env.storage()
            .persistent()
            .set(&DataKey::ProductIndex(count), &id);

        env.events().publish(
            (Symbol::new(&env, "product_registered"), id.clone()),
            product.clone(),
        );

        product
    }

    /// Add a tracking event for a product.
    /// `caller` must be the product owner or an address in `authorized_actors`.
    pub fn add_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        require_not_paused(&env);
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

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
            event_type: event_type.clone(),
            metadata,
            archived: false,
            archived_at: 0,
        };

        let mut active_events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        active_events.push_back(event.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id.clone()), &active_events);

        env.events().publish(
            (Symbol::new(&env, "event_added"), product_id, event_type),
            event.clone(),
        );

        event
    }

    /// Archive a tracking event by its index within the product's event list.
    /// Only the product owner may archive events.
    /// Archived events are moved to a separate archived list and removed from
    /// the active list, preserving full integrity for auditing.
    pub fn archive_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        event_index: u32,
    ) -> TrackingEvent {
        require_not_paused(&env);
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if product.owner != caller {
            panic!("only the product owner can archive events");
        }
        caller.require_auth();

        let events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        if event_index >= events.len() {
            panic!("event index out of bounds");
        }

        let mut target = events.get(event_index).unwrap();
        if target.archived {
            panic!("event is already archived");
        }

        target.archived = true;
        target.archived_at = env.ledger().timestamp();

        // Rebuild active list without the archived event
        let mut active: Vec<TrackingEvent> = Vec::new(&env);
        for i in 0..events.len() {
            if i != event_index {
                active.push_back(events.get(i).unwrap());
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id.clone()), &active);

        // Append to archived list
        let mut archived: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::ArchivedEvents(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        archived.push_back(target.clone());
        env.storage()
            .persistent()
            .set(&DataKey::ArchivedEvents(product_id.clone()), &archived);

        env.events().publish(
            (Symbol::new(&env, "event_archived"), product_id),
            target.clone(),
        );

        target
    }

    /// List all archived events for a product.
    pub fn list_archived_events(env: Env, product_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::ArchivedEvents(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get product details.
    pub fn get_product(env: Env, id: String) -> Product {
        env.storage()
            .persistent()
            .get(&DataKey::Product(id))
            .expect("product not found")
    }

    /// Get all active (non-archived) tracking events for a product.
    pub fn get_tracking_events(env: Env, product_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns true if a product with the given id is registered, false otherwise.
    pub fn product_exists(env: Env, id: String) -> bool {
        env.storage().persistent().has(&DataKey::Product(id))
    }

    /// Returns the number of active tracking events recorded for `product_id`.
    pub fn get_events_count(env: Env, product_id: String) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<TrackingEvent>>(&DataKey::Events(product_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Transfer product ownership.
    pub fn transfer_ownership(env: Env, product_id: String, new_owner: Address) -> bool {
        require_not_paused(&env);
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
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
        require_not_paused(&env);
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();
        product.authorized_actors.push_back(actor.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "actor_authorized"), product_id),
            actor,
        );

        true
    }

    /// Remove an authorized actor from a product.
    /// Only the product owner may call this.
    /// Returns true if the actor was removed, false if they were not in the list.
    pub fn remove_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        require_not_paused(&env);
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        let mut found = false;
        let mut new_actors = Vec::new(&env);
        for i in 0..product.authorized_actors.len() {
            let current_actor = product.authorized_actors.get(i).unwrap();
            if current_actor != actor {
                new_actors.push_back(current_actor);
            } else {
                found = true;
            }
        }

        product.authorized_actors = new_actors;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);

        found
    }

    /// Update product metadata (name and origin).
    /// Only the product owner may call this.
    pub fn update_product_metadata(
        env: Env,
        product_id: String,
        name: String,
        origin: String,
    ) -> Product {
        require_not_paused(&env);
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        product.name = name;
        product.origin = origin;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "product_updated"), product_id),
            product.clone(),
        );

        product
    }

    /// Get the list of authorized actors for a product.
    pub fn get_authorized_actors(env: Env, product_id: String) -> Vec<Address> {
        env.storage()
            .persistent()
            .get::<DataKey, Product>(&DataKey::Product(product_id))
            .map(|p| p.authorized_actors)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get the total number of registered products.
    pub fn get_product_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0)
    }

    /// List products with pagination.
    pub fn list_products(env: Env, offset: u64, limit: u64) -> Vec<String> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);

        let mut products = Vec::new(&env);
        let end = core::cmp::min(offset + limit, count);

        for i in offset..end {
            if let Some(product_id) =
                env.storage()
                    .persistent()
                    .get::<DataKey, String>(&DataKey::ProductIndex(i))
            {
                products.push_back(product_id);
            }
        }

        products
    }

    // ── Certification Registry ────────────────────────────────────────────────

    /// Issue a certification for a product.
    /// Only the product owner or an authorized actor may issue certifications.
    pub fn issue_certification(
        env: Env,
        product_id: String,
        caller: Address,
        cert_id: String,
        cert_type: String,
        reference: String,
    ) -> Certification {
        require_not_paused(&env);
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            panic!("caller is not authorized");
        }
        caller.require_auth();

        // Prevent duplicate cert IDs
        if env
            .storage()
            .persistent()
            .has(&DataKey::CertById(cert_id.clone()))
        {
            panic!("certification id already exists");
        }

        let cert = Certification {
            cert_id: cert_id.clone(),
            product_id: product_id.clone(),
            issuer: caller,
            issued_at: env.ledger().timestamp(),
            cert_type: cert_type.clone(),
            reference,
            revoked: false,
            revoked_at: 0,
        };

        // Store by cert_id for fast lookup
        env.storage()
            .persistent()
            .set(&DataKey::CertById(cert_id.clone()), &cert);

        // Append to product's certification list
        let mut product_certs: Vec<Certification> = env
            .storage()
            .persistent()
            .get(&DataKey::Certifications(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        product_certs.push_back(cert.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Certifications(product_id.clone()), &product_certs);

        env.events().publish(
            (Symbol::new(&env, "cert_issued"), product_id, cert_type),
            cert.clone(),
        );

        cert
    }

    /// Revoke a certification. Only the original issuer or product owner may revoke.
    pub fn revoke_certification(
        env: Env,
        product_id: String,
        caller: Address,
        cert_id: String,
    ) -> Certification {
        require_not_paused(&env);
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let mut cert: Certification = env
            .storage()
            .persistent()
            .get(&DataKey::CertById(cert_id.clone()))
            .expect("certification not found");

        if cert.product_id != product_id {
            panic!("certification does not belong to this product");
        }
        if cert.revoked {
            panic!("certification is already revoked");
        }

        let is_owner = product.owner == caller;
        let is_issuer = cert.issuer == caller;
        if !is_owner && !is_issuer {
            panic!("only the product owner or original issuer can revoke");
        }
        caller.require_auth();

        cert.revoked = true;
        cert.revoked_at = env.ledger().timestamp();

        // Update by cert_id
        env.storage()
            .persistent()
            .set(&DataKey::CertById(cert_id.clone()), &cert);

        // Update within product's certification list
        let certs: Vec<Certification> = env
            .storage()
            .persistent()
            .get(&DataKey::Certifications(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        let mut updated_certs: Vec<Certification> = Vec::new(&env);
        for i in 0..certs.len() {
            let c = certs.get(i).unwrap();
            if c.cert_id == cert_id {
                updated_certs.push_back(cert.clone());
            } else {
                updated_certs.push_back(c);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::Certifications(product_id.clone()), &updated_certs);

        env.events().publish(
            (Symbol::new(&env, "cert_revoked"), product_id, cert_id),
            cert.clone(),
        );

        cert
    }

    /// Verify a certification: returns the cert if it exists and is not revoked.
    /// Panics if the cert does not exist, does not belong to the product, or is revoked.
    pub fn verify_certification(
        env: Env,
        product_id: String,
        cert_id: String,
    ) -> Certification {
        let cert: Certification = env
            .storage()
            .persistent()
            .get(&DataKey::CertById(cert_id.clone()))
            .expect("certification not found");

        if cert.product_id != product_id {
            panic!("certification does not belong to this product");
        }
        if cert.revoked {
            panic!("certification has been revoked");
        }

        cert
    }

    /// Get all certifications for a product (active and revoked).
    pub fn get_certifications(env: Env, product_id: String) -> Vec<Certification> {
        env.storage()
            .persistent()
            .get(&DataKey::Certifications(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ── Emergency Stop ────────────────────────────────────────────────────────

    /// Bootstrap: add the first guardian. Can only be called when the guardians
    /// list is empty (i.e. contract is freshly deployed). The caller becomes the
    /// initial guardian and must authorise the call.
    pub fn init_guardian(env: Env, guardian: Address) {
        let existing: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        if !existing.is_empty() {
            panic!("guardians already initialised");
        }
        guardian.require_auth();
        let mut guardians = Vec::new(&env);
        guardians.push_back(guardian.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Guardians, &guardians);
        env.events().publish(
            (Symbol::new(&env, "guardian_added"),),
            guardian,
        );
    }

    /// Add a new guardian. Only an existing guardian may call this.
    pub fn add_guardian(env: Env, caller: Address, new_guardian: Address) {
        caller.require_auth();
        require_guardian(&env, &caller);
        let mut guardians: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        if guardians.contains(&new_guardian) {
            panic!("address is already a guardian");
        }
        guardians.push_back(new_guardian.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Guardians, &guardians);
        env.events().publish(
            (Symbol::new(&env, "guardian_added"),),
            new_guardian,
        );
    }

    /// Remove a guardian. Only an existing guardian may call this.
    /// The last guardian cannot be removed (must always have at least one).
    pub fn remove_guardian(env: Env, caller: Address, target: Address) {
        caller.require_auth();
        require_guardian(&env, &caller);
        let guardians: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        if guardians.len() <= 1 {
            panic!("cannot remove the last guardian");
        }
        let mut updated = Vec::new(&env);
        let mut found = false;
        for i in 0..guardians.len() {
            let g = guardians.get(i).unwrap();
            if g == target {
                found = true;
            } else {
                updated.push_back(g);
            }
        }
        if !found {
            panic!("address is not a guardian");
        }
        env.storage()
            .persistent()
            .set(&DataKey::Guardians, &updated);
        env.events().publish(
            (Symbol::new(&env, "guardian_removed"),),
            target,
        );
    }

    /// Pause or unpause the contract. Only a guardian may call this.
    /// When paused, all write operations are rejected.
    /// Read operations remain available at all times.
    pub fn set_pause_state(env: Env, caller: Address, paused: bool) {
        caller.require_auth();
        require_guardian(&env, &caller);
        env.storage()
            .persistent()
            .set(&DataKey::Paused, &paused);
        env.events().publish(
            (Symbol::new(&env, "pause_state_changed"),),
            paused,
        );
    }

    /// Returns true if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Returns the current list of guardians.
    pub fn get_guardians(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, soroban_sdk::Address, soroban_sdk::Address, String) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let owner = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-001");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory A"),
            &owner,
        );
        (env, contract_id, owner, product_id)
    }

    fn add_event(
        env: &Env,
        contract_id: &soroban_sdk::Address,
        product_id: &String,
        caller: &soroban_sdk::Address,
    ) {
        let client = SupplyLinkContractClient::new(env, contract_id);
        client.add_tracking_event(
            product_id,
            caller,
            &String::from_str(env, "Warehouse"),
            &String::from_str(env, "SHIPPING"),
            &String::from_str(env, "{}"),
        );
    }

    // ── Basic event count tests ───────────────────────────────────────────────

    #[test]
    fn test_unknown_product_returns_zero() {
        let env = Env::default();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let unknown = String::from_str(&env, "does-not-exist");
        assert_eq!(client.get_events_count(&unknown), 0);
    }

    #[test]
    fn test_registered_product_no_events_returns_zero() {
        let (env, contract_id, _owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_events_count(&product_id), 0);
    }

    #[test]
    fn test_one_event_returns_one() {
        let (env, contract_id, owner, product_id) = setup();
        add_event(&env, &contract_id, &product_id, &owner);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_events_count(&product_id), 1);
    }

    #[test]
    fn test_multiple_events_returns_correct_count() {
        let (env, contract_id, owner, product_id) = setup();
        for _ in 0..5 {
            add_event(&env, &contract_id, &product_id, &owner);
        }
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_events_count(&product_id), 5);
    }

    #[test]
    fn test_count_equals_vec_len() {
        let (env, contract_id, owner, product_id) = setup();
        for _ in 0..3 {
            add_event(&env, &contract_id, &product_id, &owner);
        }
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let count = client.get_events_count(&product_id);
        let events = client.get_tracking_events(&product_id);
        assert_eq!(count, events.len());
    }

    // ── Archival tests ────────────────────────────────────────────────────────

    /// Archived event no longer appears in active timeline
    #[test]
    fn test_archived_event_removed_from_active() {
        let (env, contract_id, owner, product_id) = setup();
        add_event(&env, &contract_id, &product_id, &owner);
        add_event(&env, &contract_id, &product_id, &owner);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        // Archive the first event (index 0)
        client.archive_tracking_event(&product_id, &owner, &0u32);

        let active = client.get_tracking_events(&product_id);
        assert_eq!(active.len(), 1, "only one active event should remain");
        assert!(!active.get(0).unwrap().archived);
    }

    /// Archived event appears in list_archived_events
    #[test]
    fn test_archived_event_queryable() {
        let (env, contract_id, owner, product_id) = setup();
        add_event(&env, &contract_id, &product_id, &owner);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        client.archive_tracking_event(&product_id, &owner, &0u32);

        let archived = client.list_archived_events(&product_id);
        assert_eq!(archived.len(), 1);
        let ev = archived.get(0).unwrap();
        assert!(ev.archived);
        assert!(ev.archived_at > 0);
    }

    /// Archived event retains all original fields (integrity preserved)
    #[test]
    fn test_archived_event_integrity_preserved() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let original = client.add_tracking_event(
            &product_id,
            &owner,
            &String::from_str(&env, "Port of Rotterdam"),
            &String::from_str(&env, "SHIPPING"),
            &String::from_str(&env, r#"{"vessel":"MV Stellar"}"#),
        );

        client.archive_tracking_event(&product_id, &owner, &0u32);

        let archived = client.list_archived_events(&product_id);
        let ev = archived.get(0).unwrap();
        assert_eq!(ev.location, original.location);
        assert_eq!(ev.actor, original.actor);
        assert_eq!(ev.event_type, original.event_type);
        assert_eq!(ev.metadata, original.metadata);
        assert_eq!(ev.timestamp, original.timestamp);
    }

    /// Non-owner cannot archive events
    #[test]
    #[should_panic(expected = "only the product owner can archive events")]
    fn test_non_owner_cannot_archive() {
        let (env, contract_id, owner, product_id) = setup();
        add_event(&env, &contract_id, &product_id, &owner);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let stranger = soroban_sdk::Address::generate(&env);
        client.archive_tracking_event(&product_id, &stranger, &0u32);
    }

    /// Archiving an already-archived event panics
    #[test]
    #[should_panic(expected = "event index out of bounds")]
    fn test_archive_already_archived_panics() {
        let (env, contract_id, owner, product_id) = setup();
        add_event(&env, &contract_id, &product_id, &owner);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.archive_tracking_event(&product_id, &owner, &0u32);
        // Event is gone from active list; index 0 no longer exists
        client.archive_tracking_event(&product_id, &owner, &0u32);
    }

    /// list_archived_events returns empty for product with no archived events
    #[test]
    fn test_list_archived_events_empty() {
        let (env, contract_id, owner, product_id) = setup();
        add_event(&env, &contract_id, &product_id, &owner);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let archived = client.list_archived_events(&product_id);
        assert_eq!(archived.len(), 0);
    }

    /// Active count decrements after archival; archived count increments
    #[test]
    fn test_archive_moves_event_between_lists() {
        let (env, contract_id, owner, product_id) = setup();
        for _ in 0..3 {
            add_event(&env, &contract_id, &product_id, &owner);
        }
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        client.archive_tracking_event(&product_id, &owner, &1u32);

        assert_eq!(client.get_events_count(&product_id), 2);
        assert_eq!(client.list_archived_events(&product_id).len(), 1);
    }

    // ── Certification tests ───────────────────────────────────────────────────

    fn issue_cert(
        env: &Env,
        contract_id: &soroban_sdk::Address,
        product_id: &String,
        caller: &soroban_sdk::Address,
        cert_id: &str,
    ) -> Certification {
        let client = SupplyLinkContractClient::new(env, contract_id);
        client.issue_certification(
            product_id,
            caller,
            &String::from_str(env, cert_id),
            &String::from_str(env, "ORGANIC"),
            &String::from_str(env, "https://registry.example/cert/123"),
        )
    }

    #[test]
    fn test_issue_certification_success() {
        let (env, contract_id, owner, product_id) = setup();
        let cert = issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        assert_eq!(cert.cert_id, String::from_str(&env, "cert-001"));
        assert_eq!(cert.product_id, product_id);
        assert!(!cert.revoked);
        assert_eq!(cert.revoked_at, 0);
    }

    #[test]
    fn test_get_certifications_returns_issued() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-002");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let certs = client.get_certifications(&product_id);
        assert_eq!(certs.len(), 2);
    }

    #[test]
    fn test_verify_certification_valid() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let cert = client.verify_certification(&product_id, &String::from_str(&env, "cert-001"));
        assert!(!cert.revoked);
    }

    #[test]
    fn test_revoke_certification_success() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let revoked = client.revoke_certification(
            &product_id,
            &owner,
            &String::from_str(&env, "cert-001"),
        );
        assert!(revoked.revoked);
        assert!(revoked.revoked_at > 0);
    }

    #[test]
    #[should_panic(expected = "certification has been revoked")]
    fn test_verify_revoked_cert_panics() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.revoke_certification(
            &product_id,
            &owner,
            &String::from_str(&env, "cert-001"),
        );
        client.verify_certification(&product_id, &String::from_str(&env, "cert-001"));
    }

    #[test]
    #[should_panic(expected = "certification id already exists")]
    fn test_duplicate_cert_id_rejected() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-dup");
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-dup");
    }

    #[test]
    #[should_panic(expected = "caller is not authorized")]
    fn test_unauthorized_cannot_issue_cert() {
        let (env, contract_id, _owner, product_id) = setup();
        let stranger = soroban_sdk::Address::generate(&env);
        issue_cert(&env, &contract_id, &product_id, &stranger, "cert-unauth");
    }

    #[test]
    #[should_panic(expected = "only the product owner or original issuer can revoke")]
    fn test_unauthorized_cannot_revoke_cert() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        let stranger = soroban_sdk::Address::generate(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.revoke_certification(
            &product_id,
            &stranger,
            &String::from_str(&env, "cert-001"),
        );
    }

    #[test]
    fn test_authorized_actor_can_issue_cert() {
        let (env, contract_id, owner, product_id) = setup();
        let actor = soroban_sdk::Address::generate(&env);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.add_authorized_actor(&product_id, &actor);
        let cert = issue_cert(&env, &contract_id, &product_id, &actor, "cert-actor");
        assert_eq!(cert.issuer, actor);
    }

    #[test]
    fn test_get_certifications_empty_for_unknown_product() {
        let env = Env::default();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let unknown = String::from_str(&env, "no-product");
        assert_eq!(client.get_certifications(&unknown).len(), 0);
    }

    #[test]
    fn test_revoked_cert_still_in_get_certifications() {
        let (env, contract_id, owner, product_id) = setup();
        issue_cert(&env, &contract_id, &product_id, &owner, "cert-001");
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.revoke_certification(
            &product_id,
            &owner,
            &String::from_str(&env, "cert-001"),
        );
        let certs = client.get_certifications(&product_id);
        assert_eq!(certs.len(), 1);
        assert!(certs.get(0).unwrap().revoked);
    }

    // ── Property-based tests ─────────────────────────────────────────────────

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_count_equals_n_events(
            product_id_str in "[a-z]{1,20}",
            n in 0usize..=50,
        ) {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register(SupplyLinkContract, ());
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);
            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            for _ in 0..n {
                client.add_tracking_event(
                    &product_id,
                    &owner,
                    &String::from_str(&env, "Warehouse"),
                    &String::from_str(&env, "SHIPPING"),
                    &String::from_str(&env, "{}"),
                );
            }
            prop_assert_eq!(client.get_events_count(&product_id), n as u32);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_archive_reduces_active_count(
            product_id_str in "[a-z]{1,20}",
            n in 1usize..=20,
        ) {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register(SupplyLinkContract, ());
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);
            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            for _ in 0..n {
                client.add_tracking_event(
                    &product_id,
                    &owner,
                    &String::from_str(&env, "Warehouse"),
                    &String::from_str(&env, "SHIPPING"),
                    &String::from_str(&env, "{}"),
                );
            }
            client.archive_tracking_event(&product_id, &owner, &0u32);
            prop_assert_eq!(client.get_events_count(&product_id), (n - 1) as u32);
            prop_assert_eq!(client.list_archived_events(&product_id).len(), 1u32);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_exists_iff_registered(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register(SupplyLinkContract, ());
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);
            prop_assert!(!client.product_exists(&product_id));
            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            prop_assert!(client.product_exists(&product_id));
        }
    }

    // ── product_exists unit tests ────────────────────────────────────────────

    #[test]
    fn test_product_exists_returns_false_for_unknown() {
        let env = Env::default();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let id = String::from_str(&env, "does-not-exist");
        assert!(!client.product_exists(&id));
    }

    #[test]
    fn test_product_exists_returns_true_after_register() {
        let (env, contract_id, _owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert!(client.product_exists(&product_id));
    }

    // ── authorized-actor auth tests ──────────────────────────────────────────

    #[test]
    fn test_authorized_actor_can_add_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let actor = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-actor-test");
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory"),
            &owner,
        );
        client.add_authorized_actor(&product_id, &actor);
        let event = client.add_tracking_event(
            &product_id,
            &actor,
            &String::from_str(&env, "Warehouse"),
            &String::from_str(&env, "SHIPPING"),
            &String::from_str(&env, "{}"),
        );
        assert_eq!(event.actor, actor);
        assert_eq!(client.get_events_count(&product_id), 1);
    }

    #[test]
    #[should_panic(expected = "caller is not authorized")]
    fn test_unauthorized_caller_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let stranger = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-unauth-test");
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory"),
            &owner,
        );
        env.as_contract(&contract_id, || {
            SupplyLinkContract::add_tracking_event(
                env.clone(),
                product_id.clone(),
                stranger.clone(),
                String::from_str(&env, "Warehouse"),
                String::from_str(&env, "SHIPPING"),
                String::from_str(&env, "{}"),
            );
        });
    }

    // ── remove_authorized_actor tests ────────────────────────────────────────

    #[test]
    fn test_remove_authorized_actor_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let actor = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-remove-test");
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory"),
            &owner,
        );
        client.add_authorized_actor(&product_id, &actor);
        assert_eq!(client.get_product(&product_id).authorized_actors.len(), 1);
        let result = client.remove_authorized_actor(&product_id, &actor);
        assert!(result);
        assert_eq!(client.get_product(&product_id).authorized_actors.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_actor_returns_false() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let actor = soroban_sdk::Address::generate(&env);
        let non_existent = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-remove-fail");
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory"),
            &owner,
        );
        client.add_authorized_actor(&product_id, &actor);
        assert!(!client.remove_authorized_actor(&product_id, &non_existent));
        assert_eq!(client.get_product(&product_id).authorized_actors.len(), 1);
    }

    // ── product count / list tests ────────────────────────────────────────────

    #[test]
    fn test_product_count_initial_zero() {
        let env = Env::default();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert_eq!(client.get_product_count(), 0);
    }

    #[test]
    fn test_product_count_increments() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        assert_eq!(client.get_product_count(), 0);
        client.register_product(
            &String::from_str(&env, "prod-1"),
            &String::from_str(&env, "Widget 1"),
            &String::from_str(&env, "Factory A"),
            &owner,
        );
        assert_eq!(client.get_product_count(), 1);
        client.register_product(
            &String::from_str(&env, "prod-2"),
            &String::from_str(&env, "Widget 2"),
            &String::from_str(&env, "Factory B"),
            &owner,
        );
        assert_eq!(client.get_product_count(), 2);
    }

    #[test]
    fn test_list_products_returns_all() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let id1 = String::from_str(&env, "prod-1");
        let id2 = String::from_str(&env, "prod-2");
        let id3 = String::from_str(&env, "prod-3");
        client.register_product(&id1, &String::from_str(&env, "W1"), &String::from_str(&env, "FA"), &owner);
        client.register_product(&id2, &String::from_str(&env, "W2"), &String::from_str(&env, "FB"), &owner);
        client.register_product(&id3, &String::from_str(&env, "W3"), &String::from_str(&env, "FC"), &owner);
        let products = client.list_products(&0, &10);
        assert_eq!(products.len(), 3);
    }

    // ── update_product_metadata tests ────────────────────────────────────────

    #[test]
    fn test_update_product_metadata_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-update");
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory A"),
            &owner,
        );
        let updated = client.update_product_metadata(
            &product_id,
            &String::from_str(&env, "Updated Widget"),
            &String::from_str(&env, "Factory B"),
        );
        assert_eq!(updated.name, String::from_str(&env, "Updated Widget"));
        assert_eq!(updated.origin, String::from_str(&env, "Factory B"));
        assert_eq!(updated.id, product_id);
        assert_eq!(updated.owner, owner);
    }

    // ── Emergency stop tests ──────────────────────────────────────────────────

    fn setup_with_guardian() -> (Env, soroban_sdk::Address, soroban_sdk::Address, soroban_sdk::Address, String) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let guardian = soroban_sdk::Address::generate(&env);
        let owner = soroban_sdk::Address::generate(&env);
        let product_id = String::from_str(&env, "prod-pause");
        client.init_guardian(&guardian);
        client.register_product(
            &product_id,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory"),
            &owner,
        );
        (env, contract_id, guardian, owner, product_id)
    }

    /// Contract starts unpaused
    #[test]
    fn test_is_paused_default_false() {
        let env = Env::default();
        let contract_id = env.register(SupplyLinkContract, ());
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        assert!(!client.is_paused());
    }

    /// Guardian can pause and unpause
    #[test]
    fn test_guardian_can_pause_and_unpause() {
        let (env, contract_id, guardian, _owner, _product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.set_pause_state(&guardian, &true);
        assert!(client.is_paused());
        client.set_pause_state(&guardian, &false);
        assert!(!client.is_paused());
    }

    /// Non-guardian cannot pause
    #[test]
    #[should_panic(expected = "caller is not a guardian")]
    fn test_non_guardian_cannot_pause() {
        let (env, contract_id, _guardian, _owner, _product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let stranger = soroban_sdk::Address::generate(&env);
        client.set_pause_state(&stranger, &true);
    }

    /// Paused contract rejects register_product
    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_paused_blocks_register_product() {
        let (env, contract_id, guardian, _owner, _product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.set_pause_state(&guardian, &true);
        let new_owner = soroban_sdk::Address::generate(&env);
        client.register_product(
            &String::from_str(&env, "new-prod"),
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Factory"),
            &new_owner,
        );
    }

    /// Paused contract rejects add_tracking_event
    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_paused_blocks_add_tracking_event() {
        let (env, contract_id, guardian, owner, product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.set_pause_state(&guardian, &true);
        client.add_tracking_event(
            &product_id,
            &owner,
            &String::from_str(&env, "Warehouse"),
            &String::from_str(&env, "SHIPPING"),
            &String::from_str(&env, "{}"),
        );
    }

    /// Paused contract rejects transfer_ownership
    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_paused_blocks_transfer_ownership() {
        let (env, contract_id, guardian, _owner, product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.set_pause_state(&guardian, &true);
        let new_owner = soroban_sdk::Address::generate(&env);
        client.transfer_ownership(&product_id, &new_owner);
    }

    /// Read operations remain available while paused
    #[test]
    fn test_paused_allows_reads() {
        let (env, contract_id, guardian, _owner, product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.set_pause_state(&guardian, &true);
        // These must not panic
        let _ = client.get_product(&product_id);
        let _ = client.get_tracking_events(&product_id);
        let _ = client.is_paused();
        let _ = client.get_guardians();
        let _ = client.get_product_count();
    }

    /// Unpausing restores write access
    #[test]
    fn test_unpause_restores_writes() {
        let (env, contract_id, guardian, owner, product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.set_pause_state(&guardian, &true);
        client.set_pause_state(&guardian, &false);
        // Should succeed after unpause
        client.add_tracking_event(
            &product_id,
            &owner,
            &String::from_str(&env, "Warehouse"),
            &String::from_str(&env, "SHIPPING"),
            &String::from_str(&env, "{}"),
        );
        assert_eq!(client.get_events_count(&product_id), 1);
    }

    /// init_guardian cannot be called twice
    #[test]
    #[should_panic(expected = "guardians already initialised")]
    fn test_init_guardian_only_once() {
        let (env, contract_id, _guardian, _owner, _product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let second = soroban_sdk::Address::generate(&env);
        client.init_guardian(&second);
    }

    /// add_guardian works and new guardian can pause
    #[test]
    fn test_add_guardian_can_pause() {
        let (env, contract_id, guardian, _owner, _product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let second = soroban_sdk::Address::generate(&env);
        client.add_guardian(&guardian, &second);
        client.set_pause_state(&second, &true);
        assert!(client.is_paused());
    }

    /// Cannot remove the last guardian
    #[test]
    #[should_panic(expected = "cannot remove the last guardian")]
    fn test_cannot_remove_last_guardian() {
        let (env, contract_id, guardian, _owner, _product_id) = setup_with_guardian();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.remove_guardian(&guardian, &guardian);
    }
}
