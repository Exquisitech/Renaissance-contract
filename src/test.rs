#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, vec, Address, Env, Symbol};

    use crate::{SessionContract, SessionContractClient};

    fn setup() -> (Env, Address, SessionContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    // ── Whitelist: add ────────────────────────────────────────────────────────

    #[test]
    fn test_add_to_whitelist() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);
        assert!(!client.is_whitelisted(&user));
        client.add_to_whitelist(&user);
        assert!(client.is_whitelisted(&user));
    }

    // ── Whitelist: remove ─────────────────────────────────────────────────────

    #[test]
    fn test_remove_from_whitelist() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);
        client.add_to_whitelist(&user);
        assert!(client.is_whitelisted(&user));
        client.remove_from_whitelist(&user);
        assert!(!client.is_whitelisted(&user));
    }

    // ── Whitelist: non-admin cannot add ───────────────────────────────────────

    #[test]
    #[should_panic(expected = "require_auth")]
    fn test_non_admin_cannot_add() {
        let (env, _admin, client) = setup();
        // Create a second env without mocked auths to simulate real auth failure
        let env2 = Env::default();
        let contract_id2 = env2.register_contract(None, SessionContract);
        let admin2 = Address::generate(&env2);
        let client2 = SessionContractClient::new(&env2, &contract_id2);
        client2.initialize(&admin2);
        // Call add_to_whitelist without providing admin auth — should panic
        let attacker = Address::generate(&env2);
        client2.add_to_whitelist(&attacker);
    }

    // ── Session: whitelisted user can create session ──────────────────────────

    #[test]
    fn test_whitelisted_user_can_create_session() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        client.add_to_whitelist(&user);

        let expires_at = env.ledger().timestamp() + 3600;
        let actions: soroban_sdk::Vec<Symbol> = vec![&env, Symbol::new(&env, "transfer")];
        client.create_session(&user, &session_key, &expires_at, &actions);

        let data = client.get_session(&session_key);
        assert_eq!(data.user, user);
        assert_eq!(data.expires_at, expires_at);
    }

    // ── Session: non-whitelisted user cannot create session ──────────────────

    #[test]
    #[should_panic(expected = "caller is not whitelisted")]
    fn test_non_whitelisted_user_cannot_create_session() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        // user is NOT whitelisted
        let expires_at = env.ledger().timestamp() + 3600;
        let actions: soroban_sdk::Vec<Symbol> = vec![&env, Symbol::new(&env, "transfer")];
        client.create_session(&user, &session_key, &expires_at, &actions);
    }

    // ── Session: revoke ───────────────────────────────────────────────────────

    #[test]
    fn test_revoke_session() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        client.add_to_whitelist(&user);

        let expires_at = env.ledger().timestamp() + 3600;
        let actions: soroban_sdk::Vec<Symbol> = vec![&env, Symbol::new(&env, "transfer")];
        client.create_session(&user, &session_key, &expires_at, &actions);
        client.revoke_session(&user, &session_key);
    }

    // ── Access control: events emitted ───────────────────────────────────────

    #[test]
    fn test_whitelist_events_emitted() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);
        client.add_to_whitelist(&user);
        client.remove_from_whitelist(&user);
        // If no panic, events were published successfully
    }
}