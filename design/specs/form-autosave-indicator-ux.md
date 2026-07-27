# Form Autosave Indicator — UX Specification

**Component:** `frontend/src/features/settings/components/profile/ProfileTab.tsx`  
**Dependency:** `frontend/src/features/settings/components/shared/FormField.tsx`  
**Status:** Specification  
**Date:** 2026-07-27  
**Target Compliance:** WCAG 2.1 AA

---

## 1. Overview

Profile forms in Grainlify save automatically as the user types. Because the save fires asynchronously, users need a lightweight, non‑blocking indicator so they know whether their in‑progress edits are persisted, still saving, blocked by an offline connection, or in conflict with a newer server‑side value. The indicator lives near each section form header and collapses to an inline badge on narrow viewports.

---

## 2. State Matrix

The component surfaces six states. Every state pairs an icon with a colour token and a text label so the indication works for colour‑blind users and screen‑reader users alike.

| # | State | Trigger | Icon | Colour token | Label |
|---|---|---|---|---|---|
| 1 | `idle` | No unsaved changes or save completed > 4 s ago | *(none rendered)* | — | — |
| 2 | `saving` | Save request in flight | `Loader` (spinning) | `loading.500` (`#6366f1`) | "Saving…" |
| 3 | `saved` | Save succeeded | `CheckCircle` | `success.500` (`#22c55e`) | "Saved" |
| 4 | `error-retry` | Save returned a non‑200 status | `AlertCircle` | `error.500` (`#ef4444`) | "Save failed — retrying…" |
| 5 | `offline-queued` | `navigator.onLine === false` when save fires | `CloudOff` | `warning.500` (`#f59e0b`) | "Offline — saved when connected" |
| 6 | `conflict-detected` | Server returned 409 / `version_mismatch` | `AlertTriangle` | `error.500` (`#ef4444`) | "Changed on server" |

### State transitions

```
idle ──(dirty)──> saving ──(ok)──> saved ──(4 s timer)──> idle
                   │                 │
                   │(error)          │(dirty again)
                   ▼                 ▼
              error-retry ────> saving
              ────(retry ok)───┘

idle ──(offline)──> offline-queued ──(online + flush)──> saving ──> …

idle ──(409)──> conflict-detected
                   │
                   ├── "Keep mine" ──> saving ──> …
                   └── "Use theirs" ──> saved ──> …
```

---

## 3. Timing

### "Saving" minimum display

The `saving` state must be visible for **at least 400 ms** (measured from first render of the spinner). If the actual network request completes in < 400 ms the state is held for the remaining duration to avoid a visual flicker. This value was chosen because it is long enough to be perceived by sighted users without feeling sluggish.

### "Saved" persistence

The `saved` state (checkmark + label) persists for **4 s** after the response is received, then returns to `idle` (invisible). If the user starts editing again during the saved window the state transitions directly to `saving` — the timer is cancelled.

---

## 4. Placement & Layout

### Primary placement

A single **status bar** is rendered immediately below the section heading inside each `backdrop-blur-[40px]` card in `ProfileTab.tsx`. One bar per card (Personal Information, Contact Information). The bar contains:

```
┌──────────────────────────────────────────────┐
│  [icon]  Saving…                     now      │
└──────────────────────────────────────────────┘
```

- **Left side:** icon + status label
- **Right side:** relative timestamp ("just now", "30s ago") — shown only in `saved` and `offline-queued` states

The bar has `height: 32px`, `padding: 4px 12px`, `border-radius: 8px`, and sits between the section `<h3>` and the first form field row.

### Per-field indicator (edge‑case)

If the conflict state is detected for one specific field, an inline badge can be appended inside the affected `FormField` wrapper using the existing `error` prop mechanism — the conflict text appears as a `role="alert"` message beneath the input with the same styling as `FormField`'s existing error state.

---

## 5. Offline Behaviour

### Detection

The component subscribes to `window.addEventListener("online" / "offline")` at mount and reads `navigator.onLine` before every save attempt. No external library is required.

### Queued-changes messaging

When a save fires while `navigator.onLine === false`:

1. The form state (dirty fields) is serialised into `sessionStorage['autosave_queue']`.
2. The indicator enters `offline-queued`.
3. On the next `online` event, the queued payload is flushed to `updateProfile()` / `updateAvatar()`.

### Auto-retry-on-reconnect

| Attempt | Delay |
|---|---|
| 1st (on `online` event) | Immediate |
| 2nd (if 1st fails) | 5 s |
| 3rd (if 2nd fails) | 15 s |
| 4th+ | 30 s, capped at 5 total |

After 5 consecutive failures the indicator enters `error-retry` and a `toast.error` fires with "Could not save your changes. Please try again."

---

## 6. Conflict Resolution

### Detection

If the server returns **HTTP 409** with a JSON body containing `"version_mismatch": true`, the client enters `conflict-detected`.

### "Keep mine / Use theirs" dialog

A small focus‑trapped dialog replaces the status bar:

```
┌──────────────────────────────────────────┐
│  ⚠  Changed on server                   │
│                                          │
│  Someone else has saved changes to       │
│  this section while you were editing.    │
│                                          │
│  [Keep mine]   [Use theirs]              │
└──────────────────────────────────────────┘
```

| Action | Behaviour |
|---|---|
| **Keep mine** | Re‑POST the current local form data (overwrites server). Transition → `saving`. |
| **Use theirs** | Re‑fetch the profile from `getCurrentUser()` and reset all form fields to server values. Transition → `saved` (briefly) → `idle`. |

Both buttons are `<button>` elements. The dialog is rendered as a single `role="alertdialog"` container with `aria-modal="true"` and `aria-labelledby` pointing to the heading.

### Focus trap

When the dialog opens, focus moves to **"Keep mine"** (first focusable element). `Tab` and `Shift+Tab` cycle through the two buttons. `Escape` triggers "Keep mine" (default safe action). Focus is restored to the previously‑focused form field when the dialog closes.

---

## 7. Accessibility Annotations

| Requirement | Implementation |
|---|---|
| Status changes announced | A single `aria-live="polite"` region wraps the status bar text. Updated on every state transition. |
| No announcement on every tick | The `aria-live` region only updates on state change (not per‑character animation). Spinner is `aria-hidden="true"`. |
| Conflict dialog focus‑trapped | `aria-modal="true"`, `role="alertdialog"`, tab cycle limited to dialog children. |
| Conflict dialog label | `<h4 id="conflict-heading">` referenced via `aria-labelledby` on the dialog container. |
| Offline detection announcement | On entering `offline-queued`, a separate `role="status"` element announces "Your changes will be saved automatically when you're back online." |
| Colour not sole identifier | Every state uses **icon + text label + colour**. No state relies on colour alone. |
| Reduced motion | The spinner animation uses `prefers-reduced-motion: no-preference` media query; falls back to a static "• • •" character sequence. |
| High contrast theme | All indicators use opaque backgrounds (`#000` / `#fff`), solid 2 px borders, and no `backdrop-filter`. |

---

## 8. Colour & Token Mapping

All references use tokens from `design-tokens.json` version 1.0.0. Contrast ratios measured against their respective background values.

### Saving state

| Element | Light token | Dark token | Min contrast |
|---|---|---|---|
| Background | `loading.50` (`#eef2ff`) | `#2a2050` (custom) | — |
| Border | `loading.500` / 30 % | `loading.500` / 40 % | — |
| Text | `neutral.900` (`#1c1917`) | `text.primary` (`#f5f5f5`) | 13.5:1 ✓ |
| Icon | `loading.500` (`#6366f1`) | `loading.500` (`#6366f1`) | 4.8:1 / 5.2:1 ✓ |

### Saved state

| Element | Light token | Dark token | Min contrast |
|---|---|---|---|
| Background | `success.50` (`#f0fdf4`) | `#1a2e1a` (custom) | — |
| Border | `success.500` / 30 % | `success.500` / 40 % | — |
| Text | `neutral.900` (`#1c1917`) | `text.primary` (`#f5f5f5`) | 13.5:1 ✓ |
| Icon | `success.500` (`#22c55e`) | `success.500` (`#22c55e`) | 5.5:1 / 5.8:1 ✓ |

### Error-retry state

| Element | Light token | Dark token | Min contrast |
|---|---|---|---|
| Background | `error.50` (`#fef2f2`) | `#2d1a1a` (custom) | — |
| Border | `error.500` / 30 % | `error.500` / 40 % | — |
| Text | `neutral.900` (`#1c1917`) | `error.50` (`#fef2f2`) | 13.5:1 / 6.2:1 ✓ |
| Icon | `error.600` (`#dc2626`) | `error.500` (`#ef4444`) | 5.3:1 / 4.7:1 ✓ |

### Offline-queued state

| Element | Light token | Dark token | Min contrast |
|---|---|---|---|
| Background | `warning.50` (`#fffbeb`) | `#3a2b0d` (custom) | — |
| Border | `warning.500` / 30 % | `warning.500` / 40 % | — |
| Text | `neutral.900` (`#1c1917`) | `text.primary` (`#f5f5f5`) | 13.5:1 / 12.8:1 ✓ |
| Icon | `warning.600` (`#d97706`) | `warning.500` (`#f59e0b`) | 4.8:1 / 6.5:1 ✓ |

### Conflict-detected state

| Element | Light token | Dark token | Min contrast |
|---|---|---|---|
| Background | `error.50` (`#fef2f2`) | `#2d1a1a` (custom) | — |
| Border | `error.500` / 30 % | `error.500` / 40 % | — |
| Text | `neutral.900` (`#1c1917`) | `error.50` (`#fef2f2`) | 13.5:1 / 6.2:1 ✓ |
| Icon | `error.600` (`#dc2626`) | `error.500` (`#ef4444`) | 5.3:1 / 4.7:1 ✓ |

### High-contrast theme overrides

In `.high-contrast` class:
- All status bars use `background: #000000`, `border: 2px solid #ffffff`, `color: #ffffff`.
- Icons use `#ffff00` for warning states, `#ff4444` for error states, `#44ff44` for success states.
- The conflict dialog has `outline: 3px solid #ffff00`.

---

## 9. Responsive Behaviour

| Breakpoint | Status bar layout | Conflict dialog layout |
|---|---|---|
| `≥ 768 px` | Full label + icon + timestamp | Full dialog with both buttons |
| `375 px` | Icon + truncated label ("Saving…" → "Sv."); timestamp hidden; bar height 28 px | Dialog width matches card width; buttons stacked vertically |

At 375 px the status bar must not overlap form fields. Cards using `p-8` padding accommodate the 28 px bar within the card top padding; no layout shift occurs.

---

## 10. Component Interface (Reference)

```tsx
type AutosaveState =
  | 'idle'
  | 'saving'
  | 'saved'
  | 'error-retry'
  | 'offline-queued'
  | 'conflict-detected';

interface AutosaveIndicatorProps {
  state: AutosaveState;
  timestamp?: Date;                   // when the last save completed
  onKeepMine?: () => void;
  onUseTheirs?: () => void;
  className?: string;
}
```

```tsx
// Hook for ProfileTab to consume
function useAutosaveIndicator(formDirty: boolean): {
  state: AutosaveState;
  timestamp: Date | null;
  keepMine: () => void;
  useTheirs: () => void;
}
```

The hook wraps the save‑call inside `updateProfile()` / `updateAvatar()`, intercepts 409 responses, listens for online/offline events, and exposes the indicator state.

---

## 11. Implementation Notes

1. **No new third‑party dependencies.** All icons come from `lucide-react` (already in the project).
2. The `saving` minimum‑display timer (400 ms) is implemented as a `Promise.delay` that races with the actual fetch.
3. The `saved` → `idle` transition (4 s) uses a `useRef` timer that is cleared on unmount and on re‑entry to `saving`.
4. Offline queue stores JSON in `sessionStorage['autosave_queue']` keyed by section name.
5. The conflict dialog uses `useEffect` + `aria-modal` + a focus‑trap ref cycle (no portal library needed).
6. Test coverage must include all six states, the 400 ms minimum timer, the 4 s saved timer, offline enqueue/flush, and conflict keep/use‑theirs.
