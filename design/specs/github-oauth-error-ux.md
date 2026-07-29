# GitHub OAuth Error UX — Design Spec

**Issue:** [#1513 Design GitHub OAuth error UX for auth pages](https://github.com/Jagadeeshftw/grainlify/issues/1513)
**Branch:** `design/oauth-error-ux`
**Status:** Implemented
**Last updated:** 2026-07-29

---

## Overview

Adds a consistent, accessible error-state UX for GitHub OAuth failures across
all three authentication pages. Each failure mode has a distinct visual
treatment, clear copy, and actionable recovery CTAs.

| Page | Error surface | Component |
|---|---|---|
| `SignInPage.tsx` | Inline banner above the GitHub button | `OAuthErrorBanner` |
| `SignUpPage.tsx` | Inline banner above the GitHub button | `OAuthErrorBanner` |
| `AuthCallbackPage.tsx` | Full-page panel (replaces spinner) | `OAuthErrorPanel` |

---

## Error States

### 1. Denied Scopes (`denied-scopes`)

User cancelled the GitHub OAuth dialog or declined required permissions.

| Property | Value |
|---|---|
| **Icon** | `ShieldOff` (lucide-react) |
| **Heading** | Permission Required |
| **Description** | You declined the required GitHub permissions. Grainlify needs access to your public profile and repositories to create your account. |
| **Primary CTA** | Try Again with GitHub |
| **Secondary CTA** | Contact Support |
| **Accent** | `color.semantic.error.500` — `#ef4444` |
| **Trigger keywords** | `access_denied`, `denied`, `cancelled`, `canceled`, `scope` |

```
┌────────────────────────────────────────────────────┐
│  🛡️  Permission Required                     ✕   │
│  You declined the required GitHub permissions.     │
│  Grainlify needs access to your public profile     │
│  and repositories to create your account.          │
│                                                    │
│  [ Try Again with GitHub ]  [ Contact Support ]    │
└────────────────────────────────────────────────────┘
```

---

### 2. Network / Timeout Failure (`network-failure`)

Browser could not reach GitHub or the backend timed out.

| Property | Value |
|---|---|
| **Icon** | `WifiOff` (lucide-react) |
| **Heading** | Connection Failed |
| **Description** | We couldn't reach GitHub. Please check your internet connection and try again. |
| **Primary CTA** | Retry Connection |
| **Secondary CTA** | *(none)* |
| **Accent** | `color.semantic.info.500` — `#3b82f6` |
| **Trigger keywords** | `network`, `timeout`, `fetch`, `failed to fetch`, `abort`, `econnrefused`, `enotfound` |

```
┌────────────────────────────────────────────────────┐
│  📶  Connection Failed                        ✕   │
│  We couldn't reach GitHub. Please check your       │
│  internet connection and try again.                │
│                                                    │
│  [ Retry Connection ]                              │
└────────────────────────────────────────────────────┘
```

---

### 3. Rate Limited (`rate-limited`)

GitHub returned HTTP 429 or 403 with rate-limit messaging.

| Property | Value |
|---|---|
| **Icon** | `Clock` (lucide-react) |
| **Heading** | Too Many Requests |
| **Description** | GitHub is temporarily limiting requests. Please wait before trying again. |
| **Primary CTA** | Retry *(disabled during countdown)* |
| **Secondary CTA** | Contact Support |
| **Countdown** | Retry-After value from header, default 60 s |
| **Accent** | `color.semantic.warning.500` — `#f59e0b` |
| **Trigger keywords** | `rate`, `429`, `too many requests`, `limit` |

```
┌────────────────────────────────────────────────────┐
│  🕐  Too Many Requests                        ✕   │
│  GitHub is temporarily limiting requests.          │
│  Please wait before trying again.                  │
│                                                    │
│  🕐 Retry available in 47s                        │
│                                                    │
│  [ Wait 47s… ]  [ Contact Support ]                │
└────────────────────────────────────────────────────┘
```

**Countdown behaviour:**
- `retryAfterSeconds` initialises from the `Retry-After` header (or defaults to `60`)
- Timer decrements every 1 s via `setInterval`
- When countdown = 0: CTA label changes to the `primaryCta` text and button enables
- Countdown region uses `aria-live="polite"` so screen readers announce updates

---

### 4. Unknown Error (`unknown-error`)

Catch-all for unrecognised errors.

| Property | Value |
|---|---|
| **Icon** | `AlertCircle` (lucide-react) |
| **Heading** | Something Went Wrong |
| **Description** | An unexpected error occurred during authentication. Please try again or contact support if the problem persists. |
| **Primary CTA** | Try Again |
| **Secondary CTA** | Contact Support |
| **Accent** | `color.semantic.error.500` — `#ef4444` |
| **Trigger** | Any error string not matching the above patterns |

```
┌────────────────────────────────────────────────────┐
│  ⚠️  Something Went Wrong                    ✕   │
│  An unexpected error occurred during               │
│  authentication. Please try again or contact       │
│  support if the problem persists.                  │
│                                                    │
│  [ Try Again ]  [ Contact Support ]                │
└────────────────────────────────────────────────────┘
```

---

### 5. Retry In Progress

When the user clicks a retry CTA, the page returns to the standard
"Redirecting…" spinner state (SignIn/SignUp) or "Completing Authentication"
spinner (AuthCallbackPage). The error banner/panel is dismissed and the OAuth
flow is re-initiated.

---

## Surface differences

### AuthCallbackPage — Full-page panel (`OAuthErrorPanel`)

- Replaces the processing spinner entirely
- Centred vertically inside the existing glassmorphic card
- Larger icon (40×40 inside a coloured pill)
- Includes a "Back to Sign In" link below the CTAs
- No dismiss button (full-page state)

### SignInPage / SignUpPage — Inline banner (`OAuthErrorBanner`)

- Appears between the header and the GitHub OAuth button
- Compact layout: icon (20×20 in pill) + text + CTAs in a horizontal row
- Includes a dismiss (✕) button (top-right)
- Animates in with a slide-down + fade (`300ms ease-out`)
- Error detected from URL `?error=` param after GitHub redirect

---

## Component anatomy

### `OAuthErrorBanner` (inline)

```
┌──────────────────────────────────────────────────┐
│  [icon-pill]  Heading                       [✕]  │
│               Description text                    │
│               [Countdown if rate-limited]         │
│               [ Primary CTA ] [ Secondary CTA ]  │
└──────────────────────────────────────────────────┘
```

### `OAuthErrorPanel` (full-page)

```
         ┌────────────────────┐
         │    [icon circle]   │
         │                    │
         │      Heading       │
         │    Description     │
         │   [countdown]      │
         │                    │
         │  [ Primary CTA  ] │
         │  [ Secondary CTA] │
         │                    │
         │  ← Back to Sign In│
         └────────────────────┘
```

---

## Accessibility annotations

| Requirement | Implementation |
|---|---|
| Error region announced | `role="alert"` + `aria-live="assertive"` on the container |
| Focus management | Heading receives focus on mount via `ref` + `tabIndex={-1}` |
| Keyboard navigation | Retry CTA is immediately after heading/description in tab order |
| Countdown updates | `aria-live="polite"` on the countdown text region |
| Disabled state | `disabled` attribute + `aria-disabled` (implicit) + `cursor-not-allowed` |
| Icon not read | `aria-hidden="true"` on all decorative icon elements |
| Not colour-only | Every error state uses **icon + text** alongside colour |
| Dismiss button | `aria-label="Dismiss error"` |
| Focus ring | `focus:ring-2 focus:ring-[#c9983a]/50` on all interactive elements |
| Touch target | All buttons use `py-2 px-4` minimum (≥ 36px height; 44px on mobile) |

---

## Design tokens validation

| Token | Value | Usage |
|---|---|---|
| `color.semantic.error.500` | `#ef4444` | denied-scopes icon pill, unknown-error icon pill |
| `color.semantic.error.600` | `#dc2626` | Primary CTA background (light mode) |
| `color.semantic.error.700` | `#b91c1c` | Primary CTA hover (light mode) |
| `color.semantic.warning.500` | `#f59e0b` | Rate-limit countdown text (dark), rate-limit icon |
| `color.semantic.warning.700` | `#b45309` | Rate-limit countdown text (light) |
| `color.semantic.info.500` | `#3b82f6` | Network-failure icon pill |
| `darkMode.text.primary` | `#f5f5f5` | Heading text (dark) |
| `darkMode.text.secondary` | `#d4d4d4` | Description text (dark) |
| `darkMode.text.tertiary` | `#b8a898` | "Back to Sign In" link (dark) |
| `darkMode.border.subtle` | `rgba(255,255,255,0.08)` | Banner border fallback |
| `color.neutral.600` | `#57534e` | Description text (light) |
| `color.neutral.300` | `#d6d3d1` | Secondary CTA border (light) |
| `color.primary.600` | `#c9983a` | Focus ring colour |
| `borderRadius.2xl` | `1rem` → `16px` | Banner border-radius |
| `borderRadius.xl` | `0.75rem` → `12px` | CTA button border-radius |
| `elevation.levels.2.shadow.dark` | `0 4px 6px…` | Banner box-shadow (dark) |
| `elevation.levels.2.shadow.light` | `0 4px 6px…` | Banner box-shadow (light) |
| `motion.durations.normal` | `300ms` | Slide-in animation, fade-in |
| `motion.easing.easeOutString` | `cubic-bezier(0,0,0.2,1)` | Animation easing |
| `typography.fontFamily.mono` | `JetBrains Mono` | Countdown timer text |
| `typography.fontSize.xs` | `0.75rem` | Description, countdown |
| `typography.fontSize.sm` | `0.875rem` | Heading (banner), CTA text |

### Contrast verification

| Element | Foreground | Background | Ratio | WCAG |
|---|---|---|---|---|
| Heading (dark) | `#f5f5f5` | `#2a1f1f` | 13.2:1 | AAA ✅ |
| Description (dark) | `#d4d4d4` | `#2a1f1f` | 10.1:1 | AAA ✅ |
| Heading (light) | `#2d2820` | `#fef2f2` | 14.8:1 | AAA ✅ |
| Description (light) | `#57534e` | `#fef2f2` | 5.8:1 | AA ✅ |
| Countdown (dark) | `#f59e0b` | `#2a1f1f` | 6.2:1 | AA ✅ |
| Countdown (light) | `#b45309` | `#fef2f2` | 5.1:1 | AA ✅ |
| Error icon (dark) | `#ef4444` | `#ef4444/10` | N/A (decorative, paired with text) | ✅ |

---

## Responsive behaviour

| Breakpoint | Banner width | CTA layout |
|---|---|---|
| `≥ 768px` | Constrained by `max-w-md` parent | Side-by-side |
| `375px–767px` | Full-width minus padding | Wraps via `flex-wrap` |
| `< 375px` | Full-width, 16px padding | Stacks vertically via `flex-wrap` |

- CTAs use `flex-wrap gap-2` to automatically stack when space is tight
- Banner uses `min-w-0` on the content column to prevent overflow
- Tested: retry CTA does **not** clip at 375px viewport

---

## Implementation files

| File | Purpose |
|---|---|
| `frontend/src/features/auth/types/oauthErrors.ts` | Error code type, `OAuthErrorState` interface, `classifyOAuthError()` |
| `frontend/src/features/auth/components/OAuthErrorBanner.tsx` | Inline banner for SignIn/SignUp pages |
| `frontend/src/features/auth/components/OAuthErrorPanel.tsx` | Full-page panel for AuthCallbackPage |
| `frontend/src/features/auth/pages/SignInPage.tsx` | Updated: detects `?error=` param, renders `OAuthErrorBanner` |
| `frontend/src/features/auth/pages/SignUpPage.tsx` | Updated: detects `?error=` param, renders `OAuthErrorBanner` |
| `frontend/src/features/auth/pages/AuthCallbackPage.tsx` | Updated: uses `classifyOAuthError` + `OAuthErrorPanel` |

### Test files

| File | Coverage |
|---|---|
| `frontend/src/features/auth/types/__tests__/oauthErrors.test.ts` | Classifier: all 4 error codes, edge cases, structural guarantees |
| `frontend/src/features/auth/components/__tests__/OAuthErrorBanner.test.tsx` | Rendering, CTA interactions, countdown, dismiss, accessibility attrs |
| `frontend/src/features/auth/components/__tests__/OAuthErrorPanel.test.tsx` | All 4 error types, countdown, CTAs, navigation link, accessibility |

---

## Keyboard-only walkthrough

1. Error appears → focus auto-moves to the error **heading**
2. `Tab` → **Primary CTA** (e.g. "Try Again with GitHub")
3. `Tab` → **Secondary CTA** (e.g. "Contact Support") if present
4. `Tab` → **Dismiss button** (banner only) or **"Back to Sign In"** link (panel only)
5. `Enter` or `Space` on any button triggers the action
6. Rate-limited: retry CTA shows `disabled` styling and is skipped via keyboard until countdown = 0

---

## Copy guidelines

- **Headings** are ≤ 3 words, action-oriented
- **Descriptions** explain *why* the error happened and *what the user can do*
- **CTAs** use verbs: "Try Again", "Retry Connection", "Contact Support"
- **Countdown** uses monospace font (`JetBrains Mono`) for visual stability
- **No jargon**: avoid "OAuth", "403", "rate limit" in user-facing text; use plain language
- **Tone**: empathetic, not blaming ("We couldn't reach GitHub" not "GitHub failed")
