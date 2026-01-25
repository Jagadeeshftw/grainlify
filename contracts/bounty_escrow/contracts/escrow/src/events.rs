use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyEscrowInitialized {
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

pub fn emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"), );
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLocked {
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
}

pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsReleased {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_funds_released(env: &Env, event: FundsReleased) {
    let topics = (symbol_short!("f_rel"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsRefunded {
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
}

pub fn emit_funds_refunded(env: &Env, event: FundsRefunded) {
    let topics = (symbol_short!("f_ref"), event.bounty_id);
    env.events().publish(topics, event.clone());
}