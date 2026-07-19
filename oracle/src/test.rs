#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{testutils::AddressEnvTestUtils, Address, Env, Vec};

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1);
    oracles.push_back(oracle2);

    // Register the contract
    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    // Initialize
    client.initialize(&admin, &oracles);

    // Check that oracles are correctly set
    let stored_oracles = client.get_oracles();
    assert_eq!(stored_oracles.len(), 2);
    assert!(stored_oracles.contains(&oracle1));
    assert!(stored_oracles.contains(&oracle2));
}

#[test]
#[should_panic(expected = "InternalError")]
fn test_initialize_insufficient_oracles() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1); // Only one oracle - should fail

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);
}

#[test]
fn test_submit_result_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1.clone());
    oracles.push_back(oracle2.clone());

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    // Submit a valid result
    let match_id = 123;
    let started_at = now - 3600; // 1 hour ago
    let finished_at = now - 600; // 10 minutes ago

    oracle1.require_auth();
    client.submit_result(&oracle1, &match_id, &2, &1, &started_at, &finished_at);

    // Result shouldn't be finalized yet (needs second confirmation)
    assert!(!client.is_finalized(&match_id));
}

#[test]
#[should_panic(expected = "OracleUnauthorized")]
fn test_submit_result_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1);
    oracles.push_back(oracle2);

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    // Unauthorized address tries to submit
    let match_id = 123;
    client.submit_result(&unauthorized, &match_id, &2, &1, &(now - 3600), &(now - 600));
}

#[test]
#[should_panic(expected = "InvalidMatchWindow")]
fn test_submit_result_late_submission() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1.clone());
    oracles.push_back(oracle2.clone());

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    // Try to submit with finished_at in the future
    let match_id = 123;
    let started_at = now + 3600; // Starts in 1 hour
    let finished_at = now + 7200; // Finishes in 2 hours

    oracle1.require_auth();
    client.submit_result(&match_id, &2, &1, &started_at, &finished_at);
}

#[test]
#[should_panic(expected = "ResultAlreadyExists")]
fn test_double_submit() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1.clone());
    oracles.push_back(oracle2.clone());

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    let match_id = 123;
    let started_at = now - 3600;
    let finished_at = now - 600;

    // First submission from oracle1
    oracle1.require_auth();
    client.submit_result(&oracle1, &match_id, &2, &1, &started_at, &finished_at);

    // Second submission from oracle2 for the same match - should fail
    oracle2.require_auth();
    client.submit_result(&oracle2, &match_id, &3, &1, &started_at, &finished_at);
}

#[test]
fn test_confirm_and_finalize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1.clone());
    oracles.push_back(oracle2.clone());

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    let match_id = 123;
    let started_at = now - 3600;
    let finished_at = now - 600;

    // Submit from oracle1
    oracle1.require_auth();
    client.submit_result(&oracle1, &match_id, &2, &1, &started_at, &finished_at);

    // Confirm from oracle2 - this should finalize
    oracle2.require_auth();
    client.confirm_result(&oracle2, &match_id);

    // Check if finalized
    assert!(client.is_finalized(&match_id));

    // Get the result and verify
    let result = client.get_result(&match_id);
    assert_eq!(result.home_score, 2);
    assert_eq!(result.away_score, 1);
    assert_eq!(result.match_id, match_id);
    assert!(result.finalized);
}

#[test]
#[should_panic(expected = "CannotConfirmOwnSubmission")]
fn test_cannot_confirm_own_submission() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1.clone());
    oracles.push_back(oracle2.clone());

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    let match_id = 123;
    let started_at = now - 3600;
    let finished_at = now - 600;

    // Submit from oracle1
    oracle1.require_auth();
    client.submit_result(&oracle1, &match_id, &2, &1, &started_at, &finished_at);

    // Try to confirm own submission - should fail
    oracle1.require_auth();
    client.confirm_result(&oracle1, &match_id);
}

#[test]
#[should_panic(expected = "AlreadyConfirmed")]
fn test_double_confirm() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);

    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle1.clone());
    oracles.push_back(oracle2.clone());
    oracles.push_back(oracle3.clone());

    let contract_id = env.register_contract(None, FootballOracleContract);
    let client = FootballOracleContractClient::new(&env, &contract_id);

    client.initialize(&admin, &oracles);

    // Set ledger timestamp
    let now = 1620000000;
    env.ledger().set_timestamp(now);

    let match_id = 123;
    let started_at = now - 3600;
    let finished_at = now - 600;

    // Submit from oracle1
    oracle1.require_auth();
    client.submit_result(&oracle1, &match_id, &2, &1, &started_at, &finished_at);

    // Confirm from oracle2
    oracle2.require_auth();
    client.confirm_result(&oracle2, &match_id);

    // Try to confirm again from oracle2 - should fail
    oracle2.require_auth();
    client.confirm_result(&oracle2, &match_id);
}
