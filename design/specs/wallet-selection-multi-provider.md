# Wallet Selection Multi-Provider Enhancements — Design Spec

**Issue:** [#1393 Design wallet-selection enhancements for multi-provider with provider-status detection](https://github.com/AnnieIj/grainlify/issues/1393)  
**Branch:** `design/wallet-selection-multi-provider-enhancements`  
**Status:** Implemented  
**Last updated:** 2026-06-25

---

## Overview

The current SignInPage only supports GitHub OAuth. This spec introduces a new **Connect Stellar Wallet** flow with multi-provider support (Freighter, Albedo, WalletConnect, Hana) and real-time availability detection. The new WalletConnectionModal is also reusable across LandingPage and ProjectDetailPage for wallet-based interactions.

---

## Screens in scope

| Screen | Breakpoints |
|---|---|
| WalletConnectionModal — provider grid | 1440px, 375px |
| WalletConnect QR pane (expanded) | 1440px, 375px |

---

## 1. WalletConnectionModal — provider grid

### Location
`frontend/src/features/auth/components/WalletConnectionModal.tsx`  
Triggered from SignInPage via **"Connect Stellar Wallet"** button.

### 1440px layout

```
╔══════════════════════════════════════════════════╗
║  Connect Wallet                               ✕  ║
╠══════════════════════════════════════════════════╣
║  Select a wallet to connect to Grainlify on      ║
║  the Stellar network.                            ║
║                                                  ║
║  ┌───────────────┐  ┌───────────────┐           ║
║  │  Freighter    │  │  Albedo       │           ║
║  │  ● Installed  │  │  ● Not inst.  │           ║
║  │  Official...  │  │  Web-based... │           ║
║  └───────────────┘  └───────────────┘           ║
║   [Install Albedo]                               ║
║                                                  ║
║  ┌───────────────┐  ┌───────────────┐           ║
║  │  WalletConnect│  │  Hana         │           ║
║  │  ● Installed  │  │  ● Degraded   │           ║
║  │  Scan QR...   │  │  Multi-chain  │           ║
║  └───────────────┘  └───────────────┘           ║
║                                                  ║
║  ⓘ Only connect wallets you own. Grainlify...   ║
╚══════════════════════════════════════════════════╝
```

### 375px layout

- Modal slides up from bottom on mobile (full-width, `rounded-t-[28px]`)
- Provider grid remains 2-column but stacks more vertically
- QR pane (when open) renders below the grid within the scrollable modal

### Component structure

```
WalletConnectionModal
├── Header (title + close button)
├── Description text
├── Provider grid (2x2)
│   ├── ProviderButton × 4 (Freighter, Albedo, WalletConnect, Hana)
│   │   ├── Provider name + status badge
│   │   ├── Short description
│   │   └── (if not-installed) Install link
│   └── (if WalletConnect selected) WalletQRPane
└── Footer notice (security reminder)
```

---

## 2. Provider availability detection

Runs on modal mount by inspecting `window` object for injected provider APIs:

| Provider | Detection logic | Status mapping |
|---|---|---|
| Freighter | `window.freighter` exists | installed / not-installed |
| Albedo | `window.albedo` exists | installed / not-installed |
| WalletConnect | Always available (connection via QR) | installed (always) |
| Hana | `window.hana` exists | installed / degraded / not-installed |

### Status indicators

| Status | Dot color | Label | Icon | Text class |
|---|---|---|---|---|
| `installed` | `bg-green-400` | Installed | `CheckCircle2` | `text-green-400` |
| `degraded` | `bg-orange-400` | Degraded | `AlertTriangle` | `text-orange-400` |
| `not-installed` | `bg-red-400` | Not installed | `WifiOff` | `text-red-400` |

---

## 3. Install prompt flow

When user clicks a `not-installed` provider:
- Button is inert (no connect action)
- Install link renders below the provider card with `ExternalLink` icon
- Opens in new tab → deep link to extension/app install page

Install URLs:
- **Freighter:** https://www.freighter.app
- **Albedo:** https://albedo.link
- **WalletConnect:** https://walletconnect.com/explorer
- **Hana:** https://hanawallet.io

---

## 4. WalletConnect QR pane

### Location
`frontend/src/features/auth/components/WalletQRPane.tsx`  
Rendered inside WalletConnectionModal when WalletConnect card is selected.

### Features

- **QR code generation** — Uses `qrcode` package (browser canvas) to render a 220×220 QR with light/dark theme support
- **Countdown timer** — 120s default (2 min); displays `MM:SS` format
- **Expired overlay** — When countdown hits zero, renders overlay on QR with "Refresh" button
- **Copy URI fallback** — Text input with truncated URI + copy icon; announces "URI copied" via `aria-live`
- **Refresh action** — `onRefresh` callback re-generates URI (in production this calls WalletConnect SDK)

### 1440px layout

```
┌──────────────────────────────────────────────┐
│        [QR code 220×220]                     │
│                                              │
│   Expires in 1:45                            │
│   Open your mobile wallet app and scan...   │
│                                              │
│   Or copy connection URI                    │
│   ┌───────────────────────────────────┐     │
│   │ wc:00e46b69-d0cc...  [📋 Copy]   │     │
│   └───────────────────────────────────┘     │
└──────────────────────────────────────────────┘
```

### QR expiry states

| Countdown | QR visible | Overlay | Action |
|---|---|---|---|
| `> 0s` | Yes | None | Normal |
| `0s` | Yes | Black overlay + Refresh button | Refresh to get new URI |
| `> 0s` after refresh | Yes | None | Timer resets |

---

## 5. Accessibility

### WalletConnectionModal

- `role="dialog"`, `aria-modal="true"`, `aria-labelledby="wallet-modal-title"`
- Focus moves to Close button on open; returns to trigger button on close
- Focus trap: `Tab`/`Shift+Tab` cycles within modal; `Escape` closes
- Body scroll locked (`body.overflow = hidden`) while modal is open
- Backdrop click closes modal

### WalletQRPane

- QR image has descriptive `aria-label`: _"WalletConnect QR code — scan with your mobile wallet"_
- Countdown timer has `aria-live="polite"` so screen readers announce expiry
- Copy button announces state change: _"URI copied to clipboard"_ via `aria-live` + `.sr-only` span

### Provider cards

- `aria-label` includes provider name + status: _"Freighter — Installed"_
- `aria-pressed` on WalletConnect card when QR pane is open
- Install links have clear accessible names with `ExternalLink` icon

---

## 6. Design tokens

| Token | Usage |
|---|---|
| `color.primary.600` (`#c9983a`) | Wallet icon, accent highlights |
| `color.primary.700` (`#a2792c`) | Active selection underline, gradient button |
| `color.semantic.success.400` | Installed status dot/text |
| `color.semantic.warning.400` | Degraded status dot/text |
| `color.semantic.error.400` | Not-installed status dot/text |
| `modal.zIndexBase` | `10000` |
| `modal.borderRadius` | `28px` (modal), `16px` (provider cards) |

---

## 7. Integration points

### SignInPage

- New button: **"Connect Stellar Wallet"** below GitHub sign-in
- Opens `WalletConnectionModal` on click
- `onConnect(providerId)` callback → integrate Stellar wallet SDK per provider (Freighter, Albedo, etc.)

### LandingPage (future)

- Add same wallet button in hero CTA section

### ProjectDetailPage (future)

- Add wallet-connection trigger in "Contribute" flow

---

## 8. Security notes

- **No private key exposure** — Modal never requests seed phrase or private key
- **Install links validated** — All deep links point to official provider domains
- **WalletConnect URI scoping** — URIs are session-scoped and expire after 2 minutes
- **Provider detection sandboxed** — Detection wrapped in try-catch to prevent injection errors from blocking modal render

---

## 9. QA checklist

### Modal interactions
- [ ] Clicking "Connect Stellar Wallet" opens modal
- [ ] `Escape` and backdrop click both close modal
- [ ] Focus moves into modal on open and returns to trigger on close
- [ ] `Tab` / `Shift+Tab` stay within modal (focus trap)
- [ ] Body cannot scroll while modal is open

### Provider grid
- [ ] Installed providers show green dot + "Installed"
- [ ] Not-installed providers show red dot + "Not installed" + install link
- [ ] Degraded providers show orange dot + "Degraded"
- [ ] Clicking installed non-QR provider calls `onConnect(id)`
- [ ] Clicking WalletConnect toggles QR pane

### QR pane (WalletConnect)
- [ ] QR code renders correctly in light and dark themes
- [ ] Countdown decrements from 2:00 to 0:00
- [ ] Expired QR shows overlay with "Refresh" button
- [ ] Refresh button generates new URI and resets timer
- [ ] Copy button copies URI to clipboard and announces success

### Accessibility
- [ ] Color contrast meets WCAG 2.1 AA (4.5:1 text, 3:1 UI)
- [ ] QR image has descriptive `aria-label`
- [ ] Countdown timer announces expiry via `aria-live`
- [ ] Install links have accessible names

### Responsive (375px)
- [ ] Modal slides up from bottom, full-width
- [ ] Provider grid remains 2-column
- [ ] QR pane fits within scrollable modal

---

## 10. Implementation files

| File | Purpose |
|---|---|
| `frontend/src/features/auth/types/index.ts` | `WalletProvider`, `WalletProviderStatus`, `WalletProviderId` types |
| `frontend/src/features/auth/components/WalletConnectionModal.tsx` | Main modal: provider grid, status detection, install prompts |
| `frontend/src/features/auth/components/WalletQRPane.tsx` | QR code, countdown timer, copy-URI fallback |
| `frontend/src/features/auth/pages/SignInPage.tsx` | Connect Stellar Wallet button + modal state |
| `design/specs/wallet-selection-multi-provider.md` | This spec |

---

## 11. Future enhancements

- Persist last-used provider in `localStorage` and pre-select on next open
- Add WalletConnect event listeners to auto-close modal when pairing completes
- Extend to ProjectDetailPage and LandingPage
- Add analytics event tracking for provider selection distribution
