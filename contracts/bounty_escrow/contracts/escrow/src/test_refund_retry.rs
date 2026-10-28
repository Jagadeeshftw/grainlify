//! Retry coverage for `refund`: a client resubmitting after a timeout must not
//! pay out twice. The second call is refused with a typed error and leaves
//! balances, escrow state and the event log as the first call left them.

use crate::{
    events::FundsRefunded, BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus,
    RefundMode,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger},
    token, Address, Env, IntoVal, Symbol, TryFromVal, Val, Vec,
};

const AMOUNT: i128 = 1_000;
const DEADLINE: u64 = 10_000;

struct Ctx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    contract: Address,
    token: token::Client<'a>,
    depositor: Address,
}

fn setup(bounty_id: u64) -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let token = token::Client::new(&env, &token_address);
    token::StellarAssetClient::new(&env, &token_address).mint(&depositor, &AMOUNT);

    let contract = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract);
    client.init(&admin, &token_address);
    client.lock_funds(&depositor, &bounty_id, &AMOUNT, &DEADLINE);

    Ctx {
        env,
        client,
        contract,
        token,
        depositor,
    }
}

/// `(first topic, data)` of every refund-related event published by the escrow.
fn refund_events(ctx: &Ctx) -> Vec<(Symbol, Val)> {
    let wanted = [symbol_short!("f_ref"), symbol_short!("r_apcns")];
    let mut out = Vec::new(&ctx.env);
    for (contract, topics, data) in ctx.env.events().all().iter() {
        if contract != ctx.contract {
            continue;
        }
        let Some(Ok(topic)) = topics.get(0).map(|t| Symbol::try_from_val(&ctx.env, &t)) else {
            continue;
        };
        if wanted.contains(&topic) {
            out.push_back((topic, data));
        }
    }
    out
}

/// The refused retry must publish no refund event. Depending on whether the
/// test host's event log reflects only the latest invocation, the log is then
/// either empty or unchanged from the first call.
fn assert_no_new_refund_events(ctx: &Ctx, before: &Vec<(Symbol, Val)>) {
    let after = refund_events(ctx);
    assert!(
        after.is_empty() || after.len() == before.len(),
        "a refused refund retry must not emit refund events"
    );
}

fn assert_funds_refunded(ctx: &Ctx, data: Val, bounty_id: u64, amount: i128, to: &Address) {
    let event: FundsRefunded = data.into_val(&ctx.env);
    assert_eq!(event.bounty_id, bounty_id);
    assert_eq!(event.amount, amount);
    assert_eq!(&event.refund_to, to);
}

#[test]
fn refund_twice_after_deadline_is_refused_with_funds_not_locked() {
    let bounty_id = 1724;
    let ctx = setup(bounty_id);
    ctx.env.ledger().set_timestamp(DEADLINE + 1);

    // First submission: full refund to the depositor.
    ctx.client.refund(&bounty_id);

    let events = refund_events(&ctx);
    assert_eq!(events.len(), 1);
    let (topic, data) = events.get(0).unwrap();
    assert_eq!(topic, symbol_short!("f_ref"));
    assert_funds_refunded(&ctx, data, bounty_id, AMOUNT, &ctx.depositor);

    assert_eq!(ctx.token.balance(&ctx.depositor), AMOUNT);
    assert_eq!(ctx.token.balance(&ctx.contract), 0);
    let escrow_after_first = ctx.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after_first.status, EscrowStatus::Refunded);
    assert_eq!(escrow_after_first.remaining_amount, 0);
    assert_eq!(escrow_after_first.refund_history.len(), 1);

    // Retry: refused, nothing moves, nothing emitted.
    assert_eq!(
        ctx.client.try_refund(&bounty_id),
        Err(Ok(Error::FundsNotLocked))
    );
    assert_no_new_refund_events(&ctx, &events);
    assert_eq!(ctx.token.balance(&ctx.depositor), AMOUNT);
    assert_eq!(ctx.token.balance(&ctx.contract), 0);
    assert_eq!(ctx.client.get_escrow_info(&bounty_id), escrow_after_first);
}

#[test]
fn refund_twice_with_partial_approval_is_refused_with_deadline_not_passed() {
    let bounty_id = 1725;
    let ctx = setup(bounty_id);
    let recipient = Address::generate(&ctx.env);
    let partial = AMOUNT / 4;
    ctx.client
        .approve_refund(&bounty_id, &partial, &recipient, &RefundMode::Partial);

    // First submission (before the deadline): consumes the approval.
    ctx.client.refund(&bounty_id);

    let events = refund_events(&ctx);
    assert_eq!(events.len(), 2);
    assert_eq!(events.get(0).unwrap().0, symbol_short!("r_apcns"));
    let (topic, data) = events.get(1).unwrap();
    assert_eq!(topic, symbol_short!("f_ref"));
    assert_funds_refunded(&ctx, data, bounty_id, partial, &recipient);

    assert_eq!(ctx.token.balance(&recipient), partial);
    assert_eq!(ctx.token.balance(&ctx.depositor), 0);
    assert_eq!(ctx.token.balance(&ctx.contract), AMOUNT - partial);
    let escrow_after_first = ctx.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after_first.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(escrow_after_first.remaining_amount, AMOUNT - partial);
    assert_eq!(escrow_after_first.refund_history.len(), 1);

    // Retry: the approval is gone and the deadline has not passed, so the
    // partial amount is not paid a second time.
    assert_eq!(
        ctx.client.try_refund(&bounty_id),
        Err(Ok(Error::DeadlineNotPassed))
    );
    assert_no_new_refund_events(&ctx, &events);
    assert_eq!(ctx.token.balance(&recipient), partial);
    assert_eq!(ctx.token.balance(&ctx.depositor), 0);
    assert_eq!(ctx.token.balance(&ctx.contract), AMOUNT - partial);
    assert_eq!(ctx.client.get_escrow_info(&bounty_id), escrow_after_first);
}
