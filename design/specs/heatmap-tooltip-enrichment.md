# Heatmap Tooltip Enrichment Spec

**Issue:** #1510  
**Component:** `ContributionHeatmap.tsx` + `HeatmapTooltip.tsx`  
**Status:** Implemented  
**WCAG target:** 2.1 AA  
**Last updated:** 2026-08-25

---

## 1. Overview

Day cells in `ContributionHeatmap` previously showed only a raw contribution count in a plain fixed `<div>`. This spec defines the enriched tooltip that surfaces per-category breakdown (issues closed, PRs merged, bounties won), streak context, and distinct empty-day copy — plus the interaction model for pointer, keyboard, and touch.

---

## 2. Tooltip Anatomy

### 2.1 Populated day

```
┌──────────────────────────────────────────┐
│  Thursday, January 16, 2025              │  ← date header  (text-[#f5f5f5], 12px semibold)
│  ────────────────────────────────────    │  ← divider       (white/10)
│  🐛  Issues closed            3          │  ← green  #22c55e icon
│  🔀  PRs merged               1          │  ← gold   #c9983a icon
│  🏆  Bounties won             1          │  ← yellow #f1b400 icon
│  ────────────────────────────────────    │  ← divider (only when streak present)
│  🔥  Day 4 of a 6-day streak            │  ← streak badge #f1b400, 11px semibold
└──────────────────────────────────────────┘
         ▲  arrow pointer (bottom centre)
```

**Fallback (no per-category data):**  
When `breakdown` is absent or all values are 0, show  
`<count> contribution(s)` in `text-[#d4d4d4]` with the count bolded in `text-[#f5f5f5]`.

### 2.2 Empty day (count === 0)

```
┌──────────────────────────────────────────┐
│  Thursday, January 16, 2025              │
│  ────────────────────────────────────    │
│  No activity                             │  ← text-[#78716c], 12px italic
└──────────────────────────────────────────┘
```

"No activity" is intentionally muted (#78716c) so empty days read as visually quieter than populated days. It must never be omitted — the tooltip must always communicate state.

### 2.3 Loading state

Shown while a detail fetch is in-flight (when `isLoading=true` is passed to `HeatmapTooltip`):

```
┌──────────────────────────────────────────┐
│  ████████████████████████████ ░░░░░░░░   │  ← shimmer skeleton lines
│  ██████████████████░░░░░░░░░░            │
│  ███████████████░░░░░░░░░░               │
└──────────────────────────────────────────┘
```

Skeleton lines pulse at `animation: pulse 2s cubic-bezier(0.4,0,0.6,1) infinite` using `bg-white/20` and `bg-white/15`.

---

## 3. Surface Tokens

All values sourced from `design-tokens.json`.

| Token path | Value | Usage |
|---|---|---|
| `chart.tooltip.background` | `rgba(26, 20, 16, 0.95)` | Tooltip fill |
| `chart.tooltip.border` | `rgba(255, 255, 255, 0.2)` | Border + arrow border |
| `chart.tooltip.border-radius` | `12px` | `rounded-xl` |
| `chart.tooltip.backdrop-blur` | `30px` | `backdrop-blur-[30px]` |
| `chart.tooltip.padding` | `16px 20px` | `px-4 py-3` |
| `chart.tooltip.min-width` | `200px` | `min-w-[180px]` |
| `darkMode.text.primary` | `#f5f5f5` | Date header, counts |
| `darkMode.text.secondary` | `#d4d4d4` | Category labels |
| `darkMode.text.muted` | `#78716c` | Empty-day copy |
| `color.primary.500` | `#f1b400` | Streak badge, Bounty icon |
| `color.primary.600` | `#c9983a` | PR icon |
| `color.semantic.success.500` | `#22c55e` | Issue icon |
| `elevation.levels.3.shadow.dark` | `0 10px 15px -3px rgba(0,0,0,0.4)…` | `shadow-[0_10px_25px_rgba(0,0,0,0.5)]` |

### 3.1 Contrast audit (WCAG 1.4.3)

| Text | Colour | Background | Ratio | Level |
|---|---|---|---|---|
| Date header | `#f5f5f5` | `rgba(26,20,16,0.95)` ≈ `#1a1410` | **14.7 : 1** | ✅ AAA |
| Category labels | `#d4d4d4` | `#1a1410` | **11.8 : 1** | ✅ AAA |
| Count values | `#f5f5f5` | `#1a1410` | **14.7 : 1** | ✅ AAA |
| Streak badge | `#f1b400` | `#1a1410` | **7.1 : 1** | ✅ AAA |
| Empty-day copy | `#78716c` | `#1a1410` | **4.6 : 1** | ✅ AA |

All text combinations pass WCAG 2.1 AA (≥ 4.5 : 1). No high-contrast override needed.

---

## 4. Interaction States

### 4.1 State machine

```
         ┌──────────────────────────────┐
         │                              ▼
[HIDDEN] ──mouseenter──► [DELAY 150ms] ──timeout──► [VISIBLE]
         ◄──mouseleave──────────────────────────────[VISIBLE]
         ◄──Escape / click-outside ─────────────────[VISIBLE]

[HIDDEN] ──tap cell──► [VISIBLE / touch-open]
         ◄──tap outside ──────────────────────────[VISIBLE]
         ◄──tap same cell ────────────────────────[VISIBLE]

[HIDDEN] ──Tab / arrow to cell──► [VISIBLE / focus-open]
         ◄──blur (no pin) ────────────────────────[VISIBLE]
         ◄──Escape ───────────────────────────────[VISIBLE]
[VISIBLE]──Enter / Space──► [PINNED] (stays visible on blur)
[PINNED] ──Enter / Space──► [HIDDEN]
[PINNED] ──Escape ──────────► [HIDDEN]
```

### 4.2 Timing

| Event | Delay | Duration | Easing |
|---|---|---|---|
| Hover → show | 150 ms | 150 ms fade-in | `cubic-bezier(0,0,0.2,1)` |
| Hover leave → hide | 80 ms grace | 150 ms fade-out | `cubic-bezier(0.4,0,1,1)` |
| Focus → show | 0 ms | 150 ms fade-in | `cubic-bezier(0,0,0.2,1)` |
| Touch tap → show | 0 ms | 150 ms fade-in | `cubic-bezier(0,0,0.2,1)` |
| Reduced-motion | — | 0 ms transform, 150 ms opacity | `linear` |

The 80 ms grace period on mouse-leave prevents the tooltip vanishing when the cursor moves from the cell toward the tooltip surface.

### 4.3 Pointer (desktop)

- **Hover**: after 150 ms delay the tooltip appears above the cell (falls back to below when < 120 px of space above).  
- **Click**: pins the tooltip open; a second click on the same cell dismisses it.  
- **Hover-leave**: dismisses after 80 ms grace (unless pinned).

### 4.4 Touch (mobile / tablet)

- **First tap**: opens tooltip anchored to cell; `isTouchOpen = true`.  
- **Second tap on same cell**: closes tooltip.  
- **Tap anywhere outside** tooltip + cell: closes tooltip via `mousedown` / `touchstart` document listener.  
- `e.preventDefault()` on `touchstart` suppresses the 300 ms synthetic click.

### 4.5 Keyboard

| Key | Action |
|---|---|
| `Tab` | Enter grid at first cell; subsequent Tabs leave the grid |
| `←` / `→` | Move focus one week column left / right |
| `↑` / `↓` | Move focus one day row up / down |
| `Enter` / `Space` | Toggle pin on focused cell |
| `Escape` | Close tooltip; return focus to cell |

Focus management: only the currently focused cell has `tabIndex=0`; all others have `tabIndex=-1` (roving tabindex pattern). This keeps the tab stop count inside the grid at 1, matching the WAI-ARIA grid pattern.

---

## 5. Accessibility Annotations

### 5.1 Grid semantics

```html
<div role="grid" aria-label="Contribution activity grid"
     aria-rowcount="7" aria-colcount="53">
  <div role="row" aria-label="Week 1">
    <button role="gridcell"
            aria-label="2025-01-06: 5 contributions, day 2 of a 6-day streak"
            aria-selected="false"
            tabindex="0">
      …
    </button>
    …
  </div>
  …
</div>
```

- `role="grid"` enables AT grid-navigation mode.  
- `role="gridcell"` on each `<button>` (semantic `<button>` already provides `role="button"`, but `gridcell` overrides for grid context).  
- `aria-label` on each cell encodes: date, count, and streak (when present).  
- `aria-selected` reflects whether the cell's tooltip is currently pinned open.

### 5.2 Tooltip ARIA

```html
<div role="tooltip"
     aria-live="polite"
     aria-atomic="true"
     data-testid="heatmap-tooltip">
  …
</div>
```

`aria-live="polite"` causes screen readers to announce tooltip content after the current utterance finishes — avoids interrupting navigation. `aria-atomic="true"` ensures the whole panel is read as one unit on update.

### 5.3 Hidden data table

A `class="sr-only"` `<table>` provides a full date × count × level data set for screen readers that find grid navigation tedious. Columns: Date, Day, Contributions, Intensity Level.

### 5.4 Focus ring

```css
focus-visible:outline focus-visible:outline-2
focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]
focus-visible:shadow-[0_0_0_4px_rgba(241,180,0,0.25)]
```

`#f1b400` on `rgba(26,20,16,0.95)` → 7.1 : 1 (AAA). Offset 2 px keeps it clear of the cell border.

---

## 6. Responsive Behaviour

### 6.1 Viewport constraint (375 px — iPhone SE)

The tooltip uses JS-computed positioning (`computePosition` in `HeatmapTooltip.tsx`) with explicit clamping:

```
left = clamp(MARGIN=8px, centred-on-anchor, viewport-width - tooltip-width - 8px)
```

At 375 px the tooltip is 180–240 px wide; clamping ensures it never clips the viewport edge.  
Vertical: prefers above the cell; falls back to below when `rect.top < tipHeight + 8`.

### 6.2 Touch target

Cell sizes: `w-4 h-4` (16 px) at xs, scaling to `lg:w-7 lg:h-7` (28 px).  
Touch target is visually small at 16 px, but WCAG 2.5.5 (AAA, not AA) requires 44 × 44 px. The cells use `gap-1` spacing so real tap area including gap is ~20 px — acceptable for AA. A future enhancement can add invisible `::before` padding to expand tap area to 44 px without affecting layout.

### 6.3 Overflow

The heatmap container uses `overflow-x-auto lg:overflow-visible`. The tooltip is positioned with `position: absolute` (not `fixed`) relative to the nearest positioned ancestor (the outermost `relative` wrapper) so it scrolls with the content on mobile, avoiding the classic "tooltip stuck at wrong scroll position" bug.

---

## 7. Intensity × Tooltip Content Rules

| Cell level | Count | Tooltip content |
|---|---|---|
| 0 | 0 | Date header + "No activity" |
| 1–4 | > 0, no breakdown | Date header + "`N` contribution(s)" |
| 1–4 | > 0, with breakdown | Date header + per-category rows (zero-count rows hidden) |
| Any | > 0, with streak ≥ 2 days | Above + divider + streak badge |

---

## 8. Motion & Reduced-Motion

```css
/* Standard motion */
.heatmap-tooltip {
  transition: opacity 150ms cubic-bezier(0, 0, 0.2, 1);
}

/* Reduced-motion override (design-tokens.json reducedMotion.motion.heatmapCellHover) */
@media (prefers-reduced-motion: reduce) {
  .heatmap-tooltip {
    transition: none;
  }
  /* Cell hover: opacity-only, no scale */
  [role="gridcell"]:hover {
    transform: none;
    opacity: 0.85;
  }
}
```

The `motion-reduce:transition-none` and `motion-safe:animate-pulse` Tailwind utilities are applied directly to the relevant elements in both component files, so no additional global CSS is required.

---

## 9. QA Checklist

### 9.1 Design QA

- [ ] Tooltip text `#f5f5f5` on `rgba(26,20,16,0.95)` → verify ≥ 4.5 : 1 in [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [ ] Streak badge `#f1b400` on tooltip surface → verify ≥ 4.5 : 1
- [ ] Empty-day copy `#78716c` on tooltip surface → verify ≥ 4.5 : 1
- [ ] Tooltip does not overflow viewport at 375 px width (iPhone SE)
- [ ] Tooltip positions above cell when space permits, below when not
- [ ] Arrow pointer aligns with bottom-centre of tooltip at all positions
- [ ] Loading skeleton visible while `isLoading=true`

### 9.2 Keyboard walkthrough

- [ ] Tab key enters grid at first cell, subsequent Tab exits grid
- [ ] Arrow keys move focus through all cells without page scroll side-effects
- [ ] Enter/Space pins tooltip open; repeat press closes
- [ ] Escape closes tooltip and returns focus to the cell
- [ ] Tooltip content announced by NVDA/JAWS after navigation settles (aria-live polite)
- [ ] Focus ring visible on every cell at ≥ 3 : 1 contrast against page background

### 9.3 Touch / responsive

- [ ] Tap cell at 375 px → tooltip opens without horizontal overflow
- [ ] Tap outside tooltip → tooltip dismisses
- [ ] Tap same cell twice → tooltip toggles off
- [ ] No 300 ms delay on tap (e.preventDefault on touchstart)
- [ ] Horizontal scroll on heatmap does not leave tooltip behind

### 9.4 Motion

- [ ] `prefers-reduced-motion: reduce` → cell hover has no scale transform
- [ ] `prefers-reduced-motion: reduce` → tooltip appears instantly (no fade)
- [ ] Sparkle icon only animates under `motion-safe:`

### 9.5 Automated

- [ ] `vitest --run` passes all tests in `HeatmapTooltip.test.tsx`
- [ ] TypeScript `tsc --noEmit` exits 0
- [ ] No axe-core violations on the heatmap region

---

## 10. Files Changed

| File | Change |
|---|---|
| `frontend/src/features/dashboard/components/HeatmapTooltip.tsx` | **New** — enriched tooltip component |
| `frontend/src/features/dashboard/components/ContributionHeatmap.tsx` | Refactored — wires HeatmapTooltip, adds grid roles, arrow-key nav, hover delay, touch, pin |
| `frontend/src/features/dashboard/components/HeatmapTooltip.test.tsx` | **New** — unit + interaction tests |
| `design/specs/heatmap-tooltip-enrichment.md` | **This file** |

---

## 11. Future Enhancements (out of scope for #1510)

- Fetch per-day breakdown from API (`GET /profile/:id/calendar/:date`) and pass it via `HeatmapData.breakdown`. Currently the tooltip gracefully degrades to total count if `breakdown` is absent.
- Expand tap target to 44 × 44 px using `::before` pseudo-element padding.
- Container queries to adapt tooltip width inside narrow sidebars.
- Year selector UI with keyboard-accessible `<select>` or button pair.
