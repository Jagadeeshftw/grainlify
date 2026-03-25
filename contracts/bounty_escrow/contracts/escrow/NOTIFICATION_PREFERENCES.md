# Notification Preferences (Escrow Metadata)

## Overview
Notification preferences are stored on-chain as a compact bitfield in `EscrowMetadata.notification_prefs`.
They are **hints** for off-chain systems (indexers, bots, schedulers) that want to honor creator/admin
preferences about which lifecycle events should trigger notifications.

This keeps preferences portable across indexers without introducing heavy on-chain storage.

## Flags

Each preference is a bit in a `u32`. Multiple flags can be combined with bitwise OR.
A value of `0` means **no explicit preference** (off-chain systems may use their own defaults).

| Flag | Value | Meaning |
| --- | --- | --- |
| `NOTIFY_ON_LOCK` | `1 << 0` | Notify when funds are locked into escrow. |
| `NOTIFY_ON_RELEASE` | `1 << 1` | Notify when funds are released. |
| `NOTIFY_ON_DISPUTE` | `1 << 2` | Notify when a dispute is opened or resolved. |
| `NOTIFY_ON_EXPIRATION` | `1 << 3` | Notify when an escrow expires. |

## Events

Preference changes are emitted via the `NotificationPreferencesUpdated` event. Off-chain systems should:

1. Listen for `npref` topic events.
2. Track the latest `new_prefs` value per `bounty_id`.
3. Use `created == true` to treat the event as the initial preference set.
4. Use `created == false` to treat the event as an update/override.

Event payload fields:

- `version`: event version (currently `2`).
- `bounty_id`: the escrow identifier.
- `previous_prefs`: prior preference value (0 if unset).
- `new_prefs`: the updated preference value.
- `actor`: the admin or creator who set the preference.
- `created`: whether this was the first preference set for the escrow.
- `timestamp`: ledger timestamp when set.

## Interpretation Guidance

- Preferences are **advisory**: off-chain systems may apply additional filters or throttling.
- If no preference is set (`notification_prefs == 0`), treat as “no explicit preference.”
- If a flag is set, emit notifications for that event type when it occurs.
- Multiple flags may be set and should be interpreted independently.
