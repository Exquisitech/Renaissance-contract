#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger};
use soroban_sdk::token::{self, StellarAssetClient};
use soroban_sdk::{vec, Address, Env, IntoVal, Symbol};

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// Register + initialize a vault with a mocked-auth env and return the pieces
/// most tests need: the env, admin, authorized betting contract, and vault id.
fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let betting_contract = Address::generate(&env);

    let vault_id = env.register_contract(None, RenaissanceVaultContract);
    let client = RenaissanceVaultContractClient::new(&env, &vault_id);
    client.initialize(&admin, &betting_contract);

    (env, admin, betting_contract, vault_id)
}

fn client<'a>(env: &Env, vault_id: &Address) -> RenaissanceVaultContractClient<'a> {
    RenaissanceVaultContractClient::new(env, vault_id)
}

fn new_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

// ── Initialization ──────────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let asset = new_token(&env, &admin);
    assert_eq!(vault.get_vault_balance(&asset), 0);
    assert!(!vault.is_paused());
}

// ── Deposit / withdraw happy paths (not paused) ───────────────────────────────────

#[test]
fn test_deposit_withdraw() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);

    vault.deposit(&user, &500, &token);
    let bal = vault.get_user_balance(&user, &token);
    assert_eq!(bal.available, 500);
    assert_eq!(bal.locked, 0);
    assert_eq!(vault.get_vault_balance(&token), 500);

    vault.withdraw(&user, &200, &token);
    let bal = vault.get_user_balance(&user, &token);
    assert_eq!(bal.available, 300);
    assert_eq!(vault.get_vault_balance(&token), 300);
}

#[test]
fn test_withdraw_insufficient_balance() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);
    vault.deposit(&user, &300, &token);

    match vault.try_withdraw(&user, &400, &token) {
        Err(Ok(e)) => assert_eq!(e, VaultError::InsufficientBalance),
        _ => panic!("expected InsufficientBalance"),
    }
}

// ── Lock / payout via authorized betting contract ─────────────────────────────────

#[test]
fn test_lock_for_bet_success() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);
    vault.deposit(&user, &500, &token);

    vault.lock_for_bet(&user, &200, &token, &123);
    let bal = vault.get_user_balance(&user, &token);
    assert_eq!(bal.available, 300);
    assert_eq!(bal.locked, 200);
    assert!(vault.is_locked_for_bet(&123, &user, &token));
}

#[test]
fn test_lock_for_bet_insufficient_balance() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);
    vault.deposit(&user, &100, &token);

    match vault.try_lock_for_bet(&user, &200, &token, &123) {
        Err(Ok(e)) => assert_eq!(e, VaultError::InsufficientBalance),
        _ => panic!("expected InsufficientBalance"),
    }
}

#[test]
fn test_payout_success() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let winner = Address::generate(&env);
    mint(&env, &token, &winner, 1_000);
    vault.deposit(&winner, &500, &token);
    vault.lock_for_bet(&winner, &200, &token, &123);
    vault.payout(&winner, &300, &token, &123);

    let bal = vault.get_user_balance(&winner, &token);
    assert_eq!(bal.available, 600); // 300 left + 300 winnings
    assert_eq!(bal.locked, 0);
    assert!(!vault.is_locked_for_bet(&123, &winner, &token));
}

// ── Pausable: administration ──────────────────────────────────────────────────────

#[test]
fn test_pause_unpause_toggles_state() {
    let (env, _admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);

    assert!(!vault.is_paused());
    vault.pause();
    assert!(vault.is_paused());
    vault.unpause();
    assert!(!vault.is_paused());
}

#[test]
fn test_pause_is_idempotent() {
    let (env, _admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);

    vault.pause();
    // Pausing again is a no-op and must not error.
    vault.pause();
    assert!(vault.is_paused());
}

#[test]
fn test_pause_emits_event_logging_admin() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);

    vault.pause();
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                vault_id.clone(),
                (Symbol::new(&env, "Paused"),).into_val(&env),
                admin.into_val(&env),
            ),
        ]
    );
}

#[test]
fn test_unpause_emits_event_logging_admin() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);

    vault.pause();
    vault.unpause();
    // Both admin actions are logged: Paused then Unpaused, each carrying admin.
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                vault_id.clone(),
                (Symbol::new(&env, "Paused"),).into_val(&env),
                admin.clone().into_val(&env),
            ),
            (
                vault_id.clone(),
                (Symbol::new(&env, "Unpaused"),).into_val(&env),
                admin.into_val(&env),
            ),
        ]
    );
}

#[test]
fn test_pause_requires_admin_auth() {
    // Fresh env: authorize only the initialize call, then drop auth so the
    // pause() admin check has nothing to satisfy require_auth with.
    let env = Env::default();
    let admin = Address::generate(&env);
    let betting_contract = Address::generate(&env);
    let vault_id = env.register_contract(None, RenaissanceVaultContract);
    let vault = client(&env, &vault_id);

    env.mock_all_auths();
    vault.initialize(&admin, &betting_contract);

    // Remove mocked authorizations; admin.require_auth() must now fail.
    env.set_auths(&[]);
    assert!(vault.try_pause().is_err());
}

// ── Pausable: critical operations revert while paused ─────────────────────────────

#[test]
fn test_deposit_reverts_when_paused() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);

    vault.pause();
    match vault.try_deposit(&user, &500, &token) {
        Err(Ok(e)) => assert_eq!(e, VaultError::Paused),
        _ => panic!("expected Paused"),
    }
}

#[test]
fn test_withdraw_reverts_when_paused() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);
    vault.deposit(&user, &500, &token);

    vault.pause();
    match vault.try_withdraw(&user, &100, &token) {
        Err(Ok(e)) => assert_eq!(e, VaultError::Paused),
        _ => panic!("expected Paused"),
    }
}

#[test]
fn test_lock_for_bet_reverts_when_paused() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);
    vault.deposit(&user, &500, &token);

    vault.pause();
    match vault.try_lock_for_bet(&user, &200, &token, &123) {
        Err(Ok(e)) => assert_eq!(e, VaultError::Paused),
        _ => panic!("expected Paused"),
    }
}

#[test]
fn test_payout_reverts_when_paused() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let winner = Address::generate(&env);
    mint(&env, &token, &winner, 1_000);
    vault.deposit(&winner, &500, &token);
    vault.lock_for_bet(&winner, &200, &token, &123);

    vault.pause();
    match vault.try_payout(&winner, &300, &token, &123) {
        Err(Ok(e)) => assert_eq!(e, VaultError::Paused),
        _ => panic!("expected Paused"),
    }
}

#[test]
fn test_operations_resume_after_unpause() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let user = Address::generate(&env);
    mint(&env, &token, &user, 1_000);

    vault.pause();
    assert!(vault.try_deposit(&user, &500, &token).is_err());

    vault.unpause();
    // Deposit now succeeds and moves tokens into the vault.
    vault.deposit(&user, &500, &token);
    assert_eq!(vault.get_user_balance(&user, &token).available, 500);
    let tc = token::Client::new(&env, &token);
    assert_eq!(tc.balance(&user), 500);
}

// ── Emergency / admin flows ───────────────────────────────────────────────────────

#[test]
fn test_emergency_withdraw_schedule_and_execute() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);

    let to = Address::generate(&env);
    mint(&env, &token, &vault_id, 1_000);

    // Schedule.
    vault.emergency_withdraw(&token, &to, &500);
    // Fast forward past the 24h timelock.
    env.ledger().set_timestamp(24 * 60 * 60 + 1);
    // Execute.
    vault.emergency_withdraw(&token, &to, &500);

    let tc = token::Client::new(&env, &token);
    assert_eq!(tc.balance(&to), 500);
}

#[test]
fn test_emergency_withdraw_cancel() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let token = new_token(&env, &admin);
    let to = Address::generate(&env);

    vault.emergency_withdraw(&token, &to, &500);
    vault.cancel_emergency_withdraw(&token);

    env.ledger().set_timestamp(24 * 60 * 60 + 1);
    mint(&env, &token, &vault_id, 1_000);
    // Cancelled: this call re-schedules rather than transferring.
    vault.emergency_withdraw(&token, &to, &500);

    let tc = token::Client::new(&env, &token);
    assert_eq!(tc.balance(&to), 0);
}

#[test]
fn test_set_admin_schedule_and_execute() {
    let (env, _admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);
    let new_admin = Address::generate(&env);

    vault.set_admin(&new_admin);
    env.ledger().set_timestamp(48 * 60 * 60 + 1);
    vault.set_admin(&new_admin);

    // The new admin can now drive admin-only flows.
    vault.cancel_set_admin();
}

#[test]
fn test_recover_token() {
    let (env, admin, _betting, vault_id) = setup();
    let vault = client(&env, &vault_id);

    // Tracked token cannot be recovered.
    let tracked = new_token(&env, &admin);
    let user = Address::generate(&env);
    mint(&env, &tracked, &user, 1_000);
    vault.deposit(&user, &500, &tracked);

    let to = Address::generate(&env);
    assert!(vault.try_recover_token(&tracked, &to, &100).is_err());

    // Untracked token sent directly to the contract can be recovered.
    let untracked = new_token(&env, &admin);
    mint(&env, &untracked, &vault_id, 500);
    vault.recover_token(&untracked, &to, &500);

    let tc = token::Client::new(&env, &untracked);
    assert_eq!(tc.balance(&to), 500);
}
