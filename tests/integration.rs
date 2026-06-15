<rust>
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_counter() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CounterContract);
        let client = CounterContractClient::new(&env, &contract_id);
        let address = Address::random(&env);

        let count = client.increment(&address);
        assert_eq!(count, 1);

        let count = client.increment(&address);
        assert_eq!(count, 2);

        let count = client.decrement(&address);
        assert_eq!(count, 1);

        let count = client.get_count();
        assert_eq!(count, 1);
    }
}
</rust>
