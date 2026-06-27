#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype,
    Address, Env, Vec, symbol_short,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OracleError {
    OracleUnauthorized  = 1,
    AlreadySubmitted    = 2,
    AlreadyConfirmed    = 3,
    ResultNotFound      = 4,
    AlreadyFinalized    = 5,
    WindowViolation     = 6, // submission outside match window
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchResult {
    pub match_id:    u64,
    pub home_score:  u32,
    pub away_score:  u32,
    pub started_at:  u64,
    pub finished_at: u64,
    pub finalized:   bool,
}

// Storage keys
#[contracttype]
pub enum DataKey {
    Oracles,
    Result(u64),
    Confirmations(u64),
    Submitter(u64),
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    // ------------------------------------------------------------------
    // Admin: initialise the list of authorised oracles (called once)
    // ------------------------------------------------------------------
    pub fn initialize(env: Env, oracles: Vec<Address>) {
        if env.storage().instance().has(&DataKey::Oracles) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Oracles, &oracles);
    }

    // ------------------------------------------------------------------
    // submit_result — first oracle posts the result
    // ------------------------------------------------------------------
    pub fn submit_result(
        env:        Env,
        caller:     Address,
        match_id:   u64,
        home_score: u32,
        away_score: u32,
        started_at: u64,
        finished_at: u64,
    ) -> Result<(), OracleError> {
        caller.require_auth();
        Self::require_oracle(&env, &caller)?;

        // Window guard: current ledger time must be inside [started_at, finished_at]
        let now = env.ledger().timestamp();
        if now < started_at || now > finished_at {
            return Err(OracleError::WindowViolation);
        }

        if env.storage().persistent().has(&DataKey::Result(match_id)) {
            return Err(OracleError::AlreadySubmitted);
        }

        let result = MatchResult { match_id, home_score, away_score, started_at, finished_at, finalized: false };
        env.storage().persistent().set(&DataKey::Result(match_id), &result);
        env.storage().persistent().set(&DataKey::Submitter(match_id), &caller);

        let mut confs: Vec<Address> = Vec::new(&env);
        confs.push_back(caller.clone());
        env.storage().persistent().set(&DataKey::Confirmations(match_id), &confs);

        env.events().publish((symbol_short!("ResultSub"), match_id), (home_score, away_score));

        // If there's only 1 oracle, finalize immediately
        Self::try_finalize(&env, match_id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // confirm_result — subsequent oracle adds confirmation
    // ------------------------------------------------------------------
    pub fn confirm_result(
        env:      Env,
        caller:   Address,
        match_id: u64,
    ) -> Result<(), OracleError> {
        caller.require_auth();
        Self::require_oracle(&env, &caller)?;

        let result: MatchResult = env.storage().persistent()
            .get(&DataKey::Result(match_id))
            .ok_or(OracleError::ResultNotFound)?;

        if result.finalized {
            return Err(OracleError::AlreadyFinalized);
        }

        let mut confs: Vec<Address> = env.storage().persistent()
            .get(&DataKey::Confirmations(match_id))
            .unwrap_or_else(|| Vec::new(&env));

        // Prevent double-confirm by same oracle
        for existing in confs.iter() {
            if existing == caller {
                return Err(OracleError::AlreadyConfirmed);
            }
        }

        confs.push_back(caller.clone());
        env.storage().persistent().set(&DataKey::Confirmations(match_id), &confs);

        env.events().publish((symbol_short!("ResultCon"), match_id), caller);

        Self::try_finalize(&env, match_id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Getters
    // ------------------------------------------------------------------
    pub fn get_result(env: Env, match_id: u64) -> Result<MatchResult, OracleError> {
        env.storage().persistent()
            .get(&DataKey::Result(match_id))
            .ok_or(OracleError::ResultNotFound)
    }

    pub fn is_finalized(env: Env, match_id: u64) -> bool {
        env.storage().persistent()
            .get::<DataKey, MatchResult>(&DataKey::Result(match_id))
            .map(|r| r.finalized)
            .unwrap_or(false)
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------
    fn require_oracle(env: &Env, caller: &Address) -> Result<(), OracleError> {
        let oracles: Vec<Address> = env.storage().instance()
            .get(&DataKey::Oracles)
            .unwrap_or_else(|| Vec::new(env));
        for o in oracles.iter() {
            if o == *caller {
                return Ok(());
            }
        }
        Err(OracleError::OracleUnauthorized)
    }

    fn try_finalize(env: &Env, match_id: u64) {
        let confs: Vec<Address> = env.storage().persistent()
            .get(&DataKey::Confirmations(match_id))
            .unwrap_or_else(|| Vec::new(env));

        let oracles: Vec<Address> = env.storage().instance()
            .get(&DataKey::Oracles)
            .unwrap_or_else(|| Vec::new(env));

        // Require at least 2 confirmations (or all oracles if fewer than 2 exist)
        let threshold = if oracles.len() >= 2 { 2u32 } else { oracles.len() };

        if confs.len() >= threshold {
            let mut result: MatchResult = env.storage().persistent()
                .get(&DataKey::Result(match_id))
                .unwrap();
            result.finalized = true;
            env.storage().persistent().set(&DataKey::Result(match_id), &result);
            env.events().publish((symbol_short!("ResultFin"), match_id), ());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger, LedgerInfo}, vec, Env};

    fn setup() -> (Env, OracleContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(OracleContract, ());
        let client = OracleContractClient::new(&env, &contract_id);

        let o1 = Address::generate(&env);
        let o2 = Address::generate(&env);
        client.initialize(&vec![&env, o1.clone(), o2.clone()]);
        (env, client, o1, o2)
    }

    fn set_time(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 20,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
    }

    // Happy path: 2-of-2 finalizes result
    #[test]
    fn test_finalize_on_two_confirmations() {
        let (env, client, o1, o2) = setup();
        set_time(&env, 1000);
        client.submit_result(&o1, &1, &2, &1, &500, &2000);
        assert!(!client.is_finalized(&1));
        client.confirm_result(&o2, &1);
        assert!(client.is_finalized(&1));
        let r = client.get_result(&1);
        assert_eq!(r.home_score, 2);
        assert_eq!(r.away_score, 1);
    }

    // Double-submit by same oracle is rejected
    #[test]
    fn test_double_submit() {
        let (env, client, o1, _o2) = setup();
        set_time(&env, 1000);
        client.submit_result(&o1, &2, &1, &0, &500, &2000);
        let err = client.try_submit_result(&o1, &2, &1, &0, &500, &2000).unwrap_err();
        assert_eq!(err.unwrap(), OracleError::AlreadySubmitted);
    }

    // Double-confirm by same oracle is rejected
    #[test]
    fn test_double_confirm() {
        let (env, client, o1, _o2) = setup();
        set_time(&env, 1000);
        client.submit_result(&o1, &3, &0, &0, &500, &2000);
        let err = client.try_confirm_result(&o1, &3).unwrap_err();
        assert_eq!(err.unwrap(), OracleError::AlreadyConfirmed);
    }

    // Late submission (after finished_at) is rejected
    #[test]
    fn test_late_submission() {
        let (env, client, o1, _o2) = setup();
        set_time(&env, 5000); // after finished_at=2000
        let err = client.try_submit_result(&o1, &4, &1, &0, &500, &2000).unwrap_err();
        assert_eq!(err.unwrap(), OracleError::WindowViolation);
    }

    // Early submission (before started_at) is rejected
    #[test]
    fn test_early_submission() {
        let (env, client, o1, _o2) = setup();
        set_time(&env, 100); // before started_at=500
        let err = client.try_submit_result(&o1, &5, &1, &0, &500, &2000).unwrap_err();
        assert_eq!(err.unwrap(), OracleError::WindowViolation);
    }

    // Unauthorized caller is rejected
    #[test]
    fn test_unauthorized_submit() {
        let (env, client, _o1, _o2) = setup();
        set_time(&env, 1000);
        let intruder = Address::generate(&env);
        let err = client.try_submit_result(&intruder, &6, &1, &0, &500, &2000).unwrap_err();
        assert_eq!(err.unwrap(), OracleError::OracleUnauthorized);
    }

    // Unauthorized confirm is rejected
    #[test]
    fn test_unauthorized_confirm() {
        let (env, client, o1, _o2) = setup();
        set_time(&env, 1000);
        client.submit_result(&o1, &7, &1, &0, &500, &2000);
        let intruder = Address::generate(&env);
        let err = client.try_confirm_result(&intruder, &7).unwrap_err();
        assert_eq!(err.unwrap(), OracleError::OracleUnauthorized);
    }
}
