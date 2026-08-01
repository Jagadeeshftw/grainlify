# Wallet Balance & Gas Fee Disclosure — Design Spec

**Issue:** [#1514 Design wallet balance display and estimated gas fee disclosure component](https://github.com/Jagadeeshftw/grainlify/issues/1514)
**Branch:** `design/wallet-balance-gas-disclosure`
**Status:** Implemented
**Last updated:** 2026-07-27

---

## Overview

Adds a **WalletBalanceFeeDisplay** component inside `WalletConnectionModal.tsx` that shows the connected wallet's native token balance, its USD equivalent, and the estimated transaction fee with an informational tooltip. The component handles five visual states: default, insufficient-balance, loading, stale, and fee-unavailable.

---

## Component anatomy

```
┌─────────────────────────────────────────────────────┐
│  [Stellar logo]  1,245.80 XLM                       │
│                   ≈ $312.45 USD                     │
│                                                     │
│  Est. network fee  ℹ     0.00001 XLM  (≈ $0.0000025)      │
└─────────────────────────────────────────────────────┘
```

### Sub-elements

| Slot | Content | Token / class |
|---|---|---|
| Token icon | Stellar (XLM) logo, 24×24 | `rounded-full`, `border border-white/10` |
| Balance amount | Formatted number + ticker | `text-[16px] font-semibold` |
| USD equivalent | "≈ $X.XX USD" | `text-[13px] text-[#b8a898]` (dark) / `text-[#7a6b5a]` (light) |
| Fee label | "Est. network fee" + info icon | `text-[13px]` + `Info` icon 14×14 |
| Fee amount | Formatted fee + USD | `text-[13px] font-mono` |

---

## States

### 1. Default (balance loaded, fee available)

- Token icon, balance, USD equivalent, and fee line all visible
- Tooltip on info icon explains: _"Estimated Stellar network fee for signing transactions, including payouts…"_ (see `walletFeeDisclosureCopy.ts`)

### 2. Insufficient balance

- Balance text turns `text-[#ef4444]` (dark) / `text-[#dc2626]` (light) — maps to `color.semantic.error.600`
- Warning icon (`AlertTriangle`) appears inline after balance amount
- Alert banner (`role="alert"`) explains insufficient XLM for typical network fees
- Fee line remains visible but fee row uses reduced opacity (`opacity-60`)
- `aria-invalid="true"` on the balance region

### 3. Loading

- Skeleton placeholders replace all text lines (three `SkeletonLoader` blocks)
- `aria-busy="true"` on the container
- Skeleton heights: 16px (balance), 12px (USD), 12px (fee)

### 4. Stale data

- A subtle amber banner appears: _"Balance may be outdated. Pull to refresh."_
- Banner uses `bg-[#f59e0b]/[0.08] text-[#f59e0b]` (semantic.warning)
- Balance and fee values remain visible but prefixed with a `Clock` icon (12×12)
- When `lastUpdated` is provided, the clock exposes `aria-label` / `title` with _"Last updated Xm ago"_

### 5. Fee unavailable

- Fee line shows: _"Fee unavailable"_ in muted italic text
- `AlertCircle` icon with `aria-label` describing that the wallet will show the exact fee at confirmation
- Balance section unaffected

### 6. Partial quote data (edge)

| Condition | UI behavior |
|---|---|
| `usdEquivalent === null` | Balance row shows _"USD equivalent unavailable"_ |
| `estimatedFee` set, `feeUsdEquivalent === null` | Native fee + ticker only; no `(≈ $…)` suffix |
| `feeUsdEquivalent < 0.01` | USD suffix renders as _"< $0.01"_ |
| `balance` with thousands separators | Insufficient check strips commas before numeric compare |

---

## Accessibility annotations

| Requirement | Implementation |
|---|---|
| Live region for balance updates | `aria-live="polite"` on balance wrapper `div` |
| Busy state during fetch | `aria-busy="true"` on container while loading |
| Keyboard-navigable tooltip | Info icon is a `<button>` with `aria-describedby` pointing to tooltip id; tooltip uses `role="tooltip"` |
| Tooltip open/close | `Enter` / `Space` toggles; `Escape` closes; focus returns to trigger |
| Insufficient balance | `aria-invalid="true"` on balance region; `role="alert"` on warning banner |
| Screen reader announcements | "Balance: 1245.80 XLM, approximately 312.45 US dollars" |

---

## Design tokens validation

| Token | Value | Usage in component |
|---|---|---|
| `color.primary.600` | `#c9983a` | Accent highlights, focus ring |
| `color.semantic.error.600` | `#dc2626` | Insufficient balance text (light) |
| `color.semantic.warning.500` | `#f59e0b` | Stale banner text, stale icon |
| `darkMode.text.primary` | `#f5f5f5` | Balance amount text (dark) |
| `darkMode.text.tertiary` | `#b8a898` | USD equivalent, muted text (dark) |
| `darkMode.border.subtle` | `rgba(255,255,255,0.08)` | Container border |
| `darkMode.background.glassMedium` | `rgba(255,255,255,0.08)` | Skeleton loader background |
| `typography.fontFamily.mono` | `JetBrains Mono` | Fee amount (monospace alignment) |
| `borderRadius.2xl` | `1rem` | Container border-radius |
| `elevation.levels.1.shadow.dark` | `0 1px 3px rgba(0,0,0,0.2)` | Container shadow |
| `animation.duration.fast` | `150ms` | Tooltip fade-in |
| `motion.easing.easeOutString` | `cubic-bezier(0, 0, 0.2, 1)` | Tooltip transition |

---

## Integration — WalletConnectionModal.tsx

The `WalletBalanceFeeDisplay` renders **above the provider grid**, inside the existing modal, only when a wallet is connected (`balance` prop is non-null). When no wallet is connected the component is not rendered.

### Component structure (within modal)

```
WalletConnectionModal
├── Header (title + close button)
├── Description text
├── WalletBalanceFeeDisplay  ← NEW
├── Provider grid (2×2)
│   ├── ...
├── QR pane (optional)
└── Footer notice
```

### Props interface

```typescript
interface WalletBalanceFeeDisplayProps {
  balance: string | null;
  ticker?: string;
  usdEquivalent: number | null;
  estimatedFee: string | null;
  feeUsdEquivalent: number | null;
  isLoading: boolean;
  isStale: boolean;
  lastUpdated?: Date | null;
}
```

---

## Implementation files

| File | Purpose |
|---|---|
| `frontend/src/features/auth/components/WalletBalanceFeeDisplay.tsx` | New component |
| `frontend/src/features/auth/components/walletFeeDisclosureCopy.ts` | Centralized disclosure strings + format helpers |
| `frontend/src/features/auth/components/__tests__/WalletBalanceFeeDisplay.test.tsx` | Component regression tests |
| `frontend/src/features/auth/components/__tests__/walletFeeDisclosureCopy.test.ts` | Copy + formatter unit tests |
| `frontend/src/features/auth/components/WalletConnectionModal.tsx` | Integration point |
| `design/specs/wallet-balance-gas-fee-disclosure.md` | This spec |

---

## QA checklist

### Visual states
- [ ] Default: balance, USD, fee all render correctly in dark and light themes
- [ ] Insufficient balance: red text + warning icon, fee dimmed
- [ ] Loading: three skeleton blocks with shimmer
- [ ] Stale: amber banner + clock icon, values still visible
- [ ] Fee unavailable: muted text, alert-circle icon, balance unaffected

### Accessibility
- [ ] `aria-live="polite"` announces balance changes
- [ ] `aria-busy="true"` set during loading
- [ ] Info tooltip keyboard-navigable (Tab → Enter/Space → Escape)
- [ ] Tooltip has `role="tooltip"` and is `aria-describedby` by trigger
- [ ] Insufficient balance sets `aria-invalid="true"`
- [ ] Screen reader reads full balance and fee in natural language

### Responsive
- [ ] Component fits within 480px modal max-width
- [ ] Component fits at 375px (mobile) without overflow
- [ ] Fee line wraps gracefully on narrow screens
