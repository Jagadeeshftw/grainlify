
#![cfg(test)]
extern crate std;
use crate::{ProgramEscrowContract, ProgramEscrowContractClient, ProgramFilter, PayoutFilter, Pagination, ProgramData};
use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env, String};

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

fn create_test_env<'a>(env: &'a Env) -> (ProgramEscrowContractClient<'a>, Address, Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    
    let admin = Address::generate(env);
    let (token, token_client, token_admin) = create_token_contract(env, &admin);
    
    // Initialize admin if needed (set_admin) or just rely on defaults for now if not strictly required for queries
    // BUT initialize_program requires nothing special other than args
    
    (client, admin, token, token_client, token_admin)
}

#[test]
fn test_get_programs_filtering() {
    let env = Env::default();
    let (client, _admin, token, _token_client, _token_admin) = create_test_env(&env);
    
    let backend1 = Address::generate(&env);
    let backend2 = Address::generate(&env);
    let token2 = Address::generate(&env); 
    
    // Create Programs
    let p1 = String::from_str(&env, "P1");
    let p2 = String::from_str(&env, "P2");
    let p3 = String::from_str(&env, "P3");
    
    client.initialize_program(&p1, &backend1, &token);
    client.initialize_program(&p2, &backend1, &token2);
    client.initialize_program(&p3, &backend2, &token);
    
    // Filter by Authorized Key (backend1)
    let filter_key = ProgramFilter {
        authorized_key: Some(backend1.clone()),
        token_address: None,
    };
    let page = Pagination { start_index: 0, limit: 10 };
    let progs_key = client.get_programs(&filter_key, &page);
    assert_eq!(progs_key.len(), 2); // P1 and P2
    
    // Filter by Token Address (token)
    let filter_token = ProgramFilter {
        authorized_key: None,
        token_address: Some(token.clone()),
    };
    let progs_token = client.get_programs(&filter_token, &page);
    assert_eq!(progs_token.len(), 2); // P1 and P3
    
    // Combined Filter
    let filter_both = ProgramFilter {
        authorized_key: Some(backend1.clone()),
        token_address: Some(token.clone()),
    };
    let progs_both = client.get_programs(&filter_both, &page);
    assert_eq!(progs_both.len(), 1); // P1
}

#[test]
fn test_get_payouts_filtering() {
    let env = Env::default();
    let (client, _admin, token, _token_client, token_admin) = create_test_env(&env);
    
    let backend = Address::generate(&env);
    let p1 = String::from_str(&env, "P1");
    client.initialize_program(&p1, &backend, &token);
    
    // Lock funds
    let amount = 1000i128;
    client.lock_program_funds(&p1, &amount);
    token_admin.mint(&client.address, &amount); // Mock funding the contract for payouts
    // Actually lock_program_funds converts caller funds.
    // We should mint to caller first.
    // But since we mock auths, we can just assume funds are there if we mint to the source?
    // Wait, lock_program_funds takes funds FROM caller.
    // I need to mint to 'backend' or whoever calls it?
    // client.lock_program_funds takes funds from caller. 
    // In test env, caller is usually unset/random if not specified with `mock_auths`.
    // Actually, `lock_program_funds` checks `program_data.authorized_payout_key.require_auth()`? 
    // No, `lock_program_funds` usually requires auth from the one providing funds.
    // Let's check `lock_program_funds` implementation.
    // "Transfer funds from caller" -> `client.transfer(&env.current_contract_address(), &amount)`.
    // Wait, typical pattern: `token_client.transfer_from(...)` or `transfer`.
    // Usually implementation requires auth.
    
    // Let's just bypass complex funding and validation if possible or mock correctly.
    // We'll trust `single_payout` logic to add to history.
    // But `single_payout` checks balance.
    // So we must fund it.
    
    // Simpler: Just mint directly to contract address to simulate locked funds?
    // `ProgramEscrow` tracks `total_funds` and `remaining_balance`.
    // `lock_program_funds` updates these.
    
    // Let's mint to an arbitrary address "funder" using token_admin
    let funder = Address::generate(&env);
    token_admin.mint(&funder, &10000);
    
    // We need to call lock_program_funds AS the funder.
    // In Soroban tests, we can use `client.mock_auths(&[])` but to switch caller identity for `token.transfer` inside contract...
    // Actually `lock_program_funds` usually takes `from` address?
    // Checking lib.rs... `pub fn lock_program_funds(env: Env, program_id: String, amount: i128)`
    // It calls `token_client.transfer(&from, &contract, &amount)`.
    // Wait, `lock_program_funds` in `program-escrow` usually infers `from` or takes it.
    // Line 778: `fn lock_program_funds(env: Env, program_id: String, amount: i128)`
    // Inside: `let token_client...`
    // It transfers from whom?
    // Ah, usually it relies on `env.invoker()`.
    // But Soroban v20+ doesn't have `invoker()`.
    // It usually requires `depositor: Address` arg.
    // Let's check `lock_program_funds` signature in `program-escrow/src/lib.rs`.
    
    // Assuming it takes `from` or similar. If not, it might be broken or I missed it.
    // Actually let's assume I can just mint to contract and manually update storage? No, encapsulation.
    
    // Re-checking `program-escrow/src/lib.rs` (lines 700-1499).
    // I didn't verify `lock_program_funds` implementation details closely in the view.
    // BUT `test_batch_payout_mismatched_lengths` in step 116 calls `client.lock_program_funds(&prog_id, &10_000_0000000);`
    // It passes 2 args. 
    // And `initialize_program` takes 3 args.
    
    // So I will follow that pattern.
    // It seems `lock_program_funds` implies the caller is the one paying? 
    // If I mock all auths, it might just succeed if logic allows.
    // But `token_client.transfer` needs a source.
    
    // For `get_payouts`, I need `payout_history` to be populated.
    // `single_payout` or `batch_payout` adds to history.
    
    // I'll try to simulate payouts.
    // 1. Initialize
    // 2. Lock funds (mocking success)
    let amount_locked = 5000i128;
    // We need to ensure the contract has tokens to pay out.
    // So mint to contract address.
    token_admin.mint(&client.address, &amount_locked);
    
    // We also need to update contract state to know it has balance.
    // Calling `lock_program_funds` might fail if it tries to pull from "caller" and caller has no funds.
    // But if we mock auths, maybe it works if we don't check balance of caller?
    // Token contract checks balance.
    
    // Let's assume `lock_program_funds` pulls from an implicit caller. 
    // In tests `client.lock_program_funds` comes from... test env?
    // Usually we specify `client.mock_auths`.
    
    // To be safe, I'll bypass `lock_program_funds` if it's tricky, and just focus on what I can control.
    // But I can't write to storage directly from test easily without key visibility.
    
    // Let's try to just run it:
    // Mint to a "funder".
    // call lock.
    // But client call doesn't specify sender.
    // Maybe `program-escrow` logic is: `token.transfer_from(&payout_key, ...)`?
    
    // Correct approach for integration test:
    // 1. Mint to `backend` (authorized key).
    token_admin.mint(&backend, &amount_locked);
    // 2. Call `lock_program_funds`. If it pulls from `backend` (auth key), it works.
    // If it pulls from `env.caller()`, then in test `client.lock_program_funds` usually uses a default caller or we verify who it calls.
    
    // Let's try `lock_program_funds`. If it fails, I'll debug.
    client.lock_program_funds(&p1, &amount_locked);
    
    // 3. Perform Payouts
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    
    let now = env.ledger().timestamp();
    
    client.single_payout(&p1, &recipient1, &100);
    // Advance time
    env.ledger().set_timestamp(now + 100);
    client.single_payout(&p1, &recipient2, &200);
    
    // Query Payouts
    let filter_r1 = PayoutFilter {
        recipient: Some(recipient1.clone()),
        min_amount: None, max_amount: None, start_time: None, end_time: None
    };
    let payouts_r1 = client.get_payouts(&p1, &filter_r1);
    assert_eq!(payouts_r1.len(), 1);
    assert_eq!(payouts_r1.get(0).unwrap().amount, 100);
    
    let filter_amt = PayoutFilter {
        recipient: None,
        min_amount: Some(150),
        max_amount: None, start_time: None, end_time: None
    };
    let payouts_amt = client.get_payouts(&p1, &filter_amt);
    assert_eq!(payouts_amt.len(), 1);
    assert_eq!(payouts_amt.get(0).unwrap().recipient, recipient2);
}
