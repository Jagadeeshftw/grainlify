/// # Gas / Cost Profiling — Bounty Escrow Contract
///
/// Issue: #600 – Add Comprehensive Gas/Cost Profiling for Critical Flows
/// Branch: perf/gas-cost-profiling-critical-flows
///
/// ## Running the profiler
///
/// ```bash
/// # Run ALL profiling tests with output visible (required for table rows)
/// cargo test gas_profile -- --nocapture --test-threads=1
///
/// # Run a single flow
/// cargo test gas_profile::lock -- --nocapture
/// cargo test gas_profile::batch -- --nocapture
///
/// # Run the full consolidated report table
/// cargo test gas_profile_scaling_summary -- --nocapture
/// ```
///
/// Each test prints a Markdown table row. The `gas_profile_scaling_summary` test
/// prints a full consolidated table suitable for pasting directly into GAS_COST_REPORT.md.
///
/// ## Why these numbers are reliable
///
/// Soroban's `env.budget()` meters are deterministic for a fixed input set.
/// `env.budget().reset_unlimited()` is called between setup and each measured call
/// so that only the operation under test is counted. Running the same test twice
/// on the same binary always produces identical values.
#[cfg(test)]
mod gas_profile {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        token, Address, Env, Vec,
    };

    use crate::{
        BountyEscrowContract, BountyEscrowContractClient, EscrowStatus, LockFundsItem,
        RefundMode, ReleaseFundsItem,
    };

    // =========================================================================
    // Budget capture helpers
    // =========================================================================

    struct BudgetDelta {
        cpu: u64,
        mem: u64,
    }

    /// Capture Soroban budget meters before and after `f`, return the deltas.
    fn measure<F: FnOnce()>(env: &Env, f: F) -> BudgetDelta {
        let cpu_before = env.budget().cpu_instruction_count();
        let mem_before = env.budget().memory_bytes_count();
        f();
        BudgetDelta {
            cpu: env.budget().cpu_instruction_count().saturating_sub(cpu_before),
            mem: env.budget().memory_bytes_count().saturating_sub(mem_before),
        }
    }

    fn print_header() {
        println!();
        println!(
            "| {:<50} | {:>16} | {:>12} |",
            "Scenario", "CPU Instructions", "Mem Bytes"
        );
        println!(
            "|{}|{}|{}|",
            "-".repeat(52),
            "-".repeat(18),
            "-".repeat(14)
        );
    }

    fn print_row(label: &str, cpu: u64, mem: u64) {
        println!("| {:<50} | {:>16} | {:>12} |", label, cpu, mem);
    }

    // =========================================================================
    // Shared test setup
    // =========================================================================

    struct Setup {
        env: Env,
        client: BountyEscrowContractClient<'static>,
        contract_id: Address,
        admin: Address,
        depositor: Address,
        contributor: Address,
        token_id: Address,
        token_sac: token::StellarAssetClient<'static>,
    }

    impl Setup {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();
            // Disable budget limits so setup never hits ExceededLimit
            env.budget().reset_unlimited();

            let admin = Address::generate(&env);
            let depositor = Address::generate(&env);
            let contributor = Address::generate(&env);

            let token_id = env.register_stellar_asset_contract(admin.clone());
            let token_sac = token::StellarAssetClient::new(&env, &token_id);

            let contract_id = env.register_contract(None, BountyEscrowContract);
            let client = BountyEscrowContractClient::new(&env, &contract_id);

            client.init(&admin, &token_id);

            // Whitelist the depositor so anti-abuse rate limiting doesn't skew gas numbers
            client.set_whitelist(&depositor, &true);

            Setup {
                env,
                client,
                contract_id,
                admin,
                depositor,
                contributor,
                token_id,
                token_sac,
            }
        }

        fn mint(&self, to: &Address, amount: i128) {
            self.token_sac.mint(to, &amount);
        }

        fn deadline(&self) -> u64 {
            self.env.ledger().timestamp() + 3_600
        }

        fn advance_time(&self, delta: u64) {
            let now = self.env.ledger().timestamp();
            self.env.ledger().set(LedgerInfo {
                timestamp: now + delta,
                ..self.env.ledger().get()
            });
        }

        fn lock(&self, bounty_id: u64, amount: i128) -> BudgetDelta {
            let deadline = self.deadline();
            measure(&self.env, || {
                self.client
                    .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
            })
        }

        fn release(&self, bounty_id: u64) -> BudgetDelta {
            measure(&self.env, || {
                self.client.release_funds(&bounty_id, &self.contributor);
            })
        }

        fn refund(&self, bounty_id: u64) -> BudgetDelta {
            measure(&self.env, || {
                self.client.refund(&bounty_id);
            })
        }
    }

    // =========================================================================
    // 1. INIT
    // =========================================================================

    #[test]
    fn gas_profile_init() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(admin.clone());
        let cid = env.register_contract(None, BountyEscrowContract);
        let cli = BountyEscrowContractClient::new(&env, &cid);

        print_header();
        let d = measure(&env, || {
            cli.init(&admin, &token_id);
        });
        print_row("init", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 2. LOCK FUNDS
    // =========================================================================

    #[test]
    fn gas_profile_lock_first_bounty() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        // reset so only the lock call is measured
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.lock(1, 1_000);
        print_row("lock_funds (1st bounty, index empty)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_lock_tenth_bounty() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 10_000);
        for i in 1..10u64 {
            s.lock(i, 1_000);
        }
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.lock(10, 1_000);
        print_row("lock_funds (10th bounty, index len=9)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_lock_large_amount() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000_000_000);
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.lock(1, 1_000_000_000);
        print_row("lock_funds (amount=1_000_000_000)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 3. RELEASE FUNDS
    // =========================================================================

    #[test]
    fn gas_profile_release_happy_path() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.release(1);
        print_row("release_funds (full amount)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 4. PARTIAL RELEASE
    // =========================================================================

    #[test]
    fn gas_profile_partial_release_first_tranche() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.partial_release(&1, &s.contributor.clone(), &400);
        });
        print_row("partial_release (1st tranche 400/1000)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_partial_release_final_tranche() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.client.partial_release(&1, &s.contributor.clone(), &600);
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.partial_release(&1, &s.contributor.clone(), &400);
        });
        print_row("partial_release (final tranche 400/400)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 5. REFUND
    // =========================================================================

    #[test]
    fn gas_profile_refund_after_deadline() {
        let s = Setup::new();
        let dl = s.env.ledger().timestamp() + 100;
        s.mint(&s.depositor.clone(), 1_000);
        s.client.lock_funds(&s.depositor, &1, &1_000, &dl);
        s.advance_time(200);
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.refund(1);
        print_row("refund (standard, after deadline)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_refund_admin_approved_full() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.client
            .approve_refund(&1, &1_000, &s.depositor.clone(), &RefundMode::Full);
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.refund(1);
        print_row("refund (admin-approved full, before deadline)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_refund_admin_approved_partial() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.client
            .approve_refund(&1, &400, &s.depositor.clone(), &RefundMode::Partial);
        s.env.budget().reset_unlimited();
        print_header();
        let d = s.refund(1);
        print_row("refund (admin-approved partial 400/1000)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 6. PAUSE / UNPAUSE
    // =========================================================================

    #[test]
    fn gas_profile_pause_single_operation() {
        let s = Setup::new();
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client
                .set_paused(&Some(true), &None, &None, &None)
                .unwrap();
        });
        print_row("set_paused (lock=true)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_pause_all_operations() {
        let s = Setup::new();
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client
                .set_paused(&Some(true), &Some(true), &Some(true), &None)
                .unwrap();
        });
        print_row("set_paused (lock+release+refund=true)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_unpause_all_operations() {
        let s = Setup::new();
        s.client
            .set_paused(&Some(true), &Some(true), &Some(true), &None)
            .unwrap();
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client
                .set_paused(&Some(false), &Some(false), &Some(false), &None)
                .unwrap();
        });
        print_row("set_paused (all=false, full unpause)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 7. CLAIM FLOW (authorize_claim → claim → cancel)
    // =========================================================================

    #[test]
    fn gas_profile_authorize_claim() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.client.set_claim_window(&86_400);
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.authorize_claim(&1, &s.contributor.clone());
        });
        print_row("authorize_claim", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_claim_execute() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.client.set_claim_window(&86_400);
        s.client.authorize_claim(&1, &s.contributor.clone());
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.claim(&1);
        });
        print_row("claim (execute authorized claim)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_cancel_pending_claim() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.client.set_claim_window(&86_400);
        s.client.authorize_claim(&1, &s.contributor.clone());
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.cancel_pending_claim(&1);
        });
        print_row("cancel_pending_claim", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 8. BATCH LOCK – n = 1, 5, 10, 20 (MAX_BATCH_SIZE for this contract)
    // =========================================================================

    fn do_batch_lock(s: &Setup, n: u32, base_id: u64) -> BudgetDelta {
        let deadline = s.deadline();
        s.mint(&s.depositor.clone(), 100 * n as i128);
        let mut items: Vec<LockFundsItem> = Vec::new(&s.env);
        for i in 0..n as u64 {
            items.push_back(LockFundsItem {
                bounty_id: base_id + i,
                depositor: s.depositor.clone(),
                amount: 100,
                deadline,
            });
        }
        s.env.budget().reset_unlimited();
        measure(&s.env, || {
            s.client.batch_lock_funds(&items);
        })
    }

    #[test]
    fn gas_profile_batch_lock_n1() {
        let s = Setup::new();
        print_header();
        let d = do_batch_lock(&s, 1, 1_000);
        print_row("batch_lock_funds (n=1)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_batch_lock_n5() {
        let s = Setup::new();
        print_header();
        let d = do_batch_lock(&s, 5, 2_000);
        print_row("batch_lock_funds (n=5)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_batch_lock_n10() {
        let s = Setup::new();
        print_header();
        let d = do_batch_lock(&s, 10, 3_000);
        print_row("batch_lock_funds (n=10)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_batch_lock_n20() {
        // n=20 is MAX_BATCH_SIZE for this contract
        let s = Setup::new();
        print_header();
        let d = do_batch_lock(&s, 20, 4_000);
        print_row("batch_lock_funds (n=20, MAX_BATCH_SIZE)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 9. BATCH RELEASE – n = 1, 5, 10, 20
    // =========================================================================

    fn setup_n_locked(s: &Setup, n: u32, base_id: u64) {
        let deadline = s.deadline();
        s.mint(&s.depositor.clone(), 1_000 * n as i128);
        for i in 0..n as u64 {
            s.client
                .lock_funds(&s.depositor, &(base_id + i), &1_000, &deadline);
        }
        s.env.budget().reset_unlimited();
    }

    fn do_batch_release(s: &Setup, n: u32, base_id: u64) -> BudgetDelta {
        let mut items: Vec<ReleaseFundsItem> = Vec::new(&s.env);
        for i in 0..n as u64 {
            items.push_back(ReleaseFundsItem {
                bounty_id: base_id + i,
                contributor: s.contributor.clone(),
            });
        }
        measure(&s.env, || {
            s.client.batch_release_funds(&items);
        })
    }

    #[test]
    fn gas_profile_batch_release_n1() {
        let s = Setup::new();
        setup_n_locked(&s, 1, 5_000);
        print_header();
        let d = do_batch_release(&s, 1, 5_000);
        print_row("batch_release_funds (n=1)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_batch_release_n5() {
        let s = Setup::new();
        setup_n_locked(&s, 5, 6_000);
        print_header();
        let d = do_batch_release(&s, 5, 6_000);
        print_row("batch_release_funds (n=5)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_batch_release_n10() {
        let s = Setup::new();
        setup_n_locked(&s, 10, 7_000);
        print_header();
        let d = do_batch_release(&s, 10, 7_000);
        print_row("batch_release_funds (n=10)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_batch_release_n20() {
        let s = Setup::new();
        setup_n_locked(&s, 20, 8_000);
        print_header();
        let d = do_batch_release(&s, 20, 8_000);
        print_row("batch_release_funds (n=20, MAX_BATCH_SIZE)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 10. QUERY / VIEW OPERATIONS
    // =========================================================================

    #[test]
    fn gas_profile_get_escrow_info() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.get_escrow_info(&1);
        });
        print_row("get_escrow_info (1 escrow)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_get_aggregate_stats_10() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 10_000);
        for i in 1..=10u64 {
            s.lock(i, 1_000);
        }
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.get_aggregate_stats();
        });
        print_row("get_aggregate_stats (10 escrows)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_query_by_status_10() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 10_000);
        for i in 1..=10u64 {
            s.lock(i, 1_000);
        }
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client
                .query_escrows_by_status(&EscrowStatus::Locked, &0, &10);
        });
        print_row("query_escrows_by_status (Locked, 10 results)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_get_refund_eligibility() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.lock(1, 1_000);
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.get_refund_eligibility(&1);
        });
        print_row("get_refund_eligibility", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 11. FULL LIFECYCLE FLOWS
    // =========================================================================

    #[test]
    fn gas_profile_lifecycle_lock_release() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.env.budget().reset_unlimited();
        print_header();
        let lock_d = s.lock(1, 1_000);
        s.env.budget().reset_unlimited();
        let release_d = s.release(1);
        print_row("  lock_funds", lock_d.cpu, lock_d.mem);
        print_row("  release_funds", release_d.cpu, release_d.mem);
        print_row(
            "TOTAL lock→release",
            lock_d.cpu + release_d.cpu,
            lock_d.mem + release_d.mem,
        );
        assert!(lock_d.cpu + release_d.cpu > 0);
    }

    #[test]
    fn gas_profile_lifecycle_lock_refund() {
        let s = Setup::new();
        let dl = s.env.ledger().timestamp() + 100;
        s.mint(&s.depositor.clone(), 1_000);
        s.env.budget().reset_unlimited();
        let lock_d = measure(&s.env, || {
            s.client.lock_funds(&s.depositor, &1, &1_000, &dl);
        });
        s.advance_time(200);
        s.env.budget().reset_unlimited();
        let refund_d = s.refund(1);
        print_header();
        print_row("  lock_funds", lock_d.cpu, lock_d.mem);
        print_row("  refund", refund_d.cpu, refund_d.mem);
        print_row(
            "TOTAL lock→refund",
            lock_d.cpu + refund_d.cpu,
            lock_d.mem + refund_d.mem,
        );
        assert!(lock_d.cpu + refund_d.cpu > 0);
    }

    #[test]
    fn gas_profile_lifecycle_lock_authorize_claim_claim() {
        let s = Setup::new();
        s.mint(&s.depositor.clone(), 1_000);
        s.env.budget().reset_unlimited();
        let lock_d = s.lock(1, 1_000);
        s.client.set_claim_window(&86_400);
        s.env.budget().reset_unlimited();
        let auth_d = measure(&s.env, || {
            s.client.authorize_claim(&1, &s.contributor.clone());
        });
        s.env.budget().reset_unlimited();
        let claim_d = measure(&s.env, || {
            s.client.claim(&1);
        });
        print_header();
        print_row("  lock_funds", lock_d.cpu, lock_d.mem);
        print_row("  authorize_claim", auth_d.cpu, auth_d.mem);
        print_row("  claim", claim_d.cpu, claim_d.mem);
        print_row(
            "TOTAL lock→authorize→claim",
            lock_d.cpu + auth_d.cpu + claim_d.cpu,
            lock_d.mem + auth_d.mem + claim_d.mem,
        );
        assert!(lock_d.cpu + auth_d.cpu + claim_d.cpu > 0);
    }

    // =========================================================================
    // 12. ANTI-ABUSE CONFIG OPS
    // =========================================================================

    #[test]
    fn gas_profile_update_anti_abuse_config() {
        let s = Setup::new();
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.update_anti_abuse_config(&3_600, &100, &60);
        });
        print_row("update_anti_abuse_config", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    #[test]
    fn gas_profile_set_whitelist() {
        let s = Setup::new();
        let addr = Address::generate(&s.env);
        s.env.budget().reset_unlimited();
        print_header();
        let d = measure(&s.env, || {
            s.client.set_whitelist(&addr, &true);
        });
        print_row("set_whitelist (add)", d.cpu, d.mem);
        assert!(d.cpu > 0);
    }

    // =========================================================================
    // 13. CONSOLIDATED SCALING SUMMARY TABLE
    //
    //   cargo test gas_profile_scaling_summary -- --nocapture
    //
    // Copy the output directly into GAS_COST_REPORT.md
    // =========================================================================

    #[test]
    fn gas_profile_scaling_summary() {
        println!();
        println!("# Gas Cost Profiling Report — Bounty Escrow v1");
        println!("# Run: cargo test gas_profile_scaling_summary -- --nocapture");
        println!();

        // ── batch_lock_funds ──────────────────────────────────────────────────
        println!("## batch_lock_funds  (single depositor, whitelisted, amount=100 each)");
        println!();
        println!(
            "| {:>5} | {:>16} | {:>12} | {:>18} | {:>14} |",
            "n", "Total CPU", "Total Mem", "CPU / item", "Mem / item"
        );
        println!(
            "|{}|{}|{}|{}|{}|",
            "-".repeat(7),
            "-".repeat(18),
            "-".repeat(14),
            "-".repeat(20),
            "-".repeat(16)
        );
        for &n in &[1u32, 5, 10, 20] {
            let s = Setup::new();
            let d = do_batch_lock(&s, n, (n as u64) * 10_000);
            println!(
                "| {:>5} | {:>16} | {:>12} | {:>18} | {:>14} |",
                n,
                d.cpu,
                d.mem,
                d.cpu / n as u64,
                d.mem / n as u64,
            );
        }
        println!();

        // ── batch_release_funds ───────────────────────────────────────────────
        println!("## batch_release_funds  (amount=1000 each)");
        println!();
        println!(
            "| {:>5} | {:>16} | {:>12} | {:>18} | {:>14} |",
            "n", "Total CPU", "Total Mem", "CPU / item", "Mem / item"
        );
        println!(
            "|{}|{}|{}|{}|{}|",
            "-".repeat(7),
            "-".repeat(18),
            "-".repeat(14),
            "-".repeat(20),
            "-".repeat(16)
        );
        let bases: [u64; 4] = [20_000, 30_000, 40_000, 50_000];
        for (&n, &base) in [1u32, 5, 10, 20].iter().zip(bases.iter()) {
            let s = Setup::new();
            setup_n_locked(&s, n, base);
            let d = do_batch_release(&s, n, base);
            println!(
                "| {:>5} | {:>16} | {:>12} | {:>18} | {:>14} |",
                n,
                d.cpu,
                d.mem,
                d.cpu / n as u64,
                d.mem / n as u64,
            );
        }
        println!();

        // ── per-operation cost ────────────────────────────────────────────────
        println!("## Per-operation cost  (single call, amount=1000 unless noted)");
        println!();
        println!(
            "| {:<44} | {:>16} | {:>12} |",
            "Operation", "CPU Instructions", "Mem Bytes"
        );
        println!(
            "|{}|{}|{}|",
            "-".repeat(46),
            "-".repeat(18),
            "-".repeat(14)
        );

        macro_rules! row {
            ($label:expr, $cpu:expr, $mem:expr) => {
                println!(
                    "| {:<44} | {:>16} | {:>12} |",
                    $label, $cpu, $mem
                );
            };
        }

        // init
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            let admin = Address::generate(&env);
            let token_id = env.register_stellar_asset_contract(admin.clone());
            let cid = env.register_contract(None, BountyEscrowContract);
            let cli = BountyEscrowContractClient::new(&env, &cid);
            let d = measure(&env, || { cli.init(&admin, &token_id); });
            row!("init", d.cpu, d.mem);
        }

        // lock + release
        {
            let s = Setup::new();
            s.mint(&s.depositor.clone(), 1_000);
            s.env.budget().reset_unlimited();
            let d = s.lock(1, 1_000);
            row!("lock_funds", d.cpu, d.mem);
            s.env.budget().reset_unlimited();
            let d2 = s.release(1);
            row!("release_funds", d2.cpu, d2.mem);
        }

        // partial_release
        {
            let s = Setup::new();
            s.mint(&s.depositor.clone(), 1_000);
            s.lock(1, 1_000);
            s.env.budget().reset_unlimited();
            let d = measure(&s.env, || {
                s.client.partial_release(&1, &s.contributor.clone(), &400);
            });
            row!("partial_release (400/1000)", d.cpu, d.mem);
        }

        // refund after deadline
        {
            let s = Setup::new();
            let dl = s.env.ledger().timestamp() + 100;
            s.mint(&s.depositor.clone(), 1_000);
            s.client.lock_funds(&s.depositor, &1, &1_000, &dl);
            s.advance_time(200);
            s.env.budget().reset_unlimited();
            let d = s.refund(1);
            row!("refund  (after deadline)", d.cpu, d.mem);
        }

        // approve_refund
        {
            let s = Setup::new();
            s.mint(&s.depositor.clone(), 1_000);
            s.lock(1, 1_000);
            s.env.budget().reset_unlimited();
            let d = measure(&s.env, || {
                s.client
                    .approve_refund(&1, &1_000, &s.depositor.clone(), &RefundMode::Full);
            });
            row!("approve_refund", d.cpu, d.mem);
        }

        // set_paused (all=true)
        {
            let s = Setup::new();
            s.env.budget().reset_unlimited();
            let d = measure(&s.env, || {
                s.client
                    .set_paused(&Some(true), &Some(true), &Some(true), &None)
                    .unwrap();
            });
            row!("set_paused (all=true)", d.cpu, d.mem);
        }

        // authorize_claim
        {
            let s = Setup::new();
            s.mint(&s.depositor.clone(), 1_000);
            s.lock(1, 1_000);
            s.client.set_claim_window(&86_400);
            s.env.budget().reset_unlimited();
            let d = measure(&s.env, || {
                s.client.authorize_claim(&1, &s.contributor.clone());
            });
            row!("authorize_claim", d.cpu, d.mem);
        }

        // claim
        {
            let s = Setup::new();
            s.mint(&s.depositor.clone(), 1_000);
            s.lock(1, 1_000);
            s.client.set_claim_window(&86_400);
            s.client.authorize_claim(&1, &s.contributor.clone());
            s.env.budget().reset_unlimited();
            let d = measure(&s.env, || {
                s.client.claim(&1);
            });
            row!("claim", d.cpu, d.mem);
        }

        // get_aggregate_stats
        {
            let s = Setup::new();
            s.mint(&s.depositor.clone(), 10_000);
            for i in 1..=10u64 { s.lock(i, 1_000); }
            s.env.budget().reset_unlimited();
            let d = measure(&s.env, || { s.client.get_aggregate_stats(); });
            row!("get_aggregate_stats (10 escrows)", d.cpu, d.mem);
        }

        println!();
        println!(
            "_Setup cost excluded: `env.budget().reset_unlimited()` called before each measured call._"
        );
        println!("_Numbers are deterministic per binary build. MAX_BATCH_SIZE = 20._");
    }
}