# Responsive Design System Behavior

## Purpose

This document defines the responsive behavior of the Grainlify design system — how layout, spacing, typography, motion, and accessibility tokens adapt across viewport breakpoints. The goal is to make every responsive rule explicit so that UI surfaces remain visually coherent and regression-safe across all supported screen sizes.

## Breakpoint System

### Shared Breakpoint Definitions

The design system operates on **four breakpoint tiers**:

| Breakpoint | Label          | CSS Variable | Media Query                     |
|-----------|----------------|-------------|----------------------------------|
| `sm`      | Mobile         | `--bp-sm`   | `max-width: 767px`               |
| `md`      | Tablet         | `--bp-md`   | `min-width: 768px` and `max-width: 1023px` |
| `lg`      | Desktop        | `--bp-lg`   | `min-width: 1024px`              |
| `xl`      | Large Desktop  | `--bp-xl`   | `min-width: 1280px`              |

> **Note:** The CSS variable `--bp-sm` stores a value of `640px`. However, the actual mobile breakpoint used in all media queries and JavaScript hooks is `768px`. This is because the minimum viewport width that Grainlify supports is 320px, and the `--bp-sm` variable is reserved for future use if an `xs` (extra-small) tier is introduced. All consumers must use the documented media queries rather than relying on `--bp-sm` for logic.

### Source of Truth

Breakpoints are defined in these locations:

| Artifact              | File                                          |
|-----------------------|-----------------------------------------------|
| CSS custom properties | `frontend/src/styles/responsive.css`           |
| JS hooks              | `frontend/src/shared/hooks/useReducedMotion.ts` |
| Design tokens         | `design-tokens.json` (accessible under `accessibility.responsiveAccessibility`) |

---

## CSS Responsive Variables

### Grid & Layout

Variables declared in `responsive.css` adjust layout at each breakpoint:

```css
:root {
  --bp-sm: 640px;     /* Reserved — min supported is 320px */
  --bp-md: 768px;
  --bp-lg: 1024px;
  --bp-xl: 1280px;

  --grid-columns: 1;
  --grid-gap: 1rem;
  --container-padding: 1rem;
}
```

| Breakpoint | Grid Columns | Grid Gap  | Container Padding |
|-----------|-------------|-----------|-------------------|
| `< 768px`  | 1           | 1rem      | 1rem              |
| 768–1023   | 2           | 1.25rem   | 1.5rem            |
| 1024–1279  | 4           | 1.5rem    | 2rem              |
| ≥ 1280     | 5           | 1.5rem    | 2rem (inherited)  |

### Tailwind CSS Integration

The project uses Tailwind CSS v4 with the `@tailwindcss/vite` plugin. Default Tailwind breakpoint prefixes (`sm`, `md`, `lg`, `xl`) are used throughout the codebase. The `@custom-variant` directives in `theme.css` enable theme-specific selectors:

```css
@custom-variant dark (&:is(.dark *));
@custom-variant high-contrast (&:is(.high-contrast *));
@custom-variant reduced-motion (&:is(.reduced-motion *));
```

---

## JavaScript Responsive Hooks

### `useResponsiveBreakpoint()`

Source: `frontend/src/shared/hooks/useReducedMotion.ts`

Returns a `BreakpointState` object:

```typescript
interface BreakpointState {
  isMobile: boolean;       // max-width: 767px
  isTablet: boolean;       // 768px – 1023px
  isDesktop: boolean;      // min-width: 1024px
  isLargeDesktop: boolean; // min-width: 1280px
  breakpoint: 'sm' | 'md' | 'lg' | 'xl';
}
```

**Resolution priority:** `xl → lg → md → sm`. When multiple media queries match (e.g., `isDesktop` and `isLargeDesktop` both true at 1280px+), the highest breakpoint takes precedence.

### `useResponsiveToken()`

Source: `frontend/src/shared/hooks/useResponsiveToken.ts`

A generic hook that resolves a single value from a responsive token map by cascading **downward** from the current breakpoint to the smallest:

```typescript
function useResponsiveToken<T>(
  tokenMap: Partial<Record<Breakpoint, T>>,
  defaultValue: T,
): T
```

**Cascade behavior:**
1. Check if the current breakpoint has a value defined → return it
2. If not, check each smaller breakpoint in order (`lg → md → sm`)
3. If none are defined, return `defaultValue`

**Edge cases:**
- When multiple media queries match (e.g., `lg` and `xl` both true), only the highest matching breakpoint is used as the starting point for fallback
- The cascade never climbs **up** to a larger breakpoint — only down
- An empty token map always yields `defaultValue`

### `useResponsiveToken` Cascade Examples

| Current Breakpoint | Token Map                    | Resolved Value |
|-------------------|------------------------------|----------------|
| `xl` (≥1280px)    | `{ xl: 'a', lg: 'b' }`      | `'a'`          |
| `xl` (≥1280px)    | `{ lg: 'b', md: 'c' }`      | `'b'`          |
| `xl` (≥1280px)    | `{ sm: 'd' }`               | `'d'`          |
| `xl` (≥1280px)    | `{}`                        | `defaultValue` |
| `lg` (1024–1279)  | `{ md: 'c', sm: 'd' }`      | `'c'`          |
| `md` (768–1023)   | `{ lg: 'b' }`               | `defaultValue` |
| `sm` (<768px)     | `{ md: 'c', lg: 'b' }`      | `defaultValue` |

---

## Responsive Animation Behavior

### Motion Duration Adjustments

Defined in `design-tokens.json` under `motion.responsive`:

| Breakpoint | Duration Adjustment | Stagger Delay | Notes              |
|-----------|-------------------|---------------|---------------------|
| `sm`      | -25%              | 30ms          | Mobile optimization |
| `md`      | -10%              | 40ms          | Tablet optimization |
| `lg`      | 0%                | 50ms          | Desktop default     |

### Reduced Motion

The `.reduced-motion` theme class and `(prefers-reduced-motion: reduce)` media query override all animations:
- Transform-based animations → disabled
- Opacity-only fades → permitted up to 150ms
- Skeleton shimmer → static block
- List stagger → all items appear instantly
- Modal enter/exit → instant opacity cut

---

## Responsive Accessibility Behavior

### Touch Targets

Defined in `design-tokens.json` under `accessibility.focus`:

| Breakpoint | Min Touch Target | Focus Indicator     |
|-----------|-----------------|---------------------|
| `sm`      | 44×44px         | Enhanced visibility |
| `md`      | 44×44px         | Standard visibility |
| `lg`      | 40×40px         | Standard visibility |
| `xl`      | 40×40px         | Standard visibility |

### Text Zoom

- Supports up to 200% zoom without horizontal scroll (WCAG 1.4.4)
- Tested with browser zoom and browser zoom settings

---

## Theme Responsiveness

### Dark Mode

Toggle via `.dark` class on `<html>`. Applied by `ThemeContext.tsx`. Dark mode tokens are defined in `design-tokens.json` under `darkMode` and `color.darkMode`. The transition between light and dark mode is instant (no animation).

### High-Contrast Mode

Toggle via `.high-contrast` class on `<html>`. Key responsive behaviors:
- Glassmorphism disabled (backdrop-filter: none)
- All borders ≥ 2px
- Focus ring: 3px solid yellow (#ffff00)
- Skeleton loaders: static (no animation)

### Reduced-Motion Mode

Toggle via `.reduced-motion` class on `<html>`. Also respects `(prefers-reduced-motion: reduce)` OS-level preference. See "Reduced Motion" section above.

---

## Responsive UI Surface Behavior

The following UI surfaces have explicit responsive handling:

### App Shell Navigation

| Feature              | Mobile (<768px)               | Tablet/Desktop (768px+)        |
|---------------------|-------------------------------|--------------------------------|
| Primary nav         | Drawer overlay                | Sidebar (persistent)           |
| Breadcrumbs         | Hidden                        | Visible on detail pages        |
| Scroll behavior     | Page scrolls                  | Sidebar stays fixed            |
| Touch targets       | 44×44px minimum               | 40×40px minimum                |

### Contribution Heatmap

| Feature              | Mobile (<768px)               | Desktop (1024px+)              |
|---------------------|-------------------------------|--------------------------------|
| Layout              | Horizontal scroll             | Full-width                     |
| Typography          | Responsive font sizing        | Standard                       |

### Reward Certificate Preview Modal

| Feature              | Mobile (<768px)               | Desktop (1024px+)              |
|---------------------|-------------------------------|--------------------------------|
| Modal width         | Full-screen (100vw)           | Centered, max-width            |
| Dismiss             | Swipe down                    | Click outside / Escape         |

### Media Embed

- Aspect ratio maintained via `aspect-ratio` CSS property
- Responsive width: 100% on all breakpoints
- Max-width constrained at `lg+`

---

## Regression Surface

Changes to any of the following files may introduce responsive regressions:

### Core Responsive Files
- `frontend/src/styles/responsive.css` — breakpoint variables, grid/layout defaults
- `frontend/src/shared/hooks/useReducedMotion.ts` — JS breakpoint detection
- `frontend/src/shared/hooks/useResponsiveToken.ts` — responsive token resolution
- `design-tokens.json` — design token definitions including `motion.responsive` and `accessibility.responsiveAccessibility`

### Theme Files
- `frontend/src/styles/theme.css` — dark, high-contrast, reduced-motion themes
- `frontend/src/styles/index.css` — style entry point (import order matters)
- `frontend/src/styles/tailwind.css` — Tailwind v4 setup and `tw-animate-css`

### Hooks
- `frontend/src/shared/hooks/useMediaQuery.ts` — underlying media query hook
- `frontend/src/shared/hooks/useReducedMotion.ts` — breakpoint, reduced motion, dark mode

### UI Surfaces
- `frontend/src/features/dashboard/components/ContributionHeatmap.tsx`
- `frontend/src/features/dashboard/components/RewardsChart.tsx`
- `frontend/src/features/ProfilePage/RewardCertificateSection.tsx`
- `frontend/src/shared/components/MediaEmbed.tsx`

### Test Files
- `frontend/src/shared/hooks/__tests__/useResponsiveBreakpoint.test.ts`
- `frontend/src/shared/hooks/__tests__/useResponsiveToken.test.ts`

---

## Testing Guidelines

When adding or modifying responsive behavior:

1. **Boundary tests**: Test each breakpoint boundary (767px, 768px, 1023px, 1024px, 1279px, 1280px)
2. **Cascade tests**: Verify token fallback through all cascade levels (xl→lg→md→sm→default)
3. **Theme interaction**: Verify behavior under dark, high-contrast, and reduced-motion themes
4. **Resize simulation**: Test transition between breakpoints during resize
5. **Determinism**: Assert that the same viewport always produces the same result on re-render
6. **Edge cases**:
   - Empty token maps
   - Partial token maps (gaps in the cascade)
   - 768px and 1024px exact boundaries (no off-by-one errors)
   - Concurrent breakpoint matches (e.g., isDesktop + isLargeDesktop)
