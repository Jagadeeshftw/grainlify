# Session Timeout Warning — UX Specification

**Component:** `frontend/src/shared/components/SessionTimeoutBanner.tsx`  
**Context:** `frontend/src/shared/contexts/AuthContext.tsx`  
**Status:** Specification  
**Date:** 2026-07-26  
**Target Compliance:** WCAG 2.1 AA

---

## 1. Overview

When a user's JWT approaches expiry, a non-modal banner appears at the top of the viewport to warn them. The user can extend their session ("Stay signed in") or dismiss the banner without affecting their session. On expiry, the session ends, a forced-logout screen replaces the current view, and a "Sign back in" CTA preserves the user's last route so they return to exactly where they left off.

---

## 2. Banner States

The component transitions through four discrete states driven by `AuthContext`.

| State | Trigger | Visual treatment |
|---|---|---|
| `banner-hidden` | Default; token `> 5 min` from expiry | Banner not rendered |
| `warning-visible` | Token `≤ 5 min` and `> 1 min` from expiry | Amber/gold warning strip |
| `critical` | Token `≤ 1 min` from expiry | Red strip, stronger visual weight |
| `expired` | Token expiry reached | Full forced-logout screen replaces the viewport |

State is computed from two timers set when a token is stored. Both timers are cleared on logout or token refresh.

---

## 3. Banner Placement & Structure

- **Position:** fixed, top of viewport, full-width, `z-50`.
- **Stacking:** banner sits below any existing nav header (`z-40`) — implement as `z-50` on the banner layer which is rendered _before_ the nav in DOM order, so the nav header overlaps it at mobile widths. At `≥ 768 px` (`md:`) the banner does not obscure nav because nav is a sidebar or positioned below the banner strip.
- **375 px constraint:** at mobile width the banner height is capped at `48 px` (`py-2 px-4`), single-line layout. "Stay signed in" CTA collapses to an icon-only button with a visually hidden label to stay within the strip.
- **Non-modal:** focus is never stolen from the user's current task. The banner renders with `tabIndex={-1}` on its container; the CTA buttons are naturally focusable but not auto-focused on mount.
- **Dismissible without extending:** the ✕ close button dismisses the banner for the current warning window without calling token refresh. A separate timer can re-show the banner in the critical state if the user dismissed during the warning window.

---

## 4. Countdown Copy

Countdown is calculated in `AuthContext` and passed as `secondsRemaining` to the banner. Copy is derived from that value:

| State | Copy template | Example |
|---|---|---|
| `warning-visible` | `"Your session expires in {M}:{SS} — stay signed in to keep working."` | `"Your session expires in 4:32 — stay signed in to keep working."` |
| `warning-visible` (< 2 min) | `"Your session expires in {M}:{SS}."` | `"Your session expires in 1:45."` |
| `critical` | `"Session expiring in {SS} seconds."` | `"Session expiring in 42 seconds."` |
| `critical` (≤ 10 s) | `"Session expiring in {SS} seconds — save your work now."` | `"Session expiring in 8 seconds — save your work now."` |
| `expired` | *(shown on forced-logout screen, not banner)* | — |

Where `{M}` = whole minutes remaining, `{SS}` = zero-padded seconds within the current minute.

The countdown string is **not** injected into the `role="alert"` region on every tick. Instead:
- The alert region announces once on state entry (`warning-visible` → first render, `critical` → first render).
- A separate `aria-live="off"` element renders the live countdown for sighted users. Screen readers hear the initial announcement only.

---

## 5. "Stay Signed In" CTA Behaviour

1. User clicks "Stay signed in".
2. `AuthContext.refreshSession()` is called — it re-fetches `/me` with the current token (the back-end issues a fresh token or re-validates the existing one). If the API returns a new `Authorization` header, `setAuthToken()` is called with the new value; otherwise the existing token is re-affirmed and both countdown timers are reset.
3. The banner transitions to `banner-hidden`.
4. A `sonner` success micro-toast fires: `toast.success("You're still signed in.")` with a 3 000 ms duration.
5. If the refresh call fails (network error, 401), the banner transitions to `expired` immediately and the forced-logout screen renders.

---

## 6. Forced-Logout Screen

Rendered as a full-viewport overlay (`fixed inset-0 z-[100]`) that replaces the current view on the `expired` state. It is **not** a modal dialog — it covers the full screen so background content is inaccessible.

### Layout

```
┌─────────────────────────────────────┐
│  [Grainlify logo mark]              │
│                                     │
│  Your session has ended             │
│  For your security, you've been     │
│  signed out after a period of       │
│  inactivity.                        │
│                                     │
│  [Sign back in →]                   │
│                                     │
│  You'll be taken back to the page   │
│  you were on.                       │
└─────────────────────────────────────┘
```

### "Sign back in" CTA behaviour

- Clicking the CTA calls `window.location.pathname + window.location.search` to capture the last route, writes it to `sessionStorage['authReturnTo']`, then navigates to `/signin`. This mirrors the existing `ProtectedRoute` redirect pattern.
- After successful OAuth callback, `AuthCallbackPage` reads `sessionStorage['authReturnTo']` and redirects the user back to their last route (existing behaviour — no changes needed there).

### Accessibility

- The overlay renders `role="main"` with `aria-label="Session ended"`.
- On mount, a single `role="alert"` element announces `"Your session has ended. Please sign back in."` — announced once.
- "Sign back in" button has `aria-label="Sign back in and return to your previous page"`.

---

## 7. Accessibility Annotations

| Requirement | Implementation |
|---|---|
| `role="alert"` on banner | Applied to the announcement-only child element, not the entire banner container |
| Announced once per state transition | The `role="alert"` child is conditionally rendered/updated only on state change, not on every countdown tick |
| Focus not stolen | Banner container: `tabIndex={-1}`, no `autoFocus` on any child |
| Keyboard accessible | "Stay signed in" and ✕ close are `<button>` elements with `focus-visible` ring matching project tokens (`#a2792c` light / `#f1b400` dark) |
| Screen reader countdown | A separate visually-hidden `aria-live="polite" aria-atomic="true"` element announces at 5-min entry and 1-min entry only (via state change), not per-second |
| Reduced motion | Countdown opacity transitions use `data-opacity-transition` attribute; no slide or scale animations on the banner |
| High contrast theme | Banner uses opaque backgrounds (`#000` / `#fff`), solid `2 px` borders, no `backdrop-filter` |

---

## 8. Color & Token Escalation

All contrast ratios measured against their respective background values.

### Warning state (`warning-visible`)

| Element | Light token | Dark token | Contrast |
|---|---|---|---|
| Banner background | `#fffaeb` (amber-50 equiv) | `#3a2b0d` | — |
| Banner border | `#f59e0b` / `30%` opacity | `#f59e0b` / `50%` opacity | — |
| Text | `#2d2820` | `#e8dfd0` | 12.8:1 / 8.3:1 ✓ |
| Icon | `#b45309` | `#f59e0b` | 4.7:1 / 6.5:1 ✓ |
| CTA text | `#2d2820` (bold) | `#f1b400` (bold) | 12.8:1 / 9.2:1 ✓ |
| CTA border/bg | `#c9983a/30` bg | `#f1b400/20` bg | — |

These token values match the `.warning` variant in `Toast.tsx` for visual consistency.

### Critical state (`critical`)

| Element | Light token | Dark token | Contrast |
|---|---|---|---|
| Banner background | `#fef2f2` (red-50 equiv) | `#2d1a1a` | — |
| Banner border | `#ef4444` / `40%` opacity | `#ef4444` / `60%` opacity | — |
| Text | `#2d2820` | `#fca5a5` | 12.8:1 / 4.7:1 ✓ |
| Icon | `#dc2626` | `#f87171` | 5.3:1 / 4.6:1 ✓ |
| CTA text | `#dc2626` (bold) | `#fca5a5` (bold) | 5.3:1 / 4.7:1 ✓ |

These token values match the `.error` variant in `Toast.tsx` for visual consistency.

### High-contrast theme overrides

In the `.high-contrast` theme class, the banner must use:
- Background: `#000000` (warning) / `#1a0000` (critical)
- Border: `2 px solid #ffffff`
- Text: `#ffffff`
- CTA outline: `3 px solid #ffff00`

---

## 9. Responsive Behaviour

| Breakpoint | Layout | Notes |
|---|---|---|
| `< 768 px` (375 px target) | Single row: icon + truncated copy + icon-only CTA + ✕ | Banner height `48 px`, no text wrapping |
| `≥ 768 px` | Icon + full copy + full CTA label + ✕ | Banner height `52 px` |
| Banner + nav coexistence | Banner is `position: fixed; top: 0`. Nav that is `position: sticky; top: 0` will be pushed down automatically when banner is present. Components using `top-0` sticky positioning must add `top-[52px]` when banner is visible. | Achieved by binding a CSS custom property `--banner-height` on `:root` via `AuthContext` state |

At 375 px the banner must not overlap the primary navigation. The existing navigation in Grainlify is fixed to the top — when the banner is visible, the navigation is shifted down by the banner's height via the `--banner-height` CSS variable set on `:root`.

---

## 10. Component Interface (Reference)

```tsx
// State enum used by AuthContext and SessionTimeoutBanner
type SessionTimeoutState =
  | 'banner-hidden'
  | 'warning-visible'
  | 'critical'
  | 'expired';

// Values surfaced by AuthContext
interface SessionTimeoutContext {
  sessionTimeoutState: SessionTimeoutState;
  secondsRemaining: number;        // 0 when expired
  staySignedIn: () => Promise<void>;
  dismissTimeoutBanner: () => void;
}
```
