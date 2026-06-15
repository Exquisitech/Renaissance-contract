#![no_std]

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
