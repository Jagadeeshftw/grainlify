//! Shared validation for human-readable escrow identifiers.

use soroban_sdk::Env;

/// Maximum length for bounty types and short identifiers.
const MAX_TAG_LEN: u32 = 50;

/// Validates a tag, type, or short identifier.
pub fn validate_tag(_env: &Env, tag: &soroban_sdk::String, field_name: &str) {
    if tag.len() > MAX_TAG_LEN {
        panic!(
            "{} exceeds maximum length of {} characters",
            field_name, MAX_TAG_LEN
        );
    }

    if tag.len() == 0 {
        panic!("{} cannot be empty", field_name);
    }
}
