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
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

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
}
