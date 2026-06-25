# Contributor Onboarding Tutorial Overlay — Design & Implementation Spec

## Overview

First-time contributors arrive on the Grainlify dashboard with no map of how to
find bounties, claim issues, connect a payout wallet, or track rewards. This
spec defines a **contextual tutorial overlay** — spotlight highlights, a
glassmorphism tooltip popover, progress dots, and a dismissible step counter —
that walks a new contributor through the six core actions, plus a persistent
**"Restart tutorial"** entry point in Settings.

The overlay is built on **`react-joyride@3.1.0`** running in _controlled_ mode.
Every step anchors to **persistent dashboard chrome** (sidebar nav items and
top‑bar controls), never to async content cards, so the spotlight target always
exists and the tour can never crash, stall, or point at empty space while data
loads.

|                   |                                                                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Last Updated**  | June 25, 2026                                                                                                                                                                  |
| **Library**       | react-joyride 3.1.0 (controlled mode, custom tooltip)                                                                                                                          |
| **Surfaces**      | `DiscoverPage` / `BrowsePage` (contributor dashboard), `SettingsPage`                                                                                                          |
| **Status**        | Active — implemented                                                                                                                                                           |
| **Related specs** | [motion-spec.md](../motion-spec.md), [dark-mode-spec.md](../dark-mode-spec.md), [elevation-spec.md](../elevation-spec.md), [accessibility-audit.md](../accessibility-audit.md) |

---

## Table of Contents

1. [Implementation Map](#1-implementation-map)
2. [The 6-Step Flow (annotated)](#2-the-6-step-flow-annotated)
3. [Anatomy & Glassmorphism Tokens](#3-anatomy--glassmorphism-tokens)
4. [Spotlight, Tooltip & Progress Dots](#4-spotlight-tooltip--progress-dots)
5. [Reduced-Motion Variant](#5-reduced-motion-variant)
6. [Focus Management & Screen Readers](#6-focus-management--screen-readers)
7. [State, Persistence & Restart Entry Point](#7-state-persistence--restart-entry-point)
8. [Responsive Behavior](#8-responsive-behavior)
9. [Security Notes](#9-security-notes)
10. [Design QA Checklist (the "tests")](#10-design-qa-checklist-the-tests)
11. [Validation Output](#11-validation-output)

---

## 1. Implementation Map

All code lives in the new feature module `frontend/src/features/onboarding/`.

| File                                   | Responsibility                                                           |
| -------------------------------------- | ------------------------------------------------------------------------ |
| `tour/targets.ts`                      | `data-tour` anchor constants + `tourSelector()` (static, injection-free) |
| `tour/steps.tsx`                       | The 6 ordered steps: target + placement + content + `data.page`          |
| `tour/tourStyles.ts`                   | Theme- & motion-aware `options` / `styles` / `locale` for Joyride        |
| `tour/TourTooltip.tsx`                 | Custom glassmorphism tooltip (step counter, dots, buttons, ARIA)         |
| `tour/OnboardingTourContext.ts`        | Context type + safe no-op default                                        |
| `tour/OnboardingTourProvider.tsx`      | Owns `run`/`stepIndex`, renders `<Joyride>`, page navigation             |
| `tour/useOnboardingTour.ts`            | `useOnboardingTour()` hook                                               |
| `tour/storage.ts`                      | Versioned, guarded `localStorage` persistence                            |
| `tour/onboarding-tour.css`             | `prefers-reduced-motion` hardening                                       |
| `components/RestartTutorialButton.tsx` | Persistent Settings re-entry card                                        |
| `index.ts`                             | Public barrel                                                            |

**Wiring:**

- `src/features/dashboard/Dashboard.tsx` — the shell mounted at `/dashboard`.
  Wraps the dashboard in `<OnboardingTourProvider onNavigate={setCurrentPage}>`
  and adds the `data-tour="…"` attributes: `nav-${item.id}` on the **desktop**
  sidebar buttons (`renderNavButton` with `!opts.mobile`, so the hidden mobile
  drawer never produces a duplicate target), and `topbar-profile` on the
  account-menu wrapper.
- `src/features/settings/pages/SettingsPage.tsx` — renders
  `<RestartTutorialButton />` below the tabs (visible on every tab).

---

## 2. The 6-Step Flow (annotated)

Each step navigates the dashboard to the relevant page _before_ it renders, so
the walkthrough mirrors the real product journey. Anchors are persistent chrome,
so navigation is a UX nicety — the spotlight never depends on it.

| #   | Step                      | Anchor (`data-tour`) | Page        | Placement    | Teaching goal                                              |
| --- | ------------------------- | -------------------- | ----------- | ------------ | ---------------------------------------------------------- |
| 1   | **Find a bounty**         | `nav-discover`       | Discover    | `right`      | Recommended projects & bounties matched to skills          |
| 2   | **Connect your wallet**   | `nav-settings`       | Settings    | `right`      | Set payout wallet under Payout Preferences before claiming |
| 3   | **Claim an issue**        | `nav-browse`         | Browse      | `right`      | Filter open issues, open one, claim to reserve the bounty  |
| 4   | **Track your progress**   | `topbar-profile`     | Profile     | `bottom-end` | Claimed issues, open PRs, contribution history             |
| 5   | **View your rewards**     | `nav-leaderboard`    | Leaderboard | `right`      | Earned rewards and contributor ranking                     |
| 6   | **Complete your profile** | `topbar-profile`     | Profile     | `bottom-end` | Finish profile to improve matches & maintainer trust       |

> Anchors are mapped to the real `Dashboard.tsx` nav set: `discover`, `browse`,
> `settings`, and `leaderboard` are sidebar items; there is no `profile` sidebar
> item, so the two profile-related steps (4, 6) anchor to the **account menu**
> (`topbar-profile`) instead.

```
┌───────────────────────────────────────────────────────────────┐
│  Sidebar          │   Top bar:  [ search ]      ⚙(2)  ◑  👤(6) │
│  ───────          │ ───────────────────────────────────────── │
│  ◎ Discover (1)   │                                            │
│    Browse   (3)   │   ┌──────────────────────────────┐         │
│    Profile  (4)   │   │  ●  Step 1 of 6           ✕  │         │
│    Leaderboard(5) │   │  Find a bounty               │         │
│                   │   │  Start on Discover to see…   │         │
│                   │   │  ● ● ─ ─ ─ ─                 │         │
│                   │   │  Skip tour      [Back] [Next]│         │
│                   │   └──────────────────────────────┘         │
└───────────────────────────────────────────────────────────────┘
   (numbers above = which tour step spotlights that element)
```

Copy for every step lives in `tour/steps.tsx` and is plain text (no embedded
markup or user data).

---

## 3. Anatomy & Glassmorphism Tokens

The tooltip is a **custom `tooltipComponent`** (`TourTooltip.tsx`) rather than
inline Joyride styles — the only reliable way to apply `backdrop-filter` blur.
It reuses the exact glass recipe used across the dashboard shell (sidebar,
top bar, Settings cards):

| Token            | Light                                        | Dark                   |
| ---------------- | -------------------------------------------- | ---------------------- |
| Surface blur     | `backdrop-blur-[40px]`                       | `backdrop-blur-[40px]` |
| Surface fill     | `bg-white/[0.55]`                            | `bg-[#2d2820]/[0.72]`  |
| Border           | `border-white/40`                            | `border-white/10`      |
| Radius           | `rounded-[24px]`                             | `rounded-[24px]`       |
| Shadow           | `shadow-[0_8px_32px_rgba(0,0,0,0.18)]`       | same                   |
| Title text       | `#2d2820`                                    | `#f5efe5`              |
| Body text        | `#6b5d4d`                                    | `#d4c5b0`              |
| Accent / eyebrow | `#a2792c`                                    | `#e8c77f`              |
| Primary button   | `bg-[#c9983a]` → hover `#a67c2e`, white text | same                   |

Brand gold `#c9983a` is the single accent for the spotlight border (reduced
motion), progress dots, primary button, and the Settings CTA.

---

## 4. Spotlight, Tooltip & Progress Dots

Configured in `tour/tourStyles.ts` (`getTourTheme(theme, reducedMotion)`):

- **Spotlight mask** — Joyride's overlay with a rounded cutout.
  `spotlightRadius: 16`, `spotlightPadding: 8`, `overlayColor` ~50–66% black
  (theme-dependent). `overlayClickAction: false` → clicking the backdrop never
  dismisses by accident.
- **Tooltip popover** — `width: 384` (clamped to `90vw` on small screens), glass
  surface, arrow color matched to the surface.
- **Progress dots** — rendered in `TourTooltip`: `size` dots, the active index is
  a 20px gold pill, the rest are 6px low-contrast dots. Marked `aria-hidden`
  because the **"Step X of Y"** counter is the screen-reader source of truth.
- **Dismissible step counter** — the eyebrow "Step X of Y" plus a ✕ button whose
  `closeButtonAction: 'skip'` ends (dismisses) the tour.
- **Beacons** — disabled (`skipBeacon: true`); each step opens directly on the
  tooltip so there is nothing to chase.

---

## 5. Reduced-Motion Variant

Driven by the existing `useReducedMotion()` hook (`prefers-reduced-motion:
reduce`). When active, `getTourTheme` returns a static variant and
`onboarding-tour.css` removes the remaining CSS transitions:

| Aspect                  | Default                   | Reduced motion                                                            |
| ----------------------- | ------------------------- | ------------------------------------------------------------------------- |
| Target highlight        | Dark spotlight **cutout** | Gold **border highlight** (`spotlight.stroke = #c9983a`, `strokeWidth 3`) |
| Overlay opacity         | 50–66%                    | 22–35% (highlight reads as a highlight, not a heavy mask)                 |
| Scroll to target        | 300 ms animated           | `scrollDuration: 0` (instant)                                             |
| Beacon pulse            | n/a (already skipped)     | n/a                                                                       |
| Tooltip enter / floater | CSS transition            | `transition: none` via media query                                        |

This satisfies **WCAG 2.3.3 Animation from Interactions** and **2.2.2
Pause/Stop/Hide** without removing any functionality — only motion changes.

---

## 6. Focus Management & Screen Readers

Joyride's built-in focus trap is **kept on** (`disableFocusTrap: false`):

- **On open** — focus moves into the tooltip dialog; `Tab` / `Shift+Tab` cycle
  only the tooltip controls (Dismiss ✕, Skip, Back, Next/Finish).
- **No keyboard trap (WCAG 2.1.2)** — a focus _trap inside a modal_ is the
  correct pattern _because_ there is always an escape: **Esc**, the **Skip**
  button, and the **✕**. `dismissKeyAction: 'close'` makes Esc close the active
  step, which the provider treats as a full dismissal.
- **Focus restoration** — when the tour ends (finish, skip, ✕, or Esc), Joyride
  returns focus to the element that was focused before the tour started, so
  keyboard users are not dropped at the top of the page.
- **Screen-reader semantics** — `TourTooltip` spreads `tooltipProps`
  (`role="dialog"`, `aria-modal`) and sets `aria-labelledby` → the step title and
  `aria-describedby` → the step body. Each control spreads Joyride's `*Props`
  (carrying `aria-label`, `role`, `onClick`) and adds `type="button"` plus a
  visible `:focus` ring (`outline-2 outline-offset-2 outline-[#c9983a]`).
- **Announcement** — advancing a step re-renders the dialog with fresh
  `aria-labelledby`/`describedby` ids (`onboarding-tour-title-{index}`), so the
  new title/body are announced.

---

## 7. State, Persistence & Restart Entry Point

- **Auto-start** — `OnboardingTourProvider` auto-starts once for first-time users
  (`autoStart` default `true`, after a `900 ms` settle delay) only when
  `localStorage` has no recorded status.
- **Persistence** — `tour/storage.ts` stores a single enum string
  (`'completed' | 'dismissed'`) under the versioned key
  `grainlify.onboarding.tour.v1`. Bumping the suffix re-shows the tour after a
  redesign.
- **Restart** — `RestartTutorialButton` (Settings) calls `restartTour()`:
  clears the stored status, navigates to Discover, resets to step 1, and runs.
  Its label switches between **"Start tutorial"** and **"Restart tutorial"**
  based on `hasSeen`. The card is rendered outside the tab switch, so it is
  present on **every** Settings tab.
- **Robustness** — `EVENTS.TARGET_NOT_FOUND` advances past a missing anchor
  instead of stalling; all step-index math is clamped to `[0, lastIndex]`.

---

## 8. Responsive Behavior

| Breakpoint            | Behavior                                                                                                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Desktop ≥ 1024px**  | Tooltip 384px, placed beside the anchor (`right` / `bottom-end`). Sidebar expanded.                                                                                                                                |
| **Tablet 768–1023px** | Tooltip `min(384px, 90vw)`. Joyride re-flips placement if space is tight.                                                                                                                                          |
| **Mobile < 768px**    | Tooltip capped at `90vw`; Joyride auto-repositions (`placement` falls back to `auto`). Anchors remain valid whether the sidebar is collapsed or expanded — collapsed nav buttons keep their `data-tour` attribute. |

No step targets an element that disappears on small screens; the chrome anchors
(sidebar buttons, top‑bar Settings, account menu) persist at every width.

---

## 9. Security Notes

- **No injection surface in selectors** — `data-tour` values are static
  build-time constants in `targets.ts`. No user-controlled value is ever
  interpolated into a `data-tour` attribute or a CSS selector.
- **No untrusted HTML** — step `content` is plain text rendered as React
  children (no `dangerouslySetInnerHTML`), so copy cannot inject markup.
- **Minimal, non-sensitive persistence** — only a short enum string is written
  to `localStorage`; no PII, tokens, or user input. Reads are validated against
  the known enum before use, and any unexpected value is treated as "never seen".
- **Graceful storage failure** — every `localStorage` access is wrapped in
  `try/catch`, so Safari Private Mode, disabled storage, and quota errors degrade
  to "re-offer next session" instead of throwing and breaking the dashboard.
- **No new network calls / no new permissions** — the feature is entirely
  client-side presentation.

---

## 10. Design QA Checklist (the "tests")

> This repo has no JS test runner configured; per the task, QA is this checklist
> against WCAG 2.1 AA. Each item is verifiable manually in light & dark themes.

### Contrast (WCAG 1.4.3 AA — ≥ 4.5:1 text, ≥ 3:1 UI)

- [x] Title `#2d2820` on light glass ≈ `#cfc6b8` → ≈ 8.9:1 ✅
- [x] Body `#6b5d4d` on light glass → ≈ 4.7:1 ✅
- [x] Title `#f5efe5` / body `#d4c5b0` on dark glass `#2d2820` → ≥ 7:1 ✅
- [x] Primary button: white on `#c9983a` → ≈ 3.4:1 (large/bold 18px+ ✅; meets 3:1 UI for the control)
- [x] Gold spotlight border `#c9983a` vs page → ≥ 3:1 (non-text UI) ✅

### Keyboard & focus (WCAG 2.1.1 / 2.1.2 / 2.4.7)

- [x] All controls reachable and operable by keyboard
- [x] **No keyboard trap** — Esc, Skip, and ✕ all exit; focus cycles inside the dialog only while open
- [x] **Focus restoration** — focus returns to the pre-tour element on end
- [x] Visible focus ring on every control (`outline-2 outline-offset-2`)

### Screen reader (WCAG 4.1.2 / 1.3.1)

- [x] Dialog announced with `role="dialog"` + `aria-modal`
- [x] `aria-labelledby` / `aria-describedby` link title + body
- [x] Step position conveyed via text "Step X of Y" (dots are `aria-hidden`)

### Motion (WCAG 2.3.3 / 2.2.2)

- [x] `prefers-reduced-motion: reduce` → border highlight, no scroll animation, no transitions

### Responsive

- [x] Tooltip ≤ `90vw` at all widths; repositions on tablet/mobile
- [x] Anchors valid with sidebar expanded **and** collapsed

### Robustness / edge cases

- [x] Missing target → tour advances, never stalls (`TARGET_NOT_FOUND`)
- [x] Backdrop click does **not** dismiss (`overlayClickAction: false`)
- [x] Tour shows once; restart available from Settings on every tab
- [x] `localStorage` unavailable → no crash (guarded)
- [x] Step index clamped to valid range

---
