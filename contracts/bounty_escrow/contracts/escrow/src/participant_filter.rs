//! Participant allowlist/blocklist, filter mode, and paginated queries.



use soroban_sdk::{symbol_short, Address, Env, Vec};
use crate::{
    anti_abuse, events,
    DataKey, DeprecationState, Error, ParticipantFilterMode, ParticipantListPage,
    PARTICIPANT_LIST_SCHEMA_VERSION_V1, MAX_PARTICIPANT_FILTER_PAGE_SIZE,
    events::{ParticipantFilterModeChanged, ParticipantFilterQueried, EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


/// Returns current deprecation state (internal). When deprecated is true, new locks are blocked.
pub(crate) fn get_deprecation_state(env: &Env) -> DeprecationState {
    env.storage()
        .instance()
        .get(&DataKey::DeprecationState)
        .unwrap_or(DeprecationState {
            deprecated: false,
            migration_target: None,
        })
}


pub(crate) fn get_participant_filter_mode(env: &Env) -> ParticipantFilterMode {
    env.storage()
        .instance()
        .get(&DataKey::ParticipantFilterMode)
        .unwrap_or(ParticipantFilterMode::Disabled)
}


pub(crate) fn read_participant_index(env: &Env, key: DataKey) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&key)
        .unwrap_or(Vec::<Address>::new(env))
}


pub(crate) fn write_participant_index(env: &Env, key: DataKey, values: &Vec<Address>) {
    env.storage().instance().set(&key, values);
}


pub(crate) fn index_contains(values: &Vec<Address>, needle: &Address) -> bool {
    for value in values.iter() {
        if value == needle.clone() {
            return true;
        }
    }
    false
}


pub(crate) fn index_insert_unique(values: &mut Vec<Address>, value: Address) {
    if !index_contains(values, &value) {
        values.push_back(value);
    }
}


pub(crate) fn index_remove(env: &Env, values: &Vec<Address>, value: &Address) -> Vec<Address> {
    let mut filtered = Vec::<Address>::new(env);
    for entry in values.iter() {
        if entry != value.clone() {
            filtered.push_back(entry);
        }
    }
    filtered
}


pub(crate) fn paginate_addresses(
    env: &Env,
    values: Vec<Address>,
    offset: u32,
    limit: u32,
) -> Vec<Address> {
    if limit == 0 {
        return Vec::new(env);
    }
    let len = values.len();
    if offset >= len {
        return Vec::new(env);
    }
    let mut out = Vec::new(env);
    let mut i = offset;
    while i < len && out.len() < limit {
        if let Some(value) = values.get(i) {
            out.push_back(value);
        }
        i += 1;
    }
    out
}


/// Enforces participant filtering: returns Err if the address is not allowed to participate
/// (lock_funds / batch_lock_funds) under the current filter mode.
pub(crate) fn check_participant_filter(env: &Env, address: Address) -> Result<(), Error> {
    let mode = get_participant_filter_mode(env);
    match mode {
        ParticipantFilterMode::Disabled => Ok(()),
        ParticipantFilterMode::BlocklistOnly => {
            if anti_abuse::is_blocklisted(env, address) {
                return Err(Error::ParticipantBlocked);
            }
            Ok(())
        }
        ParticipantFilterMode::AllowlistOnly => {
            if !anti_abuse::is_whitelisted(env, address) {
                return Err(Error::ParticipantNotAllowed);
            }
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    anti_abuse::set_whitelist(&env, address.clone(), whitelisted);
    let mut index = read_participant_index(&env, DataKey::WhitelistIndex);
    if whitelisted {
        index_insert_unique(&mut index, address.clone());
    } else {
        index = index_remove(&env, &index, &address);
    }
    write_participant_index(&env, DataKey::WhitelistIndex, &index);
    events::emit_participant_filter_entry_updated(
        &env,
        events::ParticipantFilterEntryUpdated {
            version: EVENT_VERSION_V2,
            list_type: events::ParticipantFilterListType::Allowlist,
            address,
            enabled: whitelisted,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );
    Ok(())
}


pub fn set_whitelist_entry(env: Env, address: Address, whitelisted: bool) -> Result<(), Error> {
    set_whitelist(env, address, whitelisted)
}


pub fn set_blocklist(env: Env, address: Address, blocked: bool) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    anti_abuse::set_blocklist(&env, address.clone(), blocked);
    let mut index = read_participant_index(&env, DataKey::BlocklistIndex);
    if blocked {
        index_insert_unique(&mut index, address.clone());
    } else {
        index = index_remove(&env, &index, &address);
    }
    write_participant_index(&env, DataKey::BlocklistIndex, &index);
    events::emit_participant_filter_entry_updated(
        &env,
        events::ParticipantFilterEntryUpdated {
            version: EVENT_VERSION_V2,
            list_type: events::ParticipantFilterListType::Blocklist,
            address,
            enabled: blocked,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );
    Ok(())
}


pub fn set_blocklist_entry(env: Env, address: Address, blocked: bool) -> Result<(), Error> {
    set_blocklist(env, address, blocked)
}


pub fn set_filter_mode(env: Env, mode: ParticipantFilterMode) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    let previous_mode = get_participant_filter_mode(&env);
    env.storage()
        .instance()
        .set(&DataKey::ParticipantFilterMode, &mode);
    emit_participant_filter_mode_changed(
        &env,
        ParticipantFilterModeChanged {
            previous_mode,
            new_mode: mode,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );
    Ok(())
}


pub fn get_filter_mode(env: Env) -> ParticipantFilterMode {
    get_participant_filter_mode(&env)
}


/// Return the total number of allowlisted addresses.
pub fn get_whitelist_count(env: Env) -> u32 {
    read_participant_index(&env, DataKey::WhitelistIndex).len()
}


/// Return the total number of blocklisted addresses.
pub fn get_blocklist_count(env: Env) -> u32 {
    read_participant_index(&env, DataKey::BlocklistIndex).len()
}


/// Return the participant list storage schema version initialized during `init()`.
///
/// Off-chain indexers and upgrade tooling can use this view to verify the
/// allowlist/blocklist index layout expected by paginated filter queries.
pub fn get_participant_schema_version(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ParticipantListSchemaVersion)
        .unwrap_or(0)
}


/// Return a deterministic page of allowlisted addresses with pagination metadata.
///
/// `limit` is silently capped at `MAX_PARTICIPANT_FILTER_PAGE_SIZE` (50).
/// Emits a `ParticipantFilterQueried` audit event on every call.
pub fn query_whitelist(env: Env, offset: u32, limit: u32) -> ParticipantListPage {
    let effective_limit = limit.min(MAX_PARTICIPANT_FILTER_PAGE_SIZE);
    let values = read_participant_index(&env, DataKey::WhitelistIndex);
    let total = values.len();
    let items = paginate_addresses(&env, values, offset, effective_limit);
    let result_count = items.len();
    let has_more = offset.saturating_add(result_count) < total;
    emit_participant_filter_queried(
        &env,
        ParticipantFilterQueried {
            list_type: events::ParticipantFilterListType::Allowlist,
            offset,
            limit: effective_limit,
            result_count,
            total,
            timestamp: env.ledger().timestamp(),
        },
    );
    ParticipantListPage {
        items,
        total,
        offset,
        has_more,
    }
}


/// Return a deterministic page of blocklisted addresses with pagination metadata.
///
/// `limit` is silently capped at `MAX_PARTICIPANT_FILTER_PAGE_SIZE` (50).
/// Emits a `ParticipantFilterQueried` audit event on every call.
pub fn query_blocklist(env: Env, offset: u32, limit: u32) -> ParticipantListPage {
    let effective_limit = limit.min(MAX_PARTICIPANT_FILTER_PAGE_SIZE);
    let values = read_participant_index(&env, DataKey::BlocklistIndex);
    let total = values.len();
    let items = paginate_addresses(&env, values, offset, effective_limit);
    let result_count = items.len();
    let has_more = offset.saturating_add(result_count) < total;
    emit_participant_filter_queried(
        &env,
        ParticipantFilterQueried {
            list_type: events::ParticipantFilterListType::Blocklist,
            offset,
            limit: effective_limit,
            result_count,
            total,
            timestamp: env.ledger().timestamp(),
        },
    );
    ParticipantListPage {
        items,
        total,
        offset,
        has_more,
    }
}

