#[cfg(test)]
mod test {
    use crate::{ConfigPayload, GrainlifyContract, GrainlifyContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Address, Env, IntoVal, Vec,
    };

    fn setup_test(env: &Env) -> (GrainlifyContractClient<'_>, Vec<Address>) {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(env, &contract_id);

        let mut signers = Vec::new(env);
        signers.push_back(Address::generate(env));
        signers.push_back(Address::generate(env));
        signers.push_back(Address::generate(env));

        client.init(&signers, &2u32);
        (client, signers)
    }

    #[test]
    fn test_propose_and_approve_config_change() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, signers) = setup_test(&env);

        let signer1 = signers.get(0).unwrap();
        let signer2 = signers.get(1).unwrap();

        let payload = ConfigPayload::LockFeeRate(100);
        let proposal_id = client.propose_config_change(&signer1, &payload);

        // First approval
        client.approve_config_change(&proposal_id, &signer1);

        // Verify not yet executed
        // (We can't easily check internal storage via client without view methods,
        // but we can check if it panics or if we add a view method)

        // Second approval should trigger execution
        client.approve_config_change(&proposal_id, &signer2);

        // If we had a view method for GlobalFeeRate, we'd check it here.
        // For now, we rely on the fact that it doesn't panic and we can check events.
        let events = env.events().all();
        let mut executed_event_found = false;
        for event in events.iter() {
            let topics = event.1;
            let target_topic: soroban_sdk::Val =
                soroban_sdk::symbol_short!("cfg_exec").into_val(&env);
            if topics.contains(target_topic) {
                executed_event_found = true;
                break;
            }
        }
        assert!(executed_event_found);
    }

    #[test]
    fn test_cancel_config_change() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, signers) = setup_test(&env);

        let signer1 = signers.get(0).unwrap();

        let payload = ConfigPayload::LockFeeRate(100);
        let proposal_id = client.propose_config_change(&signer1, &payload);

        client.cancel_config_change(&proposal_id, &signer1);

        // Attempting to approve a cancelled proposal should panic
        let result = client.try_approve_config_change(&proposal_id, &signer1);
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "AlreadyExecuted")]
    fn test_cannot_approve_already_executed() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, signers) = setup_test(&env);

        let signer1 = signers.get(0).unwrap();
        let signer2 = signers.get(1).unwrap();

        let payload = ConfigPayload::LockFeeRate(100);
        let proposal_id = client.propose_config_change(&signer1, &payload);

        client.approve_config_change(&proposal_id, &signer1);
        client.approve_config_change(&proposal_id, &signer2);

        // Try to approve again
        client.approve_config_change(&proposal_id, &signer1);
    }
}
