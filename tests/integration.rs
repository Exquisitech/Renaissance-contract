#![cfg(test)]

use renaissance_contract::{SessionContract, SessionContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, Symbol,
};

fn make_env(timestamp: u64) -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(timestamp);
    env
}

#[test]
fn integration_full_session_lifecycle() {
    let env = make_env(2_000_000);
    let contract_id = env.register_contract(None, SessionContract);
    let client = SessionContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let session_key = Address::generate(&env);
    let expires_at = 2_000_000 + 86_400; // 24 hours
    let allowed = vec![
        &env,
        Symbol::new(&env, "stake"),
        Symbol::new(&env, "vote"),
    ];

    // Create
    client.create_session(&user, &session_key, &expires_at, &allowed);

    // Valid during session
    assert!(client.validate_session(&session_key, &Symbol::new(&env, "stake")));
    assert!(client.validate_session(&session_key, &Symbol::new(&env, "vote")));
    assert!(!client.validate_session(&session_key, &Symbol::new(&env, "transfer")));

    // Revoke
    client.revoke_session(&user, &session_key);
    assert!(!client.validate_session(&session_key, &Symbol::new(&env, "stake")));
}

#[test]
fn integration_session_expires_by_timestamp() {
    let env = make_env(2_000_000);
    let contract_id = env.register_contract(None, SessionContract);
    let client = SessionContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let session_key = Address::generate(&env);
    let allowed = vec![&env, Symbol::new(&env, "stake")];

    client.create_session(&user, &session_key, &(2_000_000 + 3_600), &allowed);
    assert!(client.validate_session(&session_key, &Symbol::new(&env, "stake")));

    env.ledger().set_timestamp(2_000_000 + 3_601);
    assert!(!client.validate_session(&session_key, &Symbol::new(&env, "stake")));
}
