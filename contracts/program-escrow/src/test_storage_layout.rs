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
}
