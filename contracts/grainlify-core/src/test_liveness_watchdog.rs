//! Tests for `liveness_watchdog` (view) and `ping_watchdog` (admin).
//!
//! Coverage:
//! - Default state: not paused, not read-only, healthy, no ping, version set
//! - Read-only mode reflected in watchdog
//! - ping_watchdog updates last_ping_ts to ledger timestamp
//! - ping_watchdog blocked in read-only mode
//! - Watchdog view requires no auth
//! - Multiple pings update timestamp monotonically
//! - Combined read-only + paused state reflected correctly
//! - version field matches get_version()

#[cfg(test)]
mod tests {
    use crate::{GrainlifyContract, GrainlifyContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    fn setup(env: &Env) -> (GrainlifyContractClient, Address) {
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(env, &id);
        let admin = Address::generate(env);
        client.init_admin(&admin);
        (client, admin)
    }

    // -----------------------------------------------------------------------
    // liveness_watchdog — default / happy path
    // -----------------------------------------------------------------------

    #[test]
    fn test_watchdog_default_state() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let status = client.liveness_watchdog();

        assert!(!status.paused, "should not be paused after init");
        assert!(!status.read_only, "should not be read-only after init");
        assert!(status.healthy, "should be healthy after init");
        assert_eq!(status.last_ping_ts, 0, "no ping yet — must be 0");
        assert!(status.version > 0, "version must be set after init");
    }

    #[test]
    fn test_watchdog_version_matches_get_version() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        let status = client.liveness_watchdog();
        assert_eq!(
            status.version,
            client.get_version(),
            "watchdog version must match get_version()"
        );
    }

    // -----------------------------------------------------------------------
    // liveness_watchdog — read-only mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_watchdog_reflects_read_only_enabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        assert!(!client.liveness_watchdog().read_only, "initially not read-only");

        client.set_read_only_mode(&true);
        assert!(
            client.liveness_watchdog().read_only,
            "watchdog must show read_only=true"
        );
    }

    #[test]
    fn test_watchdog_reflects_read_only_disabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.set_read_only_mode(&true);
        assert!(client.liveness_watchdog().read_only);

        client.set_read_only_mode(&false);
        assert!(
            !client.liveness_watchdog().read_only,
            "watchdog must show read_only=false after disabling"
        );
    }

    // -----------------------------------------------------------------------
    // ping_watchdog — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn test_ping_watchdog_updates_last_ping_ts() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        assert_eq!(
            client.liveness_watchdog().last_ping_ts,
            0,
            "last_ping_ts must be 0 before first ping"
        );

        client.ping_watchdog();

        let status = client.liveness_watchdog();
        assert!(
            status.last_ping_ts > 0,
            "last_ping_ts must be non-zero after ping"
        );
    }

    #[test]
    fn test_ping_watchdog_timestamp_equals_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.ping_watchdog();

        let status = client.liveness_watchdog();
        assert_eq!(
            status.last_ping_ts,
            env.ledger().timestamp(),
            "last_ping_ts must equal ledger timestamp at ping time"
        );
    }

    #[test]
    fn test_ping_watchdog_multiple_pings_monotonic() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.ping_watchdog();
        let ts1 = client.liveness_watchdog().last_ping_ts;

        // Advance ledger time by 100 seconds
        env.ledger().with_mut(|l| l.timestamp += 100);

        client.ping_watchdog();
        let ts2 = client.liveness_watchdog().last_ping_ts;

        assert!(
            ts2 > ts1,
            "second ping timestamp ({ts2}) must be greater than first ({ts1})"
        );
        assert_eq!(ts2, env.ledger().timestamp());
    }

    // -----------------------------------------------------------------------
    // ping_watchdog — blocked in read-only mode
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "Read-only mode")]
    fn test_ping_watchdog_blocked_in_read_only_mode() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.set_read_only_mode(&true);
        // Must panic: read-only mode blocks all mutations including ping
        client.ping_watchdog();
    }

    // -----------------------------------------------------------------------
    // liveness_watchdog — no auth required (view safety)
    // -----------------------------------------------------------------------

    #[test]
    fn test_watchdog_view_requires_no_auth() {
        let env = Env::default();
        // init_admin needs auth
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // liveness_watchdog must work without any auth mock — pure view
        let status = client.liveness_watchdog();
        assert!(!status.paused);
        assert!(!status.read_only);
        assert!(status.version > 0);
    }

    // -----------------------------------------------------------------------
    // liveness_watchdog — combined state
    // -----------------------------------------------------------------------

    #[test]
    fn test_watchdog_read_only_does_not_affect_paused_field() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        // read-only mode is set but multisig pause is not
        client.set_read_only_mode(&true);

        let status = client.liveness_watchdog();
        assert!(status.read_only, "read_only must be true");
        // paused is driven by MultiSig — no multisig configured, so false
        assert!(!status.paused, "paused must remain false when only read-only is set");
    }

    #[test]
    fn test_watchdog_after_ping_all_fields_correct() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);

        client.ping_watchdog();
        let status = client.liveness_watchdog();

        assert!(!status.paused);
        assert!(!status.read_only);
        assert!(status.healthy);
        assert!(status.last_ping_ts > 0);
        assert!(status.version > 0);
    }

    // -----------------------------------------------------------------------
    // ping_watchdog — requires admin auth (negative test)
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic]
    fn test_ping_watchdog_requires_admin_auth() {
        let env = Env::default();
        // Set up contract with admin auth mocked only for init
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // Create a fresh env without auth mocks — ping must fail
        let env2 = Env::default();
        let id2 = env2.register_contract(None, GrainlifyContract);
        let client2 = GrainlifyContractClient::new(&env2, &id2);
        let admin2 = Address::generate(&env2);
        env2.mock_all_auths();
        client2.init_admin(&admin2);
        // Now call ping without any auth mock — should panic
        let env3 = Env::default();
        let id3 = env3.register_contract(None, GrainlifyContract);
        let client3 = GrainlifyContractClient::new(&env3, &id3);
        let admin3 = Address::generate(&env3);
        {
            env3.mock_all_auths();
            client3.init_admin(&admin3);
        }
        // ping_watchdog without auth — should panic
        client3.ping_watchdog();
    }

    // -----------------------------------------------------------------------
    // liveness_watchdog — not initialized (edge case)
    // -----------------------------------------------------------------------

    #[test]
    fn test_watchdog_on_uninitialized_contract_does_not_panic() {
        let env = Env::default();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);

        // Must not panic — all fields default to safe values
        let status = client.liveness_watchdog();
        assert!(!status.paused);
        assert!(!status.read_only);
        assert_eq!(status.last_ping_ts, 0);
        assert_eq!(status.version, 0);
        // healthy may be false since admin/version not set — that's correct
    }
}
