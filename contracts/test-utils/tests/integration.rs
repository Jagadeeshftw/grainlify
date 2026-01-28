use grainlify_test_utils::{assert_events_match, TestEnv};
use soroban_sdk::{Env, Symbol};

#[test]
fn test_library_usage() {
    let test_env = TestEnv::new();
    let env = &test_env.env;

    // Test that we can use the environment
    let _admin = test_env.admin;

    // Verify assertion helper compilation
    assert_events_match(env, &[]);
}
