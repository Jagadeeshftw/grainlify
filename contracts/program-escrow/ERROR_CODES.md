# Program Escrow Error Codes

Canonical typed errors for `contracts/program-escrow` live in
[`src/errors.rs`](./src/errors.rs) as `ContractError` (`#[contracterror]`).

Deployable entrypoints must surface failures via `Result<_, ContractError>` or
`panic_with_error!(&env, &ContractError::…)` — never bare `unwrap()` / `panic!`
(opaque host traps). Test-only modules under `#[cfg(test)]` remain exempt.

## Ranges

| Range | Domain |
| --- | --- |
| 1–99 | General (auth, validation, state) |
| 100–199 | Program management |
| 200–299 | Fund operations |
| 300–399 | Payouts |
| 400–499 | Schedules |
| 500–599 | Claims |
| 600–699 | Disputes |
| 700–799 | Fees |
| 800–899 | Circuit breaker |
| 900–999 | Threshold monitoring |
| 1000–1099 | Batch recovery |
| 1100–1199 | Token allowlist |
| 1200–1299 | Role rotation / FoT routing |
| 1300–1399 | Dynamic pricing / oracle |

## Stability

Error discriminants are stable across minor releases. New variants may be added;
existing codes are not renumbered without a major version bump. Messages in
`ContractError::message()` stay generic (no addresses or amounts).

## CI gate

`scripts/check-program-escrow-no-traps.py` fails when a new `.unwrap()` or
`panic!` appears in deployable `src/lib.rs`. Validate the gate locally:

```bash
python3 scripts/check-program-escrow-no-traps.py
python3 scripts/check-program-escrow-no-traps.py \
  --fixture scripts/tests/fixtures/program_escrow_unwrap_trap.rs \
  --expect-fail
```
