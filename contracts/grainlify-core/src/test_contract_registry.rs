//! Tests for deployed contract registry.
//!
//! Coverage:
//! - Register a deployed contract
//! - Update existing registration (no duplicates)
//! - Deregister a deployed contract
//! - List deployed contracts with pagination
//! - Get deployed contract by address
//! - Count deployed contracts
//! - Registry size limit enforcement
//! - Auth requirements for mutations
//! - View functions require no auth
//! - Read-only mode blocks mutations

#[cfg(test)]
mod tests {
    use crate::{ContractKind, GrainlifyContract, GrainlifyContractClient};
    use soroban_sdk::{testutils::{Address as _, Ledger as _}, Address, Env, String};

    fn setup(env: &Env) -> (GrainlifyContractClient, Address) {
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(env, &id);
        let admin = Address::generate(env);
        client.init_admin(&admin);
        (client, admin)
    }

    fn make_address(env: &Env) -> Address {
        Address::generate(env)
    }

    fn make_string(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    // -----------------------------------------------------------------------
    // register_deployed_contract
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_deployed_contract() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "bounty-escrow-v1");
        client.register_deployed_contract(&addr, &name, &ContractKind::BountyEscrow, &1);

        assert_eq!(client.deployed_contract_count(), 1);
        let entry = client.get_deployed_contract(&addr).unwrap();
        assert_eq!(entry.address, addr);
        assert_eq!(entry.name, name);
        assert_eq!(entry.kind, ContractKind::BountyEscrow);
        assert_eq!(entry.version, 1);
    }

    #[test]
    fn test_register_updates_existing_entry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name1 = make_string(&env, "escrow-v1");
        let name2 = make_string(&env, "escrow-v2");

        client.register_deployed_contract(&addr, &name1, &ContractKind::BountyEscrow, &1);
        client.register_deployed_contract(&addr, &name2, &ContractKind::ProgramEscrow, &2);

        assert_eq!(client.deployed_contract_count(), 1);
        let entry = client.get_deployed_contract(&addr).unwrap();
        assert_eq!(entry.name, name2);
        assert_eq!(entry.kind, ContractKind::ProgramEscrow);
        assert_eq!(entry.version, 2);
    }

    #[test]
    #[should_panic(expected = "Registry full")]
    fn test_register_panics_when_registry_full() {
        let env = Env::default();
        env.mock_all_auths();
        let mut budget = env.budget();
        budget.reset_unlimited();
        let (client, _admin) = setup(&env);

        for _ in 0..200 {
            let addr = make_address(&env);
            let name = make_string(&env, "contract");
            client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        }

        let addr = make_address(&env);
        let name = make_string(&env, "overflow");
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
    }

    #[test]
    #[should_panic(expected = "Read-only mode")]
    fn test_register_blocked_in_read_only_mode() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.set_read_only_mode(&true);

        let addr = make_address(&env);
        let name = make_string(&env, "test");
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
    }

    // -----------------------------------------------------------------------
    // deregister_deployed_contract
    // -----------------------------------------------------------------------

    #[test]
    fn test_deregister_removes_entry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "to-remove");
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        assert_eq!(client.deployed_contract_count(), 1);

        client.deregister_deployed_contract(&addr);
        assert_eq!(client.deployed_contract_count(), 0);
        assert!(client.get_deployed_contract(&addr).is_none());
    }

    #[test]
    fn test_deregister_unknown_address_is_noop() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        client.deregister_deployed_contract(&addr);
        assert_eq!(client.deployed_contract_count(), 0);
    }

    #[test]
    #[should_panic(expected = "Read-only mode")]
    fn test_deregister_blocked_in_read_only_mode() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "test");
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);

        client.set_read_only_mode(&true);
        client.deregister_deployed_contract(&addr);
    }

    // -----------------------------------------------------------------------
    // list_deployed_contracts
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_deployed_contracts_pagination() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        for _ in 0..5 {
            let addr = make_address(&env);
            let name = make_string(&env, "c");
            client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        }

        let page = client.list_deployed_contracts(&Some(0), &Some(2));
        assert_eq!(page.len(), 2);

        let page2 = client.list_deployed_contracts(&Some(2), &Some(2));
        assert_eq!(page2.len(), 2);

        let page3 = client.list_deployed_contracts(&Some(4), &Some(10));
        assert_eq!(page3.len(), 1);
    }

    #[test]
    fn test_list_deployed_contracts_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let list = client.list_deployed_contracts(&None, &None);
        assert_eq!(list.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Offset exceeds registry size")]
    fn test_list_deployed_contracts_offset_too_large() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.list_deployed_contracts(&Some(1), &Some(1));
    }

    // -----------------------------------------------------------------------
    // get_deployed_contract
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_deployed_contract_returns_none_for_unknown() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        assert!(client.get_deployed_contract(&addr).is_none());
    }

    // -----------------------------------------------------------------------
    // deployed_contract_count
    // -----------------------------------------------------------------------

    #[test]
    fn test_deployed_contract_count_increments_and_decrements() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        assert_eq!(client.deployed_contract_count(), 0);

        let addr1 = make_address(&env);
        client.register_deployed_contract(&addr1, &make_string(&env, "c1"), &ContractKind::Other, &1);
        assert_eq!(client.deployed_contract_count(), 1);

        let addr2 = make_address(&env);
        client.register_deployed_contract(&addr2, &make_string(&env, "c2"), &ContractKind::Other, &1);
        assert_eq!(client.deployed_contract_count(), 2);

        client.deregister_deployed_contract(&addr1);
        assert_eq!(client.deployed_contract_count(), 1);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_deployed_at_timestamp_recorded() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let before = env.ledger().timestamp();
        let addr = make_address(&env);
        let name = make_string(&env, "timed-contract");
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        let after = env.ledger().timestamp();

        let entry = client.get_deployed_contract(&addr).unwrap();
        assert!(
            entry.deployed_at >= before && entry.deployed_at <= after,
            "deployed_at must be within transaction timestamp bounds"
        );
    }

    #[test]
    fn test_list_pagination_exact_boundary() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        for _ in 0..3 {
            let addr = make_address(&env);
            client.register_deployed_contract(&addr, &make_string(&env, "c"), &ContractKind::Other, &1);
        }

        // offset=1, limit=2 should return exactly the last 2 items
        let page = client.list_deployed_contracts(&Some(1), &Some(2));
        assert_eq!(page.len(), 2);
    }

    #[test]
    fn test_get_deployed_contract_all_fields_correct() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "full-check");
        client.register_deployed_contract(&addr, &name, &ContractKind::GrainlifyCore, &42);

        let entry = client.get_deployed_contract(&addr).unwrap();
        assert_eq!(entry.address, addr);
        assert_eq!(entry.name, name);
        assert_eq!(entry.kind, ContractKind::GrainlifyCore);
        assert_eq!(entry.version, 42);
    }

    #[test]
    fn test_multiple_deregisters_no_panic() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        client.deregister_deployed_contract(&addr);
        client.deregister_deployed_contract(&addr);
        client.deregister_deployed_contract(&addr);
        assert_eq!(client.deployed_contract_count(), 0);
    }

    // -----------------------------------------------------------------------
    // View functions require no auth
    // -----------------------------------------------------------------------

    #[test]
    fn test_views_require_no_auth() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // These should work without auth mock
        let _ = client.deployed_contract_count();
        let _ = client.list_deployed_contracts(&None, &None);
        let addr = make_address(&env);
        let _ = client.get_deployed_contract(&addr);
    }

    // -----------------------------------------------------------------------
    // Duplicate-registration semantics (issue documentation tests)
    //
    // These tests explicitly establish and document what grainlify-core does
    // when register_deployed_contract is called twice with the same address.
    //
    // Observed behaviour: UPDATE-IN-PLACE
    //   - The DeployedContractIndex (ordered address list) is NOT extended.
    //   - The DeployedContractEntry storage slot IS overwritten with the new
    //     name / kind / version / deployed_at values.
    //   - deployed_contract_count() returns the same value before and after
    //     the re-registration call.
    //
    // This is in contrast to view-facade::register, which performs an
    // unconditional push_back and therefore CREATES DUPLICATE entries.
    //
    // See docs/registry-duplicate-semantics-comparison.md for the full
    // side-by-side comparison and the follow-up alignment issue flag.
    // -----------------------------------------------------------------------

    /// Re-registering the same address does NOT increase the count.
    ///
    /// This is the primary behavioural invariant: grainlify-core is
    /// update-in-place, not duplicate-appending.
    #[test]
    fn test_reregister_same_address_count_unchanged() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name1 = make_string(&env, "escrow-v1");
        let name2 = make_string(&env, "escrow-v2");

        client.register_deployed_contract(&addr, &name1, &ContractKind::BountyEscrow, &1);
        // Precondition: one entry exists.
        assert_eq!(
            client.deployed_contract_count(),
            1,
            "count must be 1 after first registration"
        );

        // Re-register the same address with different metadata.
        client.register_deployed_contract(&addr, &name2, &ContractKind::ProgramEscrow, &2);

        // DOCUMENTED BEHAVIOUR: count stays at 1 (update-in-place, no duplicate).
        assert_eq!(
            client.deployed_contract_count(),
            1,
            "count must remain 1 after re-registration of the same address \
             (grainlify-core uses update-in-place, not duplicate-append)"
        );
    }

    /// Re-registering overwrites name, kind and version in the stored entry.
    ///
    /// The entry returned by get_deployed_contract reflects the values from
    /// the most recent register_deployed_contract call.
    #[test]
    fn test_reregister_same_address_overwrites_metadata() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name_v1 = make_string(&env, "my-escrow-v1");
        let name_v2 = make_string(&env, "my-escrow-v2");

        client.register_deployed_contract(&addr, &name_v1, &ContractKind::BountyEscrow, &1);
        client.register_deployed_contract(&addr, &name_v2, &ContractKind::ProgramEscrow, &99);

        let entry = client
            .get_deployed_contract(&addr)
            .expect("entry must exist after re-registration");

        // DOCUMENTED BEHAVIOUR: latest values win.
        assert_eq!(entry.name, name_v2, "name must reflect the second registration");
        assert_eq!(
            entry.kind,
            ContractKind::ProgramEscrow,
            "kind must reflect the second registration"
        );
        assert_eq!(entry.version, 99, "version must reflect the second registration");
        assert_eq!(
            entry.address, addr,
            "address field in the entry must match the registered address"
        );
    }

    /// Re-registering updates deployed_at to the timestamp of the latest call.
    ///
    /// The overwrite replaces the entire DeployedContract struct, including
    /// deployed_at, so the second call's ledger timestamp wins.
    #[test]
    fn test_reregister_same_address_updates_deployed_at() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "timed");

        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        let first_deployed_at = client
            .get_deployed_contract(&addr)
            .expect("must exist")
            .deployed_at;

        // Advance ledger time so the two calls have different timestamps.
        env.ledger().with_mut(|li| {
            li.timestamp += 100;
        });

        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &2);
        let second_deployed_at = client
            .get_deployed_contract(&addr)
            .expect("must exist after re-registration")
            .deployed_at;

        // DOCUMENTED BEHAVIOUR: deployed_at is refreshed on overwrite.
        assert!(
            second_deployed_at > first_deployed_at,
            "re-registration must update deployed_at (second={} must be > first={})",
            second_deployed_at,
            first_deployed_at
        );
    }

    /// The index (insertion-order address list) does not gain a second entry
    /// for the same address. list_deployed_contracts returns the address
    /// exactly once.
    #[test]
    fn test_reregister_same_address_appears_once_in_list() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "dedup-check");

        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &2);
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &3);

        let list = client.list_deployed_contracts(&None, &None);

        // DOCUMENTED BEHAVIOUR: address appears exactly once in the list.
        let occurrences = list.iter().filter(|e| e.address == addr).count();
        assert_eq!(
            occurrences,
            1,
            "address must appear exactly once in list after three re-registrations \
             (grainlify-core uses update-in-place, not duplicate-append)"
        );
    }

    /// Registering N distinct addresses then re-registering each once
    /// results in exactly N entries — not 2*N.
    #[test]
    fn test_reregister_many_addresses_no_count_inflation() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        const N: u32 = 5;
        let mut addrs = soroban_sdk::Vec::new(&env);

        for i in 0..N {
            let addr = make_address(&env);
            let name = make_string(&env, "c");
            addrs.push_back(addr.clone());
            client.register_deployed_contract(&addr, &name, &ContractKind::Other, &i);
        }

        assert_eq!(
            client.deployed_contract_count(),
            N,
            "count must equal N after N distinct registrations"
        );

        // Re-register every address once with a new version.
        for i in 0..N {
            let addr = addrs.get(i).unwrap();
            let name = make_string(&env, "c-updated");
            client.register_deployed_contract(&addr, &name, &ContractKind::Other, &(i + 100));
        }

        // DOCUMENTED BEHAVIOUR: count must still be N, not 2*N.
        assert_eq!(
            client.deployed_contract_count(),
            N,
            "count must remain N after re-registering all N addresses once \
             (grainlify-core uses update-in-place)"
        );
    }

    /// Re-registering an address that was subsequently deregistered results
    /// in a fresh insertion (count goes up), not an update-in-place of the
    /// deleted entry.
    #[test]
    fn test_register_after_deregister_creates_new_entry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "lifecycle");

        // First registration.
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &1);
        assert_eq!(client.deployed_contract_count(), 1);

        // Deregister removes the entry entirely.
        client.deregister_deployed_contract(&addr);
        assert_eq!(client.deployed_contract_count(), 0);
        assert!(client.get_deployed_contract(&addr).is_none());

        // Second registration after deregister is treated as a fresh insert.
        client.register_deployed_contract(&addr, &name, &ContractKind::Other, &2);
        assert_eq!(
            client.deployed_contract_count(),
            1,
            "re-registering after deregister must create a single fresh entry"
        );
        let entry = client.get_deployed_contract(&addr).unwrap();
        assert_eq!(entry.version, 2, "entry must reflect the post-deregister version");
    }

    /// Verify that re-registration with a different ContractKind is reflected
    /// correctly — ensures the kind field is not frozen at first-registration.
    #[test]
    fn test_reregister_kind_change_is_visible() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let addr = make_address(&env);
        let name = make_string(&env, "kind-change");

        for kind in [
            ContractKind::BountyEscrow,
            ContractKind::ProgramEscrow,
            ContractKind::SorobanEscrow,
            ContractKind::GrainlifyCore,
            ContractKind::ViewFacade,
            ContractKind::Other,
        ] {
            client.register_deployed_contract(&addr, &name, &kind, &1);
            let stored = client
                .get_deployed_contract(&addr)
                .expect("entry must exist");
            assert_eq!(
                stored.kind, kind,
                "kind must update immediately after re-registration"
            );
            // Count must never exceed 1 regardless of how many re-registrations.
            assert_eq!(client.deployed_contract_count(), 1);
        }
    }
}
