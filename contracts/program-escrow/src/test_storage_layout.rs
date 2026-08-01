#[cfg(test)]
mod test {
    use crate::{
        DataKey, PauseFlags, ProgramEscrowContract, ProgramEscrowContractClient,
        STORAGE_SCHEMA_VERSION,
    };
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup_test(env: &Env) -> (ProgramEscrowContractClient, Address) {
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize_contract(&admin);
        (client, admin)
    }

    #[test]
    fn test_storage_schema_version_constant() {
        assert_eq!(STORAGE_SCHEMA_VERSION, 2);
    }

    #[test]
    fn test_verify_storage_layout_returns_correct_struct() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        let layout = client.verify_storage_layout();
        assert_eq!(layout.schema_version, 2);
        assert!(layout.admin_set);
        assert!(layout.pause_flags_set);
        assert!(layout.maintenance_mode_set);
        assert!(layout.read_only_mode_set);
    }

    #[test]
    fn test_all_required_instance_keys_readable() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            assert!(env.storage().instance().has(&DataKey::Admin));
            assert!(env.storage().instance().has(&DataKey::PauseFlags));
            assert!(env.storage().instance().has(&DataKey::MaintenanceMode));
            assert!(env.storage().instance().has(&DataKey::ReadOnlyMode));

            let _admin_val: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            let _pause: PauseFlags = env.storage().instance().get(&DataKey::PauseFlags).unwrap();
            let _maint: bool = env
                .storage()
                .instance()
                .get(&DataKey::MaintenanceMode)
                .unwrap();
            let _ro: bool = env
                .storage()
                .instance()
                .get(&DataKey::ReadOnlyMode)
                .unwrap();
        });
    }

    #[test]
    fn test_legacy_and_registry_storage_consistency() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);
        
        let program_id = soroban_sdk::String::from_str(&env, "prog-123");
        let creator = Address::generate(&env);
        let payout_key = Address::generate(&env);
        let token_address = Address::generate(&env);
        
        // initialize program
        client.initialize_program(
            &program_id,
            &payout_key,
            &token_address,
            &creator,
            &None,
            &None,
        );
        
        // Assert get_program_info and get_program_info_v2 return the exact same data
        let info1 = client.get_program_info();
        let info2 = client.get_program_info_v2(&program_id);
        
        assert_eq!(info1.program_id, info2.program_id);
        assert_eq!(info1.authorized_payout_key, info2.authorized_payout_key);
        assert_eq!(info1.token_address, info2.token_address);
        assert_eq!(info1.total_funds, info2.total_funds);
        assert_eq!(info1.remaining_balance, info2.remaining_balance);
        
        // lock funds (legacy)
        let amount = 1000;
        client.lock_program_funds(&amount);
        
        let info1_after = client.get_program_info();
        let info2_after = client.get_program_info_v2(&program_id);
        
        assert_eq!(info1_after.total_funds, info2_after.total_funds);
        assert_eq!(info1_after.remaining_balance, info2_after.remaining_balance);
        assert_eq!(info1_after.total_funds, amount);
    }
}
