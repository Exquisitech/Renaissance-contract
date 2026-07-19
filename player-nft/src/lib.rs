#![no_std]
#![forbid(unsafe_code)]

use soroban_sdk::{contract, contractimpl, Env, Symbol};

/// Initial contract boundary for player NFT functionality.
#[contract]
pub struct PlayerNftContract;

#[contractimpl]
impl PlayerNftContract {
    pub fn contract_name(env: Env) -> Symbol {
        Symbol::new(&env, "player_nft")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn contract_name_is_stable() {
        let env = Env::default();
        let contract_id = env.register_contract(None, PlayerNftContract);
        let client = PlayerNftContractClient::new(&env, &contract_id);
        assert_eq!(client.contract_name(), Symbol::new(&env, "player_nft"));
    }
}
