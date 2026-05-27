# Client SDK Idempotency Key Generation Guide

This guide is for client SDK authors integrating with the Program Escrow
contract. It defines the recommended key format, namespace isolation rules,
and pseudocode examples for generating safe, collision-resistant idempotency
keys.

For server-side handling (storage schema, retry logic, event specs) see
`IDEMPOTENCY_KEYS_IMPLEMENTATION.md`.

---

## Why Idempotency Keys Matter

The contract stores each key permanently and returns the original result on
any retry. A key collision — two different operations sharing the same key —
causes the second operation to silently return the first operation's result
without executing. This can strand funds or produce incorrect payout records.

---

## Recommended Key Format
| Segment       | Description                                              | Example                        |
|---------------|----------------------------------------------------------|--------------------------------|
| `program_id`  | The on-chain program identifier                          | `hackathon-2024`               |
| `payout_type` | Operation type: `single` or `batch`                      | `single`                       |
| `recipient`   | Recipient Stellar address (first 8 chars for batch)      | `GABC1234`                     |
| `nonce`       | Cryptographically random 16-byte hex string              | `a3f1c2d4e5b6a7f8`             |

### Full Examples
### Maximum Length

The contract enforces a hard limit of **256 characters**. Keep each segment
short:

- `program_id`: max 64 chars
- `payout_type`: 6 chars (`single` or `batch`)
- `recipient`: first 8 chars of the Stellar address
- `nonce`: 16-char hex (8 random bytes)

Total with separators: ~100 chars — well within the limit.

---

## Namespace Isolation

Keys are **globally scoped** in contract storage — there is no per-program
namespace. Two programs using the same key format without `program_id` as a
prefix will collide.

### Rules

1. **Always prefix with `program_id`.**
   Keys without a program prefix can collide across programs sharing the same
   contract instance.

2. **Never reuse a key across operation types.**
   A key used for `single_payout` must never be reused for `batch_payout`,
   even for the same recipient and amount.

3. **Never reuse a key across programs.**
   Even if `program_id` values are unique, always include it in the key so
   collisions are impossible by construction.

4. **Never use predictable values as the sole nonce.**
   Incrementing counters, timestamps alone, or recipient addresses alone are
   not sufficient — combine them with random bytes.

### Collision Risk Examples
---

## Pseudocode Examples

### Web Client (JavaScript / TypeScript)

```typescript
### CLI Client (Rust)

```rust

```rust
use rand::Rng;

/// Generate a collision-resistant idempotency key for a single payout.
///
/// # Arguments
/// * `program_id` - On-chain program identifier (max 64 chars)
/// * `recipient`  - Full Stellar address of the recipient
///
/// # Panics
/// Panics if the generated key exceeds 256 characters.
pub fn generate_single_payout_key(program_id: &str, recipient: &str) -> String {
    let nonce: [u8; 8] = rand::thread_rng().gen();
    let nonce_hex = hex::encode(nonce);
    let recipient_prefix = &recipient[..8.min(recipient.len())];
    let key = format!("{}-single-{}-{}", program_id, recipient_prefix, nonce_hex);

    assert!(
        key.len() <= 256,
        "Idempotency key too long: {} chars",
        key.len()
    );
    key
}

/// Generate a collision-resistant idempotency key for a batch payout.
///
/// # Arguments
/// * `program_id`  - On-chain program identifier (max 64 chars)
/// * `recipients`  - Slice of recipient Stellar addresses
///
/// # Panics
/// Panics if recipients is empty or the generated key exceeds 256 characters.
pub fn generate_batch_payout_key(program_id: &str, recipients: &[&str]) -> String {
    assert!(!recipients.is_empty(), "Recipients must not be empty");
    let nonce: [u8; 8] = rand::thread_rng().gen();
    let nonce_hex = hex::encode(nonce);
    let first_prefix = &recipients[0][..8.min(recipients[0].len())];
    let count = recipients.len();
    let key = format!(
        "{}-batch-{}-{}r-{}",
        program_id, first_prefix, count, nonce_hex
    );

    assert!(
        key.len() <= 256,
        "Idempotency key too long: {} chars",
        key.len()
    );
    key
}

// Usage
fn main() {
    let single_key = generate_single_payout_key(
        "hackathon-2024",
        "GABC1234DEFG5678HIJK9012LMNO3456PQRS7890TUVW1234XYZ5",
    );
    println!("{}", single_key);
    // -> "hackathon-2024-single-GABC1234-a3f1c2d4e5b6a7f8"

    let recipients = vec![
        "GABC1234DEFG5678HIJK9012LMNO3456PQRS7890TUVW1234XYZ5",
        "GXYZ5678ABCD1234EFGH5678IJKL9012MNOP3456QRST7890UVWX",
        "GHIJ9012KLMN3456OPQR7890STUV1234WXYZ5678ABCD9012EFGH",
    ];
    let batch_key = generate_batch_payout_key("hackathon-2024", &recipients);
    println!("{}", batch_key);
    // -> "hackathon-2024-batch-GABC1234-3r-9b8c7d6e5f4a3b2c"
}
```

---

## Retry Safety Rules

1. **Store the key before submitting the transaction.** If the network drops
   the response, you need the original key to retry — regenerating produces a
   new key and a duplicate payout.

2. **Retry with the exact same key and parameters.** The contract validates
   the key and returns the original result. Changing any parameter with the
   same key will still return the cached result — not execute a new operation.

3. **Implement exponential backoff.** Do not hammer the RPC on failure:

```typescript
async function payoutWithRetry(
  client: EscrowClient,
  key: string,
  recipient: string,
  amount: bigint,
  maxAttempts = 3
): Promise<PayoutResult> {
  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    try {
      return await client.singlePayout(recipient, amount, key);
    } catch (err) {
      if (attempt === maxAttempts - 1) throw err;
      await sleep(Math.pow(2, attempt) * 1000); // 1s, 2s, 4s
    }
  }
  throw new Error("Max retry attempts exceeded");
}
```

---

## Security Assumptions

1. **Nonce entropy.** The 8-byte (64-bit) random nonce provides 1-in-2^64
   collision resistance per (program_id, payout_type, recipient) tuple. Use
   a cryptographically secure RNG (`crypto.randomBytes` in Node.js,
   `rand::thread_rng` in Rust).

2. **Key secrecy is not required.** Idempotency keys are not secrets — they
   are stored on-chain and visible in events. Their purpose is uniqueness,
   not confidentiality.

3. **Keys are permanent.** The contract never deletes idempotency records.
   Do not rely on key expiration for cleanup.

4. **Global scope.** Keys are not namespaced per program in storage. The
   `program_id` prefix in the recommended format is a client-side convention
   that prevents collisions — it is not enforced by the contract.

---

## Related Files

| File | Role |
|------|------|
| `IDEMPOTENCY_KEYS_IMPLEMENTATION.md` | Server-side handling, storage schema, events |
| `contracts/program-escrow/src/lib.rs` | Contract implementation |
| `contracts/program-escrow/src/test_batch_operations.rs` | Integration tests including key generation conventions |
