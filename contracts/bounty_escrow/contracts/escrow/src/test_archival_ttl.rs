#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _, Ledger as _},
    token, Address, Env, TryFromVal,
};

const START_LEDGER: u32 = 100;

struct ArchivalSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    depositor: Address,
    recipient: Address,
}

impl ArchivalSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(START_LEDGER);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        client.init(&admin, &token_id);
        token::StellarAssetClient::new(&env, &token_id).mint(&depositor, &1_000_000);

        env.as_contract(&contract_id, || {
            env.storage()
                .instance()
                .extend_ttl(ARCHIVAL_MARKER_TTL, ARCHIVAL_MARKER_TTL);
        });

        Self {
            env,
            contract_id,
            admin,
            depositor,
            recipient,
        }
    }

    fn client(&self) -> BountyEscrowContractClient<'_> {
        BountyEscrowContractClient::new(&self.env, &self.contract_id)
    }

    fn lock(&self, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 10_000;
        self.client()
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }

    fn ttl(&self, key: &DataKey) -> u32 {
        self.env.as_contract(&self.contract_id, || {
            self.env.storage().persistent().get_ttl(key)
        })
    }

    fn marker(&self, key: &DataKey) -> u32 {
        self.env.as_contract(&self.contract_id, || {
            self.env.storage().persistent().get(key).unwrap()
        })
    }
}

#[test]
fn archival_escrow_probe_is_typed_before_at_and_after_expiration() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 1;

    assert_eq!(
        client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Missing
    );

    setup.lock(bounty_id, 1_000);
    let live_until = setup.marker(&DataKey::EscrowTtl(bounty_id));
    assert_eq!(live_until, START_LEDGER + ESCROW_LIVE_TTL);
    assert_eq!(
        client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Live
    );

    setup.env.ledger().set_sequence_number(live_until - 1);
    assert_eq!(
        client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(live_until);
    assert_eq!(
        client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(live_until + 1);
    assert_eq!(
        client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Archived
    );
}

#[test]
fn archival_probe_recognizes_legacy_live_records_without_markers() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 12;
    setup.lock(bounty_id, 1_000);

    setup.env.as_contract(&setup.contract_id, || {
        setup
            .env
            .storage()
            .persistent()
            .remove(&DataKey::EscrowTtl(bounty_id));
    });

    assert_eq!(
        client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Live
    );
}

#[test]
fn archival_terminal_read_tolerates_missing_legacy_indexes() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 13;
    setup.lock(bounty_id, 1_000);

    setup.env.as_contract(&setup.contract_id, || {
        let mut escrow: Escrow = setup
            .env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        escrow.status = EscrowStatus::Released;
        setup
            .env
            .storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
        setup
            .env
            .storage()
            .persistent()
            .remove(&DataKey::EscrowIndex);
        setup
            .env
            .storage()
            .persistent()
            .remove(&DataKey::EscrowIndexTtl);
        setup
            .env
            .storage()
            .persistent()
            .remove(&DataKey::DepositorIndex(setup.depositor.clone()));
        setup
            .env
            .storage()
            .persistent()
            .remove(&DataKey::DepositorIndexTtl(setup.depositor.clone()));
    });

    assert_eq!(
        client.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
    assert_eq!(
        setup.marker(&DataKey::EscrowTtl(bounty_id)),
        START_LEDGER + ESCROW_ARCHIVAL_TTL
    );
    assert_eq!(
        client.probe_index_archival(),
        PersistentRecordStatus::Missing
    );
    assert_eq!(
        client.probe_depositor_index_archival(&setup.depositor),
        PersistentRecordStatus::Missing
    );
}

#[test]
fn archival_claim_probe_is_typed_before_at_and_after_expiration() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 2;

    assert_eq!(
        client.probe_claim_archival(&bounty_id),
        PersistentRecordStatus::Missing
    );
    setup.lock(bounty_id, 1_000);
    client.set_claim_window(&1_000_000);
    client.authorize_claim(&bounty_id, &setup.recipient, &DisputeReason::Other);

    let live_until = setup.marker(&DataKey::ClaimTtl(bounty_id));
    assert_eq!(live_until, START_LEDGER + CLAIM_LIVE_TTL);
    setup.env.ledger().set_sequence_number(live_until - 1);
    assert_eq!(
        client.probe_claim_archival(&bounty_id),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(live_until);
    assert_eq!(
        client.probe_claim_archival(&bounty_id),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(live_until + 1);
    assert_eq!(
        client.probe_claim_archival(&bounty_id),
        PersistentRecordStatus::Archived
    );
}

#[test]
fn archival_commitment_probe_is_typed_before_at_and_after_expiration() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 3;
    setup.lock(bounty_id, 1_000);
    let unknown_capability_id = BytesN::from_array(&setup.env, &[0; 32]);
    assert_eq!(
        client.probe_commitment_archival(&unknown_capability_id),
        PersistentRecordStatus::Missing
    );

    let capability_id = client.issue_capability(
        &setup.admin,
        &setup.recipient,
        &CapabilityAction::Release,
        &bounty_id,
        &500,
        &(setup.env.ledger().timestamp() + 1_000_000),
        &2,
    );
    let live_until = setup.marker(&DataKey::CapabilityTtl(capability_id.clone()));
    assert_eq!(live_until, START_LEDGER + COMMITMENT_LIVE_TTL);

    setup.env.ledger().set_sequence_number(live_until - 1);
    assert_eq!(
        client.probe_commitment_archival(&capability_id),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(live_until);
    assert_eq!(
        client.probe_commitment_archival(&capability_id),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(live_until + 1);
    assert_eq!(
        client.probe_commitment_archival(&capability_id),
        PersistentRecordStatus::Archived
    );
}

#[test]
fn archival_indexes_are_typed_before_at_and_after_expiration() {
    let setup = ArchivalSetup::new();
    let client = setup.client();

    assert_eq!(
        client.probe_index_archival(),
        PersistentRecordStatus::Missing
    );
    assert_eq!(
        client.probe_depositor_index_archival(&setup.depositor),
        PersistentRecordStatus::Missing
    );

    setup.lock(4, 1_000);
    let global_live_until = setup.marker(&DataKey::EscrowIndexTtl);
    let depositor_live_until = setup.marker(&DataKey::DepositorIndexTtl(setup.depositor.clone()));
    assert_eq!(global_live_until, START_LEDGER + INDEX_LIVE_TTL);
    assert_eq!(depositor_live_until, global_live_until);

    setup
        .env
        .ledger()
        .set_sequence_number(global_live_until - 1);
    assert_eq!(client.probe_index_archival(), PersistentRecordStatus::Live);
    assert_eq!(
        client.probe_depositor_index_archival(&setup.depositor),
        PersistentRecordStatus::Live
    );
    setup.env.ledger().set_sequence_number(global_live_until);
    assert_eq!(client.probe_index_archival(), PersistentRecordStatus::Live);
    assert_eq!(
        client.probe_depositor_index_archival(&setup.depositor),
        PersistentRecordStatus::Live
    );
    setup
        .env
        .ledger()
        .set_sequence_number(global_live_until + 1);
    assert_eq!(
        client.probe_index_archival(),
        PersistentRecordStatus::Archived
    );
    assert_eq!(
        client.probe_depositor_index_archival(&setup.depositor),
        PersistentRecordStatus::Archived
    );
}

#[test]
fn archival_index_read_renews_live_records() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 5;
    setup.lock(bounty_id, 1_000);
    setup.env.as_contract(&setup.contract_id, || {
        setup.env.storage().persistent().extend_ttl(
            &DataKey::Escrow(bounty_id),
            INDEX_LIVE_TTL,
            INDEX_LIVE_TTL,
        );
    });

    let first_index_live_until = setup.marker(&DataKey::EscrowIndexTtl);
    let index_threshold = INDEX_LIVE_TTL / TTL_RENEWAL_DIVISOR;
    let threshold_ledger = first_index_live_until - index_threshold;
    setup.env.ledger().set_sequence_number(threshold_ledger);
    assert!(client.get_archived_escrows().is_empty());

    assert_eq!(
        setup.marker(&DataKey::EscrowIndexTtl),
        threshold_ledger + INDEX_LIVE_TTL
    );
    assert!(setup.marker(&DataKey::EscrowIndexTtl) > first_index_live_until);
}

#[test]
fn archival_read_and_write_renew_at_the_live_threshold() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 6;
    setup.lock(bounty_id, 1_000);

    let first_live_until = setup.marker(&DataKey::EscrowTtl(bounty_id));
    let escrow_threshold = ESCROW_LIVE_TTL / TTL_RENEWAL_DIVISOR;
    let before_threshold = first_live_until - escrow_threshold - 1;
    setup.env.ledger().set_sequence_number(before_threshold);
    assert_eq!(setup.ttl(&DataKey::Escrow(bounty_id)), escrow_threshold + 1);
    client.get_escrow_info(&bounty_id);
    assert_eq!(
        setup.marker(&DataKey::EscrowTtl(bounty_id)),
        first_live_until
    );

    setup
        .env
        .ledger()
        .set_sequence_number(first_live_until - escrow_threshold);
    client.get_escrow_info(&bounty_id);
    assert_eq!(setup.ttl(&DataKey::Escrow(bounty_id)), ESCROW_LIVE_TTL);

    client.set_claim_window(&1_000_000);
    client.authorize_claim(&bounty_id, &setup.recipient, &DisputeReason::Other);
    let claim_live_until = setup.marker(&DataKey::ClaimTtl(bounty_id));
    let claim_threshold = CLAIM_LIVE_TTL / TTL_RENEWAL_DIVISOR;
    setup
        .env
        .ledger()
        .set_sequence_number(claim_live_until - claim_threshold);
    client.authorize_claim(&bounty_id, &setup.recipient, &DisputeReason::Other);
    assert_eq!(setup.ttl(&DataKey::PendingClaim(bounty_id)), CLAIM_LIVE_TTL);
}

#[test]
fn archival_payout_and_terminal_records_receive_archival_retention() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 7;
    setup.lock(bounty_id, 1_000);

    let first_live_until = setup.marker(&DataKey::EscrowTtl(bounty_id));
    setup.env.ledger().set_sequence_number(START_LEDGER + 100);
    client.release_funds(&bounty_id, &setup.recipient);

    assert_eq!(
        setup.marker(&DataKey::EscrowTtl(bounty_id)),
        START_LEDGER + 100 + ESCROW_ARCHIVAL_TTL
    );
    assert!(setup.marker(&DataKey::EscrowTtl(bounty_id)) > first_live_until);
    assert_eq!(setup.ttl(&DataKey::Escrow(bounty_id)), ESCROW_ARCHIVAL_TTL);
    assert_eq!(setup.ttl(&DataKey::EscrowIndex), INDEX_ARCHIVAL_TTL);
    assert_eq!(
        setup.ttl(&DataKey::DepositorIndex(setup.depositor.clone())),
        INDEX_ARCHIVAL_TTL
    );
}

#[test]
fn archival_revoked_commitment_receives_archival_retention() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 8;
    setup.lock(bounty_id, 1_000);

    let capability_id = client.issue_capability(
        &setup.admin,
        &setup.recipient,
        &CapabilityAction::Release,
        &bounty_id,
        &500,
        &(setup.env.ledger().timestamp() + 1_000_000),
        &2,
    );
    client.revoke_capability(&setup.admin, &capability_id);

    assert_eq!(
        setup.ttl(&DataKey::Capability(capability_id.clone())),
        COMMITMENT_ARCHIVAL_TTL
    );
    assert_eq!(
        setup.marker(&DataKey::CapabilityTtl(capability_id)),
        START_LEDGER + COMMITMENT_ARCHIVAL_TTL
    );
}

#[test]
fn archival_cancelled_claim_is_reported_as_missing() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 9;
    setup.lock(bounty_id, 1_000);
    client.set_claim_window(&1_000_000);
    client.authorize_claim(&bounty_id, &setup.recipient, &DisputeReason::Other);

    client.cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    assert_eq!(
        client.probe_claim_archival(&bounty_id),
        PersistentRecordStatus::Missing
    );
}

#[test]
fn archival_explicit_archive_upgrades_live_retention() {
    let setup = ArchivalSetup::new();
    let client = setup.client();
    let bounty_id = 10;
    setup.lock(bounty_id, 1_000);

    let live_until = setup.marker(&DataKey::EscrowTtl(bounty_id));
    client.archive_escrow(&bounty_id);

    assert_eq!(
        setup.marker(&DataKey::EscrowTtl(bounty_id)),
        START_LEDGER + ESCROW_ARCHIVAL_TTL
    );
    assert!(setup.marker(&DataKey::EscrowTtl(bounty_id)) > live_until);
    assert_eq!(setup.ttl(&DataKey::Escrow(bounty_id)), ESCROW_ARCHIVAL_TTL);
}

#[test]
fn archival_snapshot_restore_makes_an_expired_record_readable_again() {
    let setup = ArchivalSetup::new();
    let bounty_id = 11;
    setup.lock(bounty_id, 1_000);

    let live_until = setup.marker(&DataKey::EscrowTtl(bounty_id));
    let contract_address_xdr: soroban_sdk::xdr::ScAddress = setup.contract_id.clone().into();
    let mut snapshot = setup.env.to_snapshot();
    snapshot.ledger.sequence_number = live_until + 1;
    for (_, (_, entry_live_until)) in snapshot.ledger.ledger_entries.iter_mut() {
        if entry_live_until
            .map(|entry_ttl| entry_ttl < snapshot.ledger.sequence_number)
            .unwrap_or(false)
        {
            *entry_live_until = Some(snapshot.ledger.sequence_number + ESCROW_ARCHIVAL_TTL);
        }
    }

    let restored_env = Env::from_snapshot(snapshot);
    restored_env.mock_all_auths();
    let restored_contract_id = Address::try_from_val(&restored_env, &contract_address_xdr).unwrap();
    restored_env.register_contract(Some(&restored_contract_id), BountyEscrowContract);
    let restored_client = BountyEscrowContractClient::new(&restored_env, &restored_contract_id);

    let escrow = restored_client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(
        restored_client.probe_escrow_archival(&bounty_id),
        PersistentRecordStatus::Live
    );
}
