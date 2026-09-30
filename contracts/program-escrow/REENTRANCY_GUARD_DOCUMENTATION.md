# Program Escrow Reentrancy Guard Regression Coverage

## Guarded transfer boundaries

`program-escrow` uses one instance-wide `u32` flag at `DataKey::ReentrancyGuard`:

- `1` — `NOT_ENTERED`
- `2` — `ENTERED`

The guard is acquired before business-state reads and held through every token transfer. Normal transfer paths release it after state writes and events; panic and error paths roll it back with the rest of the invocation.

| Guarded implementation | Public entry points | Named regression test |
| --- | --- | --- |
| `batch_lock` | `batch_lock` | `test_batch_lock_guard_blocks_reentrant_token_transfer` |
| `batch_release` | `batch_release` | `test_batch_release_guard_blocks_reentrant_token_transfer` |
| `batch_payout_internal` | `batch_payout`, `batch_payout_by`, `batch_payout_idempotent*`, `batch_payout_v2`, `batch_payout_with_receipt` | `test_batch_payout_guard_blocks_reentrant_token_transfer` |
| `single_payout_internal` | `single_payout`, `single_payout_by`, `single_payout_idempotent*` | `test_single_payout_guard_blocks_reentrant_token_transfer` |
| `trigger_program_releases_internal` | `trigger_program_releases`, `trigger_program_releases_by` | `test_trigger_program_releases_guard_blocks_reentrant_token_transfer` |

Each acquisition and success-path release is documented beside its production call site in `src/lib.rs` and names the regression test that protects it.

## Adversarial token

`src/malicious_reentrant.rs` defines a test-only SEP-41-shaped `ReentrantToken` with balance tracking and configurable callback actions for all five guarded implementations. During `transfer`, it:

1. Moves balances.
2. Calls the matching `program-escrow` entry point through `try_*` while the outer escrow invocation is still active.
3. Verifies that Soroban rejects the same-instance reentry.
4. Returns control to the guarded escrow invocation.

Soroban rejects direct same-instance reentry before the second contract body runs, so the host rejection alone cannot detect a missing application guard. The guarded transfer call sites therefore bracket the token call with a test-only `assert_escrow_guard` marker executed in the escrow frame. This makes acquisition and lifetime observable without changing deployable Wasm or adding a dependency.

## Mutation sensitivity

Each named test performs two valid operations against the armed token:

- The first call fails if that entry point's acquisition is removed.
- The second call fails if its success-path release is removed.

The following mutations were validated independently and restored after each run:

| Removed guard half | Expected failing test |
| --- | --- |
| `batch_lock` acquire | `test_batch_lock_guard_blocks_reentrant_token_transfer` |
| `batch_lock` release | `test_batch_lock_guard_blocks_reentrant_token_transfer` |
| `batch_release` acquire | `test_batch_release_guard_blocks_reentrant_token_transfer` |
| `batch_release` release | `test_batch_release_guard_blocks_reentrant_token_transfer` |
| `batch_payout_internal` acquire | `test_batch_payout_guard_blocks_reentrant_token_transfer` |
| `batch_payout_internal` release | `test_batch_payout_guard_blocks_reentrant_token_transfer` |
| `single_payout_internal` acquire | `test_single_payout_guard_blocks_reentrant_token_transfer` |
| `single_payout_internal` release | `test_single_payout_guard_blocks_reentrant_token_transfer` |
| `trigger_program_releases_internal` acquire | `test_trigger_program_releases_guard_blocks_reentrant_token_transfer` |
| `trigger_program_releases_internal` release | `test_trigger_program_releases_guard_blocks_reentrant_token_transfer` |

## CI and local execution

The existing `.github/workflows/e2e-upgrade-tests.yml` `Contract smoke` workflow has no pull-request path filter; its `program-escrow reentrancy guards` job runs this suite on every PR. The same command is also step 3 of `contracts/scripts/ci-contracts.sh`, so contract CI and local script runs stay aligned:

```bash
cargo test --locked \
  --manifest-path contracts/program-escrow/Cargo.toml \
  reentrancy_tests::
```

To run one mutation target locally:

```bash
cargo test --locked \
  --manifest-path contracts/program-escrow/Cargo.toml \
  reentrancy_tests::test_single_payout_guard_blocks_reentrant_token_transfer \
  -- --exact
```
