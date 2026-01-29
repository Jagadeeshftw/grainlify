use bounty_escrow::{BountyEscrowContract, BountyEscrowContractClient, RefundMode};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

fn create_escrow_contract<'a>(env: &Env) -> (BountyEscrowContractClient<'a>, Address) {
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

struct Setup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let (escrow, _escrow_address) = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

// Best-effort budget snapshot. Different Soroban SDK versions expose slightly
// different budget APIs; keep this isolated for quick fixes.
#[derive(Clone, Copy, Debug, Default)]
struct BudgetSnapshot {
    cpu_insns: u64,
    mem_bytes: u64,
}

fn snapshot_budget(env: &Env) -> BudgetSnapshot {
    // These methods exist in recent Soroban SDKs; if they ever change,
    // this is the only place that needs updating.
    BudgetSnapshot {
        cpu_insns: env.budget().cpu_instruction_cost(),
        mem_bytes: env.budget().memory_bytes_cost(),
    }
}

fn reset_budget(env: &Env) {
    env.budget().reset_default();
}

fn bench_lock_funds(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounty_escrow/lock_funds");
    for amount in [100i128, 1_000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(amount), &amount, |b, &amt| {
            b.iter(|| {
                let setup = Setup::new();
                let bounty_id = 1u64;
                let deadline = setup.env.ledger().timestamp() + 1000;

                reset_budget(&setup.env);
                setup
                    .escrow
                    .lock_funds(&setup.depositor, &bounty_id, &amt, &deadline);
                black_box(snapshot_budget(&setup.env));
            })
        });
    }
    group.finish();
}

fn bench_release_funds(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounty_escrow/release_funds");
    group.bench_function("release_single", |b| {
        b.iter(|| {
            let setup = Setup::new();
            let bounty_id = 1u64;
            let amount = 10_000i128;
            let deadline = setup.env.ledger().timestamp() + 1000;
            setup
                .escrow
                .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

            reset_budget(&setup.env);
            setup
                .escrow
                .release_funds(&bounty_id, &setup.contributor);
            black_box(snapshot_budget(&setup.env));
        })
    });
    group.finish();
}

fn bench_refund_full_after_deadline(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounty_escrow/refund");
    group.bench_function("refund_full_after_deadline", |b| {
        b.iter(|| {
            let setup = Setup::new();
            let bounty_id = 1u64;
            let amount = 10_000i128;
            let deadline = setup.env.ledger().timestamp() + 1000;
            setup
                .escrow
                .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

            // Move past deadline
            setup.env.ledger().set_timestamp(deadline + 1);

            reset_budget(&setup.env);
            setup
                .escrow
                .refund(&bounty_id, &None, &None, &RefundMode::Full);
            black_box(snapshot_budget(&setup.env));
        })
    });
    group.finish();
}

fn bench_batch_lock_funds(c: &mut Criterion) {
    use bounty_escrow::LockFundsItem;
    use soroban_sdk::Vec;

    let mut group = c.benchmark_group("bounty_escrow/batch_lock_funds");
    for batch in [1u32, 5, 10, 25] {
        group.bench_with_input(BenchmarkId::from_parameter(batch), &batch, |b, &n| {
            b.iter(|| {
                let setup = Setup::new();
                let deadline = setup.env.ledger().timestamp() + 1000;

                let mut items: Vec<LockFundsItem> = Vec::new(&setup.env);
                for i in 0..n {
                    items.push_back(LockFundsItem {
                        bounty_id: (i + 1) as u64,
                        depositor: setup.depositor.clone(),
                        amount: 1_000,
                        deadline,
                    });
                }

                reset_budget(&setup.env);
                setup.escrow.batch_lock_funds(&items);
                black_box(snapshot_budget(&setup.env));
            })
        });
    }
    group.finish();
}

fn bench_views(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounty_escrow/views");

    group.bench_function("get_escrow_info", |b| {
        b.iter(|| {
            let setup = Setup::new();
            let bounty_id = 1u64;
            let amount = 10_000i128;
            let deadline = setup.env.ledger().timestamp() + 1000;
            setup
                .escrow
                .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

            reset_budget(&setup.env);
            black_box(setup.escrow.get_escrow_info(&bounty_id));
            black_box(snapshot_budget(&setup.env));
        })
    });

    group.bench_function("get_balance", |b| {
        b.iter(|| {
            let setup = Setup::new();
            reset_budget(&setup.env);
            black_box(setup.escrow.get_balance());
            black_box(snapshot_budget(&setup.env));
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lock_funds,
    bench_release_funds,
    bench_refund_full_after_deadline,
    bench_batch_lock_funds,
    bench_views
);
criterion_main!(benches);

