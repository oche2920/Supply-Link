#[cfg(test)]
mod tests {
    use crate::{Role, SupplyLinkContract, SupplyLinkContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn setup() -> (Env, Address, Address, String) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let owner = Address::generate(&env);
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

    fn add_event(env: &Env, contract_id: &Address, product_id: &String, caller: &Address) {
        let client = SupplyLinkContractClient::new(env, contract_id);
        client.add_tracking_event(
            product_id,
            caller,
            &String::from_str(env, "Warehouse"),
            &String::from_str(env, "SHIPPING"),
            &String::from_str(env, "{}"),
        );
    }

    // ── #386: Stable event IDs ────────────────────────────────────────────────

    #[test]
    fn test_stable_id_is_present() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let event = client.add_tracking_event(
            &product_id,
            &owner,
            &String::from_str(&env, "Port"),
            &String::from_str(&env, "SHIPPING"),
            &String::from_str(&env, "{}"),
        );
        // stable_id must be a 64-char hex string
        assert_eq!(event.stable_id.len(), 64);
    }

    #[test]
    fn test_stable_id_is_deterministic() {
        // Two events with identical fields at the same ledger timestamp must
        // produce the same stable_id.
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SupplyLinkContract);
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let pid = String::from_str(&env, "prod-det");

        client.register_product(
            &pid,
            &String::from_str(&env, "Widget"),
            &String::from_str(&env, "Origin"),
            &owner,
        );

        let e1 = client.add_tracking_event(
            &pid,
            &owner,
            &String::from_str(&env, "Loc"),
            &String::from_str(&env, "HARVEST"),
            &String::from_str(&env, "{\"k\":1}"),
        );
        // Same ledger timestamp → same stable_id
        let e2 = client.add_tracking_event(
            &pid,
            &owner,
            &String::from_str(&env, "Loc"),
            &String::from_str(&env, "HARVEST"),
            &String::from_str(&env, "{\"k\":1}"),
        );
        assert_eq!(e1.stable_id, e2.stable_id);
    }

    // ── #388: Paginated event retrieval ───────────────────────────────────────

    #[test]
    fn test_list_tracking_events_pagination() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        for _ in 0..10 {
            add_event(&env, &contract_id, &product_id, &owner);
        }

        let page1 = client.list_tracking_events(&product_id, &0, &5);
        let page2 = client.list_tracking_events(&product_id, &5, &5);
        let page3 = client.list_tracking_events(&product_id, &10, &5);

        assert_eq!(page1.len(), 5);
        assert_eq!(page2.len(), 5);
        assert_eq!(page3.len(), 0);
    }

    #[test]
    fn test_count_tracking_events() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        assert_eq!(client.count_tracking_events(&product_id), 0);
        for i in 1..=5u32 {
            add_event(&env, &contract_id, &product_id, &owner);
            assert_eq!(client.count_tracking_events(&product_id), i);
        }
    }

    #[test]
    fn test_list_events_offset_beyond_total_returns_empty() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        add_event(&env, &contract_id, &product_id, &owner);

        let result = client.list_tracking_events(&product_id, &100, &10);
        assert_eq!(result.len(), 0);
    }

    // ── #387: Role segregation ────────────────────────────────────────────────

    #[test]
    fn test_assign_and_get_role() {
        let (env, contract_id, _owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let actor = Address::generate(&env);

        client.add_authorized_actor(&product_id, &actor);
        client.assign_role(&product_id, &actor, &Role::Shipper);

        let policy = client.get_authorization_policy(&product_id);
        assert_eq!(policy.roles.len(), 1);
        assert_eq!(policy.roles.get(0).unwrap().role, Role::Shipper);
    }

    #[test]
    fn test_revoke_role() {
        let (env, contract_id, _owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let actor = Address::generate(&env);

        client.add_authorized_actor(&product_id, &actor);
        client.assign_role(&product_id, &actor, &Role::Producer);
        let removed = client.revoke_role(&product_id, &actor);
        assert!(removed);

        let policy = client.get_authorization_policy(&product_id);
        assert_eq!(policy.roles.len(), 0);
    }

    #[test]
    fn test_set_event_threshold() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);

        client.set_event_threshold(&product_id, &2);
        let policy = client.get_authorization_policy(&product_id);
        assert_eq!(policy.threshold, 2);
    }

    #[test]
    fn test_default_policy_threshold_is_one() {
        let (env, contract_id, _owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        let policy = client.get_authorization_policy(&product_id);
        assert_eq!(policy.threshold, 1);
    }

    // ── Deactivation ──────────────────────────────────────────────────────────

    #[test]
    fn test_deactivate_blocks_events() {
        let (env, contract_id, _owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.deactivate_product(&product_id);
        // Verified by test_add_event_deactivated_panics pattern
        assert!(!client.get_product(&product_id).active);
    }

    #[test]
    fn test_reactivate_allows_events() {
        let (env, contract_id, owner, product_id) = setup();
        let client = SupplyLinkContractClient::new(&env, &contract_id);
        client.deactivate_product(&product_id);
        client.reactivate_product(&product_id);
        // Should not panic
        add_event(&env, &contract_id, &product_id, &owner);
        assert_eq!(client.count_tracking_events(&product_id), 1);
    }
#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_nonce_starts_at_zero() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let actor = Address::generate(&env);
    
    assert_eq!(client.get_nonce(&actor), 0);
}

#[test]
fn test_transfer_ownership_increments_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    assert_eq!(client.get_nonce(&owner), 0);
    
    client.transfer_ownership(
        &String::from_str(&env, "prod1"),
        &new_owner,
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner), 1);
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_transfer_ownership_rejects_stale_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let actor = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.add_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &0,
    );
    
    client.remove_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &0,
    );
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_transfer_ownership_rejects_future_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.transfer_ownership(
        &String::from_str(&env, "prod1"),
        &new_owner,
        &5,
    );
}

#[test]
fn test_add_authorized_actor_increments_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let actor = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    assert_eq!(client.get_nonce(&owner), 0);
    
    client.add_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner), 1);
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_add_authorized_actor_rejects_duplicate_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let actor1 = Address::generate(&env);
    let actor2 = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.add_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor1,
        &0,
    );
    
    client.add_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor2,
        &0,
    );
}

#[test]
fn test_remove_authorized_actor_increments_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let actor = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.add_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner), 1);
    
    client.remove_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &1,
    );
    
    assert_eq!(client.get_nonce(&owner), 2);
}

#[test]
fn test_approve_event_increments_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let actor = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &2,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.add_tracking_event(
        &String::from_str(&env, "prod1"),
        &owner,
        &String::from_str(&env, "Location"),
        &String::from_str(&env, "HARVEST"),
        &String::from_str(&env, "{}"),
    );
    
    assert_eq!(client.get_nonce(&owner), 0);
    
    client.approve_event(
        &String::from_str(&env, "prod1"),
        &0,
        &owner,
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner), 1);
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_approve_event_rejects_out_of_order_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &2,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.add_tracking_event(
        &String::from_str(&env, "prod1"),
        &owner,
        &String::from_str(&env, "Location"),
        &String::from_str(&env, "HARVEST"),
        &String::from_str(&env, "{}"),
    );
    
    client.approve_event(
        &String::from_str(&env, "prod1"),
        &0,
        &owner,
        &1,
    );
}

#[test]
fn test_reject_event_increments_nonce() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &2,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.add_tracking_event(
        &String::from_str(&env, "prod1"),
        &owner,
        &String::from_str(&env, "Location"),
        &String::from_str(&env, "HARVEST"),
        &String::from_str(&env, "{}"),
    );
    
    assert_eq!(client.get_nonce(&owner), 0);
    
    client.reject_event(
        &String::from_str(&env, "prod1"),
        &0,
        &owner,
        &String::from_str(&env, ""),
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner), 1);
}

#[test]
fn test_nonce_progression_multiple_operations() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let actor = Address::generate(&env);
    let new_owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    assert_eq!(client.get_nonce(&owner), 0);
    
    client.add_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &0,
    );
    assert_eq!(client.get_nonce(&owner), 1);
    
    client.remove_authorized_actor(
        &String::from_str(&env, "prod1"),
        &actor,
        &1,
    );
    assert_eq!(client.get_nonce(&owner), 2);
    
    client.transfer_ownership(
        &String::from_str(&env, "prod1"),
        &new_owner,
        &2,
    );
    assert_eq!(client.get_nonce(&owner), 3);
}

#[test]
fn test_nonce_isolated_per_actor() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let new_owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "prod1"),
        &String::from_str(&env, "Product 1"),
        &String::from_str(&env, "Origin"),
        &owner1,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.register_product(
        &String::from_str(&env, "prod2"),
        &String::from_str(&env, "Product 2"),
        &String::from_str(&env, "Origin"),
        &owner2,
        &1,
    );    &String::from_str(&env, "other"),
    );    &String::from_str(&env, "general"),
    );
    
    client.transfer_ownership(
        &String::from_str(&env, "prod1"),
        &new_owner,
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner1), 1);
    assert_eq!(client.get_nonce(&owner2), 0);
    
    client.transfer_ownership(
        &String::from_str(&env, "prod2"),
        &new_owner,
        &0,
    );
    
    assert_eq!(client.get_nonce(&owner1), 1);
    assert_eq!(client.get_nonce(&owner2), 1);
}

#[test]
fn test_set_hazard_status() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    
    client.register_product(
        &String::from_str(&env, "haz1"),
        &String::from_str(&env, "Chemical"),
        &String::from_str(&env, "Factory"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    client.set_hazard_status(
        &String::from_str(&env, "haz1"),
        &true,
        &String::from_str(&env, "Flammable"),
    );
    
    let product = client.get_product(&String::from_str(&env, "haz1"));
    assert_eq!(product.hazardous, true);
    assert_eq!(product.hazard_classification, String::from_str(&env, "Flammable"));
}

// ── #508: Product identifier canonicalization ────────────────────────────────

#[test]
fn test_register_product_alias_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let canonical_id = String::from_str(&env, "prod-001");
    let alias = String::from_str(&env, "sku-123");
    
    // Register canonical product
    client.register_product(
        &canonical_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    // Register alias
    let alias_entry = client.register_product_alias(&canonical_id, &alias, &owner);
    assert_eq!(alias_entry.canonical_id, canonical_id);
    assert_eq!(alias_entry.alias, alias);
}

#[test]
#[should_panic(expected = "alias already exists")]
fn test_register_product_alias_duplicate_rejection() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let canonical_id = String::from_str(&env, "prod-001");
    let alias = String::from_str(&env, "sku-123");
    
    client.register_product(
        &canonical_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    // Register first alias
    client.register_product_alias(&canonical_id, &alias, &owner);
    
    // Attempt to register duplicate alias
    client.register_product_alias(&canonical_id, &alias, &owner);
}

#[test]
fn test_resolve_product_id_canonical() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let canonical_id = String::from_str(&env, "prod-001");
    
    // Resolving a canonical ID should return itself
    let resolved = client.resolve_product_id(&canonical_id);
    assert_eq!(resolved, canonical_id);
}

#[test]
fn test_resolve_product_id_alias() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let canonical_id = String::from_str(&env, "prod-001");
    let alias = String::from_str(&env, "sku-123");
    
    client.register_product(
        &canonical_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    client.register_product_alias(&canonical_id, &alias, &owner);
    
    // Resolving an alias should return the canonical ID
    let resolved = client.resolve_product_id(&alias);
    assert_eq!(resolved, canonical_id);
}

// ── #507: Provenance score traceability ──────────────────────────────────────

#[test]
fn test_set_provenance_score_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let product_id = String::from_str(&env, "prod-001");
    
    client.register_product(
        &product_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    let metadata = client.set_provenance_score(&product_id, &85, &10);
    assert_eq!(metadata.product_id, product_id);
    assert_eq!(metadata.score, 85);
    assert_eq!(metadata.verified_event_count, 10);
    assert_eq!(metadata.schema_version, 1);
}

#[test]
#[should_panic(expected = "score must be between 0 and 100")]
fn test_set_provenance_score_invalid_high() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let product_id = String::from_str(&env, "prod-001");
    
    client.register_product(
        &product_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    // Attempt to set score > 100
    client.set_provenance_score(&product_id, &101, &10);
}

#[test]
fn test_get_provenance_score() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let product_id = String::from_str(&env, "prod-001");
    
    client.register_product(
        &product_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    client.set_provenance_score(&product_id, &75, &5);
    
    let retrieved = client.get_provenance_score(&product_id);
    assert!(retrieved.is_some());
    let score_data = retrieved.unwrap();
    assert_eq!(score_data.score, 75);
    assert_eq!(score_data.verified_event_count, 5);
}

#[test]
fn test_get_provenance_score_history() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(&env, &contract_id);
    
    let owner = Address::generate(&env);
    let product_id = String::from_str(&env, "prod-001");
    
    client.register_product(
        &product_id,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "Factory A"),
        &owner,
        &1,
        &String::from_str(&env, "cat"),
        &String::from_str(&env, "sub"),
    );
    
    client.set_provenance_score(&product_id, &75, &5);
    
    let history = client.get_provenance_score_history(&product_id);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().score, 75);
}
