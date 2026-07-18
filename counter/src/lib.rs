#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

const COUNTER: Symbol = symbol_short!("COUNTER");

/// `COUNTER` lives in *temporary* storage rather than persistent storage.
///
/// Temporary entries are cheaper to read/write than persistent ones (no
/// long-term rent accounting), which is a real, measurable saving on every
/// `increment`/`decrement` call — see `docs/gas-benchmarks.md`. This is safe
/// specifically *because* the counter carries no financial value and is
/// purely demonstrative: if the entry's TTL lapses from inactivity, the
/// worst case is the count silently resets to 0, which is an acceptable
/// outcome here. Contracts holding real value (e.g. `renaissance-betting`'s
/// `Bet` records) must stay on persistent storage — see the note on
/// `get_bet` in `betting/src/lib.rs`.
const COUNTER_TTL_THRESHOLD: u32 = 50;
const COUNTER_TTL_EXTEND_TO: u32 = 100;

#[contract]
pub struct CounterContract;

#[contractimpl]
impl CounterContract {
    pub fn increment(env: Env, address: Address) -> i32 {
        address.require_auth();
        let count = Self::get_count(env.clone()).saturating_add(1);
        env.storage().temporary().set(&COUNTER, &count);
        env.storage()
            .temporary()
            .extend_ttl(&COUNTER, COUNTER_TTL_THRESHOLD, COUNTER_TTL_EXTEND_TO);
        count
    }

    pub fn decrement(env: Env, address: Address) -> i32 {
        address.require_auth();
        let count = Self::get_count(env.clone()).saturating_sub(1);
        env.storage().temporary().set(&COUNTER, &count);
        env.storage()
            .temporary()
            .extend_ttl(&COUNTER, COUNTER_TTL_THRESHOLD, COUNTER_TTL_EXTEND_TO);
        count
    }

    pub fn get_count(env: Env) -> i32 {
        env.storage().temporary().get(&COUNTER).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, CounterContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CounterContract);
        let client = CounterContractClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_get_count_defaults_to_zero() {
        let (_env, client) = setup();
        assert_eq!(client.get_count(), 0);
    }

    #[test]
    fn test_increment_and_decrement() {
        let (env, client) = setup();
        let user = Address::generate(&env);
        assert_eq!(client.increment(&user), 1);
        assert_eq!(client.increment(&user), 2);
        assert_eq!(client.decrement(&user), 1);
        assert_eq!(client.get_count(), 1);
    }

    #[test]
    fn test_counter_survives_across_calls_within_ttl() {
        let (env, client) = setup();
        let user = Address::generate(&env);
        client.increment(&user);
        client.increment(&user);
        client.increment(&user);
        assert_eq!(client.get_count(), 3);
    }
}

// ── Gas benchmarks ───────────────────────────────────────────────────────────
//
// Measures CPU instructions and memory bytes charged by the Soroban host for
// each public function, and fails the test (and therefore CI, via
// `cargo test --release`) if a function exceeds its budget. Run
// `cargo test --release -p renaissance-counter gas_bench` to exercise these;
// see `docs/gas-benchmarks.md` for the last recorded numbers and target
// rationale.
#[cfg(test)]
mod gas_bench {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    // Thresholds are set with headroom above the measured baseline (see
    // docs/gas-benchmarks.md) so routine SDK/dependency updates don't cause
    // CI flakes, while still catching real regressions (e.g. accidentally
    // reintroducing an unbounded storage read).
    const INCREMENT_CPU_BUDGET: u64 = 400_000;
    const INCREMENT_MEM_BUDGET: u64 = 40_000;
    const GET_COUNT_CPU_BUDGET: u64 = 100_000;
    const GET_COUNT_MEM_BUDGET: u64 = 15_000;

    fn measure<F: FnOnce()>(env: &Env, f: F) -> (u64, u64) {
        env.budget().reset_unlimited();
        f();
        (
            env.budget().cpu_instruction_cost(),
            env.budget().memory_bytes_cost(),
        )
    }

    #[test]
    fn bench_increment_stays_within_budget() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CounterContract);
        let client = CounterContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);

        let (cpu, mem) = measure(&env, || {
            client.increment(&user);
        });

        assert!(cpu <= INCREMENT_CPU_BUDGET, "increment cpu {cpu} exceeded budget {INCREMENT_CPU_BUDGET}");
        assert!(mem <= INCREMENT_MEM_BUDGET, "increment mem {mem} exceeded budget {INCREMENT_MEM_BUDGET}");
    }

    #[test]
    fn bench_get_count_stays_within_budget() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CounterContract);
        let client = CounterContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);
        client.increment(&user);

        let (cpu, mem) = measure(&env, || {
            client.get_count();
        });

        assert!(cpu <= GET_COUNT_CPU_BUDGET, "get_count cpu {cpu} exceeded budget {GET_COUNT_CPU_BUDGET}");
        assert!(mem <= GET_COUNT_MEM_BUDGET, "get_count mem {mem} exceeded budget {GET_COUNT_MEM_BUDGET}");
    }
}
