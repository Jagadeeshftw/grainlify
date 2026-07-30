# Landing Hero Animation Choreography Spec

**Issue**: #1511  
**Status**: Ready for implementation  
**Last Updated**: 2026-07-30  
**Author**: Kiro  
**Framework**: Motion (Framer Motion v12 — `"motion": "12.23.24"`)  
**WCAG**: 2.1 AA compliant

---

## Table of Contents

1. [Overview](#1-overview)
2. [Motion Tokens Used](#2-motion-tokens-used)
3. [Element Inventory](#3-element-inventory)
4. [Frame-by-Frame Timeline](#4-frame-by-frame-timeline)
5. [State Definitions](#5-state-definitions)
6. [Reduced-Motion Variant](#6-reduced-motion-variant)
7. [CTA Hover / Focus Interaction](#7-cta-hover--focus-interaction)
8. [Responsive Choreography](#8-responsive-choreography)
9. [Accessibility Annotations](#9-accessibility-annotations)
10. [Design QA Checklist](#10-design-qa-checklist)
11. [Token Reference](#11-token-reference)

---

## 1. Overview

The Hero section is the first thing a visitor sees on the Grainlify landing page. Its entry animation should:

- Convey **trust and momentum** through ordered, purposeful motion
- Never block the user's ability to **read, click, or tab** through interactive elements
- Respect `prefers-reduced-motion` with a zero-motion fallback (opacity-only, no transforms, no stagger)
- Use only tokens from `motionConfig.ts` to ensure consistency with the rest of the design system

The choreography introduces elements from **top to bottom** in the same reading order:

```
Badge (Sparkles / "Web3 Contributors Platform")
  ↓
Headline (h1 — "Connect with Open Source Opportunities")
  ↓
Subheadline / Description (p)
  ↓
CTA Buttons (Get Started + Docs)
  ↓
Stat Strip (Active Projects / Contributors / Grants Distributed)
```

---

## 2. Motion Tokens Used

All values are sourced from `frontend/src/shared/config/motionConfig.ts`.

| Token | Value | Usage |
|---|---|---|
| `durations.fast` | 150 ms | CTA hover micro-interaction |
| `durations.normal` | 300 ms | Each element fade-in duration |
| `durations.slow` | 500 ms | Stat card entrance |
| `easing.easeOut` | `[0, 0, 0.2, 1]` | All entrance transitions |
| `easing.easeInOut` | `[0.4, 0, 0.2, 1]` | CTA glow pulse |
| `interactions.buttonHover.scale` | `1.02` | CTA primary hover scale |
| `interactions.buttonHover.duration` | 150 ms | CTA hover transition |
| `list.staggerDelay` | 50 ms | Stagger between badge → headline → sub → CTA |
| `list.initialDelay` | 100 ms | Delay before first element |
| `responsive.sm.durationMultiplier` | 0.75 | Mobile: shorten all durations by 25 % |
| `responsive.sm.staggerDelay` | 30 ms | Mobile: tighter stagger |
| `responsive.md.durationMultiplier` | 0.90 | Tablet: 10 % shorter |
| `responsive.md.staggerDelay` | 40 ms | Tablet: slightly tighter stagger |

---

## 3. Element Inventory

| # | Element | Role | ARIA notes |
|---|---------|------|------------|
| A | Section wrapper | Layout root | `<section>` — no extra ARIA needed |
| B | Orb 1 (top-left) | Decorative background | `aria-hidden="true"` |
| C | Orb 2 (bottom-right) | Decorative background | `aria-hidden="true"` |
| D | Badge | Sub-label | Single node; do **not** split into multiple `aria-label` nodes |
| E | Headline `<h1>` | Primary heading | Rendered as one `<h1>`; gradient `<span>` is purely visual |
| F | Description `<p>` | Supporting copy | Static text |
| G | CTA primary ("Get Started") | Call-to-action link | `focus-visible` must be immediately available, **not gated** by animation complete |
| H | CTA secondary ("Docs") | Secondary action | Same as G |
| I | Stat card × 3 | Data strip | Each card is a `<div>` with no interactive role — no keyboard focus |

---

## 4. Frame-by-Frame Timeline

### Desktop (≥ 1024 px) — full durations

```
T + 0 ms    Page mounts. All hero elements hidden (opacity: 0, y: 24px).
            Orbs visible immediately (decorative, no animation gate).

T + 100 ms  [D] Badge fades in + slides up from y:24 → y:0
            duration: 300 ms, easing: easeOut

T + 200 ms  [E] Headline fades in + slides up from y:24 → y:0
            duration: 300 ms, easing: easeOut
            (stagger offset: 100 ms after badge = list.staggerDelay × 2)

T + 300 ms  [F] Description fades in + slides up from y:24 → y:0
            duration: 300 ms, easing: easeOut

T + 400 ms  [G+H] CTA group fades in + slides up from y:24 → y:0
            duration: 300 ms, easing: easeOut
            Keyboard focus becomes available at T+0 (not animation-gated).

T + 600 ms  [I × 3] Stat cards enter with their own sub-stagger:
              Card 1: T+600 ms, duration 500 ms
              Card 2: T+650 ms, duration 500 ms
              Card 3: T+700 ms, duration 500 ms
            y: 16 → 0 (shorter distance = less visual noise for data)
            easing: easeOut
```

**Total choreography window**: ~1 200 ms (last card fully settled)

### Stagger table

| Element | Delay (ms) | Duration (ms) | y offset |
|---------|-----------|---------------|---------|
| Badge | 100 | 300 | 24 px |
| Headline | 200 | 300 | 24 px |
| Description | 300 | 300 | 24 px |
| CTA group | 400 | 300 | 24 px |
| Stat card 1 | 600 | 500 | 16 px |
| Stat card 2 | 650 | 500 | 16 px |
| Stat card 3 | 700 | 500 | 16 px |

---

## 5. State Definitions

### 5.1 `initial` (pre-animation)

All animated elements share the same initial state:

```ts
initial: {
  opacity: 0,
  y: 24,        // badge, headline, description, CTA group
  // OR
  y: 16,        // stat cards (shorter travel distance)
}
```

The orb divs are **not** animated — they appear immediately and rely on CSS `animate-pulse`.

### 5.2 `animating`

Elements transition from `initial` toward `animate` using the timeline above. No interaction is blocked during animation — focus is available from T+0.

### 5.3 `settled`

```ts
animate: {
  opacity: 1,
  y: 0,
  transition: {
    duration: motionConfig.durations.normal / 1000,   // 0.3 s
    ease: motionConfig.easing.easeOut,
  },
}
```

### 5.4 `reduced-motion` (static)

See [Section 6](#6-reduced-motion-variant) for the full fallback specification.

---

## 6. Reduced-Motion Variant

When `useReducedMotion()` returns `true`:

- **No transforms** — `y` offsets are removed entirely
- **No stagger** — `staggerChildren` is set to `0`
- **Instant opacity** — duration is `motionConfig.durations.instant` (0 ms), which resolves to a single paint frame (effectively instant)
- **Orbs** — the CSS `animate-pulse` class is removed; orbs are rendered statically

### Variant object

```ts
// Reduced-motion container (stagger disabled)
const reducedContainer = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: {
      staggerChildren: 0,
      delayChildren: 0,
    },
  },
};

// Reduced-motion item (opacity only, no transform, duration 0)
const reducedItem = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: { duration: 0 },
  },
};
```

### Behavior comparison

| Property | Full motion | Reduced motion |
|---|---|---|
| Opacity | 0 → 1 | 0 → 1 (instant) |
| Transform Y | 24 px → 0 | none |
| Stagger delay | 100 ms | 0 ms |
| Orb pulse | CSS animate-pulse | Static (no class) |
| CTA hover scale | 1 → 1.02 | Disabled |
| CTA glow shadow | animated | Static (Tailwind only) |

---

## 7. CTA Hover / Focus Interaction

### Primary CTA ("Get Started")

On `whileHover` and `:focus-visible`:

| Property | Value | Token |
|---|---|---|
| Scale | 1.02 | `interactions.buttonHover.scale` |
| Duration | 150 ms | `interactions.buttonHover.duration` |
| Easing | `easeOut` | `easing.easeOut` |
| Box shadow | `0 0 24px rgba(201,152,58,0.55)` | gold accent `#c9983a` |

The gold glow is an additive layer on top of the existing Tailwind `hover:shadow-[#c9983a]/50` — it deepens the halo effect.

On `whileTap`:

| Property | Value | Token |
|---|---|---|
| Scale | 0.97 | `interactions.buttonTap.scale` ≈ 0.97 |
| Duration | 100 ms | `interactions.buttonTap.duration` |

### Secondary CTA ("Docs")

On `whileHover`:

| Property | Value |
|---|---|
| Border color | `rgba(201,152,58,0.3)` (via Tailwind `hover:border-[#c9983a]/30`) |
| Scale | 1.01 (subtle, half the primary scale) |
| Duration | 150 ms |

### Focus-visible ring

Both CTAs must show a browser-native `:focus-visible` outline. Do **not** suppress the outline. The existing `rounded-[16px]` naturally clips the focus ring — add `outline-offset: 2px` via `focus-visible:outline focus-visible:outline-2 focus-visible:outline-[#c9983a]` to reinforce the gold accent in keyboard navigation.

### Reduced-motion CTA

When `useReducedMotion()` is `true`:
- `whileHover` still applies the Tailwind `hover:*` classes (CSS only, no JS animation)
- Framer Motion `whileHover` scale is set to `1` (no scale change)
- `whileTap` scale is set to `1`

---

## 8. Responsive Choreography

| Breakpoint | Duration multiplier | Stagger delay | Y offset |
|---|---|---|---|
| Mobile (< 768 px) | × 0.75 | 30 ms | 16 px |
| Tablet (768–1023 px) | × 0.90 | 40 ms | 20 px |
| Desktop (≥ 1024 px) | × 1.00 | 50 ms | 24 px |

On mobile at 375 px:

- Badge appears at ~75 ms (100 × 0.75)
- Headline at ~150 ms
- Description at ~225 ms
- CTA at ~300 ms
- Stat cards from ~450 ms with 22 ms sub-stagger
- Total choreography window: ~900 ms

The shorter travel distance (16 px on mobile vs 24 px on desktop) reduces spatial disruption on small viewports where content reflows more dramatically.

---

## 9. Accessibility Annotations

### ARIA integrity

- The animated `<h1>` is a **single DOM node** wrapping the full headline. The gradient `<span>` is visually styled only and does not fragment the heading for screen readers.
- The badge is a `<div>` with no `role` change — screen readers announce it as generic text. No ARIA label is needed.
- Stat cards are non-interactive. They require no `role="region"` or landmark unless wrapped in a navigation context.

### Keyboard focus order

```
Tab 1 → [G] "Get Started" CTA (Link)
Tab 2 → [H] "Docs" CTA (external link — rel="noopener noreferrer")
```

Focus must be available at page load **without waiting for animations to finish**. Framer Motion's `initial` state does not remove elements from the tab order — `opacity: 0` does not prevent focusability in most browsers. Verify with keyboard walkthrough that Tab reaches the CTA at T+0, regardless of animation state.

> If a screen reader announces elements before they are visible (opacity 0), that is acceptable per WCAG 1.4.1. Visibility is a presentation concern.

### Color contrast

| Text | Background | Ratio | Passes AA |
|---|---|---|---|
| `#e8dfd0` (light) on dark bg | ~`#1a1610` | ≥ 7:1 | ✅ AAA |
| `#2d2820` (dark) on light bg | ~`#f5f0e8` | ≥ 8:1 | ✅ AAA |
| `#b8a898` (subhead dark) on dark bg | ~`#1a1610` | ≥ 4.5:1 | ✅ AA |
| `#7a6b5a` (subhead light) on light bg | ~`#f5f0e8` | ≥ 4.5:1 | ✅ AA |
| Gold gradient text on dark bg | N/A | Decorative heading accent — does not carry meaning |

### Animation safety

- Total choreography duration (≤ 1 200 ms) is well below the WCAG 2.3.1 three-flash threshold.
- No looping or rapidly flashing content exists in the hero. The CSS `animate-pulse` on orbs is a slow sinusoidal opacity pulse (2 s cycle) — safe for photosensitivity.

---

## 10. Design QA Checklist

### Visual / Motion QA

- [ ] Badge slides up smoothly and settles without overshoot (easeOut, not spring)
- [ ] Headline enters 100 ms after badge — stagger is perceptible but not jarring
- [ ] Description and CTA group continue the cascade without gaps > 200 ms
- [ ] Stat cards enter with the 500 ms entrance, giving them a distinct "landing" feel
- [ ] On `prefers-reduced-motion`, all elements appear simultaneously with no movement
- [ ] Orb pulse is removed when `prefers-reduced-motion` is active

### Keyboard / Accessibility QA

- [ ] Tab key reaches "Get Started" immediately on page load (no animation gate)
- [ ] Focus ring is visible on CTA buttons in both light and dark themes (gold `#c9983a` outline)
- [ ] Screen reader (VoiceOver/NVDA) reads badge → heading → description → buttons in order without duplicate announcements
- [ ] Heading `<h1>` is announced as a single phrase ("Connect with Open Source Opportunities")

### Contrast QA

- [ ] Headline `#e8dfd0` on dark background ≥ 4.5:1 ratio
- [ ] Subhead `#b8a898` on dark background ≥ 4.5:1 ratio
- [ ] Headline `#2d2820` on light background ≥ 4.5:1 ratio
- [ ] Subhead `#7a6b5a` on light background ≥ 4.5:1 ratio

### Responsive QA (375 px viewport)

- [ ] Hero fits within viewport without horizontal overflow
- [ ] Orbs hidden on < sm breakpoint to prevent overflow (already implemented via `hidden sm:block`)
- [ ] Stagger choreography completes within ~900 ms on mobile
- [ ] Y offset is 16 px on mobile, not 24 px

---

## 11. Token Reference

```ts
// Sourced from frontend/src/shared/config/motionConfig.ts
import { motionConfig } from '@/shared/config/motionConfig';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

// Duration values (milliseconds → seconds for Framer Motion)
motionConfig.durations.fast        // 150 ms → 0.15 s
motionConfig.durations.normal      // 300 ms → 0.3 s
motionConfig.durations.slow        // 500 ms → 0.5 s

// Easing
motionConfig.easing.easeOut        // [0, 0, 0.2, 1]
motionConfig.easing.easeInOut      // [0.4, 0, 0.2, 1]

// Button interaction
motionConfig.interactions.buttonHover.scale      // 1.02
motionConfig.interactions.buttonHover.duration   // 150
motionConfig.interactions.buttonTap.scale        // 0.95 (for primary); 0.97 is acceptable

// Stagger
motionConfig.list.staggerDelay     // 50 ms → 0.05 s
motionConfig.list.initialDelay     // 100 ms → 0.1 s

// Responsive multipliers
motionConfig.responsive.sm.durationMultiplier    // 0.75
motionConfig.responsive.sm.staggerDelay          // 30 ms
motionConfig.responsive.md.durationMultiplier    // 0.90
motionConfig.responsive.md.staggerDelay          // 40 ms
```

---

*This spec is hand-off ready. Implementation PR: see Hero.tsx changes accompanying this document.*
