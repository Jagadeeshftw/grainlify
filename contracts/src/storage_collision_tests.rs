//! # Storage-Key Collision Audit
//!
//! These tests guard against storage-key collisions between the contracts that
//! share the same ledger storage space. A collision happens when two contracts
//! write different data under the same key, which silently corrupts state.
//!
//! The audit is pinned to a check that runs: `grainlify-contracts` is a
//! workspace member and `contracts/scripts/ci-contracts.sh` runs this suite on
//! every pull request, so the collision tests actually execute instead of
//! living as dead code.
//!
//! Every key must be namespaced with the prefix owned by its contract
//! ([`namespaces::PROGRAM_ESCROW`] / [`namespaces::BOUNTY_ESCROW`]).

use crate::storage_key_audit::{bounty_escrow, namespaces, program_escrow, shared, validation};
use soroban_sdk::Symbol;

/// Every storage/event key the **program-escrow** contract writes.
fn program_escrow_keys() -> Vec<Symbol> {
    vec![
        program_escrow::PROGRAM_INITIALIZED,
        program_escrow::FUNDS_LOCKED,
        program_escrow::BATCH_FUNDS_LOCKED,
        program_escrow::BATCH_FUNDS_RELEASED,
        program_escrow::BATCH_PAYOUT,
        program_escrow::PAYOUT,
        program_escrow::PAUSE_STATE_CHANGED,
        program_escrow::MAINTENANCE_MODE_CHANGED,
        program_escrow::READ_ONLY_MODE_CHANGED,
        program_escrow::PROGRAM_RISK_FLAGS_UPDATED,
        program_escrow::PROGRAM_REGISTRY,
        program_escrow::PROGRAM_REGISTERED,
        program_escrow::RELEASE_SCHEDULED,
        program_escrow::SCHEDULE_RELEASED,
        program_escrow::PROGRAM_DELEGATE_SET,
        program_escrow::PROGRAM_DELEGATE_REVOKED,
        program_escrow::PROGRAM_METADATA_UPDATED,
        program_escrow::DISPUTE_OPENED,
        program_escrow::DISPUTE_RESOLVED,
        program_escrow::PROGRAM_DATA,
        program_escrow::RECEIPT_ID,
        program_escrow::SCHEDULES,
        program_escrow::RELEASE_HISTORY,
        program_escrow::NEXT_SCHEDULE_ID,
        program_escrow::PROGRAM_INDEX,
        program_escrow::AUTH_KEY_INDEX,
        program_escrow::FEE_CONFIG,
        program_escrow::FEE_COLLECTED,
    ]
}

/// Every storage/event key the **bounty-escrow** contract writes.
fn bounty_escrow_keys() -> Vec<Symbol> {
    vec![
        bounty_escrow::BOUNTY_INITIALIZED,
        bounty_escrow::FUNDS_LOCKED,
        bounty_escrow::FUNDS_LOCKED_ANON,
        bounty_escrow::FUNDS_RELEASED,
        bounty_escrow::FUNDS_REFUNDED,
        bounty_escrow::ESCROW_PUBLISHED,
        bounty_escrow::TICKET_ISSUED,
        bounty_escrow::TICKET_CLAIMED,
        bounty_escrow::MAINTENANCE_MODE_CHANGED,
        bounty_escrow::PAUSE_STATE_CHANGED,
        bounty_escrow::RISK_FLAGS_UPDATED,
        bounty_escrow::DEPRECATION_STATE_CHANGED,
        bounty_escrow::ADMIN,
        bounty_escrow::TOKEN,
        bounty_escrow::VERSION,
        bounty_escrow::ESCROW_INDEX,
        bounty_escrow::DEPOSITOR_INDEX,
        bounty_escrow::ESCROW_FREEZE,
        bounty_escrow::ADDRESS_FREEZE,
        bounty_escrow::FEE_CONFIG,
        bounty_escrow::REFUND_APPROVAL,
        bounty_escrow::REENTRANCY_GUARD,
        bounty_escrow::MULTISIG_CONFIG,
        bounty_escrow::RELEASE_APPROVAL,
        bounty_escrow::PENDING_CLAIM,
        bounty_escrow::TICKET_COUNTER,
        bounty_escrow::CLAIM_TICKET,
        bounty_escrow::CLAIM_TICKET_INDEX,
        bounty_escrow::BENEFICIARY_TICKETS,
        bounty_escrow::CLAIM_WINDOW,
        bounty_escrow::PAUSE_FLAGS,
        bounty_escrow::AMOUNT_POLICY,
        bounty_escrow::CAPABILITY_NONCE,
        bounty_escrow::CAPABILITY,
        bounty_escrow::NON_TRANSFERABLE_REWARDS,
        bounty_escrow::DEPRECATION_STATE,
        bounty_escrow::PARTICIPANT_FILTER_MODE,
        bounty_escrow::ANONYMOUS_RESOLVER,
        bounty_escrow::TOKEN_FEE_CONFIG,
        bounty_escrow::CHAIN_ID,
        bounty_escrow::NETWORK_ID,
        bounty_escrow::MAINTENANCE_MODE,
        bounty_escrow::GAS_BUDGET_CONFIG,
        bounty_escrow::TIMELOCK_CONFIG,
        bounty_escrow::PENDING_ACTION,
        bounty_escrow::ACTION_COUNTER,
    ]
}

#[cfg(test)]
mod collision_tests {
    use super::*;

    /// Every program-escrow key must validate against the `PE_` namespace.
    #[test]
    fn test_program_escrow_namespace_compliance() {
        let keys = program_escrow_keys();
        assert!(
            !keys.is_empty(),
            "the audit must cover the program-escrow contract"
        );
        for symbol in keys {
            let rendered = symbol.to_string();
            let result = validation::validate_storage_key(symbol, namespaces::PROGRAM_ESCROW);
            assert!(
                result.is_ok(),
                "program-escrow key {} must validate with the PE_ namespace: {:?}",
                rendered,
                result.err()
            );
        }
    }

    /// Every bounty-escrow key must validate against the `BE_` namespace.
    #[test]
    fn test_bounty_escrow_namespace_compliance() {
        let keys = bounty_escrow_keys();
        assert!(
            !keys.is_empty(),
            "the audit must cover the bounty-escrow contract"
        );
        for symbol in keys {
            let rendered = symbol.to_string();
            let result = validation::validate_storage_key(symbol, namespaces::BOUNTY_ESCROW);
            assert!(
                result.is_ok(),
                "bounty-escrow key {} must validate with the BE_ namespace: {:?}",
                rendered,
                result.err()
            );
        }
    }

    /// A key owned by one contract must never validate against the other
    /// contract's namespace — that is exactly the collision we are guarding
    /// against.
    #[test]
    fn test_cross_namespace_isolation() {
        for symbol in program_escrow_keys() {
            assert!(
                validation::validate_storage_key(symbol, namespaces::BOUNTY_ESCROW).is_err(),
                "program-escrow keys must not validate with the BE_ namespace"
            );
        }
        for symbol in bounty_escrow_keys() {
            assert!(
                validation::validate_storage_key(symbol, namespaces::PROGRAM_ESCROW).is_err(),
                "bounty-escrow keys must not validate with the PE_ namespace"
            );
        }
    }

    /// No key value may be reused across the two contracts sharing storage.
    #[test]
    fn test_no_symbol_is_reused_between_contracts() {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for symbol in program_escrow_keys()
            .into_iter()
            .chain(bounty_escrow_keys().into_iter())
        {
            let rendered = symbol.to_string();
            assert!(
                seen.insert(rendered.clone()),
                "storage-key collision: {} is written by more than one contract",
                rendered
            );
        }
    }

    /// A newly introduced key that is not namespaced for its contract must fail
    /// the audit. This is the regression guard for this issue: the audit must
    /// both run *and* reject a collision when one is introduced.
    #[test]
    fn test_newly_introduced_colliding_key_fails_audit() {
        let env = soroban_sdk::Env::default();

        // A developer adds a bounty-escrow key but forgets the BE_ namespace,
        // so it aliases the program-escrow slot of the same name.
        let colliding = Symbol::new(&env, "PE_PROGRAM_DATA");
        assert!(
            validation::validate_storage_key(colliding.clone(), namespaces::BOUNTY_ESCROW)
                .is_err(),
            "a mis-namespaced key must be reported as a collision"
        );

        // A key with no namespace at all is rejected for either contract.
        let unnamed = Symbol::new(&env, "FeeConfig");
        assert!(
            validation::validate_storage_key(unnamed.clone(), namespaces::PROGRAM_ESCROW).is_err()
        );
        assert!(
            validation::validate_storage_key(unnamed, namespaces::BOUNTY_ESCROW).is_err()
        );

        // Sanity check: the correctly namespaced key still passes.
        assert!(
            validation::validate_storage_key(
                Symbol::new(&env, "PE_PROGRAM_DATA"),
                namespaces::PROGRAM_ESCROW
            )
            .is_ok()
        );
    }

    /// Shared constants must stay aligned with the namespaced layout.
    #[test]
    fn test_shared_constants_consistency() {
        assert_eq!(shared::EVENT_VERSION_V2, 2);
        assert_eq!(shared::BASIS_POINTS, 10_000);
        assert_eq!(shared::RISK_FLAG_HIGH_RISK, 1 << 0);
        assert_eq!(shared::RISK_FLAG_UNDER_REVIEW, 1 << 1);
        assert_eq!(shared::RISK_FLAG_RESTRICTED, 1 << 2);
        assert_eq!(shared::RISK_FLAG_DEPRECATED, 1 << 3);
    }

    /// Namespace prefix matching itself.
    #[test]
    fn test_namespace_prefix_validation() {
        assert!(validation::validate_namespace(
            "PE_Test",
            namespaces::PROGRAM_ESCROW
        ));
        assert!(!validation::validate_namespace(
            "BE_Test",
            namespaces::PROGRAM_ESCROW
        ));
        assert!(!validation::validate_namespace(
            "Test",
            namespaces::PROGRAM_ESCROW
        ));

        assert!(validation::validate_namespace(
            "BE_Test",
            namespaces::BOUNTY_ESCROW
        ));
        assert!(!validation::validate_namespace(
            "PE_Test",
            namespaces::BOUNTY_ESCROW
        ));
        assert!(!validation::validate_namespace(
            "Test",
            namespaces::BOUNTY_ESCROW
        ));
    }

    /// Generic, un-namespaced keys that previously collided must never appear.
    #[test]
    fn test_generic_keys_are_never_used_directly() {
        let problematic = ["Admin", "Token", "FeeConfig", "PauseFlags"];
        for symbol in program_escrow_keys()
            .into_iter()
            .chain(bounty_escrow_keys().into_iter())
        {
            let rendered = symbol.to_string();
            for bad in problematic {
                assert_ne!(rendered, bad, "generic key {} must always be namespaced", bad);
            }
            assert!(
                rendered.starts_with(namespaces::PROGRAM_ESCROW)
                    || rendered.starts_with(namespaces::BOUNTY_ESCROW),
                "key {} must carry a contract namespace",
                rendered
            );
        }
    }

    /// `symbol_short!` is capped at 9 bytes; make sure the audit keeps that
    /// invariant so keys stay compile-time constants.
    #[test]
    fn test_symbol_length_constraints() {
        for symbol in program_escrow_keys()
            .into_iter()
            .chain(bounty_escrow_keys().into_iter())
        {
            let rendered = symbol.to_string();
            assert!(
                rendered.len() <= 9,
                "symbol {} exceeds the 9-byte symbol_short limit ({} bytes)",
                rendered,
                rendered.len()
            );
        }
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    /// Previously identified collision risks are resolved by namespacing.
    #[test]
    fn test_previous_collision_risks_resolved() {
        // Before namespacing, both contracts used generic symbols such as
        // "Admin" / "FeeConfig"; those are now PE_/BE_ namespaced.
        let pe_data = program_escrow::PROGRAM_DATA;
        let pe_fee = program_escrow::FEE_CONFIG;
        assert!(validation::validate_storage_key(pe_data, namespaces::PROGRAM_ESCROW).is_ok());
        assert!(validation::validate_storage_key(pe_fee, namespaces::PROGRAM_ESCROW).is_ok());

        let be_admin = bounty_escrow::ADMIN;
        let be_fee = bounty_escrow::FEE_CONFIG;
        assert!(validation::validate_storage_key(be_admin, namespaces::BOUNTY_ESCROW).is_ok());
        assert!(validation::validate_storage_key(be_fee, namespaces::BOUNTY_ESCROW).is_ok());

        // Cross-pollination is rejected.
        let pe_cross = program_escrow::PROGRAM_DATA;
        let be_cross = bounty_escrow::ADMIN;
        assert!(validation::validate_storage_key(pe_cross, namespaces::BOUNTY_ESCROW).is_err());
        assert!(validation::validate_storage_key(be_cross, namespaces::PROGRAM_ESCROW).is_err());

        // Both contracts are represented in the audit.
        assert!(!program_escrow_keys().is_empty());
        assert!(!bounty_escrow_keys().is_empty());
    }

    /// Event symbols that used to look alike stay distinct and namespaced.
    #[test]
    fn test_event_symbol_isolation() {
        assert_ne!(program_escrow::FUNDS_LOCKED, bounty_escrow::FUNDS_LOCKED);
        assert_ne!(
            program_escrow::PAUSE_STATE_CHANGED,
            bounty_escrow::PAUSE_STATE_CHANGED
        );

        let pe_funds_locked = program_escrow::FUNDS_LOCKED;
        let be_funds_locked = bounty_escrow::FUNDS_LOCKED;
        assert!(
            validation::validate_storage_key(pe_funds_locked, namespaces::PROGRAM_ESCROW).is_ok()
        );
        assert!(
            validation::validate_storage_key(be_funds_locked, namespaces::BOUNTY_ESCROW).is_ok()
        );

        let pe_pause_changed = program_escrow::PAUSE_STATE_CHANGED;
        let be_pause_changed = bounty_escrow::PAUSE_STATE_CHANGED;
        assert!(validation::validate_storage_key(pe_pause_changed, namespaces::PROGRAM_ESCROW).is_ok());
        assert!(validation::validate_storage_key(be_pause_changed, namespaces::BOUNTY_ESCROW).is_ok());

        let pe_wrong = program_escrow::FUNDS_LOCKED;
        let be_wrong = bounty_escrow::FUNDS_LOCKED;
        assert!(validation::validate_storage_key(pe_wrong, namespaces::BOUNTY_ESCROW).is_err());
        assert!(validation::validate_storage_key(be_wrong, namespaces::PROGRAM_ESCROW).is_err());
    }
}
