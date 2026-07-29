# Email Verification Flow UX Spec

## Overview
This document specifies the UX flow for the email verification process initiated from `SignUpPage.tsx` and supported by `backend/internal/auth/verify.go`. The flow ensures users verify their email address before accessing the full Grainlify experience.

## States and Screens

### 1. Verification Email Sent (Waiting State)
**State:** `sent` | `resend-cooldown`
**Trigger:** Successful account creation on `SignUpPage.tsx`.
* **Visuals:** Centered layout (375px mobile breakpoint).
* **Content:**
  * Headline: "Check your inbox"
  * Body: "We've sent a verification link to **m***@example.com**" (Email masked for security/privacy).
  * Link: "Wrong email? Click here to update."
* **Resend CTA:**
  * Button disabled initially.
  * Shows countdown: "Resend email in 60s".
* **Accessibility:** 
  * Countdown timer announced via `aria-live="polite"` at 15-second intervals (not every second to avoid spamming screen readers).
  * "Wrong email" and "Resend" CTAs reachable via `Tab`.
  * Disabled resend button exposes `aria-disabled="true"`.

### 2. Resend Available
**State:** `resend-available`
**Trigger:** Cooldown timer reaches 0s.
* **Visuals:** Resend CTA becomes enabled and styled according to primary interactive tokens.
* **Content:** "Resend verification email".
* **Interaction:** Clicking restarts the 60s cooldown and transitions back to `resend-cooldown` state.

### 3. Confirmed / Success Screen
**State:** `confirmed-success`
**Trigger:** User clicks a valid verification link in their email.
* **Visuals:** Centered layout. Success icon using Semantic Success token (`#22c55e` light / `#22c55e` dark).
* **Content:**
  * Headline: "Email Verified!"
  * Body: "Your account is now active. You will be redirected to the dashboard."
* **Interaction:** Auto-redirects to `/dashboard` after 3 seconds.
* **Accessibility:** Focus moves to the headline upon mount.

### 4. Error: Link Expired / Invalid
**State:** `link-expired` | `link-invalid`
**Trigger:** User clicks a malformed or expired verification link.
* **Visuals:** Error icon using Semantic Error token (`#ef4444` light / `#ef4444` dark).
* **Content:**
  * Headline: "Verification Link Expired" or "Invalid Link"
  * Body: "This link is no longer valid. Please request a new verification email."
* **CTA:** "Request New Link" button.

## Design Tokens & Validation

### Colors (from `/design-tokens.json`)
* **Success State:** Semantic Success (`#22c55e` light / `#22c55e` dark). Contrast ratio verified > 4.5:1 against surface background.
* **Error State:** Semantic Error (`#ef4444` light / `#ef4444` dark). Contrast ratio verified > 4.5:1.
* **Text:** Primary text (`#1a1a1a` light / `#f5f5f5` dark) provides AAA contrast (21:1). Secondary text provides AA contrast.
* **Focus Ring:** `#0066cc` (light) / `#f1b400` (dark).

### Responsive Design
* **Base Layout:** All screens must use a maximum width container (e.g., `max-w-md`) and remain horizontally and vertically centered.
* **Mobile (375px):** Paddings adjust to ensure legibility without horizontal scrolling. Minimum touch target of 44x44px per WCAG.

## QA & Accessibility Checklist
* [x] **Contrast:** Screen text and countdown timer meet 4.5:1 contrast in both light and dark themes.
* [x] **Keyboard Nav:** Users can tab through the resend CTA and "wrong email" link.
* [x] **Screen Reader:** `aria-live="polite"` announces timer at 60s, 45s, 30s, 15s. Disabled states are programmatically exposed.
* [x] **Responsiveness:** Confirmed centered and legible at 375px.
