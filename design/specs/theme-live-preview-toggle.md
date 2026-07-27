# Theme Live-Preview Toggle Specification

## Overview
This document specifies the visual behavior, anatomy, and animation characteristics of the `ThemeContext` live-preview toggle control. It defines the implementation details distinct from the core dark mode matrix (see [`design/dark-mode-spec.md`](../dark-mode-spec.md)).

## 1. Toggle Control Anatomy
The toggle uses a 3-segment switch track design to support Light, Dark, and System preferences.

- **Layout**: A pill-shaped segmented control with a sliding active-indicator (thumb).
- **Icons**: 
  - Light: ☀️ (Sun icon, e.g., `Sun` from Lucide)
  - Dark: 🌙 (Moon icon, e.g., `Moon` from Lucide)
  - System: 💻 (Monitor icon, e.g., `Monitor` from Lucide)
- **Track Surface**: `bg-surface-secondary-dark` (`#2d2820`) in dark mode, or `bg-neutral-200` (`#e7e5e4`) in light mode.
- **Thumb (Active State Indicator)**: Opaque rounded rectangle behind the active icon.
- **Persisted-Preference Indicator**: A small "System" label appears underneath the toggle when following the OS setting (e.g., "System (Dark)").

## 2. Interactive States

| State | Track Background | Thumb Background | Active Icon Color | Inactive Icon Color |
|-------|------------------|------------------|-------------------|---------------------|
| **Light-Active** | `#e7e5e4` (neutral-200) | `#ffffff` | `#f59e0b` (warning-500) | `#a8a29e` (neutral-400) |
| **Dark-Active** | `#2d2820` (surface-secondary) | `#3a3428` (surface-tertiary) | `#f1b400` (focusRing / primary-500) | `#8b7a6a` (muted) |
| **System-Active** | Adapts to resolved theme | Adapts to resolved theme | Adapts to resolved theme | Adapts to resolved theme |
| **Toggling (Mid-Animation)** | Inherits starting theme | Transitions to target theme | Cross-fade opacity | Cross-fade opacity |

### Validation against `design-tokens.json`
- **Dark-Active Icon Contrast**: `#f1b400` on `#3a3428` provides > 4.5:1 contrast.
- **Dark-Active Track Contrast**: `#2d2820` vs background `#1a1714` is distinct.
- **Light-Active Icon Contrast**: `#f59e0b` on `#ffffff` is > 3:1 (minimum for UI components).

## 3. Page-Wide Swap Animation

### Default Experience (Motion Safe)
When the theme is toggled, a page-wide cross-fade animation is applied to backgrounds, surfaces, and text to prevent harsh flashing.

- **Properties**: `background-color`, `border-color`, `color`, `fill`, `stroke`
- **Duration**: `300ms` (matches `motion.durations.normal` token)
- **Easing**: `cubic-bezier(0.4, 0, 0.2, 1)` (matches `motion.easing.easeOut` token)
- **Implementation**: Append a transient `theme-transitioning` class to `<body>` during the swap to enforce transitions globally, then remove it to prevent hovering from triggering delays.

### Reduced-Motion Fallback (Accessibility)
For users with vestibular disorders (`prefers-reduced-motion: reduce`), or when the `"reduced-motion"` theme variant is active.

- **Behavior**: Instant swap. No cross-fade.
- **Duration**: `0ms`
- **Easing**: N/A

## 4. Accessibility Annotations

- **Role**: The toggle must be implemented as a `role="radiogroup"` with three `role="radio"` buttons, or a customized `role="switch"` if constrained to 2 states. Since we have 3 states (Light/Dark/System), a **Radio Group** is semantically correct.
- **ARIA Label**: The radio group must have `aria-label="Theme preference"`. Each button must have an `aria-label` (e.g., "Light theme", "Dark theme", "System theme").
- **State Selection**: The currently active button must have `aria-checked="true"`.
- **Screen Reader Announcement**: Upon change, use `aria-live="polite"` to announce the new state: "Theme changed to dark mode".
- **Keyboard Navigation**:
  - `Tab` moves focus into the radio group.
  - `Arrow Left` / `Arrow Right` cycles selection.
  - `Space` or `Enter` selects the focused option.
- **Focus Ring**: Must display the 2px `focusRing` outline when navigated via keyboard.

## 5. Engineering Guidelines: Flash Prevention

To prevent a "flash of wrong theme" (FOWT) on initial load:
1. Include a **blocking inline script** in the `<head>` of the HTML document.
2. The script must synchronously read the `localStorage` key (e.g., `theme`) or evaluate `window.matchMedia('(prefers-color-scheme: dark)')` if set to system.
3. Immediately append the resolved class (`dark`, `high-contrast`, etc.) to the `<html>` element before the React bundle parses and paints the DOM.

## 6. Design QA Checkpoints

- [ ] **Contrast Check**: Verify the active thumb icon meets 4.5:1 against the thumb background in both themes.
- [ ] **Keyboard Walkthrough**: Tab to toggle, cycle modes with arrow keys, verify `aria-live` announcement triggers.
- [ ] **Responsive Review**: Ensure the 3-segment toggle fits comfortably in the collapsed mobile navigation menu at 375px width (minimum touch target 44x44px per segment).
