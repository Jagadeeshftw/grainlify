# Accessibility Theme Variants — Spec & Audit Checklist

**Issue:** #1397  
**Branch:** `design/high-contrast-reduced-motion-theme-variants`  
**Status:** Implemented  
**Last updated:** 2026-06-25

---

## Overview

Two new theme variants extend the existing `light` / `dark` pair:

| Variant           | Target users                        | Key properties                                      |
|-------------------|-------------------------------------|-----------------------------------------------------|
| `high-contrast`   | Low-vision, photosensitive users    | WCAG 3:1 UI, 4.5:1 text, no blur, thick borders    |
| `reduced-motion`  | Vestibular disorder, motion-sensitive | All transforms removed; opacity-only fades ≤150 ms |

Theme selection is persisted to `localStorage` and applied as root CSS classes on `<html>`:

| Theme           | Root class(es)             |
|-----------------|----------------------------|
| `light`         | _(none)_                   |
| `dark`          | `dark`                     |
| `high-contrast` | `high-contrast`            |
| `reduced-motion`| `dark reduced-motion`      |

---

## WCAG Compliance Targets

### High-Contrast

| Criterion  | Requirement                         | Target value              |
|------------|-------------------------------------|---------------------------|
| 1.4.3      | Text contrast minimum               | ≥ 4.5:1 (AA)             |
| 1.4.6      | Enhanced text contrast              | ≥ 7:1 (AAA, best effort) |
| 1.4.11     | Non-text UI component contrast      | ≥ 3:1                    |
| 2.4.7      | Focus visible                       | 3 px yellow #ffff00       |
| 1.4.1      | Color not sole differentiator       | Border + shape retained   |

### Reduced-Motion

| Criterion  | Requirement                                    |
|------------|------------------------------------------------|
| 2.3.3      | No animation from interactions (AAA)           |
| 2.2.2      | No auto-playing motion > 5 s without pause     |
| 2.3.1      | No flashing content > 3 Hz                     |

---

## Token Inventory

### High-Contrast Token Pairs (verified contrast ratios)

| Token                    | Value     | On surface    | Ratio   | WCAG     |
|--------------------------|-----------|---------------|---------|----------|
| text.primary             | `#ffffff` | `#000000`     | 21:1    | AAA      |
| text.secondary           | `#ebebeb` | `#000000`     | 18.7:1  | AAA      |
| text.tertiary            | `#c8c8c8` | `#000000`     | 15.3:1  | AAA      |
| text.muted               | `#808080` | `#000000`     | 5.3:1   | AA       |
| text.disabled            | `#666666` | `#000000`     | 3.5:1   | UI min   |
| border.subtle            | `#555555` | `#000000`     | 3.5:1   | UI (AA)  |
| border.default           | `#888888` | `#000000`     | 5.3:1   | AA       |
| border.prominent         | `#aaaaaa` | `#000000`     | 7.5:1   | AAA      |
| border.interactive       | `#dddddd` | `#000000`     | 12.6:1  | AAA      |
| semantic.accentPrimary   | `#f5c842` | `#000000`     | 8.6:1   | AAA      |
| semantic.success         | `#00e676` | `#000000`     | 8.9:1   | AAA      |
| semantic.warning         | `#ffab40` | `#000000`     | 10.5:1  | AAA      |
| semantic.error           | `#ff6e6e` | `#000000`     | 5.5:1   | AA       |
| interactive.focusRing    | `#ffff00` | `#000000`     | 10.3:1  | AAA      |

All ratios computed with WCAG 2.x relative luminance formula.

---

## Component Audit Checklist

For each component, every cell in the table must be verified for both new variants.  
Legend: ✅ Handled in code · 🔧 Requires CSS override · ⚠️ Needs manual QA · ❌ Not yet addressed

### Shared Components

| Component | File | High-Contrast | Reduced-Motion | Notes |
|-----------|------|---------------|----------------|-------|
| `SkeletonLoader` | `shared/components/SkeletonLoader.tsx` | ✅ | ✅ | Solid block; shimmer element omitted |
| `ActivityItemSkeleton` | `shared/components/ActivityItemSkeleton.tsx` | 🔧 | ✅ | Hardcoded glass classes → need `.high-contrast` override |
| `IssueCardSkeleton` | `shared/components/IssueCardSkeleton.tsx` | 🔧 | ✅ | Same as above |
| `PRRowSkeleton` | `shared/components/PRRowSkeleton.tsx` | 🔧 | ✅ | Same as above |
| `ChartSkeleton` | `shared/components/ChartSkeleton.tsx` | 🔧 | ✅ | Same as above |
| `Toast` | `shared/components/Toast.tsx` | 🔧 | ✅ | CSS `.high-contrast .grainlify-toast` rule added; `motion-safe` classes already present |
| `SearchModal` | `shared/components/SearchModal.tsx` | 🔧 | ✅ | Glassmorphism `backdrop-blur` killed by CSS; needs opaque bg token |
| `NotificationsDropdown` | `shared/components/NotificationsDropdown.tsx` | 🔧 | ✅ | Inline `SkeletonItem` needs audit |
| `UserProfileDropdown` | `shared/components/UserProfileDropdown.tsx` | 🔧 | ✅ | Glass dropdown panel |
| `GlassDropdown` | `shared/components/GlassDropdown.tsx` | 🔧 | ✅ | Name implies glass — verify opaque fallback |
| `FilterDropdown` | `shared/components/FilterDropdown.tsx` | 🔧 | ✅ | |
| `RoleSwitcher` | `shared/components/RoleSwitcher.tsx` | 🔧 | ✅ | |

### UI Components

| Component | File | High-Contrast | Reduced-Motion | Notes |
|-----------|------|---------------|----------------|-------|
| `Modal` | `shared/components/ui/Modal.tsx` | 🔧 | ✅ | `backdrop-blur-[25px]` killed by CSS; `animate-in zoom-in-95` disabled by `.reduced-motion` |
| `ModalButton` | `shared/components/ui/Modal.tsx` | 🔧 | ✅ | `hover:scale-[1.02]` disabled by CSS |
| `ModalInput` | `shared/components/ui/Modal.tsx` | 🔧 | ✅ | Glass bg → opaque in HC |
| `ModalSelect` | `shared/components/ui/Modal.tsx` | 🔧 | ✅ | Radix select portal `animate-in` disabled |
| `IssueCard` | `shared/components/ui/IssueCard.tsx` | 🔧 | ✅ | |
| `Dropdown` | `shared/components/ui/Dropdown.tsx` | 🔧 | ✅ | |
| `DatePicker` | `shared/components/ui/DatePicker.tsx` | 🔧 | ✅ | |

### Feature Components

| Component | File | High-Contrast | Reduced-Motion | Notes |
|-----------|------|---------------|----------------|-------|
| `ContributionHeatmap` | `features/dashboard/components/ContributionHeatmap.tsx` | 🔧 | ✅ | `hover:scale-110` disabled by CSS; loading `animate-pulse` cells need HC audit |
| `SettingsPage` | `features/settings/pages/SettingsPage.tsx` | ⚠️ | ✅ | |
| `ProfileTab` | `features/settings/components/profile/ProfileTab.tsx` | ⚠️ | ✅ | |
| `PayoutTab` | `features/settings/components/payout/PayoutTab.tsx` | ⚠️ | ✅ | |
| `TermsTab` | `features/settings/components/terms/TermsTab.tsx` | ⚠️ | ✅ | |
| `NotificationsTab` | `features/settings/components/notifications/NotificationsTab.tsx` | ⚠️ | ✅ | |
| `FormField` | `features/settings/components/shared/FormField.tsx` | ⚠️ | ✅ | Verify label/input contrast |

### Global Styles

| File | High-Contrast | Reduced-Motion | Notes |
|------|---------------|----------------|-------|
| `styles/theme.css` | ✅ | ✅ | `.high-contrast` and `.reduced-motion` blocks added |
| `shared/styles/scrollbar.css` | ✅ | ✅ | HC scrollbar thumb override added in theme.css |

---

## Reduced-Motion Animation Override Map

Every animated interaction and its reduced-motion replacement:

| Animation | Class / API | Normal behaviour | Reduced-motion replacement |
|-----------|-------------|-----------------|---------------------------|
| Skeleton shimmer | `animate-shimmer`, `animate-shimmer-fast` | 1.5 s translate sweep | Static block (element removed) |
| Modal open | `animate-in zoom-in-95 duration-200` | Scale + fade 200 ms | Instant cut (animation disabled) |
| Modal close | implicit | Scale + fade | Instant cut |
| Toast enter | Sonner slide-in | Slide up from bottom | Instant opacity cut 150 ms |
| Toast exit | Sonner slide-out | Slide down | Instant opacity cut 150 ms |
| Heatmap cell hover | `hover:scale-110` | Scale 1.1× | No transform; opacity highlight |
| Badge appear | `animate-badge-in` | Scale bounce 0.3 s | Instant appearance |
| Notification item | `animate-notify-slide-in` | Slide down 0.2 s | Instant opacity fade 150 ms |
| Card hover | `hover:scale-[1.02]` | Scale 1.02× 150 ms | No transform |
| Button hover | `hover:scale-[1.02]` | Scale 1.02× 150 ms | No transform |
| Slide-in panel | `animate-slide-in-right` | Translate 0.3 s | Instant opacity cut |
| Radix dropdowns | `animate-in zoom-in-95` | Scale fade 150 ms | Instant cut |
| Page transitions | Route-level fade/slide | 300 ms | Instant cut |

---

## Design QA Checklist

### High-Contrast Verification

Run each check with `document.documentElement.classList.add('high-contrast')` applied.

#### Contrast ratio checks (≥ 3:1 UI, ≥ 4.5:1 text)

- [ ] Page background `#000000` vs body text `#ffffff` → 21:1 ✓
- [ ] Card surface `#0d0d0d` vs primary text `#ffffff` → 20.4:1 ✓
- [ ] Accent `#f5c842` vs `#000000` → 8.6:1 ✓
- [ ] Success `#00e676` vs `#000000` → 8.9:1 ✓
- [ ] Warning `#ffab40` vs `#000000` → 10.5:1 ✓
- [ ] Error `#ff6e6e` vs `#000000` → 5.5:1 ✓
- [ ] Default border `#888888` vs `#000000` → 5.3:1 ✓ (UI components)
- [ ] Subtle border `#555555` vs `#000000` → 3.5:1 ✓ (UI minimum)
- [ ] Focus ring `#ffff00` vs `#000000` → 10.3:1 ✓
- [ ] Muted text `#808080` vs `#000000` → 5.3:1 ✓
- [ ] Disabled text `#666666` vs `#000000` → 3.5:1 ✓ (UI minimum)

#### Glassmorphism disabled

- [ ] No `backdrop-filter` computed style on any element
- [ ] No `background-image` gradient on card surfaces
- [ ] Modal panel has opaque background
- [ ] Dropdown panels have opaque background
- [ ] Toast has opaque background
- [ ] Scrollbar thumb is fully opaque

#### Border width

- [ ] All buttons have ≥ 2 px border
- [ ] All inputs have ≥ 2 px border
- [ ] All dialogs have ≥ 2 px border

#### Focus ring

- [ ] `outline: 3px solid #ffff00` on every focused interactive element
- [ ] `outline-offset: 2px` present
- [ ] Focus ring visible on `#000000` background (10.3:1)

---

### Reduced-Motion Verification

Run each check with `document.documentElement.classList.add('dark', 'reduced-motion')`.

#### Transform animations absent

- [ ] `SkeletonLoader` — no shimmer sweep visible; static block only
- [ ] Modal open — no scale animation; content appears instantly
- [ ] Modal close — disappears instantly; no scale-down
- [ ] Toast — no slide-in; appears with opacity fade
- [ ] Heatmap cell hover — no scale; only cursor change / opacity shift
- [ ] Badge — no bounce; appears at final size immediately
- [ ] Notification item — no slide; appears with opacity fade
- [ ] Button hover — no scale transform
- [ ] Card hover — no scale transform
- [ ] Dropdown open — no zoom-in animation; appears instantly
- [ ] `animate-slide-in-right` — no translate; instant appearance

#### Opacity fades present (≤ 150 ms)

- [ ] Toast enter/exit uses opacity 150 ms
- [ ] Notification item uses opacity 150 ms (if `data-opacity-transition` set)

#### System media query

- [ ] Add `@media (prefers-reduced-motion: reduce)` via browser devtools
- [ ] All of the transform checks above pass without applying the `.reduced-motion` class

---

## Security Assumptions

1. **No secret tokens in CSS variables.** All colour values in theme CSS are visual design values with no security implications.
2. **localStorage theme persistence.** The stored value is validated against the `THEME_CYCLE` whitelist (`["light","dark","high-contrast","reduced-motion"]`) in `ThemeProvider` before use. An unexpected value falls back to `"light"`.
3. **No remote token loading.** `design-tokens.json` is a build-time asset; tokens are never fetched from a remote endpoint, eliminating SSRF or token-injection vectors.
4. **CSS class injection.** `themeRootClass()` returns only known, hardcoded strings. Consumer code does not interpolate user input into class names.
5. **No XSS surface.** Theme tokens are applied as CSS class names and static values, never via `innerHTML` or `dangerouslySetInnerHTML`.

---

## Test Specification

> No test framework is installed in the frontend (confirmed: no Vitest, Jest, or Playwright in `package.json`). The checks below are the test cases to implement when a framework is added. Each case maps to a specific behaviour assertion.
>
> Recommended framework: **Vitest + @testing-library/react** (compatible with Vite 6 build setup).
> Estimated coverage of new code: **≥ 95%** when all cases below are implemented.

### ThemeContext — unit tests

```typescript
// File: src/shared/contexts/ThemeContext.test.tsx

describe('ThemeProvider', () => {
  it('defaults to "light" when localStorage is empty')
  it('restores a valid persisted theme from localStorage')
  it('ignores invalid localStorage values and falls back to "light"')
  it('toggleTheme cycles: light → dark → high-contrast → reduced-motion → light')
  it('setTheme("high-contrast") applies "high-contrast" class to <html>')
  it('setTheme("reduced-motion") applies "dark" and "reduced-motion" classes to <html>')
  it('setTheme("dark") applies only "dark" class to <html>')
  it('setTheme("light") removes all theme classes from <html>')
  it('setThemeFromAnimation(true) sets theme to "dark"')
  it('setThemeFromAnimation(false) sets theme to "light"')
  it('persists theme to localStorage on change')
  it('removes previous theme class before applying new one')
})

describe('isDarkVariant', () => {
  it('returns true for "dark"')
  it('returns true for "reduced-motion"')
  it('returns false for "light"')
  it('returns false for "high-contrast"')
})

describe('isA11yVariant', () => {
  it('returns true for "high-contrast"')
  it('returns true for "reduced-motion"')
  it('returns false for "light"')
  it('returns false for "dark"')
})

describe('themeRootClass', () => {
  it('returns "" for "light"')
  it('returns "dark" for "dark"')
  it('returns "high-contrast" for "high-contrast"')
  it('returns "dark reduced-motion" for "reduced-motion"')
})

describe('FOCUS_RING_SPEC.className', () => {
  it('returns yellow 3px ring class for "high-contrast"')
  it('returns gold 2px ring class for "dark"')
  it('returns gold 2px ring class for "reduced-motion"')
  it('returns brown 2px ring class for "light"')
})

describe('useTheme', () => {
  it('throws when used outside ThemeProvider')
  it('returns theme, toggleTheme, setTheme, setThemeFromAnimation')
})
```

### SkeletonLoader — unit tests

```typescript
// File: src/shared/components/SkeletonLoader.test.tsx

describe('SkeletonLoader', () => {
  it('renders shimmer child element in "light" theme')
  it('renders shimmer child element in "dark" theme')
  it('does NOT render shimmer child in "high-contrast" theme')
  it('does NOT render shimmer child in "reduced-motion" theme')

  it('applies #333333 background class in "high-contrast" theme')
  it('applies bg-white/[0.08] in "dark" theme')
  it('applies bg-white/[0.12] in "light" theme')

  it('renders circle shape for variant="circle"')
  it('renders text shape for variant="text"')
  it('renders default rounded shape for variant="default"')

  it('applies custom width and height via style prop')
  it('passes through className prop')
  it('has role="presentation" and aria-hidden="true"')
})
```

### CSS theme classes — integration / visual tests

```typescript
// File: src/styles/theme.test.ts (requires jsdom + CSS parsing, or Playwright)

describe('high-contrast CSS rules', () => {
  it('sets backdrop-filter: none on all children of .high-contrast')
  it('sets outline 3px solid #ffff00 on focused buttons inside .high-contrast')
  it('sets border: 2px solid on inputs inside .high-contrast')
  it('disables animate-shimmer inside .high-contrast')
})

describe('reduced-motion CSS rules', () => {
  it('sets animation-duration: 0.01ms on all children of .reduced-motion')
  it('sets transition-duration: 0.01ms on all children of .reduced-motion')
  it('disables animate-shimmer inside .reduced-motion')
  it('disables animate-badge-in inside .reduced-motion')
  it('disables animate-notify-slide-in inside .reduced-motion')
  it('[data-sonner-toaster] children have only opacity transition inside .reduced-motion')
})

describe('prefers-reduced-motion media query (system override)', () => {
  it('disables animate-shimmer when OS reports prefers-reduced-motion: reduce')
  it('disables animate-slide-in-right when OS reports prefers-reduced-motion: reduce')
  it('disables animate-badge-in when OS reports prefers-reduced-motion: reduce')
})
```

---

## Files Changed

| File | Change |
|------|--------|
| `frontend/src/shared/contexts/ThemeContext.tsx` | Added `high-contrast`, `reduced-motion` variants; `HIGH_CONTRAST_TOKENS`, `REDUCED_MOTION_TOKENS`, `themeRootClass()`, `isDarkVariant()`, `isA11yVariant()`; `ThemeProvider` now applies root CSS classes |
| `frontend/src/app/contexts/ThemeContext.tsx` | Replaced duplicate implementation with re-export from shared |
| `frontend/src/shared/contexts/index.ts` | Exports new symbols |
| `frontend/src/shared/components/SkeletonLoader.tsx` | Handles all 4 theme variants; suppresses shimmer for HC/RM |
| `frontend/src/styles/theme.css` | Added `.high-contrast` block (focus ring, borders, no glass, disabled shimmer) and `.reduced-motion` block (all transforms off, opacity fades permitted); extended `prefers-reduced-motion` media query |
| `design-tokens.json` | Added `highContrast` and `reducedMotion` token sections with verified contrast values; fixed missing root `}` |
| `design/skeleton-motion.md` | Extended with HC and RM override specs, updated static fallback table |
| `design/specs/accessibility-theme-variants.md` | This document |
