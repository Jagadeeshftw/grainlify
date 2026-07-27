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
| `xl`      | 1280px    | `xl:`            | Tailwind-only (not in hook)          |

Breakpoint boundaries match Tailwind v4 defaults and the values used in
`BrowsePage.tsx` grids and filter drawer visibility.

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

### `useResponsiveBreakpoint(): BreakpointState`

Returns the current viewport bucket along with boolean flags.

```tsx
const { isMobile, isTablet, isDesktop, breakpoint } = useResponsiveBreakpoint()
```

**Edge-case guarantee:** Exactly one of `isMobile`, `isTablet`, `isDesktop` is
always `true`. There is never a render cycle where all three are `false`.

### `useResponsiveToken<T>(tokenMap, defaultValue): T`

Resolves a breakpoint-aware token from a `Partial<Record<Breakpoint, T>>` map.
Falls back to the nearest smaller breakpoint when the current one is not defined,
or to `defaultValue` when none match.

```tsx
const columns = useResponsiveToken({ sm: 1, md: 2, lg: 4, xl: 5 }, 1)
```

**Fallback chain:** current → current-1 → … → sm → defaultValue

### `useReducedMotion(): boolean`

OS-level `prefers-reduced-motion`. Initialized deterministically on first render.

### `usePrefersDarkMode(): boolean`

OS-level `prefers-color-scheme: dark`. Initialized deterministically on first render.

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

## Test Coverage

| Hook                    | File                                      | Key assertions                                 |
|-------------------------|-------------------------------------------|------------------------------------------------|
| `useMediaQuery`         | `hooks/__tests__/useMediaQuery.test.ts`   | Initial match, change listener, SSR safety     |
| `useResponsiveBreakpoint` | `hooks/__tests__/useResponsiveBreakpoint.test.ts` | All 3 breakpoint states, deterministic init |
| `useResponsiveToken`    | `hooks/__tests__/useResponsiveToken.test.ts` | Exact match, fallback chain, default, determinism |
| `useReducedMotion`      | Same as breakpoint test                   | Default false, respects `prefers-reduced-motion` |
| `usePrefersDarkMode`    | Same as breakpoint test                   | Default false, respects `prefers-color-scheme` |

## Migration Guide (from old pattern)

### `useResponsiveBreakpoint` (old → new)

**Old** — non-deterministic, used `window.addEventListener('resize')`:

```tsx
const { isMobile, isTablet, isDesktop } = useResponsiveBreakpoint()
// ⚠️ On first render all three could be false simultaneously
```

**New** — deterministic, uses `useMediaQuery` backed by `matchMedia`:

```tsx
const { isMobile, isTablet, isDesktop, breakpoint } = useResponsiveBreakpoint()
// ✅ Exactly one flag is always true. Plus the new `breakpoint` string enum.
```

The old function signature is fully backward compatible — existing callers
accessing only `isMobile`, `isTablet`, `isDesktop` continue to work.
