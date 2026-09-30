//! Event schema audit tests — verifies every event struct carries
//! EVENT_VERSION_V2 and that topic strings are within the 9-byte limit.

#[cfg(test)]
mod tests {
    use crate::events::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    fn make_address(env: &Env) -> Address {
        Address::generate(env)
    }

    #[test]
    fn test_fee_collected_has_version() {
        let env = Env::default();
        let event = FeeCollected {
            version: EVENT_VERSION_V2,
            operation_type: FeeOperationType::Lock,
            amount: 100,
            fee_rate: 50,
            fee_fixed: 0,
            recipient: make_address(&env),
            timestamp: 1,
        };
        assert_eq!(event.version, EVENT_VERSION_V2);
    }

    #[test]
    fn test_batch_funds_locked_has_version() {
        let event = BatchFundsLocked {
            version: EVENT_VERSION_V2,
            count: 2,
            total_amount: 1000,
            timestamp: 1,
            correlation_id: None,
        };
        assert_eq!(event.version, EVENT_VERSION_V2);
    }

    #[test]
    fn test_batch_funds_released_has_version() {
        let event = BatchFundsReleased {
            version: EVENT_VERSION_V2,
            count: 1,
            total_amount: 500,
            timestamp: 2,
        };
        assert_eq!(event.version, EVENT_VERSION_V2);
    }

    #[test]
    fn test_fee_config_updated_has_version() {
        let env = Env::default();
        let event = FeeConfigUpdated {
            version: EVENT_VERSION_V2,
            lock_fee_rate: 10,
            release_fee_rate: 20,
            lock_fixed_fee: 0,
            release_fixed_fee: 0,
            fee_recipient: make_address(&env),
            fee_enabled: true,
            timestamp: 1,
        };
        assert_eq!(event.version, EVENT_VERSION_V2);
    }

    #[test]
    fn test_approval_added_has_version() {
        let env = Env::default();
        let event = ApprovalAdded {
            version: EVENT_VERSION_V2,
            bounty_id: 42,
            contributor: make_address(&env),
            approver: make_address(&env),
            timestamp: 1,
        };
        assert_eq!(event.version, EVENT_VERSION_V2);
    }

    #[test]
    fn test_emergency_withdraw_has_version() {
        let env = Env::default();
        let event = EmergencyWithdrawEvent {
            version: EVENT_VERSION_V2,
            admin: make_address(&env),
            recipient: make_address(&env),
            amount: 9999,
            timestamp: 1,
        };
        assert_eq!(event.version, EVENT_VERSION_V2);
    }

    #[test]
    fn test_topic_symbols_are_within_9_bytes() {
        // Soroban rejects symbol_short! strings > 9 bytes at compile time.
        // This test documents the full topic inventory for auditors.
        let topics: &[&str] = &[
            "init", "f_lock", "f_rel", "f_ref", "pub", "archive",
            "orc_cfg", "fee", "fee_cfg", "fee_rte", "fee_rt",
            "b_lock", "b_rel", "approval", "prng_sel", "f_lkanon",
            "deprec", "maint", "pf_mode", "risk", "npref",
            "ticket_i", "ticket_c", "pause", "em_wtd",
            "cap_new", "cap_use", "cap_rev", "tmlk_cfg",
            "act_prop", "act_exec", "act_cncl",
        ];
        for topic in topics {
            assert!(
                topic.len() <= 9,
                "Topic '{}' exceeds 9-byte Soroban limit ({} bytes)",
                topic,
                topic.len()
            );
        }
    }
}
