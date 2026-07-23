use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug)]
pub struct PlatformConfig {
    pub admin: Address,
    pub fee_bps: u32,
    pub platform_wallet: Address,
    pub usdc_token: Address,
}
