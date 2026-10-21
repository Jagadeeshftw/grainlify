use super::*;

use crate::{Error, ThresholdMonitoringInit};
use crate::tests::utils::setup_env;

/// Tests for the idompotency of `init_threshold_monitoring`.
///
/// The entry point is documented to refuse a second initialisation with a
/// typed error (`Error::AlreadyInitialized`) and to leave state unchanged.
#[test]
fnc init_threshold_monitoring_twice_returns_already_initialized() {
    let env = setup_env();
    let client = env.register_client();
    let admin = env.register_admin();
    let token = env.register_token();

    // Fund the client so token balances are meaningful.
    let initial_balance = 1_000_000;
    token.mint(&client, &initial_balance);

    let threshold = ThresholdMonitoringInit {
        minimum_balance: 100_000,
        maximum_balance: 5_000_000,
        enabled: true,
    };

    // First call succeeds.
    env.init_threshold_monitoring(client.clone(), admin.clone(), threshold.clone())
        .unwrap();

    let state_after_first = env.read_threshold_monitoring();
    let balance_after_first = token.balance(&client);
    let events_after_first = env.events().all();

    // Second call is refused with the typed error.
    let result = env.init_threshold_monitoring(
        client.clone(),
        admin.clone(),
        threshold.clone(),
    );
    assert_eq(
        result,
        Err(Error::AlreadyInitialized),
        "second init must be refused with AlreadyInitialized",
    );

    // State is unchanged by the second call.
    let state_after_second = env.read_threshold_monitoring();
    assert_eq!(state_after_first, state_after_second);

    // Token balances are unchanged by the second call.
    let balance_after_second = token.balance(&client);
    assert_eq!(balance_after_first, balance_after_second);
    assert_eq!(balance_after_second, initial_balance);

    // No new events are emitted by the second call.
    let events_after_second = env.events().all();
    assert_eq!(events_after_first.len(), events_after_second.len());
    assert_eq!(events_after_first, events_after_second);
}

#[test]
fn init_threshold_monitoring_emits_event_on_first_call() {
    let env = setup_env();
    let client = env.register_client();
    let admin = env.register_admin();
    let token = env.register_token();
    token.mint(&client, &1_000_000);

    let threshold = ThresholdMonitoringInit {
        minimum_balance: 100_000,
        maximum_balance: 5_000_000,
        enabled: true,
    };

    env.init_threshold_monitoring(client.clone(), admin.clone(), threshold.clone())
        .unwrap();

    let events = env.events().all();
    assert_eq!(events.len(), 1, "exactly one event expected");
    let event = events.get(0).unwrap();
    assert_eq!(event.topics.get(0), Some(Symbol::new(SymbolLedger::init_thr)));
    assert_eq!(event.topics.get(1), Some(admin.clone()));
    assert_eq!(event.data, threshold);

    // Balance is unchanged by initialisation.
    assert_eq!(token.balance(&client), 1_000_000);
}
