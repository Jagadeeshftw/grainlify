#[cfg(test)]
mod test {
    use crate::{monitoring, DataKey, GrainlifyContract, GrainlifyContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

    fn setup_test(env: &Env) -> (GrainlifyContractClient, Address) {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.init_admin(&admin);
        (client, admin)
    }

    #[test]
    fn test_healthy_state_passes_verification() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);

        // Record some successful operations
        monitoring::track_operation(&env, Symbol::new(&env, "op1"), admin.clone(), true);
        monitoring::track_operation(&env, Symbol::new(&env, "op2"), admin.clone(), true);

        // Verify state is healthy
        assert!(monitoring::verify_invariants(&env));
    }

    #[test]
    fn test_tampered_state_fails_verification() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);

        // Record a single successful operation
        monitoring::track_operation(&env, Symbol::new(&env, "op1"), admin.clone(), true);

        // Verify healthy initially
        assert!(monitoring::verify_invariants(&env));

        // TAMPER: Manually overwrite OPERATION_COUNT (op_count) in storage to 0
        // while leaving ERROR_COUNT or tracks that imply operations happened.
        // Actually, let's make ERROR_COUNT > OPERATION_COUNT.

        let op_key = Symbol::new(&env, "op_count");
        let err_key = Symbol::new(&env, "err_count");

        // Force 5 errors but only 2 total operations (Inconsistent!)
        env.storage().persistent().set(&op_key, &2u64);
        env.storage().persistent().set(&err_key, &5u64);

        // Verify that verification detects the drift
        assert!(
            !monitoring::verify_invariants(&env),
            "Invariants should fail when error_count > operation_count"
        );
    }

    #[test]
    fn test_user_drift_tampering() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);

        let op_key = Symbol::new(&env, "op_count");
        let usr_key = Symbol::new(&env, "usr_count");

        // Force 10 unique users but only 5 operations (Inconsistent!)
        env.storage().persistent().set(&op_key, &5u64);
        env.storage().persistent().set(&usr_key, &10u64);

        // Verify that verification detects the drift
        assert!(
            !monitoring::verify_invariants(&env),
            "Invariants should fail when unique_users > operation_count"
        );
    }
}
