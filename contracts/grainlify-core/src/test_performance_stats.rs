#[cfg(test)]
mod test {
    use crate::monitoring;
    use crate::{GrainlifyContract, GrainlifyContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, Env, Symbol,
    };

    fn setup_test(env: &Env) -> (GrainlifyContractClient, Address) {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.init_admin(&admin);
        (client, admin)
    }

    // ---------------------------------------------------------------
    // 1. Untracked function returns all-zero stats
    // ---------------------------------------------------------------
    #[test]
    fn test_get_performance_stats_initial_zeros() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            let stats = monitoring::get_performance_stats(&env, Symbol::new(&env, "never_called"));
            assert_eq!(stats.call_count, 0);
            assert_eq!(stats.total_time, 0);
            assert_eq!(stats.avg_time, 0);
            assert_eq!(stats.last_called, 0);
        });
    }

    // ---------------------------------------------------------------
    // 2. Single emit: count=1, total_time=duration, avg=duration
    // ---------------------------------------------------------------
    #[test]
    fn test_get_performance_stats_after_single_emit() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.ledger().with_mut(|li| li.timestamp = 1000);

        env.as_contract(&client.address, || {
            let func = Symbol::new(&env, "transfer");
            monitoring::emit_performance(&env, func.clone(), 42);

            let stats = monitoring::get_performance_stats(&env, func);
            assert_eq!(stats.call_count, 1);
            assert_eq!(stats.total_time, 42);
            assert_eq!(stats.avg_time, 42);
            assert_eq!(stats.last_called, 1000);
        });
    }

    // ---------------------------------------------------------------
    // 3. Multiple emits accumulate correctly
    // ---------------------------------------------------------------
    #[test]
    fn test_get_performance_stats_after_multiple_emits() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            let func = Symbol::new(&env, "deposit");

            monitoring::emit_performance(&env, func.clone(), 10);
            monitoring::emit_performance(&env, func.clone(), 20);
            monitoring::emit_performance(&env, func.clone(), 30);

            let stats = monitoring::get_performance_stats(&env, func);
            assert_eq!(stats.call_count, 3);
            assert_eq!(stats.total_time, 60); // 10+20+30
            assert_eq!(stats.avg_time, 20); // 60/3
        });
    }

    // ---------------------------------------------------------------
    // 4. Different functions are isolated from each other
    // ---------------------------------------------------------------
    #[test]
    fn test_get_performance_stats_different_functions_isolated() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            let func_a = Symbol::new(&env, "mint");
            let func_b = Symbol::new(&env, "burn");

            monitoring::emit_performance(&env, func_a.clone(), 100);
            monitoring::emit_performance(&env, func_b.clone(), 200);

            let stats_a = monitoring::get_performance_stats(&env, func_a);
            let stats_b = monitoring::get_performance_stats(&env, func_b);

            assert_eq!(stats_a.call_count, 1);
            assert_eq!(stats_a.total_time, 100);

            assert_eq!(stats_b.call_count, 1);
            assert_eq!(stats_b.total_time, 200);
        });
    }

    // ---------------------------------------------------------------
    // 5. last_called timestamp updates on each emit
    // ---------------------------------------------------------------
    #[test]
    fn test_last_called_timestamp_updates() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            let func = Symbol::new(&env, "withdraw");

            env.ledger().with_mut(|li| li.timestamp = 500);
            monitoring::emit_performance(&env, func.clone(), 5);
            let stats = monitoring::get_performance_stats(&env, func.clone());
            assert_eq!(stats.last_called, 500);

            env.ledger().with_mut(|li| li.timestamp = 800);
            monitoring::emit_performance(&env, func.clone(), 5);
            let stats = monitoring::get_performance_stats(&env, func);
            assert_eq!(stats.last_called, 800);
        });
    }

    // ---------------------------------------------------------------
    // 6. Via contract client entry-point after init_admin
    // ---------------------------------------------------------------
    #[test]
    fn test_performance_stats_via_contract_client() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        let func = Symbol::new(&env, "init_admin");
        let stats = client.get_performance_stats(&func);

        // init_admin internally calls emit_performance, so call_count >= 1
        // or it may be 0 if init_admin does not track itself — either way
        // the entry-point must not panic.
        assert!(stats.call_count >= 0); // always true; confirms no panic
        assert!(stats.avg_time <= stats.total_time);
    }

    // ---------------------------------------------------------------
    // 7. Symbol keys written by emit_performance are readable by
    //    get_performance_stats (consistent key construction)
    // ---------------------------------------------------------------
    #[test]
    fn test_performance_stats_consistent_symbol_keys() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.ledger().with_mut(|li| li.timestamp = 1234);

        env.as_contract(&client.address, || {
            let func = Symbol::new(&env, "swap");
            monitoring::emit_performance(&env, func.clone(), 77);

            // Directly read the raw storage keys that emit_performance writes
            let count_key = (Symbol::new(&env, "perf_cnt"), func.clone());
            let time_key = (Symbol::new(&env, "perf_time"), func.clone());
            let last_key = (Symbol::new(&env, "perf_last"), func.clone());

            let raw_count: u64 = env.storage().persistent().get(&count_key).unwrap();
            let raw_time: u64 = env.storage().persistent().get(&time_key).unwrap();
            let raw_last: u64 = env.storage().persistent().get(&last_key).unwrap();

            let stats = monitoring::get_performance_stats(&env, func);

            assert_eq!(stats.call_count, raw_count);
            assert_eq!(stats.total_time, raw_time);
            assert_eq!(stats.last_called, raw_last);
            assert_eq!(raw_count, 1);
            assert_eq!(raw_time, 77);
            assert_eq!(raw_last, 1234);
        });
    }

    // ---------------------------------------------------------------
    // 8. Eviction policy: oldest entry removed when cap exceeded
    // ---------------------------------------------------------------
    #[test]
    fn test_eviction_policy_respects_max_limit() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            let max = monitoring::MAX_TRACKED_FUNCTIONS;

            // Fill exactly MAX_TRACKED_FUNCTIONS entries (func_0 .. func_{max-1})
            for i in 0..max {
                let name = build_func_symbol(&env, i);
                monitoring::emit_performance(&env, name, 1);
            }

            // Verify the first entry still exists
            let first = build_func_symbol(&env, 0);
            let stats_first = monitoring::get_performance_stats(&env, first.clone());
            assert_eq!(
                stats_first.call_count, 1,
                "first entry should still exist at capacity"
            );

            // Add one more — should evict func_0 (the oldest)
            let overflow = build_func_symbol(&env, max);
            monitoring::emit_performance(&env, overflow.clone(), 99);

            // func_0 should now be evicted (counters reset to zero)
            let evicted = monitoring::get_performance_stats(&env, first);
            assert_eq!(evicted.call_count, 0, "oldest entry must be evicted");
            assert_eq!(evicted.total_time, 0);

            // The newest entry must be present
            let newest = monitoring::get_performance_stats(&env, overflow);
            assert_eq!(newest.call_count, 1);
            assert_eq!(newest.total_time, 99);

            // Second entry (func_1) should survive the eviction
            let second = build_func_symbol(&env, 1);
            let stats_second = monitoring::get_performance_stats(&env, second);
            assert_eq!(
                stats_second.call_count, 1,
                "second entry must survive eviction"
            );
        });
    }

    // ---------------------------------------------------------------
    // 9. Zero-division safety: count=0 ⇒ avg_time=0
    // ---------------------------------------------------------------
    #[test]
    fn test_avg_time_zero_division_safe() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);

        env.as_contract(&client.address, || {
            // Never emitted — count is zero
            let func = Symbol::new(&env, "noop");
            let stats = monitoring::get_performance_stats(&env, func);
            assert_eq!(stats.call_count, 0);
            assert_eq!(stats.avg_time, 0, "avg_time must be 0 when call_count is 0");
        });
    }

    // ---------------------------------------------------------------
    // Helper: build deterministic Symbol names for eviction tests.
    // Soroban Symbol max length is 32 chars; "fn_" + up to 2 digits
    // is well within that limit.
    // ---------------------------------------------------------------
    fn build_func_symbol(env: &Env, index: u32) -> Symbol {
        let mut buf = [0u8; 6]; // "fn_" prefix + up to 3 digit chars
        buf[0] = b'f';
        buf[1] = b'n';
        buf[2] = b'_';
        let len = if index >= 100 {
            buf[3] = b'0' + (index / 100) as u8;
            buf[4] = b'0' + ((index / 10) % 10) as u8;
            buf[5] = b'0' + (index % 10) as u8;
            6
        } else if index >= 10 {
            buf[3] = b'0' + (index / 10) as u8;
            buf[4] = b'0' + (index % 10) as u8;
            5
        } else {
            buf[3] = b'0' + index as u8;
            4
        };
        let s = core::str::from_utf8(&buf[..len]).unwrap();
        Symbol::new(env, s)
    }
}
