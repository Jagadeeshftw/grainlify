//! Anti-abuse rate limiting: per-address operation windows, cooldowns, and whitelist/blocklist.
use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AntiAbuseConfig {
    pub window_size: u64,     // Window size in seconds
    pub max_operations: u32,  // Max operations allowed in window
    pub cooldown_period: u64, // Minimum seconds between operations
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressState {
    pub last_operation_timestamp: u64,
    pub window_start_timestamp: u64,
    pub operation_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AntiAbuseKey {
    Config,
    State(Address),
    Whitelist(Address),
    Blocklist(Address),
    Admin,
}

pub fn get_config(env: &Env) -> AntiAbuseConfig {
    env.storage()
        .instance()
        .get(&AntiAbuseKey::Config)
        .unwrap_or(AntiAbuseConfig {
            window_size: 3600, // 1 hour default
            max_operations: 100,
            cooldown_period: 60, // 1 minute default
        })
}

#[allow(dead_code)]
pub fn set_config(env: &Env, config: AntiAbuseConfig) {
    env.storage().instance().set(&AntiAbuseKey::Config, &config);
}

pub fn is_whitelisted(env: &Env, address: Address) -> bool {
    env.storage()
        .instance()
        .has(&AntiAbuseKey::Whitelist(address))
}

pub fn set_whitelist(env: &Env, address: Address, whitelisted: bool) {
    if whitelisted {
        env.storage()
            .instance()
            .set(&AntiAbuseKey::Whitelist(address), &true);
    } else {
        env.storage()
            .instance()
            .remove(&AntiAbuseKey::Whitelist(address));
    }
}

pub fn is_blocklisted(env: &Env, address: Address) -> bool {
    env.storage()
        .instance()
        .has(&AntiAbuseKey::Blocklist(address))
}

pub fn set_blocklist(env: &Env, address: Address, blocked: bool) {
    if blocked {
        env.storage()
            .instance()
            .set(&AntiAbuseKey::Blocklist(address), &true);
    } else {
        env.storage()
            .instance()
            .remove(&AntiAbuseKey::Blocklist(address));
    }
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&AntiAbuseKey::Admin)
}

// Retained for operator admin rotation; not yet wired to a contract entrypoint.
#[allow(dead_code)]
pub fn set_admin(env: &Env, admin: Address) {
    env.storage().instance().set(&AntiAbuseKey::Admin, &admin);
}

pub fn check_rate_limit(env: &Env, address: Address) {
    if is_whitelisted(env, address.clone()) {
        return;
    }

    let config = get_config(env);
    let now = env.ledger().timestamp();
    let key = AntiAbuseKey::State(address.clone());

    let mut state: AddressState =
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(AddressState {
                last_operation_timestamp: 0,
                window_start_timestamp: now,
                operation_count: 0,
            });

    // 1. Cooldown check
    if state.last_operation_timestamp > 0
        && now
            < state
                .last_operation_timestamp
                .saturating_add(config.cooldown_period)
    {
        env.events().publish(
            (symbol_short!("abuse"), symbol_short!("cooldown")),
            (address.clone(), now),
        );
        panic!("Operation in cooldown period");
    }

    // 2. Window check
    if now
        >= state
            .window_start_timestamp
            .saturating_add(config.window_size)
    {
        // New window
        state.window_start_timestamp = now;
        state.operation_count = 1;
    } else {
        // Same window
        if state.operation_count >= config.max_operations {
            env.events().publish(
                (symbol_short!("abuse"), symbol_short!("limit")),
                (address.clone(), now),
            );
            panic!("Rate limit exceeded");
        }
        state.operation_count += 1;
    }

    state.last_operation_timestamp = now;
    env.storage().persistent().set(&key, &state);

    // Extend TTL for state (approx 1 day)
    env.storage().persistent().extend_ttl(&key, 17280, 17280);
}
