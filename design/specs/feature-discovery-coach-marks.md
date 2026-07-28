# Contextual Feature-Discovery Coach Marks — Design & Implementation Spec

## Overview

The sequential onboarding tour (`OnboardingTourProvider.tsx`) walks new users through
dashboard chrome on first visit. **Feature-discovery coach marks** are a separate,
lighter-weight pattern: single-target hints that surface once when a user first
encounters a specific advanced feature later in their journey (e.g. BrowsePage
advanced filters, MaintainersPage bounty analytics). They are independent of the
sequential tour and do not interfere with it.

|                   |                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| **Last Updated**  | July 27, 2026                                                                                                               |
| **Pattern**       | Single-target coach mark (pointer + highlight ring + copy bubble + "Got it" dismiss)                                         |
| **Surfaces**      | `BrowsePage` (advanced filter drawer FAB), `MaintainersPage` (bounty analytics section)                                      |
| **Status**        | New — design + implementation                                                                                               |
| **Related specs** | [onboarding-tutorial-overlay.md](./onboarding-tutorial-overlay.md), [design-tokens.json](../../design-tokens.json)          |

---

## Table of Contents

1. [Differentiation from Sequential Tour](#1-differentiation-from-sequential-tour)
2. [Anatomy & Component Structure](#2-anatomy--component-structure)
3. [State Machine](#3-state-machine)
4. [Persistence Rules](#4-persistence-rules)
5. [Stacking & Queuing](#5-stacking--queuing)
6. [Accessibility](#6-accessibility)
7. [Design Tokens Validation](#7-design-tokens-validation)
8. [Responsive Behavior](#8-responsive-behavior)
9. [Reduced Motion](#9-reduced-motion)
10. [Implementation Map](#10-implementation-map)

---

## 1. Differentiation from Sequential Tour

| Aspect                  | Sequential Tour (`OnboardingTourProvider`)          | Feature-Discovery Coach Marks                    |
| ----------------------- | --------------------------------------------------- | ------------------------------------------------ |
| Trigger                 | First-ever dashboard visit                          | First encounter of a specific feature            |
| Scope                   | Multi-step, full-page traversal                     | Single-target, inline                            |
| Library                 | `react-joyride` controlled mode                     | Custom lightweight React component               |
| Dismissal               | Skip / Next / Finish / Close                        | "Got it" button or Escape key                    |
| Persistence key         | `grainlify.onboarding.tour.v1`                      | `grainlify.coach-mark.<feature-id>.v1`           |
| Shows simultaneously?   | N/A (sequential)                                    | No — max one visible at a time, queued           |
| Re-trigger              | Via Settings "Restart tutorial"                     | Never (once dismissed, hidden permanently)       |

---

## 2. Anatomy & Component Structure

```
┌─────────────────────────────────────────────┐
│  ┌──── Highlight ring (2px accent border)   │
│  │  ┌──────────────────────────────────┐    │
│  │  │  Target element (e.g. FAB)       │    │
│  │  └──────────────────────────────────┘    │
│  └──────────────────────────────────────────┘
│                                              │
│  ┌─── Pointer (8px triangle, accent fill)    │
│  │                                           │
│  ┌───────────────────────────────────────┐   │
│  │  Copy bubble (glassmorphism surface)  │   │
│  │  ─────────────────────────────────── │   │
│  │  Title: "Advanced Filters"            │   │
│  │  Body: "Filter by language, ecosystem │   │
│  │  and more to narrow your search."     │   │
│  │                                       │   │
│  │  [ Got it ]                           │   │
│  └───────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

### Highlight Ring
- 2px solid border using `color.primary.500` (`#f1b400`) in light mode
- 2px solid border using `darkMode.semantic.accentPrimary` (`#c9983a`) in dark mode
- 4px offset from target element
- Border-radius matches target element's border-radius

### Pointer
- 8px equilateral triangle
- Fill matches highlight ring color
- Points from the copy bubble toward the target element
- Position adapts based on target's viewport position (top/right/bottom/left)

### Copy Bubble
- Glassmorphism surface matching existing tour tooltip:
  - Light: `bg-white/[0.55] border-white/40 backdrop-blur-[40px]`
  - Dark: `bg-[#2d2820]/[0.72] border-white/10 backdrop-blur-[40px]`
- Border radius: `24px` (matches `TourTooltip`)
- Max width: `320px`
- Shadow: `0 8px 32px rgba(0,0,0,0.18)` (matches elevation level 3)

### "Got it" Button
- Background: `color.primary.500` (`#f1b400`)
- Text: `#ffffff`
- Border radius: `14px`
- Font: `13px` semibold
- Focus ring: 2px offset, `#c9983a`

---

## 3. State Machine

```
                    ┌──────────────┐
                    │  eligible    │  (user hasn't dismissed this feature)
                    │  -unseen     │
                    └──────┬───────┘
                           │
                    Feature becomes visible
                    in viewport
                           │
                           ▼
                    ┌──────────────┐
              ┌─────│   visible    │─────┐
              │     └──────┬───────┘     │
              │            │             │
         Escape /     "Got it"     Another coach
         backdrop     click         mark queued
              │            │             │
              ▼            ▼             ▼
        ┌──────────┐ ┌──────────┐ ┌──────────────┐
        │dismissed │ │dismissed │ │   queued-    │
        │  -seen   │ │  -seen   │ │ behind-other │
        └──────────┘ └──────────┘ └──────┬───────┘
                                         │
                                  Other coach mark
                                  dismissed
                                         │
                                         ▼
                                  ┌──────────────┐
                                  │   visible    │
                                  └──────────────┘
```

### State Descriptions

| State                 | Description                                                                     |
| --------------------- | ------------------------------------------------------------------------------- |
| `eligible-unseen`     | Feature is eligible; user has never dismissed this coach mark                   |
| `visible`             | Coach mark is currently displayed to the user                                   |
| `dismissed-seen`      | User clicked "Got it" or pressed Escape; coach mark is permanently hidden       |
| `queued-behind-other` | Another coach mark is visible; this one waits its turn                          |

---

## 4. Persistence Rules

Following the conventions in `tour/storage.ts`:

- **Key pattern:** `grainlify.coach-mark.<feature-id>.v1`
- **Values:** `'dismissed'` only (boolean-like, but string for consistency)
- **Validation:** Read value, check `=== 'dismissed'`; any other value treated as unseen
- **Write-once:** Only written when user explicitly dismisses (not on first show)
- **Graceful degradation:** `try/catch` around all localStorage access; failure = treat as unseen

### Feature IDs

| Feature              | `feature-id`           | Surface          |
| -------------------- | ---------------------- | ---------------- |
| BrowsePage filters   | `browse-advanced-filters` | `BrowsePage`     |
| MaintainersPage analytics | `maintainers-bounty-analytics` | `MaintainersPage` |

---

## 5. Stacking & Queuing

When multiple coach marks become eligible on the same page load:

1. **At most one** coach mark is visible at any time
2. Coach marks are queued in registration order (the order components mount/call `useCoachMark`)
3. When the visible coach mark is dismissed, the next queued coach mark appears after a `300ms` delay
4. If the user navigates away before dismissing, the visible coach mark stays visible (it's already in the DOM)
5. Queued coach marks that become ineligible (user navigates away from the surface) are dropped from the queue

### Implementation

The `CoachMarkProvider` maintains a queue via React context:
- `register(featureId)` — add to queue if not dismissed and not already registered
- `dismiss(featureId)` — mark dismissed, remove from queue, show next after delay
- `unregister(featureId)` — remove from queue (e.g. component unmounts)

---

## 6. Accessibility

| Requirement                        | Implementation                                                                  |
| ---------------------------------- | ------------------------------------------------------------------------------- |
| Live region announcement           | `aria-live="polite"` on the coach mark container; announces "Hint: <title>"    |
| Dismissible via keyboard           | `Escape` key dismisses; `Enter`/`Space` on "Got it" button dismisses           |
| No auto-dismiss                    | Coach mark never disappears without explicit user action                         |
| Focus management                   | Focus does NOT move to the coach mark (it's informational, not interactive)     |
| Screen reader semantics            | `role="note"` with `aria-label="Feature hint"`                                 |
| Contrast ratios                    | Copy text: 4.5:1 minimum against glass surface; button: 4.5:1 (white on accent)|
| Touch target                       | "Got it" button: 44x44px minimum                                               |
| Reduced motion                     | Fade-only transition (150ms opacity), no slide/transform                        |

---

## 7. Design Tokens Validation

| Element              | Light Mode Token               | Dark Mode Token                | Contrast vs Surface | WCAG |
| -------------------- | ------------------------------ | ------------------------------ | -------------------- | ---- |
| Highlight ring       | `color.primary.500` `#f1b400`  | `darkMode.semantic.accentPrimary` `#c9983a` | N/A (border) | AA (UI 3:1) |
| Copy bubble bg       | `bg-white/[0.55]`              | `bg-[#2d2820]/[0.72]`          | Glass surface        | AA   |
| Title text           | `#2d2820`                      | `#f5efe5`                      | >7:1                 | AAA  |
| Body text            | `#6b5d4d`                      | `#d4c5b0`                      | >4.5:1               | AA   |
| "Got it" bg          | `color.primary.500` `#f1b400`  | `color.primary.500` `#f1b400`  | N/A                  | AA   |
| "Got it" text        | `#ffffff`                      | `#ffffff`                      | >7:1 on `#f1b400`    | AAA  |
| Pointer fill         | `color.primary.500` `#f1b400`  | `darkMode.semantic.accentPrimary` `#c9983a` | N/A | AA   |

---

## 8. Responsive Behavior

| Viewport Width  | Behavior                                                                      |
| --------------- | ----------------------------------------------------------------------------- |
| `< 375px`       | Copy bubble repositions to stay on-screen; may flip from below to above target |
| `375px - 768px` | Copy bubble max-width: `280px`; pointer size reduced to `6px`                 |
| `> 768px`       | Full-size copy bubble (320px max-width); standard pointer (8px)               |

When the target is near a viewport edge:
1. Calculate available space in all four directions
2. Place the copy bubble in the direction with most space
3. Flip the pointer to point back toward the target
4. If no direction has enough space, scale down the bubble and reposition

---

## 9. Reduced Motion

When `prefers-reduced-motion: reduce` is active:
- No entrance animation (coach mark appears instantly)
- No pointer slide-in
- Dismissal is instant (opacity 0)
- Follows `reducedMotion` tokens: `opacityFadeDuration: 150ms`, `transitionDuration: 0ms`

---

## 10. Implementation Map

All code lives in `frontend/src/features/onboarding/coach-marks/`.

| File                        | Responsibility                                                        |
| --------------------------- | --------------------------------------------------------------------- |
| `storage.ts`                | Per-feature `localStorage` persistence (mirrors `tour/storage.ts`)    |
| `CoachMarkContext.ts`       | Context type + provider for queue management                          |
| `CoachMarkProvider.tsx`     | Queue owner, renders active coach mark, Escape listener               |
| `CoachMarkTooltip.tsx`      | Glassmorphism bubble with pointer, copy, "Got it" button              |
| `useCoachMark.ts`           | Hook for surfaces to register/unregister their coach mark             |
| `coach-marks.css`           | `prefers-reduced-motion` hardening + pointer/positioning keyframes    |
| `index.ts`                  | Public barrel exports                                                 |
