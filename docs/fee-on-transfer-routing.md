# Fee-on-Transfer (FoT) Routing

## Overview

Fee-on-transfer (FoT) tokens deduct a fee on every transfer, meaning the
recipient receives less than the amount sent. Without compensation, this causes
payouts to fall short of the intended amounts — a 10 % FoT fee on a 100 USDC
payout leaves the winner with only 90 USDC.

The **FoT routing layer** solves this by querying an AMM router contract before
each transfer to compute the exact gross amount needed so that the recipient
receives the intended net amount after FoT deductions.

## Architecture

```
                    ┌──────────────────────┐
                    │   Program Escrow      │
                    │   Contract            │
                    └──────┬───────┬────────┘
                           │       │
                    ┌──────┘       └──────┐
                    ▼                     ▼
          ┌──────────────────┐  ┌──────────────────┐
          │  FoT Router      │  │  Token Contract  │
          │  (AMM / Oracle)  │  │  (FoT or SAC)    │
          └────────┬─────────┘  └──────────────────┘
                   │
                   │ quote(token, net_amount) → gross_amount
                   ▼
          ┌──────────────────┐
          │  Recipient       │
          │  (gets net after │
          │   FoT deduction) │
          └──────────────────┘
```

### Flow

1. Caller invokes `single_payout` / `batch_payout` with the **intended net
   amount**.
2. Protocol fees are deducted (if enabled) to arrive at the **recipient net**.
3. If a `FotRouter` is configured, the contract calls
   `router.quote(token, recipient_net)` to get the **gross transfer amount**.
4. The contract transfers the gross amount; the FoT token deducts its fee,
   delivering the intended net to the recipient.
5. `remaining_balance` is debited by the actual outflow
   (`protocol_fee + gross_transfer`).

## Configuration

### Setting the Router

Only the contract admin may set or clear the router configuration.

```rust
// Set router with 1 % slippage tolerance
contract.set_fot_router(&router_address, &100);

// Clear router (restores backward-compatible behavior)
contract.clear_fot_router();
```

### Router Interface

The router contract must expose:

```rust
fn quote(env: Env, token: Address, amount: i128) -> i128;
```

- **`token`**: The address of the token being transferred.
- **`amount`**: The net amount intended for the recipient.
- **Returns**: The gross amount to send so that, after any FoT deduction, the
  recipient receives `amount`.

### Slippage

`slippage_bps` (basis points, 1 bp = 0.01 %) is a buffer applied on top of the
quoted amount:

```
adjusted_gross = quoted_gross × (10_000 + slippage_bps) / 10_000
```

Maximum allowed slippage is **500 bps (5 %)**.

## Storage

The `FotRouter` config is stored as an optional field on `ProgramData`:

```rust
pub struct FotRouter {
    pub router_contract: Address,
    pub slippage_bps: u32,
}

pub struct ProgramData {
    // ... existing fields ...
    pub fot_router: Option<FotRouter>,
}
```

When `fot_router` is `None`, payout behavior is **identical** to the
pre-routing implementation (backward compatible).

## Events

| Event | Symbol | Description |
|-------|--------|-------------|
| `FotRouterSetEvent` | `FotRtrS` | Emitted when router config is set or updated |
| `FotRouterClearedEvent` | `FotRtrC` | Emitted when router config is cleared |

## Error Codes

| Code | Constant | Description |
|------|----------|-------------|
| 1300 | `FotRoutingFailed` | Router returned invalid result or call failed |

## Security Considerations

### Router Trust
The router contract is a **trusted component**. A malicious router could:
- Return inflated gross amounts to drain the escrow contract
- Return deflated amounts causing payouts to fall short

**Mitigation**: Only the contract admin can set the router. Choose a router
that is audited and verified.

### Slippage
Slippage adds a buffer above the quoted amount. Higher slippage provides more
robustness against quote changes but increases the debit from
`remaining_balance`. Keep slippage as low as possible (typically 10–100 bps).

### Balance Checks
The balance check (`remaining_balance >= total_debit`) runs **after** computing
the routed transfer amounts, so the FoT markup is always accounted for before
any transfer occurs.

### Reentrancy
All routing logic runs inside the existing reentrancy guard, which is acquired
before any external call (including the router `quote` call).

### Router Failure
If the router call fails or returns a non-positive amount, the payout panics.
No funds are transferred in a failed state.

## Testing

The test suite in `test_fot_routing.rs` covers:

| Test | Scenario |
|------|----------|
| `test_no_router_payout_matches_existing_behavior` | Backward compatibility: no router == existing behavior |
| `test_single_payout_router_preserves_net` | Single payout with 10 % FoT |
| `test_batch_payout_router_preserves_net` | Batch payout with 10 % FoT |
| `test_single_payout_with_slippage` | Single payout with 5 % slippage |
| `test_single_payout_zero_slippage_high_fot` | 20 % FoT, zero slippage |
| `test_batch_payout_with_slippage` | Batch payout with 2 % slippage |
| `test_router_zero_fot_fee_no_op` | Router configured but no FoT (no-op) |
| `test_router_fot_differs_from_token` | Router under-estimates FoT |
| `test_insufficient_balance_with_routing` | Balance check includes FoT markup |
| `test_clear_router_restores_no_routing` | Clearing router restores old behavior |
| `test_batch_insufficient_balance_with_routing` | Batch balance check with FoT |
| `test_single_payout_fee_waived_with_routing` | Fee waiver + routing |
| `test_set_fot_router_emits_event` | Event emission on set |
| `test_clear_fot_router_emits_event` | Event emission on clear |
| `test_set_fot_router_rejects_excessive_slippage` | Slippage > 5 % rejected |
| `test_batch_payout_fee_waived_with_routing` | Batch + fee waiver + routing |
| `test_router_no_fot_on_token` | Router configured but token has 0 % FoT |
| `test_fot_router_in_program_data` | ProgramData.fot_router is accessible |
| `test_clear_router_removes_config` | After clear, ProgramData.fot_router is None |
| `test_single_payout_record_reflects_transfer_amount` | PayoutRecord uses routed amount |
| `test_batch_payout_records_reflect_transfer_amounts` | Batch PayoutRecords use routed amounts |

## Migration

This feature is **fully backward compatible**. Deployed contracts without a
router configuration continue to work identically. To enable routing:

1. Deploy or identify an FoT router contract implementing `quote`.
2. Call `set_fot_router(router_address, slippage_bps)` via the contract admin.
3. Subsequent payouts will automatically use the router for gross amount
   computation.
4. To disable, call `clear_fot_router()`.
