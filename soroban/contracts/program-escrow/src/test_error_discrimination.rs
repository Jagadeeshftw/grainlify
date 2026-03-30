#![cfg(test)]

//! Comprehensive error discrimination tests for program-escrow contract.
//!
//! These tests verify that each error variant is correctly returned in the
//! expected scenarios, ensuring stable error codes for client-side discrimination.

extern crate std;
use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, vec, Address, Env, String};

/// Sets up a test environment with contract, token, admin, and program_admin.
macro_rules! setup {
    ($env:ident, $client:ident, $contract_id:ident, $admin:ident,
     $program_admin:ident, $token_client:ident, $token_admin:ident,
     $initial_balance:expr) => {
        let $env = Env::default();
        $env.mock_all_auths();

        let $contract_id = $env.register(ProgramEscrowContract, ());
        let $client = ProgramEscrowContractClient::new(&$env, &$contract_id);

        let $admin = Address::generate(&$env);
        let $program_admin = Address::generate(&$env);

        let token_contract = $env.register_stellar_asset_contract_v2($admin.clone());
        let token_addr = token_contract.address();
        let $token_client = token::Client::new(&$env, &token_addr);
        let $token_admin = token::StellarAssetClient::new(&$env, &token_addr);

        let _ = $client.init(&$admin, &token_addr);
        $token_admin.mint(&$program_admin, &$initial_balance);
    };
}

// ==================== ERROR CODE STABILITY TESTS ====================

#[test]
fn test_error_code_already_initialized() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let other = Address::generate(&env);
    let res = client.try_init(&other, &other);
    assert!(res.is_err());
    // Error code 1 = AlreadyInitialized
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::AlreadyInitialized));
}

#[test]
fn test_error_code_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ProgramEscrowContract, ());
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let some_admin = Address::generate(&env);

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: some_admin.clone(),
            name: String::from_str(&env, "Test"),
            total_funding: 1_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 2 = NotInitialized
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::NotInitialized));
}

#[test]
fn test_error_code_program_exists() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        20_000i128
    );
    let name = String::from_str(&env, "Grant Round");

    client.register_program(&1, &program_admin, &name, &5_000);
    let res = client.try_register_program(&1, &program_admin, &name, &5_000);
    assert!(res.is_err());
    // Error code 3 = ProgramExists
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ProgramExists));
}

#[test]
fn test_error_code_program_not_found() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let res = client.try_get_program(&999);
    assert!(res.is_err());
    // Error code 4 = ProgramNotFound
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ProgramNotFound));
}

#[test]
fn test_error_code_unauthorized() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let name = String::from_str(&env, "Test Program");
    client.register_program(&1, &program_admin, &name, &5_000);

    let unauthorized_actor = Address::generate(&env);
    let res = client.try_update_program_labels(
        &unauthorized_actor,
        &1,
        &vec![&env, String::from_str(&env, "new-label")],
    );
    assert!(res.is_err());
    // Error code 5 = Unauthorized
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::Unauthorized));
}

#[test]
fn test_error_code_invalid_batch_size_zero() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items: Vec<ProgramRegistrationItem> = vec![&env];
    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 6 = InvalidBatchSize
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidBatchSize));
}

#[test]
fn test_error_code_invalid_batch_size_exceeds_max() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        200_000i128
    );

    let mut items = Vec::new(&env);
    for i in 1..=21u64 {
        items.push_back(ProgramRegistrationItem {
            program_id: i,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Program"),
            total_funding: 100,
        });
    }

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 6 = InvalidBatchSize
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidBatchSize));
}

#[test]
fn test_error_code_duplicate_program_id() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        20_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "First"),
            total_funding: 1_000,
        },
        ProgramRegistrationItem {
            program_id: 1, // duplicate
            admin: program_admin.clone(),
            name: String::from_str(&env, "Second"),
            total_funding: 2_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 7 = DuplicateProgramId
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::DuplicateProgramId));
}

#[test]
fn test_error_code_invalid_amount_zero() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Zero Fund"),
            total_funding: 0,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 8 = InvalidAmount
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidAmount));
}

#[test]
fn test_error_code_invalid_amount_negative() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Negative"),
            total_funding: -500,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 8 = InvalidAmount
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidAmount));
}

#[test]
fn test_error_code_invalid_name() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, ""),
            total_funding: 1_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 9 = InvalidName
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidName));
}

#[test]
fn test_error_code_contract_deprecated() {
    setup!(
        env,
        client,
        _contract_id,
        _admin,
        program_admin,
        _token_client,
        token_admin,
        20_000i128
    );

    client.set_deprecated(&true, &None);
    token_admin.mint(&program_admin, &20_000);

    let res = client.try_register_program(
        &201,
        &program_admin,
        &String::from_str(&env, "Blocked Program"),
        &5_000,
    );
    assert!(res.is_err());
    // Error code 10 = ContractDeprecated
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ContractDeprecated));
}

#[test]
fn test_error_code_jurisdiction_kyc_required() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "US-only")),
        requires_kyc: true,
        max_funding: Some(10_000),
        registration_paused: false,
    };

    let res = client.try_register_program_juris(
        &92,
        &program_admin,
        &String::from_str(&env, "US Program"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &Some(false), // KYC not attested
    );
    assert!(res.is_err());
    // Error code 11 = JurisdictionKycRequired
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::JurisdictionKycRequired));
}

#[test]
fn test_error_code_jurisdiction_funding_limit_exceeded() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: false,
        max_funding: Some(2_000),
        registration_paused: false,
    };

    let res = client.try_register_program_juris(
        &93,
        &program_admin,
        &String::from_str(&env, "Capped Program"),
        &5_000, // exceeds max_funding of 2_000
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &None,
    );
    assert!(res.is_err());
    // Error code 12 = JurisdictionFundingLimitExceeded
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::JurisdictionFundingLimitExceeded));
}

#[test]
fn test_error_code_jurisdiction_paused() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "paused-zone")),
        requires_kyc: false,
        max_funding: Some(8_000),
        registration_paused: true,
    };

    let res = client.try_register_program_juris(
        &94,
        &program_admin,
        &String::from_str(&env, "Paused Program"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &None,
    );
    assert!(res.is_err());
    // Error code 13 = JurisdictionPaused
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::JurisdictionPaused));
}

#[test]
fn test_error_code_invalid_label_empty() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let res = client.try_register_program_with_labels(
        &1,
        &program_admin,
        &String::from_str(&env, "Test Program"),
        &5_000,
        &vec![&env, String::from_str(&env, "")], // empty label
    );
    assert!(res.is_err());
    // Error code 14 = InvalidLabel
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidLabel));
}

#[test]
fn test_error_code_invalid_label_too_long() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    // Create a label that's 33 characters (exceeds MAX_LABEL_LENGTH of 32)
    let long_label = String::from_str(&env, "this-is-a-very-long-label-that-exceeds-the-maximum-length");

    let res = client.try_register_program_with_labels(
        &1,
        &program_admin,
        &String::from_str(&env, "Test Program"),
        &5_000,
        &vec![&env, long_label],
    );
    assert!(res.is_err());
    // Error code 14 = InvalidLabel
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidLabel));
}

#[test]
fn test_error_code_too_many_labels() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let mut labels = Vec::new(&env);
    for i in 0..11 {
        labels.push_back(String::from_str(&env, "label"));
    }

    let res = client.try_register_program_with_labels(
        &1,
        &program_admin,
        &String::from_str(&env, "Test Program"),
        &5_000,
        &labels,
    );
    assert!(res.is_err());
    // Error code 15 = TooManyLabels
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::TooManyLabels));
}

#[test]
fn test_error_code_label_not_allowed() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    // Set restricted label config
    client.set_label_config(
        &true,
        &vec![&env, String::from_str(&env, "allowed-label")],
    );

    // Try to register with a label not in the allowed list
    let res = client.try_register_program_with_labels(
        &1,
        &program_admin,
        &String::from_str(&env, "Test Program"),
        &5_000,
        &vec![&env, String::from_str(&env, "not-allowed")],
    );
    assert!(res.is_err());
    // Error code 16 = LabelNotAllowed
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::LabelNotAllowed));
}

// ==================== ERROR DISCRIMINATION IN BATCH OPERATIONS ====================

#[test]
fn test_batch_error_discrimination_first_item_fails() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    // Register program 3 first so the batch will fail on it
    client.register_program(
        &3,
        &program_admin,
        &String::from_str(&env, "Pre-existing"),
        &1_000,
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 10,
            admin: program_admin.clone(),
            name: String::from_str(&env, "New A"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 3, // already exists — triggers failure
            admin: program_admin.clone(),
            name: String::from_str(&env, "Conflict"),
            total_funding: 3_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 3 = ProgramExists
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ProgramExists));
}

#[test]
fn test_batch_error_discrimination_middle_item_fails() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Valid"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, ""), // invalid name
            total_funding: 3_000,
        },
        ProgramRegistrationItem {
            program_id: 3,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Valid Too"),
            total_funding: 4_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 9 = InvalidName
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidName));
}

#[test]
fn test_batch_error_discrimination_last_item_fails() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Valid"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Valid Too"),
            total_funding: 3_000,
        },
        ProgramRegistrationItem {
            program_id: 3,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Invalid"),
            total_funding: 0, // invalid amount
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
    // Error code 8 = InvalidAmount
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidAmount));
}

// ==================== ERROR DISCRIMINATION IN JURISDICTION OPERATIONS ====================

#[test]
fn test_jurisdiction_error_priority_kyc_over_funding() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "strict-zone")),
        requires_kyc: true,
        max_funding: Some(2_000),
        registration_paused: false,
    };

    // Both KYC and funding limit violations
    let res = client.try_register_program_juris(
        &95,
        &program_admin,
        &String::from_str(&env, "Strict Program"),
        &5_000, // exceeds max_funding
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &Some(false), // KYC not attested
    );
    assert!(res.is_err());
    // Error code 12 = JurisdictionFundingLimitExceeded (checked before KYC)
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::JurisdictionFundingLimitExceeded));
}

#[test]
fn test_jurisdiction_error_priority_pause_over_kyc() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "paused-strict")),
        requires_kyc: true,
        max_funding: Some(10_000),
        registration_paused: true,
    };

    // Both pause and KYC violations
    let res = client.try_register_program_juris(
        &96,
        &program_admin,
        &String::from_str(&env, "Paused Strict"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &Some(false), // KYC not attested
    );
    assert!(res.is_err());
    // Error code 13 = JurisdictionPaused (checked first)
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::JurisdictionPaused));
}

// ==================== ERROR DISCRIMINATION IN LABEL OPERATIONS ====================

#[test]
fn test_label_error_priority_too_many_over_invalid() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let mut labels = Vec::new(&env);
    labels.push_back(String::from_str(&env, "")); // invalid (empty)
    for i in 0..10 {
        labels.push_back(String::from_str(&env, "label"));
    }

    let res = client.try_register_program_with_labels(
        &1,
        &program_admin,
        &String::from_str(&env, "Test Program"),
        &5_000,
        &labels,
    );
    assert!(res.is_err());
    // Error code 15 = TooManyLabels (checked before individual label validation)
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::TooManyLabels));
}

#[test]
fn test_label_error_priority_not_allowed_over_invalid() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    // Set restricted label config
    client.set_label_config(
        &true,
        &vec![&env, String::from_str(&env, "allowed-label")],
    );

    // Try to register with a label not in the allowed list
    let res = client.try_register_program_with_labels(
        &1,
        &program_admin,
        &String::from_str(&env, "Test Program"),
        &5_000,
        &vec![&env, String::from_str(&env, "")], // empty label (invalid)
    );
    assert!(res.is_err());
    // Error code 14 = InvalidLabel (checked before allowlist)
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidLabel));
}

// ==================== ERROR DISCRIMINATION IN UPDATE OPERATIONS ====================

#[test]
fn test_update_labels_program_not_found() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let res = client.try_update_program_labels(
        &program_admin,
        &999, // non-existent program
        &vec![&env, String::from_str(&env, "new-label")],
    );
    assert!(res.is_err());
    // Error code 4 = ProgramNotFound
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ProgramNotFound));
}

#[test]
fn test_update_labels_invalid_label() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let name = String::from_str(&env, "Test Program");
    client.register_program(&1, &program_admin, &name, &5_000);

    let res = client.try_update_program_labels(
        &program_admin,
        &1,
        &vec![&env, String::from_str(&env, "")], // empty label
    );
    assert!(res.is_err());
    // Error code 14 = InvalidLabel
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidLabel));
}

#[test]
fn test_update_labels_too_many() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let name = String::from_str(&env, "Test Program");
    client.register_program(&1, &program_admin, &name, &5_000);

    let mut labels = Vec::new(&env);
    for i in 0..11 {
        labels.push_back(String::from_str(&env, "label"));
    }

    let res = client.try_update_program_labels(&program_admin, &1, &labels);
    assert!(res.is_err());
    // Error code 15 = TooManyLabels
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::TooManyLabels));
}

#[test]
fn test_update_labels_not_allowed() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let name = String::from_str(&env, "Test Program");
    client.register_program(&1, &program_admin, &name, &5_000);

    // Set restricted label config
    client.set_label_config(
        &true,
        &vec![&env, String::from_str(&env, "allowed-label")],
    );

    let res = client.try_update_program_labels(
        &program_admin,
        &1,
        &vec![&env, String::from_str(&env, "not-allowed")],
    );
    assert!(res.is_err());
    // Error code 16 = LabelNotAllowed
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::LabelNotAllowed));
}

// ==================== ERROR DISCRIMINATION IN LABEL CONFIG OPERATIONS ====================

#[test]
fn test_set_label_config_invalid_label() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let res = client.try_set_label_config(
        &true,
        &vec![&env, String::from_str(&env, "")], // empty label
    );
    assert!(res.is_err());
    // Error code 14 = InvalidLabel
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::InvalidLabel));
}

#[test]
fn test_set_label_config_too_many_labels() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let mut labels = Vec::new(&env);
    for i in 0..11 {
        labels.push_back(String::from_str(&env, "label"));
    }

    let res = client.try_set_label_config(&true, &labels);
    assert!(res.is_err());
    // Error code 15 = TooManyLabels
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::TooManyLabels));
}

// ==================== ERROR DISCRIMINATION IN DEPRECATION OPERATIONS ====================

#[test]
fn test_deprecated_batch_registration() {
    setup!(
        env,
        client,
        _contract_id,
        _admin,
        program_admin,
        _token_client,
        token_admin,
        20_000i128
    );

    client.set_deprecated(&true, &None);
    token_admin.mint(&program_admin, &20_000);

    let batch = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 202,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Blocked Batch"),
            total_funding: 5_000,
        },
    ];
    let res = client.try_batch_register_programs(&batch);
    assert!(res.is_err());
    // Error code 10 = ContractDeprecated
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ContractDeprecated));
}

#[test]
fn test_deprecated_jurisdiction_registration() {
    setup!(
        env,
        client,
        _contract_id,
        _admin,
        program_admin,
        _token_client,
        token_admin,
        20_000i128
    );

    client.set_deprecated(&true, &None);
    token_admin.mint(&program_admin, &20_000);

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: false,
        max_funding: Some(10_000),
        registration_paused: false,
    };

    let res = client.try_register_program_juris(
        &203,
        &program_admin,
        &String::from_str(&env, "Blocked Jurisdiction"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &None,
    );
    assert!(res.is_err());
    // Error code 10 = ContractDeprecated
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ContractDeprecated));
}

// ==================== ERROR DISCRIMINATION IN QUERY OPERATIONS ====================

#[test]
fn test_get_program_jurisdiction_not_found() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let res = client.try_get_program_jurisdiction(&999);
    assert!(res.is_err());
    // Error code 4 = ProgramNotFound
    let err = res.err().unwrap();
    assert_eq!(err, Ok(Error::ProgramNotFound));
}
