#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use soroban_sdk::{contract, contractimpl, symbol, vec, Address, Env, Symbol, Bytes};

const COUNTER: Symbol = symbol!("COUNTER");
const TOKEN_OWNER: Symbol = symbol!("TOKEN_OWNER");
const TOKEN_METADATA: Symbol = symbol!("TOKEN_METADATA");
const BALANCE: Symbol = symbol!("BALANCE");
const LOCKED: Symbol = symbol!("LOCKED");

#[contract]
pub struct CounterContract;

#[contractimpl]
impl CounterContract {
    pub fn increment(env: Env, address: Address) -> i32 {
        address.require_auth();
        let mut count: i32 = env
            .storage()
            .persistent()
            .get(&COUNTER)
            .unwrap_or(0);
        count += 1;
        env.storage().persistent().set(&COUNTER, &count);
        count
    }

    pub fn decrement(env: Env, address: Address) -> i32 {
        address.require_auth();
        let mut count: i32 = env
            .storage()
            .persistent()
            .get(&COUNTER)
            .unwrap_or(0);
        count -= 1;
        env.storage().persistent().set(&COUNTER, &count);
        count
    }

    pub fn get_count(env: Env) -> i32 {
        env.storage().persistent().get(&COUNTER).unwrap_or(0)
    }
}

#[contract]
pub struct NFTContract;

#[contractimpl]
impl NFTContract {
    // Mint a new token; only callable by authorized admin address
    pub fn mint(env: Env, admin: Address, to: Address, token_id: u64, metadata_hash: Bytes) {
        admin.require_auth();
        // Ensure token does not already exist
        if env.storage().persistent().has(&(TOKEN_OWNER, token_id)) {
            panic!("Token already minted");
        }
        // Store ownership and metadata
        env.storage().persistent().set(&(TOKEN_OWNER, token_id), &to);
        env.storage().persistent().set(&(TOKEN_METADATA, token_id), &metadata_hash);
        // Update balance
        let bal: u32 = env.storage().persistent().get(&(BALANCE, &to)).unwrap_or(0);
        env.storage().persistent().set(&(BALANCE, &to), &(bal + 1));
        // Emit Mint event
        env.events().publish((symbol!("Mint"),), (to, token_id, metadata_hash));
    }

    // Transfer token, respecting lock flag
    pub fn transfer(env: Env, from: Address, to: Address, token_id: u64) {
        from.require_auth();
        let owner: Address = env.storage().persistent().get(&(TOKEN_OWNER, token_id)).expect("Token does not exist");
        if owner != from {
            panic!("Caller is not token owner");
        }
        // Check lock status
        let locked: bool = env.storage().persistent().get(&(LOCKED, token_id)).unwrap_or(false);
        if locked {
            panic!("Token is locked and cannot be transferred");
        }
        // Update ownership
        env.storage().persistent().set(&(TOKEN_OWNER, token_id), &to);
        // Update balances
        let from_bal: u32 = env.storage().persistent().get(&(BALANCE, &from)).unwrap_or(0);
        env.storage().persistent().set(&(BALANCE, &from), &(from_bal - 1));
        let to_bal: u32 = env.storage().persistent().get(&(BALANCE, &to)).unwrap_or(0);
        env.storage().persistent().set(&(BALANCE, &to), &(to_bal + 1));
        // Emit Transfer event
        env.events().publish((symbol!("Transfer"),), (from, to, token_id));
    }

    // Burn a token owned by the caller
    pub fn burn(env: Env, owner: Address, token_id: u64) {
        owner.require_auth();
        let token_owner: Address = env.storage().persistent().get(&(TOKEN_OWNER, token_id)).expect("Token does not exist");
        if token_owner != owner {
            panic!("Caller is not token owner");
        }
        // Remove token data
        env.storage().persistent().remove(&(TOKEN_OWNER, token_id));
        env.storage().persistent().remove(&(TOKEN_METADATA, token_id));
        // Update balance
        let bal: u32 = env.storage().persistent().get(&(BALANCE, &owner)).unwrap_or(0);
        env.storage().persistent().set(&(BALANCE, &owner), &(bal - 1));
        // Emit Burn event
        env.events().publish((symbol!("Burn"),), (owner, token_id));
    }

    // Return the owner of a token
    pub fn owner_of(env: Env, token_id: u64) -> Address {
        env.storage().persistent().get(&(TOKEN_OWNER, token_id)).expect("Token does not exist")
    }

    // Return the balance (number of tokens) owned by an address
    pub fn balance_of(env: Env, owner: Address) -> u32 {
        env.storage().persistent().get(&(BALANCE, &owner)).unwrap_or(0)
    }
}

mod test;


extern crate alloc;

use alloc::vec::Vec;
use soroban_sdk::{contract, contractimpl, symbol, vec, Address, Env, Symbol};

const COUNTER: Symbol = symbol!("COUNTER");

#[contract]
pub struct CounterContract;

#[contractimpl]
impl CounterContract {
    pub fn increment(env: Env, address: Address) -> i32 {
        address.require_auth();
        let mut count: i32 = env
            .storage()
            .persistent()
            .get(&COUNTER)
            .unwrap_or(0);
        count += 1;
        env.storage().persistent().set(&COUNTER, &count);
        count
    }

    pub fn decrement(env: Env, address: Address) -> i32 {
        address.require_auth();
        let mut count: i32 = env
            .storage()
            .persistent()
            .get(&COUNTER)
            .unwrap_or(0);
        count -= 1;
        env.storage().persistent().set(&COUNTER, &count);
        count
    }

    pub fn get_count(env: Env) -> i32 {
        env.storage().persistent().get(&COUNTER).unwrap_or(0)
    }
}

mod test;
