#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

// ── Storage key ──────────────────────────────────────────────────────────────
// Keyed directly by session_key address → O(1) lookup, no secondary indexes.

#[contracttype]
pub enum DataKey {
    Session(Address),
}

// ── Data structures ───────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct SessionData {
    /// Wallet that owns this session.
    pub user: Address,
    /// Unix timestamp (seconds) after which the session is no longer valid.
    pub expires_at: u64,
    /// Whitelist of actions the session key is permitted to perform.
    pub allowed_actions: Vec<Symbol>,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct SessionContract;

#[contractimpl]
impl SessionContract {
    /// Register a new session key with time-bound, action-scoped permissions.
    ///
    /// Only the wallet owner (`user`) may call this.
    /// Session keys cannot exceed the supplied `allowed_actions` whitelist,
    /// ensuring they can never trigger asset transfers outside platform scope.
    pub fn create_session(
        env: Env,
        user: Address,
        session_key: Address,
        expires_at: u64,
        allowed_actions: Vec<Symbol>,
    ) {
        user.require_auth();

        let now = env.ledger().timestamp();
        if expires_at <= now {
            panic!("expires_at must be in the future");
        }

        let key = DataKey::Session(session_key);
        let data = SessionData {
            user,
            expires_at,
            allowed_actions,
        };
        env.storage().persistent().set(&key, &data);

        // Bump TTL so the entry stays live for at least the session duration.
        // Stellar produces ~1 ledger every 5 seconds.
        let session_secs = expires_at.saturating_sub(now);
        let ledgers_needed = ((session_secs / 5) as u32).max(100);
        env.storage()
            .persistent()
            .extend_ttl(&key, ledgers_needed, ledgers_needed);
    }

    /// Revoke a session key immediately.
    ///
    /// Only the wallet owner that created the session may revoke it.
    /// Silently succeeds if the session does not exist (idempotent).
    pub fn revoke_session(env: Env, user: Address, session_key: Address) {
        user.require_auth();

        let key = DataKey::Session(session_key);
        if let Some(data) = env
            .storage()
            .persistent()
            .get::<DataKey, SessionData>(&key)
        {
            if data.user != user {
                panic!("Only the session owner can revoke this session");
            }
            env.storage().persistent().remove(&key);
        }
    }

    /// Check whether `session_key` is currently authorised to perform `action`.
    ///
    /// Returns `false` (never panics) when:
    /// - The session does not exist.
    /// - The session has expired.
    /// - `action` is not in the session's `allowed_actions` whitelist.
    pub fn validate_session(env: Env, session_key: Address, action: Symbol) -> bool {
        let key = DataKey::Session(session_key);
        let data: SessionData = match env.storage().persistent().get(&key) {
            Some(d) => d,
            None => return false,
        };

        // Reject expired sessions.
        if env.ledger().timestamp() >= data.expires_at {
            return false;
        }

        // Enforce action-scope whitelist.
        for allowed in data.allowed_actions.iter() {
            if allowed == action {
                return true;
            }
        }
        false
    }

    /// Return session metadata for off-chain inspection.
    /// Returns `None` if no session exists for the key.
    pub fn get_session(env: Env, session_key: Address) -> Option<SessionData> {
        env.storage()
            .persistent()
            .get(&DataKey::Session(session_key))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        vec, Env,
    };

    fn make_env(timestamp: u64) -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(timestamp);
        env
    }

    #[test]
    fn test_create_and_validate_valid_action() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        let expires_at = 1_000_000 + 3_600; // +1 hour
        let allowed = vec![
            &env,
            Symbol::new(&env, "stake"),
            Symbol::new(&env, "unstake"),
        ];

        client.create_session(&user, &session_key, &expires_at, &allowed);

        assert!(client.validate_session(&session_key, &Symbol::new(&env, "stake")));
        assert!(client.validate_session(&session_key, &Symbol::new(&env, "unstake")));
    }

    #[test]
    fn test_out_of_scope_action_rejected() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        let allowed = vec![&env, Symbol::new(&env, "stake")];

        client.create_session(&user, &session_key, &(1_000_000 + 3_600), &allowed);

        // transfer is NOT in allowed_actions → session key cannot move assets
        assert!(!client.validate_session(&session_key, &Symbol::new(&env, "transfer")));
        assert!(!client.validate_session(&session_key, &Symbol::new(&env, "send")));
    }

    #[test]
    fn test_expired_session_auto_rejected() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        let allowed = vec![&env, Symbol::new(&env, "stake")];

        client.create_session(&user, &session_key, &(1_000_000 + 3_600), &allowed);

        // Fast-forward ledger past expiry
        env.ledger().set_timestamp(1_000_000 + 3_601);

        assert!(!client.validate_session(&session_key, &Symbol::new(&env, "stake")));
    }

    #[test]
    fn test_revoke_session() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        let allowed = vec![&env, Symbol::new(&env, "stake")];

        client.create_session(&user, &session_key, &(1_000_000 + 3_600), &allowed);
        assert!(client.validate_session(&session_key, &Symbol::new(&env, "stake")));

        client.revoke_session(&user, &session_key);

        assert!(!client.validate_session(&session_key, &Symbol::new(&env, "stake")));
    }

    #[test]
    fn test_revoke_is_idempotent() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);

        // Revoking a non-existent session should not panic
        client.revoke_session(&user, &session_key);
    }

    #[test]
    fn test_missing_session_returns_false() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let session_key = Address::generate(&env);
        assert!(!client.validate_session(&session_key, &Symbol::new(&env, "stake")));
    }

    #[test]
    fn test_get_session_returns_data() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        let expires_at = 1_000_000 + 7_200u64;
        let allowed = vec![&env, Symbol::new(&env, "vote")];

        client.create_session(&user, &session_key, &expires_at, &allowed);

        let data = client.get_session(&session_key).unwrap();
        assert_eq!(data.user, user);
        assert_eq!(data.expires_at, expires_at);
    }

    #[test]
    #[should_panic(expected = "expires_at must be in the future")]
    fn test_cannot_create_already_expired_session() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let session_key = Address::generate(&env);
        let allowed = vec![&env, Symbol::new(&env, "stake")];

        // expires_at in the past → should panic
        client.create_session(&user, &session_key, &999_999, &allowed);
    }

    #[test]
    #[should_panic(expected = "Only the session owner can revoke this session")]
    fn test_non_owner_cannot_revoke() {
        let env = make_env(1_000_000);
        let contract_id = env.register_contract(None, SessionContract);
        let client = SessionContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let session_key = Address::generate(&env);
        let allowed = vec![&env, Symbol::new(&env, "stake")];

        client.create_session(&owner, &session_key, &(1_000_000 + 3_600), &allowed);

        // attacker tries to revoke owner's session → should panic
        client.revoke_session(&attacker, &session_key);
    }
}
