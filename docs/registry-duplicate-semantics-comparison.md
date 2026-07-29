# Registry Duplicate-Registration Semantics: grainlify-core vs view-facade

> **Status**: Documentation only — no behaviour changes in this document.  
> **Follow-up**: See [Inconsistency Flag](#inconsistency-flag--follow-up-alignment-issue) at the bottom.

---

## Background

The Grainlify protocol contains two independently implemented contract registries:

| Contract | File | Entry-point |
|---|---|---|
| `grainlify-core` | `contracts/grainlify-core/src/lib.rs` | `register_deployed_contract` |
| `view-facade` | `contracts/view-facade/src/lib.rs` | `register` |

Both registries accept an `(address, kind, version)` tuple and store it in
instance storage. They diverge in what happens when the **same address is
submitted a second time** (duplicate registration).

This document establishes the **observed, test-verified** behaviour of each
registry so that consumers, integrators, and future maintainers understand the
contracts without having to re-read and re-derive the behaviour from source.

---

## Side-by-Side Comparison

| Aspect | `grainlify-core` | `view-facade` |
|---|---|---|
| **Duplicate strategy** | **Update-in-place** | **Duplicate-append** |
| **Index grows on re-registration?** | ❌ No | ✅ Yes |
| **Entry data overwritten?** | ✅ Yes (name, kind, version, deployed_at) | ✅ Yes (new entry pushed; old entry stays) |
| **`count()` after re-registration** | Unchanged (same as before) | Incremented by 1 |
| **Address appears in list N times after N calls?** | No — exactly once always | Yes — N times |
| **`get_*(address)` after re-registration** | Returns latest metadata | Returns **first** match (oldest metadata) |
| **Storage key for entry** | `DeployedContractEntry(Address)` — keyed by address | `Registry` Vec — appended to sequentially |
| **Storage key for index** | `DeployedContractIndex` Vec — not extended on duplicate | `Registry` Vec — always extended |
| **Cap enforcement on re-registration** | Cap check skipped for existing address | Cap check applies regardless |
| **Admin auth required** | ✅ Yes | ✅ Yes |
| **Read-only mode guard** | ✅ Yes | N/A (no read-only mode in view-facade) |
| **Module-level doc claim** | Not documented in module doc | Claims update-in-place ("updated, not duplicated") |
| **Function-level doc note** | No note (behaviour implied by code) | `# Note` warns duplicates ARE created |
| **Test-verified?** | ✅ Yes — see `test_contract_registry.rs` | ⚠️ Partial — existing tests do not explicitly assert duplicate counts |

---

## Detailed Behaviour: grainlify-core

### Source location
`contracts/grainlify-core/src/lib.rs` — function `register_deployed_contract`

### Logic (simplified)

```rust
let existed = env.storage().instance()
    .has(&DataKey::DeployedContractEntry(address.clone()));

if !existed {
    // Only push to the ordered index on first registration.
    if index.len() >= MAX_DEPLOYED_CONTRACTS {
        panic!("Registry full");
    }
    index.push_back(address.clone());
}

// Always write the entry — overwrites on re-registration.
let entry = DeployedContract { address, name, kind, version, deployed_at: now };
env.storage().instance()
    .set(&DataKey::DeployedContractEntry(address), &entry);
env.storage().instance()
    .set(&DataKey::DeployedContractIndex, &index);
```

### Consequences

- `DeployedContractEntry(addr)` is keyed per-address — only one slot exists per address.
- `DeployedContractIndex` is the ordered list of addresses; it is only extended for genuinely new addresses.
- `deployed_contract_count()` reads `DeployedContractIndex.len()` — unchanged by re-registration.
- `get_deployed_contract(addr)` always returns the latest values after any number of re-registrations.
- `list_deployed_contracts()` iterates the index, so each address appears exactly once.

### Verified tests (contracts/grainlify-core/src/test_contract_registry.rs)

| Test | What it proves |
|---|---|
| `test_reregister_same_address_count_unchanged` | count does not increase on re-registration |
| `test_reregister_same_address_overwrites_metadata` | name/kind/version reflect the latest call |
| `test_reregister_same_address_updates_deployed_at` | deployed_at is refreshed |
| `test_reregister_same_address_appears_once_in_list` | address appears exactly once in list |
| `test_reregister_many_addresses_no_count_inflation` | N re-registrations never inflate count beyond N |
| `test_register_after_deregister_creates_new_entry` | deregister + re-register = fresh entry, count = 1 |
| `test_reregister_kind_change_is_visible` | every ContractKind change is immediately reflected |

---

## Detailed Behaviour: view-facade

### Source location
`contracts/view-facade/src/lib.rs` — function `register`

### Logic (simplified)

```rust
let mut registry: Vec<RegisteredContract> = env.storage().instance()
    .get(&DataKey::Registry)
    .unwrap_or(Vec::new(&env));

// Cap check applies to every call, including re-registration.
if registry.len() >= MAX_REGISTRY_SIZE {
    return Err(FacadeError::RegistryFull);
}

// Unconditional push — no duplicate guard.
registry.push_back(RegisteredContract { address, kind, version });

env.storage().instance().set(&DataKey::Registry, &registry);
```

### Consequences

- `Registry` is a flat `Vec<RegisteredContract>` — there is no secondary index keyed by address.
- Every call appends a new element regardless of whether the address already exists.
- `contract_count()` returns `Registry.len()`, which **increments** on every `register` call.
- `get_contract(addr)` performs an `O(n)` linear scan and returns the **first** (oldest) match.
  After N re-registrations there are N entries for the same address; `get_contract` returns
  the one pushed earliest.
- `list_contracts()` / `list_contracts_all()` return all entries including duplicates.

### Documentation inconsistency within the file

The **module-level doc** (top of `lib.rs`) states:

> "When `register` is called with an address that is already in the registry,
> the existing entry is **updated** (not duplicated) with the new `kind` and
> `version` values."

The **function-level `# Note`** (on `register` itself) contradicts this:

> "Registering the same address multiple times will create duplicate entries.
> Callers should call `get_contract` first to check for an existing entry, or
> `deregister` before re-registering with updated metadata."

**The code matches the `# Note`** — duplicates are created. The module-level
description is incorrect.

---

## Inconsistency Flag — Follow-up Alignment Issue

> ⚠️ **This section flags a candidate for a follow-up issue.**
>
> This document's scope is limited to establishing and documenting current
> behaviour. **No behaviour changes are made here.**

### Summary of inconsistencies discovered

1. **view-facade module doc vs code**: The module-level doc claims update-in-place;
   the code creates duplicates. The function-level `# Note` is accurate.

2. **grainlify-core vs view-facade runtime behaviour**: The two registries behave
   opposite to each other under duplicate registration. Consumers that interact
   with both registries may write code that assumes a shared contract and receive
   surprising results.

3. **Absence of deduplication test coverage in view-facade**: The existing
   `contracts/view-facade/src/test.rs` does not include a test that asserts
   duplicate counts after two `register` calls with the same address. The
   duplicate-creating behaviour is therefore undocumented at the test level.

### Proposed follow-up issue

**Title**: Align duplicate-registration semantics between grainlify-core and
view-facade (or document the divergence as intentional)

**Scope**:
- Decide canonical duplicate policy: update-in-place vs duplicate-append
- If update-in-place is chosen: patch `view-facade::register` to check for
  existing entries before pushing, and correct the module doc
- If duplicate-append is chosen: update `grainlify-core` to match and correct
  both sets of doc comments
- If intentionally divergent: update both module docs to explicitly state the
  chosen policy and the rationale for the difference
- Add explicit duplicate-registration tests to `view-facade/src/test.rs`
  mirroring those added to `test_contract_registry.rs`

---

## Storage Architecture Comparison

```
grainlify-core
───────────────────────────────────────────────────────────────
  Instance storage:
    DeployedContractIndex  →  Vec<Address>          (ordered; no duplicates)
    DeployedContractEntry(addr)  →  DeployedContract  (one slot per address)

  Re-registration:  overwrites DeployedContractEntry; index unchanged
  Lookup:           O(1) via DeployedContractEntry(addr)
  List:             iterates DeployedContractIndex, fetches each entry

view-facade
───────────────────────────────────────────────────────────────
  Instance storage:
    Registry  →  Vec<RegisteredContract>  (flat list; may contain duplicates)

  Re-registration:  appends new element to Registry
  Lookup:           O(n) linear scan; returns first (oldest) match
  List:             returns Registry slice directly (may include duplicates)
```

---

## References

- `contracts/grainlify-core/src/lib.rs` — `register_deployed_contract`, `deregister_deployed_contract`
- `contracts/grainlify-core/src/test_contract_registry.rs` — full test suite including new duplicate-semantics tests
- `contracts/view-facade/src/lib.rs` — `register`, `deregister`, `get_contract`, `list_contracts`
- `contracts/view-facade/src/test.rs` — existing view-facade tests
- `contracts/view-facade/DUPLICATE_REGISTRATION_POLICY.md` — prior policy discussion
