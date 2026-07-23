#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

pub mod errors;
pub mod storage;

use errors::ConfigError;
use storage::*;

#[contract]
pub struct PlatformConfigContract;

#[contractimpl]
impl PlatformConfigContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_bps: u32,
        platform_wallet: Address,
        usdc_token: Address,
    ) -> Result<(), ConfigError> {
        if is_initialized(&env) {
            return Err(ConfigError::AlreadyInitialized);
        }
        if fee_bps > 1000 {
            return Err(ConfigError::InvalidFeeBps);
        }
        set_admin(&env, &admin);
        set_fee_bps_val(&env, fee_bps);
        set_platform_wallet(&env, &platform_wallet);
        set_usdc_token(&env, &usdc_token);
        env.events().publish((symbol_short!("init"),), (admin, fee_bps));
        Ok(())
    }

    pub fn get_config(_env: Env) {
        todo!()
    }

    pub fn set_fee_bps(_env: Env, _fee_bps: u32) {
        todo!()
    }

    pub fn set_platform_wallet(_env: Env, _platform_wallet: Address) {
        todo!()
    }

    pub fn transfer_admin(_env: Env, _new_admin: Address) {
        todo!()
    }

    pub fn accept_admin(_env: Env) {
        todo!()
    }
}
