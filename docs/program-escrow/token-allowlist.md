# Token Allowlist & Decimal Normalization

## Overview

The program-escrow contract maintains a **token allowlist** that restricts which
token contract addresses may be used when initialising a program. When the list
is non-empty, only explicitly permitted tokens are accepted; an empty list
disables enforcement (any token is accepted).

Issue [#1295](https://github.com/Jagadeeshftw/grainlify/issues/1295) extends the
allowlist to **store each token's decimal precision** alongside its address. This
lets off-chain tooling display human-readable payout amounts without extra RPC
calls and provides an auditable, admin-controlled record of each token's
precision.

---

## Storage Layout

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::TokenAllowlist` | `Vec<Address>` | Legacy V1 list (backward compat) |
| `DataKey::TokenAllowlistV2` | `Vec<AllowedTokenEntry>` | V2 list with decimals |
| `DataKey::TokenDecimals(addr)` | `u32` | Per-token decimal cache (O(1) lookup) |
| `DataKey::TokenAllowlistSchemaVersion` | `u32` | Upgrade-safety marker |

### `AllowedTokenEntry`

```rust
pub struct AllowedTokenEntry {
    pub token:    Address,  // token contract address
    pub decimals: u32,      // 0–18
}
```

---

## Entrypoints

### Admin

| Function | Description |
|----------|-------------|
| `add_allowed_token_with_decimals(token, decimals)` | Add token with explicit decimal precision (preferred) |
| `add_allowed_token(token)` | Add token with `decimals = 0` (legacy, backward-compat) |
| `remove_allowed_token(token)` | Remove token; clears decimal cache |

### View

| Function | Returns |
|----------|---------|
| `get_allowed_tokens()` | `Vec<Address>` — V1-compatible list |
| `get_allowed_tokens_with_decimals()` | `Vec<AllowedTokenEntry>` — V2 list with decimals |
| `get_token_decimals(token)` | `u32` — stored decimals for a token (0 if not found) |
| `is_token_allowed(token)` | `bool` |
| `get_allowlist_schema_version()` | `u32` |

---

## Decimal Normalization

All payout `amount` parameters (`single_payout`, `batch_payout`) are expressed
in the **token's own base units** — the smallest indivisible unit of that token.

| Token | Decimals | 1 human unit = |
|-------|----------|----------------|
| USDC  | 6        | 1_000_000 base units |
| XLM   | 7        | 10_000_000 base units |
| Custom| 18       | 1_000_000_000_000_000_000 base units |

The contract does **not** re-scale amounts at payout time. Callers must supply
amounts already denominated in the token's base units. The `decimals` field is
stored for off-chain tooling only.

### Security Assumptions

1. **Admin controls decimals** — only the contract admin can add/remove tokens
   and set their decimal values. A compromised admin key can set incorrect
   decimals, but this only affects off-chain display, not on-chain transfer
   amounts.
2. **Decimals are capped at 18** — `add_allowed_token_with_decimals` panics if
   `decimals > 18` to prevent overflow in any future normalization arithmetic.
3. **Backward compatibility** — tokens added via the legacy `add_allowed_token`
   path have `decimals = 0`. Off-chain tooling should treat `0` as "unknown" and
   query the token contract directly if needed.
4. **Deterministic enforcement** — the allowlist check runs before any state
   mutation in `init_program`, so rejected tokens never produce partial writes.

---

## Events

### `TokenAllowlistUpdatedEvent`

Emitted on every `add_allowed_token*` and `remove_allowed_token` call.

```
topic:   (TkAllow,)
payload: TokenAllowlistUpdatedEvent {
    version:    u32,
    token:      Address,
    added:      bool,      // true = added, false = removed
    updated_by: Address,
    timestamp:  u64,
    decimals:   u32,       // stored decimals (0 on remove)
}
```

### `TokenRejectedEvent`

Emitted before panic when `init_program` is called with a non-allowlisted token.

```
topic:   (TkReject,)
payload: TokenRejectedEvent { version, token, program_id, timestamp }
```

---

## Example Usage

```bash
# Add USDC (6 decimals) to the allowlist
stellar contract invoke --id $CONTRACT \
  -- add_allowed_token_with_decimals \
  --token $USDC_ADDRESS \
  --decimals 6

# Add a custom 18-decimal token
stellar contract invoke --id $CONTRACT \
  -- add_allowed_token_with_decimals \
  --token $CUSTOM_TOKEN \
  --decimals 18

# Query the V2 allowlist
stellar contract invoke --id $CONTRACT \
  -- get_allowed_tokens_with_decimals

# Query decimals for a specific token
stellar contract invoke --id $CONTRACT \
  -- get_token_decimals \
  --token $USDC_ADDRESS
```
