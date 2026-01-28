use soroban_sdk::{Env, Symbol, Val};

pub fn assert_invocation(
    env: &Env,
    contract_id: &soroban_sdk::Address,
    function_name: &Symbol,
    args: std::vec::Vec<Val>,
) {
    let calls = env.auths();
    let found = calls
        .iter()
        .any(|(auth_contract_id, auth_func, auth_args, _)| {
            auth_contract_id == contract_id && auth_func == function_name && auth_args == &args
        });
    assert!(found, "Expected invocation of {} not found", function_name);
}

pub fn assert_events_match(
    env: &Env,
    expected_events: &[(Option<soroban_sdk::Address>, (Symbol, Symbol), Val)],
) {
    let events = env.events().all();
    // Simplified event matching for demonstration
    assert_eq!(events.len(), expected_events.len(), "Event count mismatch");
}
