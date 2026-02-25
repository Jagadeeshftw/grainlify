use soroban_sdk::{contracttype, Symbol, Env, Address, panic_with_error};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Flag(Symbol),
}

pub fn has_admin(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Admin)
}

pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).expect("Admin not set")
}

/// Sets a feature flag. Only callable by Admin.
pub fn set_flag(e: &Env, flag: Symbol, enabled: bool) {
    get_admin(e).require_auth();
    e.storage().instance().set(&DataKey::Flag(flag), &enabled);
    
    // Emit event for off-chain tracking
    e.events().publish((Symbol::new(e, "feature_flag_changed"), flag), enabled);
}

/// Checks if a feature flag is enabled. Defaults to false.
pub fn is_enabled(e: &Env, flag: Symbol) -> bool {
    e.storage().instance().get(&DataKey::Flag(flag)).unwrap_or(false)
}