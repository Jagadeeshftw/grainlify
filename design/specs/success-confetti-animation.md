# Success Confetti Animation Spec

## Overview
When a contributor wins a bounty, we want to introduce a celebratory moment via a confetti/success-burst animation without blocking or impeding the user's primary workflow. This specification outlines the particle behavior, timing, accessibility requirements, and a `prefers-reduced-motion` fallback based on Framer Motion.

## 1. Visual & Particle Behavior

### Color Palette
Particles draw exclusively from the gold and warm-neutral palettes defined in `/design-tokens.json` to maintain a professional, trustworthy feel aligned with the Stellar ecosystem:
- **Gold/Accent**: `#f1b400` (primary-500), `#c9983a` (primary-600), `#a67c2e` (primary-700)
- **Warm-Neutral**: `#d6d3d1` (neutral-300), `#a8a29e` (neutral-400)

### Configuration
- **Particle Count (Desktop/Tablet)**: 60 - 80 particles
- **Particle Count (Mobile <= 375px)**: 25 - 30 particles (tuned down to prevent performance jank)
- **Duration**: 2.5s - 3s total animation lifespan
- **Trigger Point**: Fires exactly when the success toast or Reward Certificate modal enters the viewport
- **Auto-clear**: After animation completion, the particle layer must unmount or zero out to clear the DOM

## 2. Interaction & Usability

### Non-blocking Interaction
- The confetti container **must** have `pointer-events: none;` applied.
- Confetti particles must **never** intercept clicks, hover states, or keyboard focus from underlying targets (e.g., links, buttons, form fields).

## 3. Accessibility (WCAG 2.1 AA)

### `prefers-reduced-motion` Fallback
This animation must respect system-level user preferences. Utilizing `useReducedMotion` (from `frontend/src/shared/hooks/useReducedMotion.ts`):
- **If `prefers-reduced-motion` is true**: The confetti particle motion is entirely bypassed.
- **Fallback**: Display a single static celebratory badge or illustration (e.g., a golden laurel wreath or static starburst icon) alongside the success text. There should be absolutely zero particle motion.

### Contrast & Independence
- **Contrast**: The success text and toast messages must maintain a minimum of 4.5:1 contrast against their background. The confetti animation sits behind or around these messages but must not drop the contrast ratio of the text below the required threshold at any point during its lifecycle.
- **Communication**: The success state must be fully communicated via the text/toast independently. The animation is purely decorative and must never be the sole indicator of success.

## 4. State Definitions

1. **Triggered**: The moment the success criteria is met (e.g., bounty won). The system calculates viewport size to determine particle count.
2. **Animating**: The confetti bursts. `pointer-events` are disabled.
3. **Settled/Cleared**: The animation ends. The DOM nodes for the particles are completely removed.
4. **Reduced-Motion-Static**: The alternative state when `prefers-reduced-motion` is active. A single static celebratory badge is rendered instantly with no animation, bypassing states 1-3.

## 5. Testing & Validation Checklist

- [ ] **Design QA**: Verify that the accompanying success text/toast meets 4.5:1 contrast independently of the animated background.
- [ ] **Keyboard Walkthrough**: Tab through the interface while the animation plays. Ensure the confetti layer never intercepts focus or click targets underneath it.
- [ ] **Responsive Review**: Validate that the particle count is reduced on mobile viewports (< 375px width) to ensure smooth performance without jank.
- [ ] **Token Validation**: Confirm all particle colors match the approved `/design-tokens.json` values.
