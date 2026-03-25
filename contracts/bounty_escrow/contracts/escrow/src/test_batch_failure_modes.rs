/// # Batch Lock and Batch Release Failure Mode Tests
///
/// These tests cover invalid batch sizes, duplicate IDs, mixed valid/invalid
/// entries, and partial failure scenarios for `batch_lock_funds` and
/// `batch_release_funds`.
///
/// Place this file at:
///   contracts/bounty_escrow/contracts/escrow/src/test_batch_failure_modes.rs
///
/// Then add to lib.rs (inside `#[cfg(test)] mod test;` block or as its own module):
///   mod test_batch_failure_modes;

#[cfg(test)]
mod test_batch_failure_modes {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        token, vec, Address, Env, Vec,
    };

    use crate::{
        BountyEscrowContract, BountyEscrowContractClient, Error, LockFundsItem, ReleaseFundsItem,
    };

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Minimal test harness: returns (env, client, admin, token_admin, token_client).
    fn setup() -> (
        Env,
        BountyEscrowContractClient<'static>,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);

        // Deploy a SAC-compatible token
        let token_id = env.register_stellar_asset_contract(token_admin.clone());

        // Deploy the escrow contract
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_id);

        (env, client, admin, token_admin, token_id)
    }

    /// Mint `amount` tokens to `recipient` using the SAC token admin.
    fn mint(
        env: &Env,
        token_id: &Address,
        token_admin: &Address,
        recipient: &Address,
        amount: i128,
    ) {
        let token_client = token::StellarAssetClient::new(env, token_id);
        token_client.mint(recipient, &amount);
    }

    /// Convenience: build a single `LockFundsItem`.
    fn lock_item(
        env: &Env,
        bounty_id: u64,
        depositor: Address,
        amount: i128,
        deadline: u64,
    ) -> LockFundsItem {
        LockFundsItem {
            bounty_id,
            depositor,
            amount,
            deadline,
        }
    }

    /// Advance ledger timestamp by `seconds`.
    fn advance_time(env: &Env, seconds: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: env.ledger().timestamp() + seconds,
            ..env.ledger().get()
        });
    }

    // =========================================================================
    // BATCH LOCK FUNDS – FAILURE MODES
    // =========================================================================

    // -------------------------------------------------------------------------
    // Batch size boundary tests
    // -------------------------------------------------------------------------

    /// An empty batch must return `InvalidBatchSize`.
    #[test]
    fn batch_lock_empty_batch_fails() {
        let (env, client, _admin, _token_admin, _token_id) = setup();
        let empty: Vec<LockFundsItem> = Vec::new(&env);
        let result = client.try_batch_lock_funds(&empty);
        assert_eq!(result, Err(Ok(Error::InvalidBatchSize)));
    }

    /// A batch of exactly 1 item (minimum valid) must succeed.
    #[test]
    fn batch_lock_single_item_succeeds() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 1_000);

        let deadline = env.ledger().timestamp() + 3_600;
        let items = vec![&env, lock_item(&env, 1, depositor, 1_000, deadline)];

        let locked = client.batch_lock_funds(&items);
        assert_eq!(locked, 1);
    }

    /// A batch exceeding MAX_BATCH_SIZE (100) must return `InvalidBatchSize`.
    /// We use 101 items to cross the boundary.
    #[test]
    fn batch_lock_exceeds_max_batch_size_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        let mut items: Vec<LockFundsItem> = Vec::new(&env);
        for i in 0..101u64 {
            let depositor = Address::generate(&env);
            mint(&env, &token_id, &token_admin, &depositor, 100);
            items.push_back(lock_item(&env, i, depositor, 100, deadline));
        }

        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::InvalidBatchSize)));
    }

    /// A batch of exactly MAX_BATCH_SIZE (100) items must succeed.
    #[test]
    fn batch_lock_exactly_max_batch_size_succeeds() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        let mut items: Vec<LockFundsItem> = Vec::new(&env);
        for i in 0..100u64 {
            let depositor = Address::generate(&env);
            mint(&env, &token_id, &token_admin, &depositor, 100);
            items.push_back(lock_item(&env, i, depositor, 100, deadline));
        }

        let locked = client.batch_lock_funds(&items);
        assert_eq!(locked, 100);
    }

    // -------------------------------------------------------------------------
    // Duplicate bounty ID tests
    // -------------------------------------------------------------------------

    /// Two items sharing the same `bounty_id` within one batch → `DuplicateBountyId`.
    #[test]
    fn batch_lock_duplicate_bounty_id_within_batch_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        let dep1 = Address::generate(&env);
        let dep2 = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &dep1, 500);
        mint(&env, &token_id, &token_admin, &dep2, 500);

        // Both use bounty_id = 99
        let items = vec![
            &env,
            lock_item(&env, 99, dep1, 500, deadline),
            lock_item(&env, 99, dep2, 500, deadline),
        ];

        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::DuplicateBountyId)));
    }

    /// Duplicate ID that also already exists in storage → `BountyExists`.
    #[test]
    fn batch_lock_bounty_id_already_exists_in_storage_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        // Pre-lock bounty 42 via the single-lock path
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 2_000);
        client.lock_funds(&depositor, &42, &1_000, &deadline);

        // Now try to batch-lock using the same bounty_id
        let dep2 = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &dep2, 500);
        let items = vec![&env, lock_item(&env, 42, dep2, 500, deadline)];

        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::BountyExists)));
    }

    // -------------------------------------------------------------------------
    // Mixed validity (some items invalid) – atomicity tests
    // -------------------------------------------------------------------------

    /// When the *second* item has a zero amount the whole batch must fail and
    /// the *first* (valid) item must NOT have been stored.
    #[test]
    fn batch_lock_invalid_amount_in_second_item_rolls_back_first() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        let dep1 = Address::generate(&env);
        let dep2 = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &dep1, 1_000);

        let items = vec![
            &env,
            lock_item(&env, 1, dep1.clone(), 1_000, deadline), // valid
            lock_item(&env, 2, dep2.clone(), 0, deadline),     // zero amount → invalid
        ];

        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));

        // Bounty 1 must NOT exist because the transaction was atomic
        let info = client.try_get_escrow_info(&1);
        assert_eq!(info, Err(Ok(Error::BountyNotFound)));
    }

    /// When the *last* item has a duplicate ID the whole batch fails and
    /// earlier valid items are not persisted.
    #[test]
    fn batch_lock_duplicate_in_last_item_rolls_back_all_previous() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        let dep1 = Address::generate(&env);
        let dep2 = Address::generate(&env);
        let dep3 = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &dep1, 100);
        mint(&env, &token_id, &token_admin, &dep2, 100);
        mint(&env, &token_id, &token_admin, &dep3, 100);

        let items = vec![
            &env,
            lock_item(&env, 10, dep1.clone(), 100, deadline),
            lock_item(&env, 11, dep2.clone(), 100, deadline),
            lock_item(&env, 11, dep3.clone(), 100, deadline), // dup of bounty 11
        ];

        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::DuplicateBountyId)));

        // Bounties 10 and 11 must not exist
        assert_eq!(
            client.try_get_escrow_info(&10),
            Err(Ok(Error::BountyNotFound))
        );
        assert_eq!(
            client.try_get_escrow_info(&11),
            Err(Ok(Error::BountyNotFound))
        );
    }

    /// Contract must be initialized before batch locking.
    #[test]
    fn batch_lock_not_initialized_fails() {
        let env = Env::default();
        env.mock_all_auths();

        // Register contract WITHOUT calling init
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        // We need a real token address just to build the item; use a dummy address
        let depositor = Address::generate(&env);
        let items = vec![
            &env,
            LockFundsItem {
                bounty_id: 1,
                depositor,
                amount: 100,
                deadline: 9_999_999,
            },
        ];

        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::NotInitialized)));
    }

    // =========================================================================
    // BATCH RELEASE FUNDS – FAILURE MODES
    // =========================================================================

    // -------------------------------------------------------------------------
    // Batch size boundary tests
    // -------------------------------------------------------------------------

    /// An empty batch must return `InvalidBatchSize`.
    #[test]
    fn batch_release_empty_batch_fails() {
        let (env, client, _admin, _token_admin, _token_id) = setup();
        let empty: Vec<ReleaseFundsItem> = Vec::new(&env);
        let result = client.try_batch_release_funds(&empty);
        assert_eq!(result, Err(Ok(Error::InvalidBatchSize)));
    }

    /// A batch of exactly 1 item (minimum valid) must succeed.
    #[test]
    fn batch_release_single_item_succeeds() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 1_000);

        let deadline = env.ledger().timestamp() + 3_600;
        client.lock_funds(&depositor, &1, &1_000, &deadline);

        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor,
            },
        ];
        let released = client.batch_release_funds(&items);
        assert_eq!(released, 1);
    }

    /// A batch exceeding MAX_BATCH_SIZE (100) must return `InvalidBatchSize`.
    #[test]
    fn batch_release_exceeds_max_batch_size_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        // Lock 101 bounties with a single depositor to stay within budget
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 101 * 100);

        let mut lock_items: Vec<LockFundsItem> = Vec::new(&env);
        for i in 0..101u64 {
            lock_items.push_back(lock_item(&env, i, depositor.clone(), 100, deadline));
        }
        // Lock in two smaller batches to avoid budget issues during setup
        let first_50: Vec<LockFundsItem> = lock_items.slice(0..50);
        let next_51: Vec<LockFundsItem> = lock_items.slice(50..101);
        client.batch_lock_funds(&first_50);
        client.batch_lock_funds(&next_51);

        let mut items: Vec<ReleaseFundsItem> = Vec::new(&env);
        for i in 0..101u64 {
            items.push_back(ReleaseFundsItem {
                bounty_id: i,
                contributor: Address::generate(&env),
            });
        }

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::InvalidBatchSize)));
    }

    // -------------------------------------------------------------------------
    // Duplicate bounty ID tests
    // -------------------------------------------------------------------------

    /// Two release items sharing the same `bounty_id` → `DuplicateBountyId`.
    #[test]
    fn batch_release_duplicate_bounty_id_within_batch_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 1_000);
        let deadline = env.ledger().timestamp() + 3_600;
        client.lock_funds(&depositor, &5, &1_000, &deadline);

        let c1 = Address::generate(&env);
        let c2 = Address::generate(&env);
        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 5,
                contributor: c1,
            },
            ReleaseFundsItem {
                bounty_id: 5,
                contributor: c2,
            },
        ];

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::DuplicateBountyId)));
    }

    // -------------------------------------------------------------------------
    // BountyNotFound tests
    // -------------------------------------------------------------------------

    /// Releasing a bounty that was never locked → `BountyNotFound`.
    #[test]
    fn batch_release_nonexistent_bounty_fails() {
        let (env, client, _admin, _token_admin, _token_id) = setup();
        let contributor = Address::generate(&env);
        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 9999,
                contributor,
            },
        ];

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::BountyNotFound)));
    }

    /// Second item does not exist; first valid item must NOT be released (atomicity).
    #[test]
    fn batch_release_nonexistent_second_item_rolls_back_first() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 1_000);
        let deadline = env.ledger().timestamp() + 3_600;
        client.lock_funds(&depositor, &1, &1_000, &deadline);

        let c1 = Address::generate(&env);
        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor: c1,
            }, // valid
            ReleaseFundsItem {
                bounty_id: 9999,
                contributor: Address::generate(&env),
            }, // missing
        ];

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::BountyNotFound)));

        // Bounty 1 must still be Locked (not Released)
        let escrow = client.get_escrow_info(&1);
        assert_eq!(escrow.status, crate::EscrowStatus::Locked);
    }

    // -------------------------------------------------------------------------
    // FundsNotLocked tests
    // -------------------------------------------------------------------------

    /// Releasing a bounty that was already released → `FundsNotLocked`.
    #[test]
    fn batch_release_already_released_bounty_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 1_000);
        let deadline = env.ledger().timestamp() + 3_600;
        client.lock_funds(&depositor, &7, &1_000, &deadline);

        let contributor = Address::generate(&env);
        // First release succeeds
        client.release_funds(&7, &contributor);

        // Try to batch-release the same bounty again
        let c2 = Address::generate(&env);
        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 7,
                contributor: c2,
            },
        ];
        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::FundsNotLocked)));
    }

    /// A mix of valid and already-refunded bounties: the refunded one triggers
    /// `FundsNotLocked` and the whole batch fails without releasing the valid one.
    #[test]
    fn batch_release_mixed_locked_and_refunded_fails_atomically() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 1; // very short deadline
        let dep1 = Address::generate(&env);
        let dep2 = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &dep1, 500);
        mint(&env, &token_id, &token_admin, &dep2, 500);

        client.lock_funds(&dep1, &20, &500, &deadline);
        client.lock_funds(&dep2, &21, &500, &deadline);

        // Advance past deadline and refund bounty 21
        advance_time(&env, 10);
        client.refund(&21, &None, &None, &crate::RefundMode::Full);

        // Now try batch release: bounty 20 is Locked (valid), bounty 21 is Refunded (invalid)
        let c1 = Address::generate(&env);
        let c2 = Address::generate(&env);
        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 20,
                contributor: c1,
            },
            ReleaseFundsItem {
                bounty_id: 21,
                contributor: c2,
            },
        ];

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::FundsNotLocked)));

        // Bounty 20 must remain Locked
        let escrow = client.get_escrow_info(&20);
        assert_eq!(escrow.status, crate::EscrowStatus::Locked);
    }

    // -------------------------------------------------------------------------
    // Authorization tests
    // -------------------------------------------------------------------------

    /// Contract must be initialized before batch releasing.
    #[test]
    fn batch_release_not_initialized_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 1,
                contributor: Address::generate(&env),
            },
        ];

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::NotInitialized)));
    }

    // =========================================================================
    // CROSS-CUTTING: partial failure atomicity
    // =========================================================================

    /// Lock 3 bounties. Batch-release the first two normally, then attempt a
    /// second batch that includes one already-released bounty and one fresh one.
    /// The whole second batch must fail and the fresh bounty must remain Locked.
    #[test]
    fn batch_release_partial_failure_does_not_release_any() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        let mut depositors = Vec::new(&env);
        for i in 30..33u64 {
            let dep = Address::generate(&env);
            mint(&env, &token_id, &token_admin, &dep, 200);
            client.lock_funds(&dep, &i, &200, &deadline);
            depositors.push_back(dep);
        }

        // Release bounties 30 and 31 via individual calls
        client.release_funds(&30, &Address::generate(&env));
        client.release_funds(&31, &Address::generate(&env));

        // Batch that includes already-released 30 and fresh 32
        let items = vec![
            &env,
            ReleaseFundsItem {
                bounty_id: 30,
                contributor: Address::generate(&env),
            }, // already released
            ReleaseFundsItem {
                bounty_id: 32,
                contributor: Address::generate(&env),
            }, // still locked
        ];

        let result = client.try_batch_release_funds(&items);
        assert_eq!(result, Err(Ok(Error::FundsNotLocked)));

        // Bounty 32 must still be Locked
        let escrow32 = client.get_escrow_info(&32);
        assert_eq!(escrow32.status, crate::EscrowStatus::Locked);
    }

    // =========================================================================
    // EDGE CASES
    // =========================================================================

    /// `batch_lock_funds` with a single depositor appearing multiple times should
    /// still work (auth is required once per unique address).
    #[test]
    fn batch_lock_same_depositor_multiple_bounties_succeeds() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 3_000);
        let deadline = env.ledger().timestamp() + 3_600;

        let items = vec![
            &env,
            lock_item(&env, 100, depositor.clone(), 1_000, deadline),
            lock_item(&env, 101, depositor.clone(), 1_000, deadline),
            lock_item(&env, 102, depositor.clone(), 1_000, deadline),
        ];

        let locked = client.batch_lock_funds(&items);
        assert_eq!(locked, 3);
    }

    /// Batch lock with amount=0 for any item → `InvalidAmount`.
    #[test]
    fn batch_lock_zero_amount_fails() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 1_000);
        let deadline = env.ledger().timestamp() + 3_600;

        let items = vec![&env, lock_item(&env, 1, depositor, 0, deadline)];
        let result = client.try_batch_lock_funds(&items);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    /// `batch_release_funds` on a batch of exactly MAX_BATCH_SIZE items should succeed.
    /// `batch_release_funds` on a batch of exactly MAX_BATCH_SIZE items should succeed.
    #[test]
    fn batch_release_exactly_max_batch_size_succeeds() {
        let (env, client, _admin, token_admin, token_id) = setup();
        let deadline = env.ledger().timestamp() + 3_600;

        // Use a single depositor for all 100 locks to stay within budget
        let depositor = Address::generate(&env);
        mint(&env, &token_id, &token_admin, &depositor, 100 * 10);

        let mut lock_items: Vec<LockFundsItem> = Vec::new(&env);
        for i in 0..100u64 {
            lock_items.push_back(lock_item(&env, i, depositor.clone(), 10, deadline));
        }
        client.batch_lock_funds(&lock_items);

        // Use a single contributor for all releases
        let contributor = Address::generate(&env);
        let mut release_items: Vec<ReleaseFundsItem> = Vec::new(&env);
        for i in 0..100u64 {
            release_items.push_back(ReleaseFundsItem {
                bounty_id: i,
                contributor: contributor.clone(),
            });
        }

        let released = client.batch_release_funds(&release_items);
        assert_eq!(released, 100);
    }
}
