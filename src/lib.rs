#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Vec};

// ── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Session(Address),
    Admin,
    Whitelist(Address),
}

// ── Data structures ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct SessionData {
    pub user: Address,
    pub expires_at: u64,
    pub allowed_actions: Vec<soroban_sdk::Symbol>,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct SessionContract;

#[contractimpl]
impl SessionContract {
    // ── Initialization ────────────────────────────────────────────────────────

    /// Set the admin address. Can only be called once (during deployment).
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    // ── Admin helpers ─────────────────────────────────────────────────────────

    fn require_admin(env: &Env) -> Address {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("contract not initialized");
        admin.require_auth();
        admin
    }

    // ── Whitelist management ──────────────────────────────────────────────────

    /// Add an address to the whitelist. Only the admin may call this.
    /// Emits an `AddedToWhitelist` event.
    pub fn add_to_whitelist(env: Env, address: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Whitelist(address.clone()), &true);
        env.events()
            .publish((symbol_short!("Whitelst"), symbol_short!("Added")), address);
    }

    /// Remove an address from the whitelist. Only the admin may call this.
    /// Emits a `RemovedFromWhitelist` event.
    pub fn remove_from_whitelist(env: Env, address: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .remove(&DataKey::Whitelist(address.clone()));
        env.events()
            .publish((symbol_short!("Whitelst"), symbol_short!("Removed")), address);
    }

    /// Returns true if `address` is currently whitelisted.
    pub fn is_whitelisted(env: Env, address: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Whitelist(address))
            .unwrap_or(false)
    }

    // ── Session management ────────────────────────────────────────────────────

    /// Register a new session key. Only whitelisted users may create sessions.
    pub fn create_session(
        env: Env,
        user: Address,
        session_key: Address,
        expires_at: u64,
        allowed_actions: Vec<soroban_sdk::Symbol>,
    ) {
        user.require_auth();

        // Enforce whitelist: only approved addresses can create sessions
        let is_approved: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Whitelist(user.clone()))
            .unwrap_or(false);
        if !is_approved {
            panic!("caller is not whitelisted");
        }

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

        let session_secs = expires_at.saturating_sub(now);
        let ledgers_needed = ((session_secs / 5) as u32).max(100);
        env.storage()
            .persistent()
            .extend_ttl(&key, ledgers_needed, ledgers_needed);
    }

    /// Retrieve a session record.
    pub fn get_session(env: Env, session_key: Address) -> SessionData {
        env.storage()
            .persistent()
            .get(&DataKey::Session(session_key))
            .expect("session not found")
    }

    /// Revoke a session key. Only the owning user may revoke their own session.
    pub fn revoke_session(env: Env, user: Address, session_key: Address) {
        user.require_auth();
        let key = DataKey::Session(session_key);
        let data: SessionData = env
            .storage()
            .persistent()
            .get(&key)
            .expect("session not found");
        if data.user != user {
            panic!("not session owner");
        }
        env.storage().persistent().remove(&key);
    }
}