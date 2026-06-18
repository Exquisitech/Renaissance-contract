#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address, i128};
use renaissance_core::{PlatformError, MatchMetadata, get_event_topic_by_string};

@contract
pub struct RenaissanceBettingContract;

#[contractimpl]
impl RenaissanceBettingContract {
    pub fn place_bet(env: Env, user: Address, amount: i128) -> Result<(), PlatformError> {
        user.require_auth();

        if amount <= 0 {
            return Err(PlatformError::InvalidAmount);
        }

        // Emit standard platform tracking log metrics
        let topic = get_event_topic_by_string(&env, "bet_placed");
        env.events().publish((topic, user), amount);

        Ok(())
    }
}