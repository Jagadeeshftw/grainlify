//! Compatibility checks for versioned contract events.

/// Returns true when `version` matches the current event schema version.
pub(crate) fn is_compatible(version: u32, current_version: u32) -> bool {
    version == current_version
}
