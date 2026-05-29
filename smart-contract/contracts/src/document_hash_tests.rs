//! Tests for document hash anchoring and verification (#460).
#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (SupplyLinkContractClient, Address, String) {
    let contract_id = env.register_contract(None, SupplyLinkContract);
    let client = SupplyLinkContractClient::new(env, &contract_id);
    let owner = Address::generate(env);
    let product_id = String::from_str(env, "prod-doc-01");
    env.mock_all_auths();
    client.register_product(
        &product_id,
        &String::from_str(env, "Doc Product"),
        &String::from_str(env, "Origin"),
        &owner,
        &1,
    );
    (client, owner, product_id)
}

/// A valid 64-char hex SHA-256 digest (all zeros for test purposes).
fn zero_hash(env: &Env) -> String {
    String::from_str(env, "0000000000000000000000000000000000000000000000000000000000000000")
}

fn alt_hash(env: &Env) -> String {
    String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

#[test]
fn test_anchor_document_hash_stores_anchor() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, owner, product_id) = setup(&env);

    let anchor = client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Certificate of Origin"),
        &zero_hash(&env),
        &owner,
    );

    assert_eq!(anchor.product_id, product_id);
    assert_eq!(anchor.hash, zero_hash(&env));
    assert_eq!(anchor.label, String::from_str(&env, "Certificate of Origin"));
    assert_eq!(anchor.anchored_by, owner);
}

#[test]
fn test_verify_document_hash_returns_true_for_anchored_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, owner, product_id) = setup(&env);

    client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Invoice"),
        &zero_hash(&env),
        &owner,
    );

    assert!(client.verify_document_hash(&product_id, &zero_hash(&env)));
}

#[test]
fn test_verify_document_hash_returns_false_for_unknown_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, owner, product_id) = setup(&env);

    client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Invoice"),
        &zero_hash(&env),
        &owner,
    );

    assert!(!client.verify_document_hash(&product_id, &alt_hash(&env)));
}

#[test]
fn test_get_document_anchors_returns_all_anchors() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, owner, product_id) = setup(&env);

    client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Doc A"),
        &zero_hash(&env),
        &owner,
    );
    client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Doc B"),
        &alt_hash(&env),
        &owner,
    );

    let anchors = client.get_document_anchors(&product_id);
    assert_eq!(anchors.len(), 2);
}

#[test]
fn test_get_document_anchors_empty_for_unknown_product() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _owner, _product_id) = setup(&env);

    let anchors = client.get_document_anchors(&String::from_str(&env, "nonexistent"));
    assert_eq!(anchors.len(), 0);
}

#[test]
fn test_verify_document_hash_false_for_no_anchors() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _owner, product_id) = setup(&env);

    assert!(!client.verify_document_hash(&product_id, &zero_hash(&env)));
}

#[test]
#[should_panic(expected = "caller is not authorized")]
fn test_anchor_document_hash_rejects_unauthorized_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _owner, product_id) = setup(&env);
    let stranger = Address::generate(&env);

    client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Fake Doc"),
        &zero_hash(&env),
        &stranger,
    );
}

#[test]
#[should_panic(expected = "hash must be a 64-char hex-encoded SHA-256 digest")]
fn test_anchor_document_hash_rejects_invalid_hash_length() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, owner, product_id) = setup(&env);

    client.anchor_document_hash(
        &product_id,
        &String::from_str(&env, "Bad Hash"),
        &String::from_str(&env, "tooshort"),
        &owner,
    );
}
