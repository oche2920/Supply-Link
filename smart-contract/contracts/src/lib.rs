#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, String, Vec, Symbol};

// ── Schema version ────────────────────────────────────────────────────────────

/// Current schema version for Product and TrackingEvent structs (#392).
pub const SCHEMA_VERSION: u32 = 1;

// ── Error codes ───────────────────────────────────────────────────────────────

/// Typed contract errors for frontend mapping (#390).
#[contracterror]
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u32)]
pub enum ContractError {
    ProductNotFound        = 1,
    ProductAlreadyExists   = 2,
    UnauthorizedActor      = 3,
    OwnershipMismatch      = 4,
    InvalidEventPayload    = 5,
    ProductRecalled        = 6,
    SelfTransferNotAllowed = 7,
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Bytes, BytesN, Env, String, Vec, Symbol};

/// Current event schema version.
///
/// Bump this constant whenever the [`TrackingEvent`] payload layout changes in
/// a backward-incompatible way. Consumers should inspect the `schema_version`
/// field (and the matching topic slot) to select the correct parser.
///
/// | Version | Changes |
/// |---------|---------|
/// | 1       | Initial versioned schema. Adds `schema_version` field. |
/// | 2       | Adds `metadata_commitment` and `private_metadata` fields for privacy-preserving (off-chain encrypted) metadata. |
pub const EVENT_SCHEMA_VERSION: u32 = 2;

mod tests;
mod resilience_tests;
mod compliance_tests;
mod document_hash_tests;

// ── Payload size limits (issue #311) ─────────────────────────────────────────
// All limits are in bytes (Soroban String::len() returns byte count).
// | Field    | Max bytes | Notes                          |
// |----------|-----------|--------------------------------|
// | id       |       128 | Storage key; keep short        |
// | name     |       256 | Human-readable label           |
// | origin   |       256 | Geographic/org string          |
// | location |       256 | Per-event location             |
// | metadata |      4096 | JSON payload                   |
const MAX_ID_LEN:       u32 = 128;
const MAX_NAME_LEN:     u32 = 256;
const MAX_ORIGIN_LEN:   u32 = 256;
const MAX_LOCATION_LEN: u32 = 256;
const MAX_METADATA_LEN: u32 = 4096;
// Privacy commitment (issue #409): a hex-encoded hash of the off-chain encrypted
// payload. A SHA-256 hex digest is 64 chars; allow headroom for other digests.
const MAX_COMMITMENT_LEN: u32 = 128;

// ── Event expiration policy (issue #314) ──────────────────────────────────────
/// Pending events expire after this many seconds (7 days).
const EXPIRATION_WINDOW: u64 = 604_800;  // 7 * 24 * 60 * 60 seconds

fn assert_len(s: &String, max: u32, field: &'static str) {
    if s.len() > max { panic!("{} exceeds max length", field); }
}

// ── Error types ──────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    ProductNotFound = 1,
    NotAuthorized = 2,
    ApproverNotAuthorized = 3,
    NoPendingEvents = 4,
    OwnerOnly = 5,
    PendingEventExpired = 6,
    InvalidNonce = 7,
    ComplianceViolation = 8,
}

// ── Compliance rule types ─────────────────────────────────────────────────────
pub const COMPLIANCE_REQUIRED_ORDER: u32 = 0;
pub const COMPLIANCE_MANDATORY_INSPECTION: u32 = 1;
pub const COMPLIANCE_MAX_TIME_BETWEEN_STAGES: u32 = 2;

/// A single compliance rule constraining event sequencing for a product.
#[contracttype]
#[derive(Clone)]
pub struct ComplianceRule {
    /// Rule kind: 0=RequiredOrder, 1=MandatoryInspection, 2=MaxTimeBetweenStages.
    pub rule_type: u32,
    /// Preceding stage that must have occurred (for types 0 and 1).
    pub from_stage: String,
    /// Stage this rule guards (the event type being submitted).
    pub to_stage: String,
    /// Max seconds allowed between from_stage and to_stage (for type 2).
    pub max_seconds: u64,
}

/// Per-product compliance policy: a collection of rules enforced on every event.
#[contracttype]
#[derive(Clone)]
pub struct CompliancePolicy {
    pub product_id: String,
    pub rules: Vec<ComplianceRule>,
}

// ── Data models ──────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct Product {
    /// Caller-supplied unique identifier for this product (e.g. `"batch-2024-001"`).
    /// Must be unique across all registered products; duplicate IDs are rejected
    /// with `"product already exists"` and leave existing state unchanged.
    pub id: String,
    pub name: String,
    pub origin: String,
    pub owner: Address,
    pub timestamp: u64,
    pub authorized_actors: Vec<Address>,
    /// Whether this product has been recalled (#393).
    pub recalled: bool,
    /// Reason provided when the product was recalled (#393).
    pub recall_reason: String,
    /// Ledger timestamp when the product was recalled; 0 if never recalled (#393).
    pub recall_timestamp: u64,
    /// Schema version of this record (#392).
    pub schema_version: u32,
    /// Hazard status (#454)
    pub hazardous: bool,
    pub hazard_classification: String,
}

#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
    /// Current lifecycle stage (#404)
    pub lifecycle_stage: LifecycleStage,
    /// Whether the product is active. Deactivated products reject new events.
    /// Number of signatures required to approve events for this product.
    /// If 0 or 1, events are recorded immediately. If > 1, events are staged
    /// as pending until the required number of approvals are received.
    pub required_signatures: u32,
    /// Lifecycle state of the product. `true` indicates the product is active
    /// and can receive tracking events. `false` indicates the product has been
    /// deactivated and is read-only. Defaults to `true` on registration.
    pub active: bool,
    /// Taxonomy category ID for this product (e.g. `"agricultural"`). (#425)
    /// Must be a recognised category from the controlled vocabulary.
    pub category: String,
    /// Taxonomy subcategory ID within `category` (e.g. `"coffee"`). (#425)
    pub subcategory: String,
}

/// A product certification issued by an authorised actor. (#428)
///
/// Certifications are stored as a `Vec<ProductCertification>` under
/// [`DataKey::Certifications`] keyed by `product_id`. Each entry carries
/// the issuer address, a string certification type from the controlled
/// vocabulary (e.g. `"fair_trade"`, `"organic"`), and a revocation flag.
#[contracttype]
#[derive(Clone)]
pub struct ProductCertification {
    /// Stable unique identifier for this certification entry.
    pub id: String,
    /// ID of the product this certification belongs to.
    pub product_id: String,
    /// Certification type key (e.g. `"fair_trade"`, `"organic"`, `"iso_9001"`).
    pub cert_type: String,
    /// Stellar address of the actor who issued this certification.
    /// Must be the product owner or an authorized actor at issuance time.
    pub issuer: Address,
    /// Ledger timestamp when the certification was issued.
    pub issued_at: u64,
    /// `true` if this certification has been revoked; `false` otherwise.
    pub revoked: bool,
    /// Ledger timestamp when the certification was revoked (0 if not revoked).
    pub revoked_at: u64,
}

/// A single supply-chain event recorded against a [`Product`].
///
/// Events are append-only. Once written they cannot be modified or deleted,
/// providing an immutable audit trail. All events for a product are stored
/// together under [`DataKey::Events`].
///
/// # Schema versioning
/// The `schema_version` field carries [`EVENT_SCHEMA_VERSION`] at write time.
/// Indexers and backend services must read this field first and dispatch to the
/// appropriate parser before accessing any other fields. The version is also
/// encoded as the **fourth topic slot** (index 3) in every emitted event so
/// consumers can filter by version without deserialising the payload.
/// Topic layout: `(event_name, product_id, event_type, schema_version)`.
///
/// # Storage
/// Stored as a `Vec<TrackingEvent>` under [`DataKey::Events`] keyed by
/// `product_id`. Storage type is `persistent`.
#[contracttype]
#[derive(Clone)]
pub struct TrackingEvent {
    /// Schema version of this event payload. Always set to
    /// [`EVENT_SCHEMA_VERSION`] at write time. Consumers must check this field
    /// before parsing any other fields.
    pub schema_version: u32,
    /// ID of the [`Product`] this event belongs to.
    pub product_id: String,
    pub location: String,
    pub actor: Address,
    pub timestamp: u64,
    pub event_type: String,
    pub metadata: String,
    /// Schema version of this record (#392).
    pub schema_version: u32,
    /// Arbitrary JSON string carrying stage-specific metadata
    /// (e.g. `{"temperature":"4°C","humidity":"60%"}`). The contract stores
    /// this opaquely; consumers are responsible for parsing it.
    ///
    /// For privacy-preserving events (see `private_metadata`) this field is an
    /// **empty string**: the plaintext is never written on-chain. The encrypted
    /// payload lives off-chain and only its hash is recorded in
    /// `metadata_commitment`.
    pub metadata: String,
    /// Stable deterministic event ID — a hex-encoded SHA-256 hash of the
    /// canonical fields: `product_id|actor|event_type|timestamp|metadata`.
    /// Invariant across contract upgrades; suitable for deep links and QR payloads.
    pub stable_id: String,
}

// ── Role types (#387) ─────────────────────────────────────────────────────────

/// Named role for an authorized actor.
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum Role {
    /// Can harvest/originate events.
    Producer,
    /// Can add processing events.
    Processor,
    /// Can add shipping events.
    Shipper,
    /// Can add retail events.
    Retailer,
    /// Can add any event type.
    Any,
}

/// Binds an actor address to a named role.
#[contracttype]
#[derive(Clone)]
pub struct ActorRole {
    pub actor: Address,
    pub role: Role,
}

/// Authorization policy for a product.
#[contracttype]
#[derive(Clone)]
pub struct AuthPolicy {
    /// Minimum number of distinct authorized actors that must sign an event
    /// for high-risk event types. 1 = single-signer (default).
    pub threshold: u32,
    /// Role assignments for this product's authorized actors.
    pub roles: Vec<ActorRole>,
}

/// A pending event awaiting multi-signature approval.
///
/// For high-value products, events are staged until the required number of
/// authorized actors have approved them.
///
/// Each pending event has a stable identifier (`pending_event_id`) that remains
/// unchanged even if other pending events in the queue are removed or approved.
/// This prevents client mistakes from index-based references that shift after
/// queue mutations.
#[contracttype]
#[derive(Clone)]
pub struct PendingEvent {
    /// Stable unique identifier for this pending event within its product.
    /// Generated at creation time and immutable. Used for deterministic targeting
    /// in approve/reject operations to avoid index-based race conditions.
    pub pending_event_id: u64,
    /// ID of the product this event is for.
    pub product_id: String,
    /// The event data awaiting approval.
    pub event: TrackingEvent,
    /// Addresses that have approved this event.
    pub approvals: Vec<Address>,
    /// Number of approvals required before the event is finalized.
    pub required_signatures: u32,
    /// Timestamp when the pending event was created.
    pub created_at: u64,
    /// Timestamp when this pending event expires (issue #314).
    pub expiration: u64,
}

/// Event rejection data with optional reason context.
///
/// Emitted when a pending event is rejected, providing audit trail
/// and optional explanation for the rejection decision.
#[contracttype]
#[derive(Clone)]
pub struct EventRejection {
    /// The product ID the rejected event was for.
    pub product_id: String,
    /// The rejected event data.
    pub event: TrackingEvent,
    /// Address of the actor who rejected the event.
    pub rejector: Address,
    /// Optional reason for rejection (max 256 characters).
    pub reason: String,
    /// Timestamp of the rejection.
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

/// An approval hop in the chain-of-custody for a product. (#499)
/// Records each actor's approval as the product moves through the supply chain.
#[contracttype]
#[derive(Clone)]
pub struct ApprovalHop {
    /// ID of the product being approved.
    pub product_id: String,
    /// Address of the actor approving the product.
    pub approver: Address,
    /// Type of approval (e.g., "RECEIVED", "INSPECTED", "SHIPPED").
    pub approval_type: String,
    /// Timestamp when the approval was recorded.
    pub timestamp: u64,
    /// Optional JSON metadata about the approval.
    pub metadata: String,
}

/// An origin attestation for a product. (#500)
/// Proves where goods came from with cryptographic proof.
#[contracttype]
#[derive(Clone)]
pub struct OriginAttestation {
    /// ID of the product being attested.
    pub product_id: String,
    /// Address of the actor attesting to the origin.
    pub attester: Address,
    /// Description of the origin (e.g., "Ethiopian highlands, Yirgacheffe region").
    pub origin_claim: String,
    /// Cryptographic hash of supporting documentation.
    pub proof_hash: String,
    /// Timestamp when the attestation was recorded.
    pub timestamp: u64,
    /// Whether this attestation has been verified.
    pub verified: bool,
}

/// An off-chain document anchored on-chain by its SHA-256 hash. (#460)
///
/// Callers compute the SHA-256 hash of the document bytes off-chain and submit
/// it here. The contract stores the hash alongside a human-readable label and
/// the anchoring actor's address. Anyone can later call `verify_document_hash`
/// to check whether a given hash matches the stored anchor.
#[contracttype]
#[derive(Clone)]
pub struct DocumentAnchor {
    /// Product this document belongs to.
    pub product_id: String,
    /// Human-readable label for the document (e.g. `"Certificate of Origin"`).
    pub label: String,
    /// Hex-encoded SHA-256 hash of the document bytes (64 chars).
    pub hash: String,
    /// Stellar address of the actor who anchored the document.
    pub anchored_by: Address,
    /// Ledger timestamp when the anchor was recorded.
    pub anchored_at: u64,
}

// ── Issue #503: Event timestamp certification ────────────────────────────────

/// Timestamp certification for an event, issued by an authorized certifier.
/// Provides cryptographic proof that an event occurred at a specific time.
#[contracttype]
#[derive(Clone)]
pub struct EventTimestampCert {
    /// Unique identifier for this certification.
    pub id: String,
    /// Product ID this certification belongs to.
    pub product_id: String,
    /// Stable ID of the event being certified.
    pub event_stable_id: String,
    /// Certified timestamp (ledger timestamp when certified).
    pub certified_timestamp: u64,
    /// Address of the certifier (authorized entity).
    pub certifier: Address,
    /// Timestamp when the certification was issued.
    pub issued_at: u64,
    /// Whether this certification has been revoked.
    pub revoked: bool,
}

// ── Issue #504: Product provenance proof notarization ────────────────────────

/// Notarized proof of product provenance, issued by authorized notaries.
/// Provides legal/regulatory proof of product history.
#[contracttype]
#[derive(Clone)]
pub struct ProvenanceNotarization {
    /// Unique identifier for this notarization.
    pub id: String,
    /// Product ID this notarization covers.
    pub product_id: String,
    /// Hash of the provenance proof document (off-chain).
    pub proof_hash: String,
    /// Address of the notary who issued this.
    pub notary: Address,
    /// Timestamp when notarization was issued.
    pub notarized_at: u64,
    /// Expiration timestamp (0 = never expires).
    pub expires_at: u64,
    /// Whether this notarization has been revoked.
    pub revoked: bool,
}

// ── Issue #505: Supply chain event certification workflows ──────────────────

/// Certifier registration and metadata.
#[contracttype]
#[derive(Clone)]
pub struct Certifier {
    /// Unique identifier for this certifier.
    pub id: String,
    /// Stellar address of the certifier.
    pub address: Address,
    /// Human-readable name of the certifier.
    pub name: String,
    /// Certification types this certifier is authorized for (e.g., "organic", "fair_trade").
    pub cert_types: Vec<String>,
    /// Timestamp when the certifier was registered.
    pub registered_at: u64,
    /// Whether this certifier is active.
    pub active: bool,
}

/// Certification of a supply chain event by an authorized certifier.
#[contracttype]
#[derive(Clone)]
pub struct EventCertification {
    /// Unique identifier for this event certification.
    pub id: String,
    /// Product ID this certification belongs to.
    pub product_id: String,
    /// Stable ID of the event being certified.
    pub event_stable_id: String,
    /// Type of certification (e.g., "quality_check", "compliance_verified").
    pub cert_type: String,
    /// Address of the certifier.
    pub certifier: Address,
    /// Certification metadata (JSON string).
    pub metadata: String,
    /// Timestamp when the certification was issued.
    pub issued_at: u64,
    /// Whether this certification has been revoked.
    pub revoked: bool,
    /// Revocation timestamp (0 if not revoked).
    pub revoked_at: u64,
}

// ── Issue #506: AI-assisted anomaly review assistant ────────────────────────

/// Anomaly detection result for a product's supply chain.
#[contracttype]
#[derive(Clone)]
pub struct AnomalyReport {
    /// Unique identifier for this report.
    pub id: String,
    /// Product ID this report is for.
    pub product_id: String,
    /// Anomaly type (e.g., "timing_gap", "temperature_deviation", "actor_mismatch").
    pub anomaly_type: String,
    /// Severity level: 1=low, 2=medium, 3=high, 4=critical.
    pub severity: u32,
    /// Human-readable description of the anomaly.
    pub description: String,
    /// Suggested remediation actions (JSON string).
    pub suggested_actions: String,
    /// Timestamp when the anomaly was detected.
    pub detected_at: u64,
    /// Whether this anomaly has been reviewed.
    pub reviewed: bool,
    /// Address of the analyst who reviewed it (if reviewed).
    pub reviewed_by: Address,
    /// Timestamp of review (0 if not reviewed).
    pub reviewed_at: u64,
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
    /// Key for pending events awaiting multi-signature approval.
    /// The inner `String` is the product ID.
    PendingEvents(String),
    /// Key for the next stable pending event ID counter.
    /// The inner `String` is the product ID.
    /// Stores a `u64` used to generate unique identifiers for pending events.
    NextPendingId(String),
    /// Key for the global product registration counter.
    ProductCount,
    ProductIndex(u64),
    /// Recall history for a product: Vec<String> of recall reasons (#393).
    RecallHistory(String),
    /// Key for the authorization policy (roles + threshold) of a product.
    AuthPolicy(String),
    /// Key for actor nonce tracking. The inner `Address` is the actor address.
    ActorNonce(Address),
    /// Key for the compliance policy of a product. The inner `String` is the product ID.
    CompliancePolicy(String),
    /// Key for approval hops in the chain-of-custody. The inner `String` is the product ID. (#499)
    ApprovalHops(String),
    /// Key for origin attestations. The inner `String` is the product ID. (#500)
    OriginAttestations(String),
    /// Key for document anchors for a product. The inner `String` is the product ID. (#460)
    DocumentAnchors(String),
    /// Key for event timestamp certifications for a product. (#503)
    EventTimestampCerts(String),
    /// Key for provenance notarizations for a product. (#504)
    ProvenanceNotarizations(String),
    /// Key for registered certifiers. The inner `String` is the certifier ID.
    Certifier(String),
    /// Key for all certifier IDs (Vec<String>).
    CertifierIndex,
    /// Key for event certifications for a product. (#505)
    EventCertifications(String),
    /// Key for anomaly reports for a product. (#506)
    AnomalyReports(String),
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct SupplyLinkContract;

#[contractimpl]
impl SupplyLinkContract {
    // ── Registration ─────────────────────────────────────────────────────────

    // ── Product registration ──────────────────────────────────────────────────

    /// Register a new product on-chain.
    ///
    /// Creates a [`Product`] entry in persistent storage and initialises the
    /// global product counter and index mapping used by
    /// [`Self::list_products`].
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment (injected by the runtime).
    /// - `id` — Caller-supplied unique product identifier. Must not already
    ///   exist; duplicate IDs are rejected with `"product already exists"`.
    /// - `name` — Human-readable product name.
    /// - `origin` — Geographic or organisational origin of the product.
    /// - `owner` — Stellar address that will own the product. This address
    ///   must sign the transaction.
    /// - `required_signatures` — Number of approvals required for events (0 or 1 = immediate, >1 = multi-sig).
    ///
    /// # Returns
    /// The newly created [`Product`] struct.
    ///
    /// # Authorization
    /// Requires `owner.require_auth()`. The transaction must be signed by
    /// `owner`.
    ///
    /// # Warning
    /// If a product with `id` already exists it will be **silently overwritten**
    /// with the new `name`, `origin`, `owner`, and `required_signatures`. The
    /// previous product's data is lost. Additionally, the global
    /// `ProductCount` and `ProductIndex` are incremented unconditionally, so a
    /// duplicate registration creates a ghost index entry pointing to the same
    /// `id`. Callers should use [`Self::product_exists`] to guard against
    /// accidental overwrites.
    ///
    /// # Panics
    /// - `"product already exists"` — if a product with `id` is already registered.
    ///   `product_count` and index mappings are NOT modified on rejection.
    ///
    /// # Emitted Events
    /// Publishes a `("product_registered", id)` event with the [`Product`]
    /// struct as the event body.
    pub fn register_product(
        env: Env,
        id: String,
        name: String,
        origin: String,
        owner: Address,
        required_signatures: u32,
        category: String,
        subcategory: String,
    ) -> Product {
        // Duplicate guard — must come before auth to avoid leaking state on
        // duplicate attempts and to keep counter/index consistent.
        if env.storage().persistent().has(&DataKey::Product(id.clone())) {
            panic!("product already exists");
        }

        owner.require_auth();
        // Issue #311: enforce size limits.
        assert_len(&id,          MAX_ID_LEN,     "id");
        assert_len(&name,        MAX_NAME_LEN,   "name");
        assert_len(&origin,      MAX_ORIGIN_LEN, "origin");
        assert_len(&category,    64,             "category");
        assert_len(&subcategory, 64,             "subcategory");
        let product = Product {
            id: id.clone(),
            name,
            origin,
            owner,
            timestamp: env.ledger().timestamp(),
            authorized_actors: Vec::new(&env),
            recalled: false,
            recall_reason: String::from_str(&env, ""),
            recall_timestamp: 0,
            schema_version: SCHEMA_VERSION,
            expiration_timestamp: 0,
            spoiled: false,
            required_signatures,
            active: true,
            category,
            subcategory,
            hazardous: false,
            hazard_classification: String::from_str(&env, ""),
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

    /// Batch-register up to 10 products in a single transaction (#389).
    pub fn register_products_batch(
        env: Env,
        owner: Address,
        ids: Vec<String>,
        names: Vec<String>,
        origins: Vec<String>,
    ) -> Vec<Product> {
        owner.require_auth();
        if ids.len() > 10 {
            panic!("batch size exceeds maximum of 10");
        }
        if ids.len() != names.len() || ids.len() != origins.len() {
            panic!("ids, names, and origins must have equal length");
        }
        let mut products = Vec::new(&env);
        let mut count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            let name = names.get(i).unwrap();
            let origin = origins.get(i).unwrap();
            let product = Product {
                id: id.clone(),
                name,
                origin,
                owner: owner.clone(),
                timestamp: env.ledger().timestamp(),
                authorized_actors: Vec::new(&env),
                recalled: false,
                recall_reason: String::from_str(&env, ""),
                recall_timestamp: 0,
                schema_version: SCHEMA_VERSION,
                hazardous: false,
                hazard_classification: String::from_str(&env, ""),
            };
            env.storage()
                .persistent()
                .set(&DataKey::Product(id.clone()), &product);
            env.storage()
                .persistent()
                .set(&DataKey::ProductIndex(count), &id);
            count += 1;
            products.push_back(product);
        }
        env.storage()
            .persistent()
            .set(&DataKey::ProductCount, &count);
        products
    }

    // ── Tracking events ───────────────────────────────────────────────────────

    /// Add a tracking event for a product. Enforces lifecycle stage transitions (#404).
    /// Add a tracking event for a product.
    ///
    /// Appends a new [`TrackingEvent`] to the product's event log. The event
    /// log is stored as a `Vec<TrackingEvent>` and grows with each call.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to record the event against.
    /// - `caller` — Address of the supply-chain participant submitting the
    ///   event. Must be the product owner or an address in
    ///   `authorized_actors`.
    /// - `location` — Free-form location string (e.g. `"Port of Hamburg"`).
    /// - `event_type` — Canonical supply-chain stage. Must be one of:
    ///   `"HARVEST"`, `"PROCESSING"`, `"SHIPPING"`, `"RETAIL"`.
    ///   Unknown values are rejected with `"invalid event_type"` (issue #310).
    /// - `metadata` — Arbitrary JSON string with stage-specific data.
    ///
    /// # Returns
    /// The newly created [`TrackingEvent`] struct.
    ///
    /// # Authorization
    /// Requires `caller.require_auth()`. The authorization check is performed
    /// *after* verifying that `caller` is the owner or an authorized actor, so
    /// unauthorized addresses are rejected before any auth overhead is incurred.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"caller is not authorized"` — if `caller` is neither the product
    ///   owner nor in `authorized_actors`.
    ///
    /// # Emitted Events
    /// - When `product.required_signatures <= 1`: publishes an
    ///   `("event_added", product_id, event_type, schema_version)` event with
    ///   the [`TrackingEvent`] struct as the event body.
    /// - When `product.required_signatures > 1`: the event is staged as
    ///   pending and an `("event_pending", product_id, event_type,
    ///   schema_version)` event is published instead. The event is not added
    ///   to the finalized log until [`Self::approve_event`] collects enough
    ///   approvals.
    pub fn add_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata: String,
    ) -> TrackingEvent {
        let mut product: Product = env
    ) -> Result<TrackingEvent, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        if !product.active {
            panic!("product is deactivated");
        }

        // Reject events on recalled products (#393)
        if product.recalled {
            panic!("product is recalled");
        }

        let caller = product.owner.clone();
        // Verify caller is owner or an authorized actor before requiring auth
        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            return Err(Error::NotAuthorized);
        }
        caller.require_auth();
        // Issue #311: enforce size limits.
        assert_len(&location, MAX_LOCATION_LEN, "location");
        assert_len(&metadata, MAX_METADATA_LEN, "metadata");

        let timestamp = env.ledger().timestamp();

        // Compute stable_id: SHA-256 of "product_id|event_type|timestamp" encoded as bytes
        let stable_id = compute_stable_id(&env, &product_id, &caller, &event_type, timestamp, &metadata);

        // Enforce lifecycle transition (#404)
        if !validate_lifecycle_transition(&env, &product.lifecycle_stage, &event_type) {
            panic!("invalid lifecycle transition");
        }

        // Advance lifecycle stage if this event triggers a transition
        if let Some(next_stage) = event_type_to_stage(&env, &event_type) {
            product.lifecycle_stage = next_stage;
            env.storage()
                .persistent()
                .set(&DataKey::Product(product_id.clone()), &product);
        }

        let event = TrackingEvent {
            schema_version: EVENT_SCHEMA_VERSION,
            product_id: product_id.clone(),
            location,
o            actor: caller,
            timestamp,
            actor: caller.clone(),
            timestamp: env.ledger().timestamp(),
            event_type: event_type.clone(),
            metadata,
            schema_version: SCHEMA_VERSION,
            stable_id,
        };

        Self::record_event(&env, &product, event.clone());

        Ok(event)
    }

    /// Add a tracking event whose metadata is private (issue #409).
    ///
    /// Identical to [`Self::add_tracking_event`] except the plaintext metadata is
    /// **never** written on-chain. The caller encrypts the sensitive metadata
    /// off-chain, stores the ciphertext off-chain, and submits only a
    /// `metadata_commitment` — a hex-encoded hash of that ciphertext. The stored
    /// event has `private_metadata = true` and an empty `metadata` field.
    ///
    /// This preserves provable provenance (anyone can hash the off-chain payload
    /// and compare it against the on-chain commitment) while keeping the contents
    /// confidential: the commitment is a one-way hash, so the plaintext cannot be
    /// recovered from on-chain data alone.
    ///
    /// # Parameters
    /// - `metadata_commitment` — Hex-encoded hash of the off-chain encrypted
    ///   payload. Must be non-empty and at most `MAX_COMMITMENT_LEN` bytes.
    ///
    /// # Authorization
    /// Identical to [`Self::add_tracking_event`].
    ///
    /// # Panics
    /// - `"commitment required for private metadata"` — if `metadata_commitment` is empty.
    /// - `"metadata_commitment exceeds max length"` — if the commitment is too long.
    ///
    /// # Errors
    /// - [`Error::ProductNotFound`] — if `product_id` is not registered.
    /// - [`Error::NotAuthorized`] — if `caller` is neither owner nor authorized actor.
    pub fn add_private_tracking_event(
        env: Env,
        product_id: String,
        caller: Address,
        location: String,
        event_type: String,
        metadata_commitment: String,
    ) -> Result<TrackingEvent, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            return Err(Error::NotAuthorized);
        }
        caller.require_auth();

        env.events().publish(
            (Symbol::new(&env, "event_added"), product_id, event_type),
            event.clone(),
        );
        assert_len(&location, MAX_LOCATION_LEN, "location");
        if metadata_commitment.len() == 0 {
            panic!("commitment required for private metadata");
        }
        assert_len(&metadata_commitment, MAX_COMMITMENT_LEN, "metadata_commitment");

        let event = TrackingEvent {
            schema_version: EVENT_SCHEMA_VERSION,
            product_id: product_id.clone(),
            location,
            actor: caller.clone(),
            timestamp: env.ledger().timestamp(),
            event_type,
            // Plaintext is NEVER stored on-chain for private events.
            metadata: String::from_str(&env, ""),
            metadata_commitment,
            private_metadata: true,
        };

        Self::record_event(&env, &product, event.clone());

        Ok(event)
    }

    /// Append a finalized event, or stage it for multi-signature approval, then
    /// emit the matching event. Shared by [`Self::add_tracking_event`] and
    /// [`Self::add_private_tracking_event`].
    fn record_event(env: &Env, product: &Product, event: TrackingEvent) {
        let product_id = event.product_id.clone();
        let event_type = event.event_type.clone();

        if product.required_signatures > 1 {
            // Stage event as pending with a stable ID
            let mut pending: Vec<PendingEvent> = env
                .storage()
                .persistent()
                .get(&DataKey::PendingEvents(product_id.clone()))
                .unwrap_or_else(|| Vec::new(env));

            let next_id: u64 = env
                .storage()
                .persistent()
                .get(&DataKey::NextPendingId(product_id.clone()))
                .unwrap_or(0u64);

            let mut approvals = Vec::new(env);
            approvals.push_back(event.actor.clone());

            let pending_event = PendingEvent {
                pending_event_id: next_id,
                product_id: product_id.clone(),
                event: event.clone(),
                approvals,
                required_signatures: product.required_signatures,
                created_at: env.ledger().timestamp(),
                expiration: env.ledger().timestamp() + EXPIRATION_WINDOW,
            };

            pending.push_back(pending_event);
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id.clone()), &pending);

            env.storage()
                .persistent()
                .set(&DataKey::NextPendingId(product_id.clone()), &(next_id + 1));

            env.events().publish(
                (Symbol::new(env, "event_pending"), product_id, event_type, EVENT_SCHEMA_VERSION),
                event,
            );
        } else {
            let mut events: Vec<TrackingEvent> = env
                .storage()
                .persistent()
                .get(&DataKey::Events(product_id.clone()))
                .unwrap_or_else(|| Vec::new(env));

            events.push_back(event.clone());
            env.storage()
                .persistent()
                .set(&DataKey::Events(product_id.clone()), &events);

            env.events().publish(
                (Symbol::new(env, "event_added"), product_id, event_type, EVENT_SCHEMA_VERSION),
                event,
            );
        }
    }

    // ── Recall management (#393) ──────────────────────────────────────────────

    /// Recall a product. Owner-only. Sets recalled=true and records the reason.
    pub fn recall_product(env: Env, product_id: String, reason: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        product.recalled = true;
        product.recall_reason = reason.clone();
        product.recall_timestamp = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Append to recall history
        let mut history: Vec<String> = env
            .storage()
            .persistent()
            .get(&DataKey::RecallHistory(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        history.push_back(reason);
        env.storage()
            .persistent()
            .set(&DataKey::RecallHistory(product_id.clone()), &history);

        env.events().publish(
            (Symbol::new(&env, "product_recalled"), product_id),
            product.recalled,
        );

        true
    }

    /// Set hazard classification for a product (#454)
    pub fn set_hazard_status(env: Env, product_id: String, hazardous: bool, classification: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        product.hazardous = hazardous;
        product.hazard_classification = classification;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);

        true
    }

    /// Lift a recall from a product. Owner-only. Sets recalled=false.
    pub fn unrecall_product(env: Env, product_id: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        product.recalled = false;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "product_unrecalled"), product_id),
            product.recalled,
        );

        true
    }

    /// Return recall information for a product: (recalled, reason, timestamp).
    pub fn get_recall_info(env: Env, product_id: String) -> (bool, String, u64) {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id))
            .expect("product not found");
        (product.recalled, product.recall_reason, product.recall_timestamp)
    }

    /// Return the full recall history (all reasons) for a product (#393).
    pub fn get_recall_history(env: Env, product_id: String) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&DataKey::RecallHistory(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ── Read-only queries ─────────────────────────────────────────────────────

    pub fn get_product(env: Env, id: String) -> Product {
    /// Retrieve a product by its ID.
    ///
    /// # Returns
    /// The [`Product`] struct stored under `id`.
    ///
    /// # Errors
    /// - [`Error::ProductNotFound`] — if no product with `id` is registered.
    pub fn get_product(env: Env, id: String) -> Result<Product, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Product(id))
            .ok_or(Error::ProductNotFound)
    }

    pub fn get_tracking_events(env: Env, product_id: String) -> Vec<TrackingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Transfer product ownership.
    /// Panics if the product is spoiled — spoiled products cannot be transferred.
    pub fn transfer_ownership(env: Env, product_id: String, new_owner: Address) -> bool {
    /// Check whether a product ID is registered.
    ///
    /// Useful for pre-flight checks before calling functions that panic on
    /// unknown IDs.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `id` — The product ID to check.
    ///
    /// # Returns
    /// `true` if a product with `id` exists in storage, `false` otherwise.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn product_exists(env: Env, id: String) -> bool {
        env.storage().persistent().has(&DataKey::Product(id))
    }

    /// Return the number of tracking events recorded for a product.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — The product ID to query.
    ///
    /// # Returns
    /// The number of events as a `u32`. Returns `0` if the product has no
    /// events or does not exist.
    ///
    /// # Note
    /// This function deserialises the full `Vec<TrackingEvent>` from storage
    /// to read its length. It has the same storage cost as
    /// `get_tracking_events(product_id).len()` and is not a cheaper
    /// alternative for large event logs.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_events_count(env: Env, product_id: String) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<TrackingEvent>>(&DataKey::Events(product_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn get_authorized_actors(env: Env, product_id: String) -> Vec<Address> {
        env.storage()
            .persistent()
            .get::<DataKey, Product>(&DataKey::Product(product_id))
            .map(|p| p.authorized_actors)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_product_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0)
    }

    pub fn list_products(env: Env, offset: u64, limit: u64) -> Vec<String> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);

        let mut products = Vec::new(&env);
        let end = core::cmp::min(offset + limit, count);

        for i in offset..end {
            if let Some(product_id) = env
                .storage()
                .persistent()
                .get::<DataKey, String>(&DataKey::ProductIndex(i))
            {
                products.push_back(product_id);
            }
        }

        products
    }

    // ── Ownership & actor management ──────────────────────────────────────────

    pub fn transfer_ownership(env: Env, product_id: String, new_owner: Address) -> bool {
    /// Transfer product ownership to a new address.
    ///
    /// Updates the `owner` field of the [`Product`] in storage. The previous
    /// owner loses all owner-gated privileges immediately. The new owner gains
    /// them immediately.
    ///
    /// # Safety Checks
    /// - Prevents no-op transfers (transferring to the current owner)
    /// - Validates that the new owner is a valid address
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to transfer.
    /// - `new_owner` — Stellar address of the incoming owner.
    ///
    /// # Returns
    /// `true` on success.
    ///
    /// # Authorization
    /// Requires the *current* `product.owner.require_auth()`. The transaction
    /// must be signed by the current owner.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"cannot transfer to current owner"` — if `new_owner` equals current owner.
    ///
    /// # Emitted Events
    /// Publishes an `("ownership_transferred", product_id)` event with
    /// `new_owner` as the event body.
    pub fn transfer_ownership(
        env: Env,
        product_id: String,
        new_owner: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        if product.spoiled {
            panic!("spoiled product cannot be transferred");
        }

        product.owner.require_auth();

        if product.owner == new_owner {
            panic!("new owner must differ from current owner");
        }

        Self::validate_and_increment_nonce(&env, &product.owner, nonce);
        
        product.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "ownership_transferred"), product_id),
            new_owner,
        );

        true
        Ok(true)
    }

    pub fn add_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
    /// Grant an address permission to add tracking events for a product.
    ///
    /// Appends `actor` to `product.authorized_actors`. Prevents duplicate entries
    /// to maintain clean governance state.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `actor` — Stellar address to authorise.
    ///
    /// # Returns
    /// `true` if `actor` was added, `false` if `actor` was already in the list.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may grant actor permissions.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"actor already authorized"` — if the actor is already in the authorized list.
    ///
    /// # Emitted Events
    /// Publishes an `("actor_authorized", product_id)` event with `actor` as
    /// the event body.
    pub fn add_authorized_actor(
        env: Env,
        product_id: String,
        actor: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        Self::validate_and_increment_nonce(&env, &product.owner, nonce);
        
        product.authorized_actors.push_back(actor.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "actor_authorized"), product_id),
            actor,
        );

        Ok(true)
    }

    pub fn remove_authorized_actor(env: Env, product_id: String, actor: Address) -> bool {
        let mut product: Product = env
    // ── #404: Lifecycle helpers ───────────────────────────────────────────────

    /// Get the current lifecycle stage of a product.
    pub fn get_lifecycle_stage(env: Env, product_id: String) -> LifecycleStage {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id))
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

    pub fn update_product_metadata(
        product.lifecycle_stage
    }

    // ── #396: Ownership transfer escrow ──────────────────────────────────────

    /// Request an ownership transfer. Creates an escrow pending acceptance.
    pub fn request_transfer_ownership(
        env: Env,
        product_id: String,
        proposed_owner: Address,
    ) -> TransferEscrow {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if product.owner == proposed_owner {
            panic!("proposed owner must differ from current owner");
        }
        product.owner.require_auth();

        let escrow = TransferEscrow {
            product_id: product_id.clone(),
            current_owner: product.owner.clone(),
            proposed_owner,
            requested_at: env.ledger().timestamp(),
            disputed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TransferEscrow(product_id.clone()), &escrow);

        env.events().publish(
            (Symbol::new(&env, "transfer_requested"), product_id),
            escrow.clone(),
        );
        escrow
    }

    // ── #499: Multi-hop approval chain-of-custody ────────────────────────────

    /// Record an approval hop in the chain-of-custody for a product.
    /// Each actor in the supply chain signs to approve the product's current state.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product being approved.
    /// - `approver` — Address of the actor approving the product.
    /// - `approval_type` — Type of approval (e.g., "RECEIVED", "INSPECTED", "SHIPPED").
    /// - `metadata` — Optional JSON metadata about the approval.
    ///
    /// # Returns
    /// The approval hop record with timestamp and approver info.
    ///
    /// # Authorization
    /// Requires `approver.require_auth()`.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    pub fn record_approval_hop(
        env: Env,
        product_id: String,
        approver: Address,
        approval_type: String,
        metadata: String,
    ) -> ApprovalHop {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        approver.require_auth();

        let hop = ApprovalHop {
            product_id: product_id.clone(),
            approver: approver.clone(),
            approval_type: approval_type.clone(),
            timestamp: env.ledger().timestamp(),
            metadata,
        };

        let mut hops: Vec<ApprovalHop> = env
            .storage()
            .persistent()
            .get(&DataKey::ApprovalHops(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        hops.push_back(hop.clone());
        env.storage()
            .persistent()
            .set(&DataKey::ApprovalHops(product_id.clone()), &hops);

        env.events().publish(
            (Symbol::new(&env, "approval_hop_recorded"), product_id),
            hop.clone(),
        );

        hop
    }

    /// Retrieve all approval hops for a product's chain-of-custody.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    ///
    /// # Returns
    /// Vector of all approval hops in chronological order.
    pub fn get_approval_hops(env: Env, product_id: String) -> Vec<ApprovalHop> {
        env.storage()
            .persistent()
            .get(&DataKey::ApprovalHops(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Verify the chain-of-custody for a product by checking approval hops.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `required_approvers` — Vector of addresses that must have approved.
    ///
    /// # Returns
    /// `true` if all required approvers have recorded an approval hop, `false` otherwise.
    pub fn verify_chain_of_custody(
        env: Env,
        product_id: String,
        required_approvers: Vec<Address>,
    ) -> bool {
        let hops: Vec<ApprovalHop> = env
            .storage()
            .persistent()
            .get(&DataKey::ApprovalHops(product_id))
            .unwrap_or_else(|| Vec::new(&env));

        for required in required_approvers.iter() {
            let mut found = false;
            for hop in hops.iter() {
                if hop.approver == required {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        true
    }

    // ── #500: Origin attestations ────────────────────────────────────────────

    /// Record an origin attestation for a product.
    /// Proves where goods came from with cryptographic proof.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `attester` — Address of the actor attesting to the origin.
    /// - `origin_claim` — Description of the origin (e.g., "Ethiopian highlands, Yirgacheffe region").
    /// - `proof_hash` — Cryptographic hash of supporting documentation.
    ///
    /// # Returns
    /// The origin attestation record.
    ///
    /// # Authorization
    /// Requires `attester.require_auth()`.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    pub fn attest_origin(
        env: Env,
        product_id: String,
        attester: Address,
        origin_claim: String,
        proof_hash: String,
    ) -> OriginAttestation {
        let _product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        attester.require_auth();

        let attestation = OriginAttestation {
            product_id: product_id.clone(),
            attester: attester.clone(),
            origin_claim,
            proof_hash,
            timestamp: env.ledger().timestamp(),
            verified: false,
        };

        let mut attestations: Vec<OriginAttestation> = env
            .storage()
            .persistent()
            .get(&DataKey::OriginAttestations(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        attestations.push_back(attestation.clone());
        env.storage()
            .persistent()
            .set(&DataKey::OriginAttestations(product_id.clone()), &attestations);

        env.events().publish(
            (Symbol::new(&env, "origin_attested"), product_id),
            attestation.clone(),
        );

        attestation
    }

    /// Verify an origin attestation for a product.
    /// Marks an attestation as verified by an authorized verifier.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `attestation_index` — Index of the attestation to verify.
    /// - `verifier` — Address of the verifier (typically product owner or auditor).
    ///
    /// # Returns
    /// `true` if verification succeeded, `false` if attestation not found.
    ///
    /// # Authorization
    /// Requires `verifier.require_auth()`.
    pub fn verify_origin_attestation(
        env: Env,
        product_id: String,
        attestation_index: u32,
        verifier: Address,
    ) -> bool {
        verifier.require_auth();

        let mut attestations: Vec<OriginAttestation> = env
            .storage()
            .persistent()
            .get(&DataKey::OriginAttestations(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        if attestation_index as usize >= attestations.len() {
            return false;
        }

        let mut attestation = attestations.get(attestation_index as usize).unwrap();
        attestation.verified = true;

        attestations.set(attestation_index as usize, attestation.clone());
        env.storage()
            .persistent()
            .set(&DataKey::OriginAttestations(product_id.clone()), &attestations);

        env.events().publish(
            (Symbol::new(&env, "origin_verified"), product_id),
            attestation,
        );

        true
    }

    /// Retrieve all origin attestations for a product.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    ///
    /// # Returns
    /// Vector of all origin attestations for the product.
    pub fn get_origin_attestations(env: Env, product_id: String) -> Vec<OriginAttestation> {
        env.storage()
            .persistent()
            .get(&DataKey::OriginAttestations(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Env, String};

    fn setup() -> (Env, SupplyLinkContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        (env, client)
    }

    fn make_str(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    // ── Basic registration ────────────────────────────────────────────────────

    #[test]
    fn test_register_product_success() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let product = client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        assert_eq!(product.id, make_str(&env, "p1"));
        assert_eq!(product.schema_version, SCHEMA_VERSION);
        assert!(!product.recalled);
    }

    #[test]
    fn test_product_exists() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        assert!(!client.product_exists(&make_str(&env, "p1")));
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        assert!(client.product_exists(&make_str(&env, "p1")));
    }
    /// Accept a pending transfer. Proposed owner confirms and takes ownership.
    pub fn accept_transfer_ownership(env: Env, product_id: String) -> bool {
        let escrow: TransferEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id.clone()))
            .expect("no pending transfer");

        if escrow.disputed {
            panic!("transfer is disputed");
        }

        escrow.proposed_owner.require_auth();

        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner = escrow.proposed_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);
        env.storage()
            .persistent()
            .remove(&DataKey::TransferEscrow(product_id.clone()));

        env.events().publish(
            (Symbol::new(&env, "transfer_accepted"), product_id),
            escrow.proposed_owner,
        );
        true
    }

    /// Cancel a pending transfer request (current owner only).
    pub fn cancel_transfer_request(env: Env, product_id: String) -> bool {
        let escrow: TransferEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id.clone()))
            .expect("no pending transfer");

        escrow.current_owner.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::TransferEscrow(product_id.clone()));

        env.events().publish(
            (Symbol::new(&env, "transfer_cancelled"), product_id),
            escrow.current_owner,
        );
        true
    }

    /// Dispute a pending transfer. Either party can raise a dispute.
    pub fn dispute_transfer_ownership(env: Env, product_id: String) -> bool {
        let mut escrow: TransferEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id.clone()))
            .expect("no pending transfer");

        // Either current owner or proposed owner can dispute
        let caller_is_owner = escrow.current_owner.clone();
        caller_is_owner.require_auth();

        escrow.disputed = true;
        env.storage()
            .persistent()
            .set(&DataKey::TransferEscrow(product_id.clone()), &escrow);

        env.events().publish(
            (Symbol::new(&env, "transfer_disputed"), product_id),
            escrow.current_owner,
        );
        true
    }

    /// Get the pending transfer escrow for a product, if any.
    pub fn get_transfer_escrow(env: Env, product_id: String) -> Option<TransferEscrow> {
        env.storage()
            .persistent()
            .get(&DataKey::TransferEscrow(product_id))
    }

    // ── #394: Pending event approval queue ───────────────────────────────────

    /// Submit an event for approval by required approvers.
    /// The event is not committed to the event log until finalized.
    pub fn submit_event_for_approval(
        env: Env,
        product_id: String,
        submitter: Address,
        location: String,
        event_type: String,
        metadata: String,
        required_approvers: Vec<Address>,
        ttl_seconds: u64,
    ) -> PendingEvent {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let is_owner = product.owner == submitter;
        let is_actor = product.authorized_actors.contains(&submitter);
        if !is_owner && !is_actor {
            panic!("submitter is not authorized");
        }
        submitter.require_auth();

        let now = env.ledger().timestamp();
        let pending = PendingEvent {
            product_id: product_id.clone(),
            submitter: submitter.clone(),
            location,
            event_type,
            metadata,
            submitted_at: now,
            required_approvers,
            approvals: Vec::new(&env),
            rejected: false,
            expires_at: now + ttl_seconds,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingEvent(product_id.clone(), submitter.clone()), &pending);

        env.events().publish(
            (Symbol::new(&env, "event_submitted"), product_id),
            submitter,
        );
        pending
    }

    /// Approve a pending event. Approver must be in required_approvers.
    pub fn approve_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
        approver: Address,
    ) -> PendingEvent {
        let mut pending: PendingEvent = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id.clone(), submitter.clone()))
            .expect("no pending event");

        if pending.rejected {
            panic!("event already rejected");
        }
        if env.ledger().timestamp() > pending.expires_at {
            panic!("pending event expired");
        }
        if !pending.required_approvers.contains(&approver) {
            panic!("approver is not a required approver");
        }
        if pending.approvals.contains(&approver) {
            panic!("already approved");
        }

        approver.require_auth();
        pending.approvals.push_back(approver.clone());

        env.storage()
            .persistent()
            .set(&DataKey::PendingEvent(product_id.clone(), submitter.clone()), &pending);

        env.events().publish(
            (Symbol::new(&env, "event_approved"), product_id),
            approver,
        );
        pending
    }

    /// Reject a pending event. Any required approver can reject.
    pub fn reject_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
        approver: Address,
    ) -> bool {
        let mut pending: PendingEvent = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id.clone(), submitter.clone()))
            .expect("no pending event");

        if !pending.required_approvers.contains(&approver) {
            panic!("approver is not a required approver");
        }
        approver.require_auth();

        pending.rejected = true;
        env.storage()
            .persistent()
            .set(&DataKey::PendingEvent(product_id.clone(), submitter.clone()), &pending);

        env.events().publish(
            (Symbol::new(&env, "event_rejected"), product_id),
            approver,
        );
        true
    }

    /// Finalize a pending event once all required approvals are collected.
    /// Commits the event to the product's event log.
    pub fn finalize_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
    ) -> TrackingEvent {
        let pending: PendingEvent = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id.clone(), submitter.clone()))
            .expect("no pending event");

        if pending.rejected {
            panic!("event was rejected");
        }
        if env.ledger().timestamp() > pending.expires_at {
            panic!("pending event expired");
        }

        // All required approvers must have approved
        for i in 0..pending.required_approvers.len() {
            let req = pending.required_approvers.get(i).unwrap();
            if !pending.approvals.contains(&req) {
                panic!("not all approvers have approved");
            }
        }

        let event = TrackingEvent {
            product_id: product_id.clone(),
            location: pending.location,
            actor: pending.submitter,
            timestamp: env.ledger().timestamp(),
            event_type: pending.event_type,
            metadata: pending.metadata,
        };

        let mut events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        events.push_back(event.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Events(product_id.clone()), &events);

        // Remove the pending event
        env.storage()
            .persistent()
            .remove(&DataKey::PendingEvent(product_id.clone(), submitter));

        env.events().publish(
            (Symbol::new(&env, "event_finalized"), product_id),
            event.clone(),
        );
        event
    }

    /// Get a pending event by product_id and submitter.
    pub fn get_pending_event(
        env: Env,
        product_id: String,
        submitter: Address,
    ) -> Option<PendingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingEvent(product_id, submitter))
    }
}

    /// Revoke an address's permission to add tracking events for a product.
    ///
    /// Rebuilds `authorized_actors` without `actor`. Because
    /// [`Self::add_authorized_actor`] prevents duplicates, at most one entry
    /// will ever be removed.
    ///
    /// # Governance Safeguards
    /// - Prevents removal of the owner from authorized actors if multi-signature
    ///   is enabled and would leave insufficient authorized actors to meet the
    ///   required signature threshold.
    /// - Ensures at least one authorized path remains for governance operations.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `actor` — Stellar address to revoke.
    ///
    /// # Returns
    /// `true` if `actor` was found and removed, `false` if `actor` was not in
    /// the authorized list.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may revoke actor permissions.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"cannot remove owner from actors"` — if attempting to remove the owner
    ///   when it would violate governance invariants.
    /// - `"removal would violate governance"` — if removal would leave insufficient
    ///   actors to meet multi-signature requirements.
    ///
    /// # Emitted Events
    /// Does not emit an event. Removal of an actor is not announced on-chain.
    /// Consumers tracking actor permissions must observe the absence of future
    /// `actor_authorized` events or query [`Self::get_authorized_actors`]
    /// directly.
    pub fn remove_authorized_actor(
        env: Env,
        product_id: String,
        actor: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        Self::validate_and_increment_nonce(&env, &product.owner, nonce);

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

        // Governance safeguard: ensure sufficient actors remain for multi-sig
        if product.required_signatures > 1 {
            // Count total authorized entities (owner + actors)
            let total_authorized = 1 + new_actors.len() as u32; // owner + remaining actors
            if total_authorized < product.required_signatures {
                panic!("removal would violate governance");
            }
        }

        product.authorized_actors = new_actors;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        // Emit event
        if found {
            env.events().publish(
                (Symbol::new(&env, "actor_removed"), product_id),
                actor,
            );
        }

        Ok(found)
    }

    /// Update the mutable metadata fields of a product.
    ///
    /// Only `name` and `origin` can be changed. The `id`, `owner`,
    /// `timestamp`, `authorized_actors`, and `required_signatures` fields are
    /// immutable through this function.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to update.
    /// - `name` — New human-readable product name.
    /// - `origin` — New origin string.
    ///
    /// # Returns
    /// The updated [`Product`] struct.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the current product owner
    /// may update metadata.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    ///
    /// # Emitted Events
    /// Publishes a `("product_updated", product_id)` event with the updated
    /// [`Product`] struct as the event body.
    pub fn update_product_metadata(
        env: Env,
        product_id: String,
        name: String,
        origin: String,
    ) -> Result<Product, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();
        // Issue #311: enforce size limits on update.
        assert_len(&name,   MAX_NAME_LEN,   "name");
        assert_len(&origin, MAX_ORIGIN_LEN, "origin");

        product.name = name;
        product.origin = origin;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "product_updated"), product_id),
            product.clone(),
        );

        Ok(product)
    }

    fn add_event(env: &Env, contract_id: &soroban_sdk::Address, product_id: &String) {
        let client = SupplyLinkContractClient::new(env, contract_id);
        // HARVEST is the only valid first event from Registered stage
        client.add_tracking_event(
            product_id,
            &String::from_str(env, "Farm"),
            &String::from_str(env, "HARVEST"),
            &String::from_str(env, "{}"),
    /// Deactivate a product, preventing new events from being recorded.
    ///
    /// Sets `product.active` to `false`. Once deactivated, a product cannot
    /// receive new tracking events. The product remains queryable but is marked
    /// as recalled/deactivated for consumer display.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to deactivate.
    ///
    /// # Returns
    /// The updated [`Product`] struct with `active = false`.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`. Only the product owner may
    /// deactivate a product.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"product already inactive"` — if the product is already deactivated.
    ///
    /// # Emitted Events
    /// Publishes a `("product_deactivated", product_id)` event with the updated
    /// [`Product`] struct as the event body.
    pub fn deactivate_product(env: Env, product_id: String) -> Result<Product, Error> {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();

        if !product.active {
            panic!("product already inactive");
        }

        product.active = false;

        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id.clone()), &product);

        env.events().publish(
            (Symbol::new(&env, "product_deactivated"), product_id),
            product.clone(),
        );

        Ok(product)
    }

    /// Return the list of addresses authorised to add events for a product.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to query.
    ///
    /// # Returns
    /// A `Vec<Address>` of authorized actors. Returns an empty vector if the
    /// product does not exist or has no authorized actors.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_authorized_actors(env: Env, product_id: String) -> Vec<Address> {
        env.storage()
            .persistent()
            .get::<DataKey, Product>(&DataKey::Product(product_id))
            .map(|p| p.authorized_actors)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Return the total number of products registered on this contract.
    ///
    /// The count is a monotonically increasing counter; it is never decremented
    /// even if products were to be removed (which is not currently supported).
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    ///
    /// # Returns
    /// A `u64` count. Returns `0` if no products have been registered.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_product_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0)
    }

    /// Return a paginated slice of product IDs in registration order.
    ///
    /// Uses the [`DataKey::ProductIndex`] mapping to look up IDs by their
    /// sequential insertion index, enabling efficient pagination without
    /// iterating all storage keys.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `offset` — Zero-based index of the first product to return.
    /// - `limit` — Maximum number of product IDs to return.
    ///
    /// # Returns
    /// A `Vec<String>` of product IDs. Returns an empty vector if `offset` is
    /// beyond the total count or no products are registered.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    ///
    /// # Example
    /// ```text
    /// // Fetch the first page of 10 products
    /// list_products(env, 0, 10)
    ///
    /// // Fetch the second page
    /// list_products(env, 10, 10)
    /// ```
    pub fn list_products(env: Env, offset: u64, limit: u64) -> Vec<String> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProductCount)
            .unwrap_or(0);

        let mut products = Vec::new(&env);
        let end = core::cmp::min(offset + limit, count);

    #[test]
    fn test_get_product_count() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        assert_eq!(client.get_product_count(), 0);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        assert_eq!(client.get_product_count(), 1);
    }

    // ── Tracking events ───────────────────────────────────────────────────────

    #[test]
    fn test_add_tracking_event_success() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        let event = client.add_tracking_event(
            &make_str(&env, "p1"),
            &owner,
            &make_str(&env, "Warehouse B"),
            &make_str(&env, "SHIPPING"),
            &make_str(&env, "{}"),
        );
        assert_eq!(event.schema_version, SCHEMA_VERSION);
        assert_eq!(client.get_events_count(&make_str(&env, "p1")), 1);
    }

    #[test]
    #[should_panic(expected = "caller is not authorized")]
    fn test_add_tracking_event_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.add_tracking_event(
            &make_str(&env, "p1"),
            &stranger,
            &make_str(&env, "Warehouse B"),
            &make_str(&env, "SHIPPING"),
            &make_str(&env, "{}"),
        );
    }

    // ── Recall (#393) ─────────────────────────────────────────────────────────

    #[test]
    fn test_recall_product_success() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.recall_product(&make_str(&env, "p1"), &make_str(&env, "Contamination found"));
        let product = client.get_product(&make_str(&env, "p1"));
        assert!(product.recalled);
        assert_eq!(product.recall_reason, make_str(&env, "Contamination found"));
    }

    #[test]
    fn test_unrecall_product_success() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.recall_product(&make_str(&env, "p1"), &make_str(&env, "Contamination found"));
        client.unrecall_product(&make_str(&env, "p1"));
        let product = client.get_product(&make_str(&env, "p1"));
        assert!(!product.recalled);
    }

    #[test]
    #[should_panic(expected = "product is recalled")]
    fn test_recalled_product_rejects_events() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.recall_product(&make_str(&env, "p1"), &make_str(&env, "Safety issue"));
        // This should panic with "product is recalled"
        client.add_tracking_event(
            &make_str(&env, "p1"),
            &owner,
            &make_str(&env, "Warehouse B"),
            &make_str(&env, "SHIPPING"),
            &make_str(&env, "{}"),
        );
    }

    // ── Batch registration (#389) ─────────────────────────────────────────────

    #[test]
    fn test_register_products_batch_success() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let mut ids = Vec::new(&env);
        let mut names = Vec::new(&env);
        let mut origins = Vec::new(&env);
        ids.push_back(make_str(&env, "b1"));
        ids.push_back(make_str(&env, "b2"));
        names.push_back(make_str(&env, "Alpha"));
        names.push_back(make_str(&env, "Beta"));
        origins.push_back(make_str(&env, "Origin A"));
        origins.push_back(make_str(&env, "Origin B"));
        let products = client.register_products_batch(&owner, &ids, &names, &origins);
        assert_eq!(products.len(), 2);
        assert_eq!(client.get_product_count(), 2);
        assert!(client.product_exists(&make_str(&env, "b1")));
        assert!(client.product_exists(&make_str(&env, "b2")));
    }

    #[test]
    fn test_recall_history_accumulates() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.recall_product(&make_str(&env, "p1"), &make_str(&env, "Reason 1"));
        client.unrecall_product(&make_str(&env, "p1"));
        client.recall_product(&make_str(&env, "p1"), &make_str(&env, "Reason 2"));
        let history = client.get_recall_history(&make_str(&env, "p1"));
        assert_eq!(history.len(), 2);
        assert_eq!(history.get(0).unwrap(), make_str(&env, "Reason 1"));
        assert_eq!(history.get(1).unwrap(), make_str(&env, "Reason 2"));
    }

    #[test]
    #[should_panic]
    fn test_recall_panics_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        // Attempt to recall without providing the owner's auth — should panic
        // We achieve this by calling recall_product with a different env that
        // has no mocked auths, which causes require_auth to fail.
        let env2 = Env::default();
        // env2 has no mocked auths; calling recall on the same contract will panic
        let client2 = SupplyLinkContractClient::new(&env2, &contract_id);
        client2.recall_product(&make_str(&env2, "p1"), &make_str(&env2, "Unauthorized recall"));
    }

    #[test]
    fn test_register_products_batch() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let mut ids = Vec::new(&env);
        let mut names = Vec::new(&env);
        let mut origins = Vec::new(&env);
        ids.push_back(make_str(&env, "b1"));
        ids.push_back(make_str(&env, "b2"));
        ids.push_back(make_str(&env, "b3"));
        names.push_back(make_str(&env, "Alpha"));
        names.push_back(make_str(&env, "Beta"));
        names.push_back(make_str(&env, "Gamma"));
        origins.push_back(make_str(&env, "Origin A"));
        origins.push_back(make_str(&env, "Origin B"));
        origins.push_back(make_str(&env, "Origin C"));
        let products = client.register_products_batch(&owner, &ids, &names, &origins);
        assert_eq!(products.len(), 3);
        assert_eq!(client.get_product_count(), 3);
        assert!(client.product_exists(&make_str(&env, "b1")));
        assert!(client.product_exists(&make_str(&env, "b2")));
        assert!(client.product_exists(&make_str(&env, "b3")));
        // Verify schema version and recall defaults
        let p = client.get_product(&make_str(&env, "b1"));
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        assert!(!p.recalled);
    }

    #[test]
    #[should_panic(expected = "batch size exceeds maximum of 10")]
    fn test_register_products_batch_exceeds_limit() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let mut ids = Vec::new(&env);
        let mut names = Vec::new(&env);
        let mut origins = Vec::new(&env);
        for i in 0..11u32 {
            let id = match i {
                0 => make_str(&env, "x0"),
                1 => make_str(&env, "x1"),
                2 => make_str(&env, "x2"),
                3 => make_str(&env, "x3"),
                4 => make_str(&env, "x4"),
                5 => make_str(&env, "x5"),
                6 => make_str(&env, "x6"),
                7 => make_str(&env, "x7"),
                8 => make_str(&env, "x8"),
                9 => make_str(&env, "x9"),
                _ => make_str(&env, "x10"),
            };
            ids.push_back(id.clone());
            names.push_back(make_str(&env, "Name"));
            origins.push_back(make_str(&env, "Origin"));
        }
        client.register_products_batch(&owner, &ids, &names, &origins);
    }

    #[test]
    #[should_panic(expected = "ids, names, and origins must have equal length")]
    fn test_register_products_batch_mismatched_lengths() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let mut ids = Vec::new(&env);
        let mut names = Vec::new(&env);
        let origins = Vec::new(&env);
        ids.push_back(make_str(&env, "b1"));
        names.push_back(make_str(&env, "Alpha"));
        names.push_back(make_str(&env, "Beta"));
        client.register_products_batch(&owner, &ids, &names, &origins);
    }

    // ── Transfer ownership ────────────────────────────────────────────────────

    #[test]
    fn test_transfer_ownership() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.transfer_ownership(&make_str(&env, "p1"), &new_owner);
        let product = client.get_product(&make_str(&env, "p1"));
        assert_eq!(product.owner, new_owner);
    }

    // ── Authorized actors ─────────────────────────────────────────────────────

    #[test]
    fn test_add_and_remove_authorized_actor() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        client.register_product(
            &make_str(&env, "p1"),
            &make_str(&env, "Widget"),
            &make_str(&env, "Factory A"),
            &owner,
        );
        client.add_authorized_actor(&make_str(&env, "p1"), &actor);
        let actors = client.get_authorized_actors(&make_str(&env, "p1"));
        assert_eq!(actors.len(), 1);
        client.remove_authorized_actor(&make_str(&env, "p1"), &actor);
        let actors = client.get_authorized_actors(&make_str(&env, "p1"));
        assert_eq!(actors.len(), 0);
    }

    // ── List products ─────────────────────────────────────────────────────────

    #[test]
    fn test_list_products_pagination() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        // Register 3 products using known IDs
        client.register_product(&make_str(&env, "p0"), &make_str(&env, "W0"), &make_str(&env, "O"), &owner);
        client.register_product(&make_str(&env, "p1"), &make_str(&env, "W1"), &make_str(&env, "O"), &owner);
        client.register_product(&make_str(&env, "p2"), &make_str(&env, "W2"), &make_str(&env, "O"), &owner);
        let page = client.list_products(&0u64, &2u64);
        assert_eq!(page.len(), 2);
        let all = client.list_products(&0u64, &10u64);
        assert_eq!(all.len(), 3);
    }

    // ── Schema version (#392) ─────────────────────────────────────────────────

    #[test]
    fn test_schema_version_on_product() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let product = client.register_product(
            &make_str(&env, "sv1"),
            &make_str(&env, "Versioned"),
            &make_str(&env, "Lab"),
            &owner,
        );
        assert_eq!(product.schema_version, 1u32);
    }

    #[test]
    fn test_schema_version_on_event() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.register_product(
            &make_str(&env, "sv1"),
            &make_str(&env, "Versioned"),
            &make_str(&env, "Lab"),
            &owner,
        );
        let event = client.add_tracking_event(
            &make_str(&env, "sv1"),
            &owner,
            &make_str(&env, "Port"),
            &make_str(&env, "SHIPPING"),
            &make_str(&env, "{}"),
        );
        assert_eq!(event.schema_version, 1u32);
    }

    /// Req 3.4 — multiple products each with one event → correct total
    #[test]
    fn test_multiple_events_returns_correct_count() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = soroban_sdk::Address::generate(&env);

        // Register 5 separate products and add one HARVEST event each
        for i in 0u32..5 {
            let pid = {
                let ids = ["pa", "pb", "pc", "pd", "pe"];
                String::from_str(&env, ids[i as usize])
            };
            client.register_product(&pid, &String::from_str(&env, "W"), &String::from_str(&env, "O"), &owner);
            client.add_tracking_event(&pid, &String::from_str(&env, "Farm"), &String::from_str(&env, "HARVEST"), &String::from_str(&env, "{}"));
            assert_eq!(client.get_events_count(&pid), 1);
        }
    }

    /// Req 3.5 — get_events_count == get_tracking_events(...).len()
    #[test]
    fn test_count_equals_vec_len() {
        let (env, contract_id, product_id) = setup();
        add_event(&env, &contract_id, &product_id);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let count = client.get_events_count(&product_id);
        let events = client.get_tracking_events(&product_id);
        assert_eq!(count, events.len());
    }

    // ── Property-based tests ─────────────────────────────────────────────────

    /// Property 1: Registered product with one HARVEST event has count == 1
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_count_equals_n_events(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);

            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            prop_assert_eq!(client.get_events_count(&product_id), 0);

            client.add_tracking_event(
                &product_id,
                &String::from_str(&env, "Farm"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            prop_assert_eq!(client.get_events_count(&product_id), 1);
        }
    }

    /// Property 2: Unknown product returns 0
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_unknown_product_returns_zero(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let product_id = String::from_str(&env, &product_id_str);
            prop_assert_eq!(client.get_events_count(&product_id), 0);
    // ── #388: Paginated event retrieval ───────────────────────────────────────

    /// Return a paginated slice of tracking events for a product.
    ///
    /// Events are returned in insertion order (oldest first).
    ///
    /// # Parameters
    /// - `product_id` — The product to query.
    /// - `offset` — Zero-based index of the first event to return.
    /// - `limit` — Maximum number of events to return.
    ///
    /// # Returns
    /// A `Vec<TrackingEvent>`. Returns an empty vector if offset is beyond
    /// the total count or the product has no events.
    pub fn list_tracking_events(
        env: Env,
        product_id: String,
        offset: u32,
        limit: u32,
    ) -> Vec<TrackingEvent> {
        let all: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id))
            .unwrap_or_else(|| Vec::new(&env));

        let total = all.len();
        if offset >= total {
            return Vec::new(&env);
        }

        let end = core::cmp::min(offset + limit, total);
        let mut page = Vec::new(&env);
        for i in offset..end {
            page.push_back(all.get(i).unwrap());
        }
        page
    }

    /// Return the total number of tracking events for a product.
    /// Alias of `get_events_count` — provided for API symmetry with `list_tracking_events`.
    pub fn count_tracking_events(env: Env, product_id: String) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<TrackingEvent>>(&DataKey::Events(product_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    // ── #387: Role segregation ────────────────────────────────────────────────

    /// Assign a named role to an authorized actor for a product.
    ///
    /// The actor must already be in `authorized_actors`. Replaces any existing
    /// role assignment for the same actor.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`.
    pub fn assign_role(env: Env, product_id: String, actor: Address, role: Role) -> bool {
    /// Approve a pending event for a high-value product.
    ///
    /// For products with `required_signatures > 1`, events are staged as pending
    /// until the required number of approvals are received. This function allows
    /// authorized actors to approve a pending event using its stable identifier.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment (injected by the runtime).
    /// - `product_id` — ID of the product.
    /// - `pending_event_id` — Stable ID of the pending event to approve.
    ///   This ID remains unchanged even if other pending events are removed.
    /// - `approver` — Address of the actor approving the event.
    /// - `nonce` — Sequential nonce for authorization, incremented by the contract.
    ///
    /// # Returns
    /// `true` if the event was finalized (all signatures received), `false` if
    /// more approvals are needed.
    ///
    /// # Authorization
    /// Requires `approver.require_auth()`. The approver must be the owner or
    /// an authorized actor.
    ///
    /// # Errors
    /// - [`Error::ProductNotFound`] — if `product_id` is not registered.
    /// - [`Error::ApproverNotAuthorized`] — if approver is not owner or actor.
    /// - [`Error::NoPendingEvents`] — if there are no pending events.
    /// - [`Error::PendingEventExpired`] — if the pending event has expired (issue #314).
    ///
    /// # Panics
    /// - `"event index out of bounds"` — if `event_index` is invalid.
    ///
    /// # Emitted Events
    /// - When the event is **not yet finalized**: no event is emitted.
    /// - When the event **is finalized** (approvals reach `required_signatures`):
    ///   publishes an `("event_finalized", product_id, event_type,
    ///   schema_version)` event with the [`TrackingEvent`] struct as the body.
    pub fn approve_event(
        env: Env,
        product_id: String,
        pending_event_id: u64,
        approver: Address,
        nonce: u64,
    ) -> Result<bool, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        let is_owner = product.owner == approver;
        let is_actor = product.authorized_actors.contains(&approver);
        if !is_owner && !is_actor {
            return Err(Error::ApproverNotAuthorized);
        }
        approver.require_auth();
        Self::validate_and_increment_nonce(&env, &approver, nonce);

        let mut pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id.clone()))
            .ok_or(Error::NoPendingEvents)?;

        // Find the pending event by stable ID (not index-based)
        let mut event_position: Option<usize> = None;
        for i in 0..pending.len() {
            if pending.get(i).unwrap().pending_event_id == pending_event_id {
                event_position = Some(i);
                break;
            }
        }

        let event_index = event_position.ok_or_else(|| {
            panic!("pending event not found")
        })?;

        let mut pending_event = pending.get(event_index).unwrap().clone();

        // Check expiration (issue #314)
        let current_time = env.ledger().timestamp();
        if current_time > pending_event.expiration {
            return Err(Error::PendingEventExpired);
        }

        if !pending_event.approvals.contains(&approver) {
            pending_event.approvals.push_back(approver.clone());
        }

        let is_finalized = pending_event.approvals.len() as u32 >= pending_event.required_signatures;

        if is_finalized {
            let mut events: Vec<TrackingEvent> = env
                .storage()
                .persistent()
                .get(&DataKey::Events(product_id.clone()))
                .unwrap_or_else(|| Vec::new(&env));

            events.push_back(pending_event.event.clone());
            env.storage()
                .persistent()
                .set(&DataKey::Events(product_id.clone()), &events);

            // Update provenance root
            let prev_root: BytesN<32> = env
                .storage()
                .persistent()
                .get(&DataKey::ProvenanceRoot(product_id.clone()))
                .unwrap_or_else(|| BytesN::from_array(&env, &[0u8; 32]));
            let new_root = Self::compute_next_provenance_root(&env, &prev_root, &pending_event.event);
            env.storage()
                .persistent()
                .set(&DataKey::ProvenanceRoot(product_id.clone()), &new_root);

            // Remove from pending
            pending.remove(event_index);
            if pending.len() > 0 {
                env.storage()
                    .persistent()
                    .set(&DataKey::PendingEvents(product_id.clone()), &pending);
            } else {
                env.storage()
                    .persistent()
                    .remove(&DataKey::PendingEvents(product_id.clone()));
            }

            env.events().publish(
                (
                    Symbol::new(&env, "event_finalized"),
                    product_id,
                    pending_event.event.event_type.clone(),
                    EVENT_SCHEMA_VERSION,
                ),
                pending_event.event,
            );

            Ok(true)
        } else {
            // Update pending event with new approval
            pending.set(event_index, pending_event);
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id), &pending);
            Ok(false)
        }
    }

    /// Reject a pending event for a high-value product.
    ///
    /// Removes a pending event from the approval queue without finalizing it.
    /// Optionally accepts a reason for the rejection for audit purposes.
    /// Uses the stable identifier of the pending event to ensure deterministic
    /// behavior even after queue mutations.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `pending_event_id` — Stable ID of the pending event to reject.
    ///   This ID remains unchanged even if other pending events are removed.
    /// - `rejector` — Address of the actor rejecting the event.
    /// - `reason` — Optional reason for rejection (max 256 characters).
    /// - `nonce` — Sequential nonce for authorization, incremented by the contract.
    ///
    /// # Returns
    /// `true` on success.
    ///
    /// # Authorization
    /// Requires `rejector.require_auth()`. The rejector must be the owner.
    ///
    /// # Panics
    /// - `"product not found"` — if `product_id` is not registered.
    /// - `"only owner can reject"` — if rejector is not the owner.
    /// - `"no pending events"` — if there are no pending events.
    /// - `"pending event not found"` — if `pending_event_id` doesn't match any pending event.
    /// - `"rejection reason too long"` — if reason exceeds 256 characters.
    /// - `"invalid nonce"` — if nonce does not match the expected sequential value.
    pub fn reject_event(
        env: Env,
        product_id: String,
        pending_event_id: u64,
        rejector: Address,
        reason: String,
        nonce: u64,
    ) -> Result<bool, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        let mut policy: AuthPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id.clone()))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            });

        // Replace existing role for actor or append
        let mut new_roles: Vec<ActorRole> = Vec::new(&env);
        let mut replaced = false;
        for i in 0..policy.roles.len() {
            let ar = policy.roles.get(i).unwrap();
            if ar.actor == actor {
                new_roles.push_back(ActorRole { actor: actor.clone(), role: role.clone() });
                replaced = true;
            } else {
                new_roles.push_back(ar);
            }
        }
        if !replaced {
            new_roles.push_back(ActorRole { actor, role });
        }
        policy.roles = new_roles;

        env.storage()
            .persistent()
            .set(&DataKey::AuthPolicy(product_id), &policy);
        true
    }

    /// Property 3: After HARVEST, count increments by one
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_add_increments_count(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);

            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            let count_before = client.get_events_count(&product_id);
            client.add_tracking_event(
                &product_id,
                &String::from_str(&env, "Farm"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            let count_after = client.get_events_count(&product_id);
            prop_assert_eq!(count_after, count_before + 1);
    /// Revoke the role assignment for an actor on a product.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`.
    pub fn revoke_role(env: Env, product_id: String, actor: Address) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        let mut policy: AuthPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id.clone()))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            });

        let mut new_roles: Vec<ActorRole> = Vec::new(&env);
        let mut found = false;
        for i in 0..policy.roles.len() {
            let ar = policy.roles.get(i).unwrap();
            if ar.actor == actor {
                found = true;
            } else {
                new_roles.push_back(ar);
            }
        }
        policy.roles = new_roles;

        env.storage()
            .persistent()
            .set(&DataKey::AuthPolicy(product_id), &policy);
        found
    }

    /// Set the minimum number of signers required for an event on this product.
    ///
    /// # Authorization
    /// Requires `product.owner.require_auth()`.
    pub fn set_event_threshold(env: Env, product_id: String, threshold: u32) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();

        let mut policy: AuthPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id.clone()))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            });
        policy.threshold = threshold;

        env.storage()
            .persistent()
            .set(&DataKey::AuthPolicy(product_id), &policy);
        true
    }

    /// Return the authorization policy (roles + threshold) for a product.
    pub fn get_authorization_policy(env: Env, product_id: String) -> AuthPolicy {
        env.storage()
            .persistent()
            .get(&DataKey::AuthPolicy(product_id))
            .unwrap_or(AuthPolicy {
                threshold: 1,
                roles: Vec::new(&env),
            })
    }

    // ── Deactivate / reactivate (#3) ──────────────────────────────────────────

    /// Deactivate a product (owner-only). Idempotent.
    pub fn deactivate_product(env: Env, product_id: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();
        product.active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
    }

    /// Reactivate a previously deactivated product (owner-only). Idempotent.
    pub fn reactivate_product(env: Env, product_id: String) -> bool {
        let mut product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");
        product.owner.require_auth();
        product.active = true;
        env.storage()
            .persistent()
            .set(&DataKey::Product(product_id), &product);
        true
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compute a stable deterministic event ID.
///
/// Concatenates `product_id`, `actor` (as bytes), `event_type`, `timestamp`
/// (big-endian u64), and `metadata` into a single byte buffer, then returns
/// the SHA-256 hash as a lowercase hex `String`.
///
/// The result is invariant as long as the input fields are identical, making
/// it safe to use as a permanent reference across contract upgrades.
fn compute_stable_id(
    env: &Env,
    product_id: &String,
    actor: &Address,
    event_type: &String,
    timestamp: u64,
    metadata: &String,
) -> String {
    // Build a byte buffer: product_id bytes + event_type bytes + timestamp (8 bytes BE) + metadata bytes
    let pid_bytes = product_id.clone().to_xdr(env);
    let et_bytes = event_type.clone().to_xdr(env);
    let meta_bytes = metadata.clone().to_xdr(env);
    let actor_bytes = actor.clone().to_xdr(env);

    let ts_bytes: [u8; 8] = timestamp.to_be_bytes();
    let ts_buf = Bytes::from_array(env, &ts_bytes);

    let mut buf = Bytes::new(env);
    buf.append(&pid_bytes);
    buf.append(&actor_bytes);
    buf.append(&et_bytes);
    buf.append(&ts_buf);
    buf.append(&meta_bytes);

    let hash = env.crypto().sha256(&buf);

    // Encode hash as lowercase hex string
    let hex_chars = b"0123456789abcdef";
    let mut hex_bytes = [0u8; 64];
    for (i, byte) in hash.to_array().iter().enumerate() {
        hex_bytes[i * 2] = hex_chars[(byte >> 4) as usize];
        hex_bytes[i * 2 + 1] = hex_chars[(byte & 0xf) as usize];
    }

    // Return the hex string representing the SHA-256 digest.
    String::from_bytes(env, &hex_bytes)
}

    /// Property 4: Count equals vec length
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn prop_count_equals_vec_len(product_id_str in "[a-z]{1,20}") {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register_contract(None, SupplyLinkContract);
            let client = SupplyLinkContractClient::new(&env, &contract_id);
            let owner = soroban_sdk::Address::generate(&env);
            let product_id = String::from_str(&env, &product_id_str);

            client.register_product(
                &product_id,
                &String::from_str(&env, "Widget"),
                &String::from_str(&env, "Origin"),
                &owner,
            );
            client.add_tracking_event(
                &product_id,
                &String::from_str(&env, "Farm"),
                &String::from_str(&env, "HARVEST"),
                &String::from_str(&env, "{}"),
            );
            let count = client.get_events_count(&product_id);
            let events = client.get_tracking_events(&product_id);
            prop_assert_eq!(count, events.len());
    /// Get pending events for a product.
    ///
    /// Returns all events awaiting multi-signature approval.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    ///
    /// # Returns
    /// A `Vec<PendingEvent>` containing all pending events for the product.
    ///
    /// # Authorization
    /// None — this is a read-only function.
    ///
    /// # Panics
    /// Does not panic.
    pub fn get_pending_events(env: Env, product_id: String) -> Vec<PendingEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Clean up expired pending events for a product.
    ///
    /// Removes all expired pending events from storage and emits a purge event
    /// for each removed entry (issue #314).
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product to clean up.
    ///
    /// # Returns
    /// Number of events purged.
    ///
    /// # Authorization
    /// None — this is a permissionless cleanup function.
    ///
    /// # Emitted Events
    /// Publishes `("pending_events_purged", product_id)` event with the count
    /// of purged events. Also publishes `("pending_event_purged", product_id)`
    /// for each individual removed event.
    pub fn cleanup_expired_events(env: Env, product_id: String) -> u32 {
        let mut pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let current_time = env.ledger().timestamp();
        let mut expired_count: u32 = 0;

        // Filter out expired events
        let mut valid_pending = Vec::new(&env);
        for i in 0..pending.len() {
            let event = pending.get(i).unwrap();
            if current_time <= event.expiration {
                valid_pending.push_back(event.clone());
            } else {
                expired_count += 1;

                // Emit event for each purged entry
                env.events().publish(
                    (Symbol::new(&env, "pending_event_purged"), product_id.clone()),
                    event.product_id.clone(),
                );
            }
        }

        if valid_pending.len() > 0 {
            env.storage()
                .persistent()
                .set(&DataKey::PendingEvents(product_id.clone()), &valid_pending);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::PendingEvents(product_id.clone()));
        }

        // Emit summary event
        env.events().publish(
            (Symbol::new(&env, "pending_events_purged"), product_id),
            expired_count,
        );

        expired_count
    }

    pub fn get_nonce(env: Env, actor: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ActorNonce(actor))
            .unwrap_or(0)
    }

    /// Get the stable pending event ID for a pending event at a given index.
    ///
    /// This function is provided for backward compatibility with clients that
    /// currently use index-based references. It bridges index-based lookups to
    /// stable IDs.
    ///
    /// # Parameters
    /// - `env` — Soroban execution environment.
    /// - `product_id` — ID of the product.
    /// - `event_index` — Zero-based index into the pending events queue.
    ///
    /// # Returns
    /// The stable `pending_event_id` of the event at that index, or panics if
    /// the index is out of bounds or no events exist.
    ///
    /// # Panics
    /// - `"no pending events"` — if there are no pending events.
    /// - `"event index out of bounds"` — if `event_index` is invalid.
    ///
    /// # Note
    /// This function should be called to convert existing index-based client code
    /// to use stable IDs. Direct index usage in approve_event/reject_event will
    /// no longer work; the stable ID must be obtained first.
    pub fn get_pending_event_id_at_index(
        env: Env,
        product_id: String,
        event_index: u32,
    ) -> u64 {
        let pending: Vec<PendingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::PendingEvents(product_id))
            .ok_or_else(|| panic!("no pending events"))?;

        if event_index >= pending.len() as u32 {
            panic!("event index out of bounds");
        }

        pending.get(event_index).unwrap().pending_event_id
    }

    /// Store or replace the compliance policy for a product.
    ///
    /// Only the product owner may call this function. Each rule in the policy
    /// is enforced on every subsequent `add_tracking_event` call.
    pub fn set_compliance_policy(
        env: Env,
        product_id: String,
        rules: Vec<ComplianceRule>,
    ) -> Result<CompliancePolicy, Error> {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .ok_or(Error::ProductNotFound)?;

        product.owner.require_auth();

        let policy = CompliancePolicy {
            product_id: product_id.clone(),
            rules,
        };

        env.storage()
            .persistent()
            .set(&DataKey::CompliancePolicy(product_id.clone()), &policy);

        env.events().publish(
            (Symbol::new(&env, "compliance_policy_set"), product_id),
            policy.clone(),
        );

        Ok(policy)
    }

    /// Retrieve the compliance policy for a product, if one has been set.
    pub fn get_compliance_policy(env: Env, product_id: String) -> Option<CompliancePolicy> {
        env.storage()
            .persistent()
            .get(&DataKey::CompliancePolicy(product_id))
    }

    fn check_compliance(
        env: &Env,
        product_id: &String,
        event_type: &String,
        current_time: u64,
    ) -> Result<(), Error> {
        let policy: CompliancePolicy = match env
            .storage()
            .persistent()
            .get(&DataKey::CompliancePolicy(product_id.clone()))
        {
            Some(p) => p,
            None => return Ok(()),
        };

        let events: Vec<TrackingEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::Events(product_id.clone()))
            .unwrap_or_else(|| Vec::new(env));

        for i in 0..policy.rules.len() {
            let rule = policy.rules.get(i).unwrap();

            if rule.to_stage != *event_type {
                continue;
            }

            if rule.rule_type == COMPLIANCE_REQUIRED_ORDER
                || rule.rule_type == COMPLIANCE_MANDATORY_INSPECTION
            {
                let mut found = false;
                for j in 0..events.len() {
                    if events.get(j).unwrap().event_type == rule.from_stage {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(Error::ComplianceViolation);
                }
            } else if rule.rule_type == COMPLIANCE_MAX_TIME_BETWEEN_STAGES {
                let mut last_from_time: Option<u64> = None;
                for j in 0..events.len() {
                    let ev = events.get(j).unwrap();
                    if ev.event_type == rule.from_stage {
                        last_from_time = Some(ev.timestamp);
                    }
                }
                if let Some(last_time) = last_from_time {
                    if current_time > last_time + rule.max_seconds {
                        return Err(Error::ComplianceViolation);
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_and_increment_nonce(env: &Env, actor: &Address, provided_nonce: u64) {
        let current_nonce: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ActorNonce(actor.clone()))
            .unwrap_or(0);

        if provided_nonce != current_nonce {
            panic!("invalid nonce");
        }

        env.storage()
            .persistent()
            .set(&DataKey::ActorNonce(actor.clone()), &(current_nonce + 1));
    }

    /// Issue a certification for a product. (#428)
    ///
    /// Stores a [`ProductCertification`] entry for the given product. Only the
    /// product owner or an authorized actor may call this function.
    ///
    /// # Parameters
    /// - `product_id` — ID of the product to certify.
    /// - `caller` — Address of the actor issuing the certification.
    /// - `cert_id` — Caller-supplied unique identifier for this certification.
    /// - `cert_type` — Certification type key (e.g. `"fair_trade"`, `"organic"`).
    ///
    /// # Authorization
    /// Requires `caller.require_auth()`. Caller must be owner or authorized actor.
    pub fn issue_certification(
        env: Env,
        product_id: String,
        caller: Address,
        cert_id: String,
        cert_type: String,
    ) -> ProductCertification {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .unwrap_or_else(|| panic!("product not found"));

        let is_owner = product.owner == caller;
        let is_actor = product.authorized_actors.contains(&caller);
        if !is_owner && !is_actor {
            panic!("caller is not authorized");
        }
        caller.require_auth();
        assert_len(&cert_id, 128, "cert_id");
        assert_len(&cert_type, 64, "cert_type");

        let cert = ProductCertification {
            id: cert_id.clone(),
            product_id: product_id.clone(),
            cert_type,
            issuer: caller,
            issued_at: env.ledger().timestamp(),
            revoked: false,
            revoked_at: 0,
        };

        let mut certs: Vec<ProductCertification> = env
            .storage()
            .persistent()
            .get(&DataKey::Certifications(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        certs.push_back(cert.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Certifications(product_id.clone()), &certs);

        env.events().publish(
            (Symbol::new(&env, "certification_issued"), product_id),
            cert.clone(),
        );

        cert
    }

    /// Revoke a previously issued certification. (#428)
    ///
    /// Sets the `revoked` flag on the matching certification entry.
    /// Only the product owner may revoke a certification.
    ///
    /// # Parameters
    /// - `product_id` — ID of the product whose certification to revoke.
    /// - `caller` — Must be the product owner.
    /// - `cert_id` — ID of the certification to revoke.
    pub fn revoke_certification(
        env: Env,
        product_id: String,
        caller: Address,
        cert_id: String,
    ) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .unwrap_or_else(|| panic!("product not found"));

        if product.owner != caller {
            panic!("only product owner can revoke certifications");
        }
        caller.require_auth();

        let mut certs: Vec<ProductCertification> = env
            .storage()
            .persistent()
            .get(&DataKey::Certifications(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        let mut updated: Vec<ProductCertification> = Vec::new(&env);
        for cert in certs.iter() {
            if cert.id == cert_id {
                let revoked_cert = ProductCertification {
                    revoked: true,
                    revoked_at: env.ledger().timestamp(),
                    ..cert.clone()
                };
                updated.push_back(revoked_cert.clone());
                env.events().publish(
                    (Symbol::new(&env, "certification_revoked"), product_id.clone()),
                    revoked_cert,
                );
                found = true;
            } else {
                updated.push_back(cert.clone());
            }
        }

        if found {
            env.storage()
                .persistent()
                .set(&DataKey::Certifications(product_id), &updated);
        }

        found
    }

    /// Return all certifications (active and revoked) for a product. (#428)
    pub fn list_certifications(
        env: Env,
        product_id: String,
    ) -> Vec<ProductCertification> {
        env.storage()
            .persistent()
            .get(&DataKey::Certifications(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }
}

#[cfg(test)]
mod rejection_reason_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_reject_event_with_reason() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-001");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        let reason = String::from_str(&env, "Invalid metadata format");

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2, &String::from_str(&env, "other"), &String::from_str(&env, "general"));
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Verify pending event exists
        let pending = client.get_pending_events(&product_id);
        assert_eq!(pending.len(), 1);

        // Reject with reason
        let result = client.reject_event(&product_id, &0, &owner, &reason, &1);
        assert_eq!(result, true);

        // Verify pending event was removed
        let pending = client.get_pending_events(&product_id);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_reject_event_with_empty_reason() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-002");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        let reason = String::from_str(&env, "");

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2, &String::from_str(&env, "other"), &String::from_str(&env, "general"));
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Reject with empty reason (should work)
        let result = client.reject_event(&product_id, &0, &owner, &reason, &1);
        assert_eq!(result, true);
    }

    #[test]
    #[should_panic(expected = "rejection reason too long")]
    fn test_reject_event_reason_too_long() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-003");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        
        // Create a reason longer than 256 characters
        let long_reason = String::from_str(&env, &"x".repeat(257));

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2, &String::from_str(&env, "other"), &String::from_str(&env, "general"));
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Try to reject with too long reason - should panic
        client.reject_event(&product_id, &0, &owner, &long_reason, &1);
    }

    #[test]
    fn test_reject_event_max_length_reason() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let actor = Address::generate(&env);
        let product_id = String::from_str(&env, "test-product-004");
        let name = String::from_str(&env, "Test Product");
        let origin = String::from_str(&env, "Test Origin");
        let location = String::from_str(&env, "Test Location");
        let event_type = String::from_str(&env, "HARVEST");
        let metadata = String::from_str(&env, "{}");
        
        // Create a reason exactly 256 characters (should work)
        let max_reason = String::from_str(&env, &"x".repeat(256));

        env.mock_all_auths();

        // Register product with multi-sig
        client.register_product(&product_id, &name, &origin, &owner, &2, &String::from_str(&env, "other"), &String::from_str(&env, "general"));
        client.add_authorized_actor(&product_id, &actor, &0);

        // Add pending event
        client.add_tracking_event(&product_id, &actor, &location, &event_type, &metadata);

        // Reject with max length reason (should work)
        let result = client.reject_event(&product_id, &0, &owner, &max_reason, &1);
        assert_eq!(result, true);
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

    // ── #460: Document hash anchoring ─────────────────────────────────────────

    /// Anchor an off-chain document hash on-chain for a product.
    ///
    /// The caller computes the SHA-256 hash of the document bytes off-chain
    /// and submits the hex-encoded digest here. The contract stores it
    /// immutably so it can be verified later via [`Self::verify_document_hash`].
    ///
    /// # Parameters
    /// - `product_id` — ID of the product this document belongs to.
    /// - `label` — Human-readable document label (max 256 bytes).
    /// - `hash` — Hex-encoded SHA-256 digest of the document (64 chars).
    /// - `caller` — Address anchoring the document; must be owner or authorized actor.
    ///
    /// # Returns
    /// The newly created [`DocumentAnchor`].
    ///
    /// # Authorization
    /// Requires `caller.require_auth()`.
    pub fn anchor_document_hash(
        env: Env,
        product_id: String,
        label: String,
        hash: String,
        caller: Address,
    ) -> DocumentAnchor {
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

        assert_len(&label, MAX_NAME_LEN, "label");
        // SHA-256 hex digest is exactly 64 chars
        if hash.len() != 64 {
            panic!("hash must be a 64-char hex-encoded SHA-256 digest");
        }

        let anchor = DocumentAnchor {
            product_id: product_id.clone(),
            label,
            hash,
            anchored_by: caller,
            anchored_at: env.ledger().timestamp(),
        };

        let mut anchors: Vec<DocumentAnchor> = env
            .storage()
            .persistent()
            .get(&DataKey::DocumentAnchors(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        anchors.push_back(anchor.clone());
        env.storage()
            .persistent()
            .set(&DataKey::DocumentAnchors(product_id.clone()), &anchors);

        env.events().publish(
            (Symbol::new(&env, "document_anchored"), product_id),
            anchor.clone(),
        );

        anchor
    }

    /// Verify whether a given hash matches any anchored document for a product.
    ///
    /// # Parameters
    /// - `product_id` — ID of the product to check.
    /// - `hash` — Hex-encoded SHA-256 digest to look up.
    ///
    /// # Returns
    /// `true` if the hash matches an existing anchor; `false` otherwise.
    pub fn verify_document_hash(env: Env, product_id: String, hash: String) -> bool {
        let anchors: Vec<DocumentAnchor> = env
            .storage()
            .persistent()
            .get(&DataKey::DocumentAnchors(product_id))
            .unwrap_or_else(|| Vec::new(&env));

        for i in 0..anchors.len() {
            if anchors.get(i).unwrap().hash == hash {
                return true;
            }
        }
        false
    }

    /// Return all document anchors for a product.
    pub fn get_document_anchors(env: Env, product_id: String) -> Vec<DocumentAnchor> {
        env.storage()
            .persistent()
            .get(&DataKey::DocumentAnchors(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ── Issue #503: Event timestamp certification ────────────────────────────

    /// Certify the timestamp of a tracking event.
    /// Only authorized certifiers can issue timestamp certifications.
    pub fn certify_event_timestamp(
        env: Env,
        product_id: String,
        event_stable_id: String,
        certifier: Address,
    ) -> EventTimestampCert {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let is_owner = product.owner == certifier;
        let is_actor = product.authorized_actors.contains(&certifier);
        if !is_owner && !is_actor {
            panic!("certifier is not authorized");
        }
        certifier.require_auth();

        let cert_id = format_cert_id(&env, &product_id, &event_stable_id);
        let now = env.ledger().timestamp();

        let cert = EventTimestampCert {
            id: cert_id.clone(),
            product_id: product_id.clone(),
            event_stable_id,
            certified_timestamp: now,
            certifier,
            issued_at: now,
            revoked: false,
        };

        let mut certs: Vec<EventTimestampCert> = env
            .storage()
            .persistent()
            .get(&DataKey::EventTimestampCerts(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        certs.push_back(cert.clone());
        env.storage()
            .persistent()
            .set(&DataKey::EventTimestampCerts(product_id.clone()), &certs);

        env.events().publish(
            (Symbol::new(&env, "event_timestamp_certified"), product_id),
            cert.clone(),
        );

        cert
    }

    /// Get all timestamp certifications for a product.
    pub fn get_event_timestamp_certs(env: Env, product_id: String) -> Vec<EventTimestampCert> {
        env.storage()
            .persistent()
            .get(&DataKey::EventTimestampCerts(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Revoke a timestamp certification.
    pub fn revoke_event_timestamp_cert(
        env: Env,
        product_id: String,
        cert_id: String,
        revoker: Address,
    ) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        let mut certs: Vec<EventTimestampCert> = env
            .storage()
            .persistent()
            .get(&DataKey::EventTimestampCerts(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for i in 0..certs.len() {
            let mut cert = certs.get(i).unwrap().clone();
            if cert.id == cert_id {
                cert.revoked = true;
                certs.set(i, cert);
                found = true;
                break;
            }
        }

        if found {
            env.storage()
                .persistent()
                .set(&DataKey::EventTimestampCerts(product_id.clone()), &certs);
            env.events().publish(
                (Symbol::new(&env, "event_timestamp_cert_revoked"), product_id),
                cert_id,
            );
        }

        found
    }

    // ── Issue #504: Product provenance proof notarization ────────────────────

    /// Notarize a product's provenance proof.
    /// Only authorized notaries can issue notarizations.
    pub fn notarize_provenance(
        env: Env,
        product_id: String,
        proof_hash: String,
        notary: Address,
        expires_at: u64,
    ) -> ProvenanceNotarization {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        let is_owner = product.owner == notary;
        let is_actor = product.authorized_actors.contains(&notary);
        if !is_owner && !is_actor {
            panic!("notary is not authorized");
        }
        notary.require_auth();

        // Proof hash should be 64 chars (SHA-256 hex)
        if proof_hash.len() != 64 {
            panic!("proof_hash must be a 64-char hex-encoded SHA-256 digest");
        }

        let notarization_id = format_notarization_id(&env, &product_id);
        let now = env.ledger().timestamp();

        let notarization = ProvenanceNotarization {
            id: notarization_id.clone(),
            product_id: product_id.clone(),
            proof_hash,
            notary,
            notarized_at: now,
            expires_at,
            revoked: false,
        };

        let mut notarizations: Vec<ProvenanceNotarization> = env
            .storage()
            .persistent()
            .get(&DataKey::ProvenanceNotarizations(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        notarizations.push_back(notarization.clone());
        env.storage()
            .persistent()
            .set(&DataKey::ProvenanceNotarizations(product_id.clone()), &notarizations);

        env.events().publish(
            (Symbol::new(&env, "provenance_notarized"), product_id),
            notarization.clone(),
        );

        notarization
    }

    /// Get all provenance notarizations for a product.
    pub fn get_provenance_notarizations(env: Env, product_id: String) -> Vec<ProvenanceNotarization> {
        env.storage()
            .persistent()
            .get(&DataKey::ProvenanceNotarizations(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Revoke a provenance notarization.
    pub fn revoke_provenance_notarization(
        env: Env,
        product_id: String,
        notarization_id: String,
    ) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        let mut notarizations: Vec<ProvenanceNotarization> = env
            .storage()
            .persistent()
            .get(&DataKey::ProvenanceNotarizations(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for i in 0..notarizations.len() {
            let mut notarization = notarizations.get(i).unwrap().clone();
            if notarization.id == notarization_id {
                notarization.revoked = true;
                notarizations.set(i, notarization);
                found = true;
                break;
            }
        }

        if found {
            env.storage()
                .persistent()
                .set(&DataKey::ProvenanceNotarizations(product_id.clone()), &notarizations);
            env.events().publish(
                (Symbol::new(&env, "provenance_notarization_revoked"), product_id),
                notarization_id,
            );
        }

        found
    }

    // ── Issue #505: Supply chain event certification workflows ──────────────

    /// Register a new certifier.
    pub fn register_certifier(
        env: Env,
        id: String,
        address: Address,
        name: String,
        cert_types: Vec<String>,
    ) -> Certifier {
        address.require_auth();

        let certifier = Certifier {
            id: id.clone(),
            address,
            name,
            cert_types,
            registered_at: env.ledger().timestamp(),
            active: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Certifier(id.clone()), &certifier);

        let mut certifier_ids: Vec<String> = env
            .storage()
            .persistent()
            .get(&DataKey::CertifierIndex)
            .unwrap_or_else(|| Vec::new(&env));
        certifier_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::CertifierIndex, &certifier_ids);

        env.events().publish(
            (Symbol::new(&env, "certifier_registered"), id),
            certifier.clone(),
        );

        certifier
    }

    /// Get a certifier by ID.
    pub fn get_certifier(env: Env, id: String) -> Option<Certifier> {
        env.storage()
            .persistent()
            .get(&DataKey::Certifier(id))
    }

    /// Certify a supply chain event.
    pub fn certify_event(
        env: Env,
        product_id: String,
        event_stable_id: String,
        cert_type: String,
        certifier_id: String,
        metadata: String,
    ) -> EventCertification {
        let certifier: Certifier = env
            .storage()
            .persistent()
            .get(&DataKey::Certifier(certifier_id.clone()))
            .expect("certifier not found");

        if !certifier.active {
            panic!("certifier is not active");
        }

        certifier.address.require_auth();

        // Verify certifier is authorized for this cert type
        let mut authorized = false;
        for i in 0..certifier.cert_types.len() {
            if certifier.cert_types.get(i).unwrap() == cert_type {
                authorized = true;
                break;
            }
        }
        if !authorized {
            panic!("certifier is not authorized for this certification type");
        }

        let cert_id = format_event_cert_id(&env, &product_id, &event_stable_id, &cert_type);
        let now = env.ledger().timestamp();

        let certification = EventCertification {
            id: cert_id.clone(),
            product_id: product_id.clone(),
            event_stable_id,
            cert_type,
            certifier: certifier.address,
            metadata,
            issued_at: now,
            revoked: false,
            revoked_at: 0,
        };

        let mut certifications: Vec<EventCertification> = env
            .storage()
            .persistent()
            .get(&DataKey::EventCertifications(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        certifications.push_back(certification.clone());
        env.storage()
            .persistent()
            .set(&DataKey::EventCertifications(product_id.clone()), &certifications);

        env.events().publish(
            (Symbol::new(&env, "event_certified"), product_id),
            certification.clone(),
        );

        certification
    }

    /// Get all event certifications for a product.
    pub fn get_event_certifications(env: Env, product_id: String) -> Vec<EventCertification> {
        env.storage()
            .persistent()
            .get(&DataKey::EventCertifications(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Revoke an event certification.
    pub fn revoke_event_certification(
        env: Env,
        product_id: String,
        cert_id: String,
    ) -> bool {
        let product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        product.owner.require_auth();

        let mut certifications: Vec<EventCertification> = env
            .storage()
            .persistent()
            .get(&DataKey::EventCertifications(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for i in 0..certifications.len() {
            let mut cert = certifications.get(i).unwrap().clone();
            if cert.id == cert_id {
                cert.revoked = true;
                cert.revoked_at = env.ledger().timestamp();
                certifications.set(i, cert);
                found = true;
                break;
            }
        }

        if found {
            env.storage()
                .persistent()
                .set(&DataKey::EventCertifications(product_id.clone()), &certifications);
            env.events().publish(
                (Symbol::new(&env, "event_certification_revoked"), product_id),
                cert_id,
            );
        }

        found
    }

    // ── Issue #506: AI-assisted anomaly review assistant ────────────────────

    /// Report an anomaly detected in a product's supply chain.
    pub fn report_anomaly(
        env: Env,
        product_id: String,
        anomaly_type: String,
        severity: u32,
        description: String,
        suggested_actions: String,
    ) -> AnomalyReport {
        let _product: Product = env
            .storage()
            .persistent()
            .get(&DataKey::Product(product_id.clone()))
            .expect("product not found");

        if severity < 1 || severity > 4 {
            panic!("severity must be between 1 and 4");
        }

        let report_id = format_anomaly_id(&env, &product_id);
        let now = env.ledger().timestamp();

        let report = AnomalyReport {
            id: report_id.clone(),
            product_id: product_id.clone(),
            anomaly_type,
            severity,
            description,
            suggested_actions,
            detected_at: now,
            reviewed: false,
            reviewed_by: Address::from_contract_id(&env, &env.current_contract_address()),
            reviewed_at: 0,
        };

        let mut reports: Vec<AnomalyReport> = env
            .storage()
            .persistent()
            .get(&DataKey::AnomalyReports(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        reports.push_back(report.clone());
        env.storage()
            .persistent()
            .set(&DataKey::AnomalyReports(product_id.clone()), &reports);

        env.events().publish(
            (Symbol::new(&env, "anomaly_reported"), product_id),
            report.clone(),
        );

        report
    }

    /// Get all anomaly reports for a product.
    pub fn get_anomaly_reports(env: Env, product_id: String) -> Vec<AnomalyReport> {
        env.storage()
            .persistent()
            .get(&DataKey::AnomalyReports(product_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Mark an anomaly report as reviewed by an analyst.
    pub fn review_anomaly(
        env: Env,
        product_id: String,
        report_id: String,
        analyst: Address,
    ) -> bool {
        analyst.require_auth();

        let mut reports: Vec<AnomalyReport> = env
            .storage()
            .persistent()
            .get(&DataKey::AnomalyReports(product_id.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for i in 0..reports.len() {
            let mut report = reports.get(i).unwrap().clone();
            if report.id == report_id {
                report.reviewed = true;
                report.reviewed_by = analyst.clone();
                report.reviewed_at = env.ledger().timestamp();
                reports.set(i, report);
                found = true;
                break;
            }
        }

        if found {
            env.storage()
                .persistent()
                .set(&DataKey::AnomalyReports(product_id.clone()), &reports);
            env.events().publish(
                (Symbol::new(&env, "anomaly_reviewed"), product_id),
                report_id,
            );
        }

        found
    }
}

// ── Helper functions ─────────────────────────────────────────────────────────

fn format_cert_id(env: &Env, product_id: &String, event_stable_id: &String) -> String {
    let mut result = String::from_str(env, "cert_");
    result.append(&product_id.clone());
    result.append(&String::from_str(env, "_"));
    result.append(&event_stable_id.clone());
    result
}

fn format_notarization_id(env: &Env, product_id: &String) -> String {
    let mut result = String::from_str(env, "notary_");
    result.append(&product_id.clone());
    result
}

fn format_event_cert_id(env: &Env, product_id: &String, event_stable_id: &String, cert_type: &String) -> String {
    let mut result = String::from_str(env, "eventcert_");
    result.append(&product_id.clone());
    result.append(&String::from_str(env, "_"));
    result.append(&event_stable_id.clone());
    result.append(&String::from_str(env, "_"));
    result.append(&cert_type.clone());
    result
}

fn format_anomaly_id(env: &Env, product_id: &String) -> String {
    let mut result = String::from_str(env, "anomaly_");
    result.append(&product_id.clone());
    result
}
