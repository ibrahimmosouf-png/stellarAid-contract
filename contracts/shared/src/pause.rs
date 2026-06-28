use soroban_sdk::{contracttype, Address, Env, Symbol};

#[derive(Clone)]
#[contracttype]
pub enum PauseDataKey {
    Paused,
}

#[derive(Clone)]
#[contracttype]
pub struct ContractPausedEvent {
    pub admin: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct ContractUnpausedEvent {
    pub admin: Address,
}

pub fn require_not_paused(env: &Env) {
    if env.storage().instance().get(&PauseDataKey::Paused).unwrap_or(false) {
        panic!("contract is paused");
    }
}

pub fn pause(env: &Env, admin: &Address) {
    admin.require_auth();
    env.storage().instance().set(&PauseDataKey::Paused, &true);
    env.events().publish(
        (Symbol::new(env, "contract_paused"),),
        ContractPausedEvent { admin: admin.clone() },
    );
}

pub fn unpause(env: &Env, admin: &Address) {
    admin.require_auth();
    env.storage().instance().set(&PauseDataKey::Paused, &false);
    env.events().publish(
        (Symbol::new(env, "contract_unpaused"),),
        ContractUnpausedEvent { admin: admin.clone() },
    );
}
