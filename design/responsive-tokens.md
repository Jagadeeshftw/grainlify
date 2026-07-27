# Responsive Token System

## Overview

The responsive token system provides deterministic, SSR-safe hooks for consuming
design tokens that vary by viewport breakpoint. It ensures that components always
get correct values on the **first render** — no flash of wrong value followed by
a re-render correction.

## Breakpoint Definitions

| Token     | Min Width | Tailwind Class   | Media Query                          |
|-----------|-----------|------------------|--------------------------------------|
| `sm`      | 0         | `sm:`            | `(max-width: 767px)`                 |
| `md`      | 768px     | `md:`            | `(min-width: 768px) and (max-width: 1023px)` |
| `lg`      | 1024px    | `lg:`            | `(min-width: 1024px)`                |
| `xl`      | 1280px    | `xl:`            | `(min-width: 1280px)`                |

Boundaries match Tailwind v4 defaults, `BrowsePage.tsx` grids, `Dashboard.tsx`
sidebar breakpoints, and the `use-mobile.ts` 768px threshold.

## CSS Custom Properties

The file `styles/responsive.css` exposes breakpoint values and grid tokens
as CSS custom properties that change value at each breakpoint:

```css
:root {
  --bp-sm: 640px;  --bp-md: 768px;
  --bp-lg: 1024px; --bp-xl: 1280px;
  --grid-columns: 1;   /* sm: 1, md: 2, lg: 4, xl: 5 */
  --grid-gap: 1rem;    /* sm: 1rem, md: 1.25rem, lg/xl: 1.5rem */
  --container-padding: 1rem; /* sm: 1rem, md: 1.5rem, lg: 2rem */
  --container-max-width: 100%;  /* sm: 100%, md: 720px, lg: 960px, xl: 1200px */
}
```

### Edge Cases — CSS Custom Properties

- **Viewport exactly at 768px:** The `min-width: 768px` media query activates
  (md breakpoint). `--grid-columns` becomes 2. The `max-width: 767px` query
  does NOT apply at exactly 768px, so there is no overlap.
- **Viewport exactly at 1024px:** `min-width: 1024px` activates (lg breakpoint).
  `--grid-columns` becomes 4. The md query also matches at 1024px+ but the lg
  query appears later in the cascade, so its values win.
- **Viewport between breakpoints (e.g., 800px):** The nearest `min-width`
  query that is satisfied applies. At 800px, `min-width: 768px` is the closest
  match; `min-width: 1024px` does not yet match.
- **Using `var(--token, fallback)`:** CSS custom properties that are undefined
  at a given breakpoint fall back to their default cascade value (inherited
  from the `:root` block). Consumers should always provide a sensible `:root`
  default so the property works even before any media query activates.
- **High-contrast / reduced-motion:** These theme variants do NOT override
  responsive CSS custom properties. Responsive properties like `--grid-columns`
  still respond to viewport changes. The theme variants only affect color,
  animation, and border tokens (see `styles/theme.css`).

## Hooks

### `useMediaQuery(query: string): boolean`

SSR-safe foundation. Initializes with the **actual** matchMedia value on the
first render (not a placeholder `false`). Listens for changes via `change`
event on the `MediaQueryList`.

**Deterministic guarantee:** The first render always reflects the current state
of the media query. No flash of wrong value.

```tsx
const isNarrow = useMediaQuery('(max-width: 767px)')
```

**Edge cases covered by tests:**
- SSR (no `window` global): returns `false`
- Dynamic query string change via rerender: re-subscribes
- Rapid change events (20 in a row): no crash, correct final state
- Unmount: cleans up the `change` listener

### `useResponsiveBreakpoint(): BreakpointState`

Returns the current viewport bucket along with boolean flags.

```tsx
const { isMobile, isTablet, isDesktop, isLargeDesktop, breakpoint } =
  useResponsiveBreakpoint()
```

| Flag             | True when              |
|------------------|------------------------|
| `isMobile`       | `< 768px`              |
| `isTablet`       | `768px – 1023px`       |
| `isDesktop`      | `>= 1024px` (includes xl) |
| `isLargeDesktop` | `>= 1280px`            |

**Edge-case guarantee:** `isLargeDesktop` is additive — `isDesktop` remains
`true` at xl viewports for backward compatibility. The `breakpoint` string
returns the most specific match (`xl` > `lg` > `md` > `sm`).

### `useResponsiveToken<T>(tokenMap, defaultValue): T`

Resolves a breakpoint-aware token from a `Partial<Record<Breakpoint, T>>` map.
Falls back to the nearest smaller breakpoint when the current one is not defined,
or to `defaultValue` when none match.

```tsx
const columns = useResponsiveToken({ sm: 1, md: 2, lg: 4, xl: 5 }, 1)
```

**Fallback chain (now includes xl):** xl → lg → md → sm → defaultValue

**Edge cases covered by tests:**
- xl-specific token resolves correctly
- Falls back from xl → lg when xl is missing
- Full chain fallback xl → lg → md → sm → default
- Empty map at xl returns defaultValue

### `useReducedMotion(): boolean`

OS-level `prefers-reduced-motion`. Initialized deterministically on first render.

### `usePrefersDarkMode(): boolean`

OS-level `prefers-color-scheme: dark`. Initialized deterministically on first render.

## `useIsMobile()` (sidebar component)

Located at `app/components/ui/use-mobile.ts`. This is a thin wrapper around
`useMediaQuery('(max-width: 767px)')` used by the sidebar layout. It now
shares the same deterministic foundation as the rest of the responsive system.

**Before:** `React.useState<boolean | undefined>(undefined)` — first render was
`!!undefined === false`, then `useEffect` corrected it. Non-deterministic.

**After:** Uses `useMediaQuery` — first render already reflects the actual
viewport. Deterministic.

## Design Token Integration

The `design-tokens.json` file contains responsive motion values under
`motion.responsive`. These can be consumed via `useResponsiveToken`:

```tsx
const durationAdjustment = useResponsiveToken(
  {
    sm: { from: '-25%', staggerDelay: '30ms' },
    md: { from: '-10%', staggerDelay: '40ms' },
    lg: { from: '0%', staggerDelay: '50ms' },
  },
  { from: '0%', staggerDelay: '50ms' },
)
```

The `motionConfig.ts` `responsive` section also uses the same breakpoint names.
When consuming these config values, prefer `useResponsiveToken` over manual
`window.innerWidth` checks.

## CSS @media Query Inventory

| File | Query | Purpose |
|------|-------|---------|
| `styles/theme.css` | `@media (prefers-reduced-motion: reduce)` | Disables shimmer, badge-in, notify-slide-in animations |
| `styles/responsive.css` | `@media (min-width: 768px/1024px/1280px)` | Grid columns, gap, container padding |
| `ProfilePage/reward-certificate-templates.css` | `@media (max-width: 960px/640px/480px)` | Certificate modal layout |
| `dashboard/pages/DataPage.tsx` | `@media (prefers-reduced-motion: reduce)` (inline `<style>`) | Disables all animations |
| `onboarding/tour/onboarding-tour.css` | `@media (prefers-reduced-motion: reduce)` | Disables Joyride animations |
| `onboarding/coach-marks/coach-marks.css` | `@media (prefers-reduced-motion: reduce)` | Disables coach mark animations |
| `leaderboard/components/LeaderboardStyles.tsx` | `@media (prefers-reduced-motion: reduce)` | Disables delta, rank animations |

All `prefers-reduced-motion` media queries mirror the `.reduced-motion` CSS
class applied by `ThemeProvider` for the reduced-motion theme variant.

## Raw `window.matchMedia` / `window.innerWidth` Audit

| File | Pattern | Migrated? |
|------|---------|-----------|
| `shared/hooks/useMediaQuery.ts` | `window.matchMedia` | ✅ Core foundation |
| `app/components/ui/use-mobile.ts` | `window.matchMedia` + `window.innerWidth` | ✅ Now uses `useMediaQuery` |
| `dashboard/Dashboard.tsx:103-174` | `window.innerWidth` + resize | ⏳ Still uses raw pattern (sidebar breakpoint) |
| `dashboard/pages/OpenSourceWeekPage.tsx:639,706` | `window.matchMedia` | ⏳ Inline usage |
| `shared/components/MediaEmbed.tsx:93` | `window.matchMedia` | ⏳ Outside React (standalone helper) |

Legend: ✅ migrated, ⏳ not yet migrated (backward-compatible, future work)

## Test Coverage

| Hook                    | File                                      | Key assertions                                 |
|-------------------------|-------------------------------------------|------------------------------------------------|
| `useMediaQuery`         | `hooks/__tests__/useMediaQuery.test.ts`   | Initial match (true/false), deterministic first render, change listener, SSR safety (no window), dynamic query change, rapid changes (20 in a row), cleanup on unmount, bidirectional transitions (match→no-match→match), multiple independent instances with different queries |
| `useResponsiveBreakpoint` | `hooks/__tests__/useResponsiveBreakpoint.test.ts` | All 4 breakpoint states (sm/md/lg/xl), `isLargeDesktop`, backward-compat `isDesktop`, deterministic rerender, exact boundary values (768px, 1024px, 1280px), window resize simulation (mobile→desktop transition) |
| `useResponsiveToken`    | `hooks/__tests__/useResponsiveToken.test.ts` | Exact match, fallback chain (including xl→lg→md→sm→default), determinism, empty map edge case, null values treated as defined (not skipped), invalid breakpoint keys ignored (xs, 2xl), changing tokenMap reference triggers re-resolution, undefined values skipped in fallback |
| `useReducedMotion`      | Same as breakpoint test                   | Default false, respects `prefers-reduced-motion` |
| `usePrefersDarkMode`    | Same as breakpoint test                   | Default false, respects `prefers-color-scheme` |

### Edge Cases Covered

#### `useMediaQuery`
- **SSR safety:** When `window` is undefined (server-side render), initializes to `false`.
- **Bidirectional changes:** Transitions from match→no-match→match are all handled correctly.
- **Multiple instances:** Independent hooks with different queries do not interfere with each other.

#### `useResponsiveBreakpoint`
- **Boundary at exactly 768px:** `isMobile=false`, `isTablet=true` (the `max-width: 767px` query
  does NOT match at exactly 768px).
- **Boundary at exactly 1024px:** `isTablet=false`, `isDesktop=true` (the tablet query range is
  `min-width: 768px` AND `max-width: 1023px`).
- **Boundary at exactly 1280px:** `isDesktop=true`, `isLargeDesktop=true`, `breakpoint='xl'`
  (additive — isDesktop remains true at xl).
- **Resize simulation:** When media query listeners fire in sequence (mobile→desktop), the
  hook correctly transitions through all states without glitches.

#### `useResponsiveToken`
- **Null values:** `null` is treated as a **defined** value (passes `!== undefined` check),
  so `{ sm: null }` returns `null` rather than falling through to `defaultValue`.
- **Undefined values:** `undefined` is skipped in the fallback chain, allowing the
  resolution to continue to the next smaller breakpoint.
- **Invalid breakpoint keys:** Extra keys like `'xs'` or `'2xl'` are ignored; they
  are not in `BREAKPOINT_ORDER` and never matched. The fallback chain uses only the 4
  canonical breakpoints: `xl → lg → md → sm → defaultValue`.
- **Changing tokenMap reference:** When the caller passes a new object reference with the
  same values, `useMemo` re-evaluates (because the dependency is `tokenMap` by reference).
  This is the expected React behavior.

## Migration Guide (from old pattern)

### `useResponsiveBreakpoint` (old → new)

**Old** — non-deterministic, used `window.addEventListener('resize')`:

```tsx
const { isMobile, isTablet, isDesktop } = useResponsiveBreakpoint()
// ⚠️ On first render all three could be false simultaneously
```

**New** — deterministic, uses `useMediaQuery` backed by `matchMedia`:

```tsx
const { isMobile, isTablet, isDesktop, isLargeDesktop, breakpoint } =
  useResponsiveBreakpoint()
// ✅ Exactly one flag is always true. Breakpoint string reflects the exact tier.
```

The old property names are fully backward compatible — existing callers
accessing only `isMobile`, `isTablet`, `isDesktop` continue to work unchanged.

### `use-mobile.ts` (old → new)

**Old** — `useState<boolean | undefined>(undefined)` + `window.innerWidth`:

```tsx
const isMobile = useIsMobile()
// ⚠️ First render: !!undefined === false (wrong value flash)
```

**New** — delegates to `useMediaQuery`:

```tsx
const isMobile = useIsMobile()
// ✅ First render: correct value. No flash.
```

Same function signature. No import changes needed.
