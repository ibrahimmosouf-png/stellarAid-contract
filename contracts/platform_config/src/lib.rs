#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

pub mod errors;
pub mod storage;
pub mod types;

use errors::ConfigError;
use storage::*;
use types::PlatformConfig;

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
        env.events().publish((symbol_short!("init"),), (admin.clone(), fee_bps));
        Ok(())
    }

    pub fn get_config(env: Env) -> PlatformConfig {
        PlatformConfig {
            admin: get_admin(&env),
            fee_bps: get_fee_bps(&env),
            platform_wallet: get_platform_wallet(&env),
            usdc_token: get_usdc_token(&env),
        }
    }

    pub fn set_fee_bps(env: Env, fee_bps: u32) -> Result<(), ConfigError> {
        let admin = get_admin(&env);
        admin.require_auth();
        if fee_bps > 1000 {
            return Err(ConfigError::InvalidFeeBps);
        }
        let old_fee = get_fee_bps(&env);
        set_fee_bps_val(&env, fee_bps);
        env.events().publish((symbol_short!("feeupdtd"),), (old_fee, fee_bps));
        Ok(())
    }

    pub fn set_platform_wallet(env: Env, platform_wallet: Address) -> Result<(), ConfigError> {
        let admin = get_admin(&env);
        admin.require_auth();
        set_platform_wallet(&env, &platform_wallet);
        Ok(())
    }

    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ConfigError> {
        let admin = get_admin(&env);
        admin.require_auth();
        set_pending_admin(&env, &new_admin);
        env.events().publish((symbol_short!("admprosd"),), new_admin);
        Ok(())
    }

    pub fn accept_admin(env: Env) -> Result<(), ConfigError> {
        let pending = get_pending_admin(&env).ok_or(ConfigError::NoPendingAdmin)?;
        pending.require_auth();
        set_admin(&env, &pending);
        env.events().publish((symbol_short!("admtxfrd"),), pending);
        Ok(())
    }
}
