#[cfg(test)]
mod test {
    use crate::{DataKey, GrainlifyContract, GrainlifyContractClient, LIVENESS_SCHEMA_VERSION, STORAGE_SCHEMA_VERSION};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup_test(env: &Env) -> (GrainlifyContractClient, Address) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.init_admin(&admin);
        (client, admin)
    }

    #[test]
    fn test_storage_schema_version_constant() {
        assert_eq!(STORAGE_SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_verify_storage_layout_after_init() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);
        assert!(client.verify_storage_layout());
    }

    #[test]
    fn test_all_instance_keys_readable_after_init() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            assert!(env.storage().instance().has(&DataKey::Admin));
            assert!(env.storage().instance().has(&DataKey::Version));
            assert!(env.storage().instance().has(&DataKey::ReadOnlyMode));
        });
    }

    // =========================================================================
    // Liveness Watchdog Tests (LW-01 … LW-08)
    // =========================================================================

    /// LW-01: liveness_watchdog returns healthy=true and version>0 after init.
    #[test]
    fn test_liveness_watchdog_healthy_after_init() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        let status = client.liveness_watchdog();
        assert!(!status.paused, "should not be paused after init");
        assert!(!status.read_only, "should not be read-only after init");
        assert!(status.healthy, "should be healthy after init");
        assert!(status.version > 0, "version must be set after init");
    }

    /// LW-02: read-only mode is reflected in watchdog.
    #[test]
    fn test_liveness_watchdog_reflects_read_only() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        client.set_read_only_mode(&true);
        let status = client.liveness_watchdog();

        assert!(status.read_only, "read_only must be true");
    }

    /// LW-03: read-only mode can be cleared and watchdog reflects it.
    #[test]
    fn test_liveness_watchdog_read_only_restored() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        client.set_read_only_mode(&true);
        client.set_read_only_mode(&false);
        let status = client.liveness_watchdog();

        assert!(!status.read_only);
        assert!(status.healthy);
    }

    /// LW-04: version field matches get_version().
    #[test]
    fn test_liveness_watchdog_version_matches_get_version() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        let status = client.liveness_watchdog();
        assert_eq!(status.version, client.get_version());
    }

    /// LW-05: get_liveness_schema_version returns 1 after init.
    #[test]
    fn test_get_liveness_schema_version_after_init() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        assert_eq!(client.get_liveness_schema_version(), LIVENESS_SCHEMA_VERSION);
    }

    /// LW-06: LivenessSchemaVersion key is present in instance storage after init.
    #[test]
    fn test_liveness_schema_version_key_written_at_init() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            assert!(
                env.storage().instance().has(&DataKey::LivenessSchemaVersion),
                "LivenessSchemaVersion must be written at init"
            );
        });
    }

    /// LW-07: last_ping_ts defaults to 0 before any ping.
    #[test]
    fn test_liveness_watchdog_last_ping_ts_defaults_to_zero() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        let status = client.liveness_watchdog();
        assert_eq!(status.last_ping_ts, 0, "last_ping_ts must be 0 before first ping");
    }

    /// LW-08: paused and read_only are independent booleans.
    #[test]
    fn test_liveness_watchdog_paused_and_read_only_independent() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        let s = client.liveness_watchdog();
        assert!(!s.paused);
        assert!(!s.read_only);

        client.set_read_only_mode(&true);
        let s = client.liveness_watchdog();
        assert!(!s.paused);
        assert!(s.read_only);

        client.set_read_only_mode(&false);
        let s = client.liveness_watchdog();
        assert!(!s.paused);
        assert!(!s.read_only);
    }
}
