#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_batch_initialize_programs_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-1"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-2"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 2);
    assert!(client.program_exists());
}

#[test]
fn test_batch_initialize_programs_empty_err() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let items: Vec<ProgramInitItem> = Vec::new(&env);
    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::InvalidBatchSizeProgram))));
}

#[test]
fn test_batch_initialize_programs_duplicate_id_err() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let pid = String::from_str(&env, "same-id");
    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: pid.clone(),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: pid,
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));
}

// =============================================================================
// EXTENDED TESTS FOR batch_initialize_programs
// =============================================================================

/// Helper: build a deterministic program ID for large-batch tests.
fn make_program_id(env: &Env, index: u32) -> String {
    let mut buf = [b'p', b'-', b'0', b'0', b'0', b'0', b'0'];
    let mut n = index;
    let mut pos = 6usize;
    loop {
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 || pos == 2 {
            break;
        }
        pos -= 1;
    }
    String::from_str(env, core::str::from_utf8(&buf).unwrap())
}

#[test]
fn test_batch_register_happy_path_five_programs() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    for i in 0..5u32 {
        items.push_back(ProgramInitItem {
            program_id: make_program_id(&env, i),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        });
    }

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 5);

    for i in 0..5u32 {
        assert!(client.program_exists_by_id(&make_program_id(&env, i)));
    }
}

#[test]
fn test_batch_register_single_item() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "solo-prog"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 1);
    assert!(client.program_exists_by_id(&String::from_str(&env, "solo-prog")));
}

#[test]
fn test_batch_register_exceeds_max_batch_size() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    for i in 0..(MAX_BATCH_SIZE + 1) {
        items.push_back(ProgramInitItem {
            program_id: make_program_id(&env, i),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        });
    }

    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::InvalidBatchSizeProgram))));
}

#[test]
fn test_batch_register_at_exact_max_batch_size() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    for i in 0..MAX_BATCH_SIZE {
        items.push_back(ProgramInitItem {
            program_id: make_program_id(&env, i),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        });
    }

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, MAX_BATCH_SIZE);

    // Spot-check first, middle, and last entries
    assert!(client.program_exists_by_id(&make_program_id(&env, 0)));
    assert!(client.program_exists_by_id(&make_program_id(&env, 50)));
    assert!(client.program_exists_by_id(&make_program_id(&env, MAX_BATCH_SIZE - 1)));
}

#[test]
fn test_batch_register_program_already_exists_error() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Register first batch
    let mut first = Vec::new(&env);
    first.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "existing"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    client
        .try_batch_initialize_programs(&first)
        .unwrap()
        .unwrap();

    // Second batch contains the same ID â€” must fail entirely
    let mut second = Vec::new(&env);
    second.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "brand-new"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    second.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "existing"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&second);
    assert!(matches!(res, Err(Ok(BatchError::ProgramAlreadyExists))));

    // "brand-new" must NOT exist â€” all-or-nothing semantics
    assert!(!client.program_exists_by_id(&String::from_str(&env, "brand-new")));
}

#[test]
fn test_batch_register_all_or_nothing_on_duplicate() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Batch with valid IDs plus a duplicate â€” entire batch must be rejected
    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "alpha"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "beta"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "alpha"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));

    // Neither program should exist
    assert!(!client.program_exists_by_id(&String::from_str(&env, "alpha")));
    assert!(!client.program_exists_by_id(&String::from_str(&env, "beta")));
}

#[test]
fn test_batch_register_duplicate_at_tail() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "unique-1"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "dup-tail"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "dup-tail"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));
}

#[test]
fn test_batch_register_different_auth_keys_and_tokens() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin_a = Address::generate(&env);
    let admin_b = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-a"),
        authorized_payout_key: admin_a.clone(),
        token_address: token_a.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-b"),
        authorized_payout_key: admin_b.clone(),
        token_address: token_b.clone(),
        reference_hash: None,
    });

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 2);
    assert!(client.program_exists_by_id(&String::from_str(&env, "prog-a")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "prog-b")));
}

#[test]
fn test_batch_register_events_emitted_per_program() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let events_before = env.events().all().len();

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "evt-1"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "evt-2"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "evt-3"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();

    let events_after = env.events().all().len();
    let new_events = events_after - events_before;

    // At least one event per registered program
    assert!(
        new_events >= 3,
        "Expected at least 3 events for 3 programs, got {}",
        new_events
    );
}

#[test]
fn test_batch_register_sequential_batches_no_conflict() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First batch
    let mut batch1 = Vec::new(&env);
    batch1.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b1-a"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    batch1.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b1-b"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let c1 = client
        .try_batch_initialize_programs(&batch1)
        .unwrap()
        .unwrap();
    assert_eq!(c1, 2);

    // Second batch â€” different IDs
    let mut batch2 = Vec::new(&env);
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b2-a"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b2-b"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let c2 = client
        .try_batch_initialize_programs(&batch2)
        .unwrap()
        .unwrap();
    assert_eq!(c2, 2);

    // All four should exist
    assert!(client.program_exists_by_id(&String::from_str(&env, "b1-a")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "b1-b")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "b2-a")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "b2-b")));
}

#[test]
fn test_batch_register_second_batch_conflicts_with_first() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First batch succeeds
    let mut batch1 = Vec::new(&env);
    batch1.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "shared"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    client
        .try_batch_initialize_programs(&batch1)
        .unwrap()
        .unwrap();

    // Second batch reuses "shared" â€” must fail
    let mut batch2 = Vec::new(&env);
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "fresh"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "shared"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&batch2);
    assert!(matches!(res, Err(Ok(BatchError::ProgramAlreadyExists))));

    // "fresh" must not exist (all-or-nothing)
    assert!(!client.program_exists_by_id(&String::from_str(&env, "fresh")));
}

// =============================================================================
// TOKEN ALLOWLIST ENFORCEMENT TESTS (#1071)
// =============================================================================
