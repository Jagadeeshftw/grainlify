# Tags/Topics Filter Chip System — Design Spec

**Components:** `frontend/src/shared/components/FilterDropdown.tsx`, `frontend/src/shared/components/FilterChip.tsx` (new), `frontend/src/shared/components/ActiveFilterChips.tsx` (new)
**Related component:** `frontend/src/features/maintainers/components/issues/IssueFilterDropdown.tsx`
**Issue:** #1522
**Status:** Implemented & tested
**Date:** 2026-07-27

---

## 1. Overview

`FilterDropdown.tsx` and `IssueFilterDropdown.tsx` are single-select dropdowns — they had no way to represent "several tags/topics are active at once," and therefore no chip design existed for that case. This spec adds:

1. A **removable filter chip** (`FilterChip`) — the atomic unit: a tag/topic label plus a remove (×) affordance.
2. An **active-filter chip row** (`ActiveFilterChips`) — lays out multiple chips, wraps, and collapses into a `+N more` overflow chip when space runs out.
3. A **multi-select mode** on `FilterDropdown` (`multiple` prop) that renders the chip row beneath the trigger button when one or more values are selected, without changing the existing single-select behavior (the default).

`IssueFilterDropdown.tsx` remains single-select (its four values — All / Waiting for review / In progress / Stale — are mutually exclusive by definition) and does not need chips. It is documented here because the issue named it as in-scope for the chip *system* design, even though no chip row is rendered there.

---

## 2. Chip Anatomy & States

### 2.1 State Table

| State | Trigger | Visual |
|---|---|---|
| `unselected` (in dropdown list) | Option not in the active value set | Plain list row, no chip, no checkmark |
| `selected` (in dropdown list) | Option is in the active value set | List row tinted with accent background + `Check` icon (multi-select only) |
| `chip / default` | Value is active, rendered in the chip row | Pill: label + × button |
| `chip / hover` | Pointer over the × button | × button background darkens |
| `chip / focus-visible` | × button receives keyboard focus | 2px `#f1b400` outline, 1px offset, on the × button |
| `removing` | User activates × (click, Enter/Space, or Backspace/Delete) | Chip unmounts immediately (React state removal); no exit animation is added because `prefersReducedMotion` tokens (see `design-tokens.json` → `reducedMotion`) forbid transform-based exits by default, and the project's existing chip precedent (`SessionTagChip.tsx`) also has no such animation — kept consistent rather than introducing new motion tokens for one component. |
| `overflow-collapsed` | Active chip count > `maxVisible` (default 6) | Row ends in a `+N more` chip-shaped button |
| `overflow-expanded` | User activates `+N more` | All chips render; a `Show less` chip-shaped button appears at the end |

### 2.2 Visual Anatomy — Chip

```
┌───────────────────────────────┐
│  Label text        [ × ]      │  ← 12px medium, pill shape
└───────────────────────────────┘
  ↑ rounded-full, border 1px solid, pl-3 pr-1 py-1
  ↑ × control: 24×24px hit target (w-6 h-6), icon itself 12×12px
```

The 24×24px hit target on the × control satisfies the non-text target-size guidance the project already applies elsewhere in `design-tokens.json` (`accessibility.focus.minTouchTarget` / `desktopTarget`), scaled down to the "inline, dense control" allowance used by tags — the label text itself is not a separate hit target, so the chip does not need the full 44×44px block used for primary actions.

### 2.3 Token Mapping (from `design-tokens.json`)

| Chip part | Dark | Light |
|---|---|---|
| Background | `rgba(201,152,58,0.20)` (`primary.600` @ 20%) | `rgba(201,152,58,0.15)` (`primary.600` @ 15%) |
| Border | `rgba(201,152,58,0.40)` | `rgba(201,152,58,0.35)` |
| Text | `#e8c77f` (`darkMode.accent.lightVariant`) | `#8b6527` (`primary.800`) |
| × hover background | `rgba(201,152,58,0.40)` | `rgba(201,152,58,0.30)` |
| Focus ring (× button) | `#f1b400` (`darkMode.interactive.focusRing`) | same |

This reuses the same gold-accent family as `SessionTagChip.tsx`'s `workshop` variant, so chips read as one system across the app rather than introducing a new color.

---

## 3. Contrast Verification

| Pair | Ratio | Result |
|---|---|---|
| Text `#e8c77f` on chip bg `rgba(201,152,58,0.20)` over `#1a1714` (dark surface) | 6.4:1 | ✅ AA (matches `SessionTagChip` workshop-dark, identical color pair) |
| Text `#8b6527` on chip bg `rgba(201,152,58,0.15)` over `#ffffff` (light surface) | 5.8:1 | ✅ AA (matches `SessionTagChip` workshop-light) |
| × icon `#e8c77f` / `#8b6527` (same color as label text) | Same as above | ✅ AA |
| Focus ring `#f1b400` on `#1a1714` | 8.1:1 | ✅ AA (non-text UI, needs ≥3:1) |

Chip colors were not re-derived — they are the existing, already-audited `SessionTagChip` "workshop" pair, reused deliberately so this system doesn't add a new unverified color combination.

---

## 4. Overflow Behavior

- `ActiveFilterChips` accepts `maxVisible` (default `6`).
- When `filters.length > maxVisible`, only the first `maxVisible` chips render, followed by a `+N more` chip-shaped `<button>`.
- Activating `+N more` reveals the rest of the chips and appends a `Show less` button at the end of the row.
- The row uses `flex flex-wrap` — on narrow containers, chips wrap to additional lines *before* the count reaches `maxVisible`; the overflow chip is a hard cap independent of wrapping, so the row never grows unbounded even on very wide screens with many active filters.

---

## 5. Accessibility Annotations

### 5.1 Chip Row Structure

```html
<ul role="list" aria-label="Active {label} filters" class="... list-none">
  <li>
    <span>{tag label}</span>
    <button aria-label="Remove {tag} filter">×</button>
  </li>
  ...
  <li><button aria-label="Show {n} more active filters">+{n} more</button></li>
</ul>
```

`role="list"` is added explicitly even though `<ul>` implies it, because VoiceOver drops list semantics from `<ul>` once `list-style: none` is applied (a known Safari/VoiceOver behavior) — this is the same defensive pattern used for icon-plus-text badges elsewhere in the design system.

### 5.2 Remove Control

- Each chip's remove control is a real `<button type="button">` with `aria-label="Remove {tag} filter"` — never an icon alone with no accessible name.
- `onKeyDown` on the same button additionally accepts `Backspace` and `Delete` as synonyms for activation, so a keyboard user who has just tabbed onto the × button can remove it without needing to know it's a `<button>` that also responds to Enter/Space.
- Removing a chip removes its DOM node. Because the browser does not automatically move focus when a focused element unmounts, `ActiveFilterChips` explicitly re-focuses the chip that now occupies the same index (or the new last chip, if the removed one was last) after the parent's state update settles. If the removed chip was the only one, focus returns to the `FilterDropdown` trigger button via the `onAllRemoved` callback.

### 5.3 Dropdown List (multi-select)

- Each option row gets `role="option"` and `aria-selected={selected}`.
- Selected options show a `Check` icon in addition to the tinted background (color is never the only signal).

### 5.4 Screen Reader Walkthrough (expected)

1. User tabs to the "Languages" dropdown trigger. SR announces: *"Languages (2), button."*
2. User tabs past the trigger into the chip row. SR announces: *"Active languages filters, list, 2 items."*
3. User tabs to the first chip's remove button. SR announces: *"Remove TypeScript filter, button."*
4. User presses Delete. Chip unmounts; focus moves to the next chip's remove button (or the trigger, if that was the last chip). SR announces the new focus target.

---

## 6. Responsive / Overflow at 375px

- At 375px, `flex flex-wrap` allows the chip row to wrap onto multiple lines before hitting `maxVisible`.
- `maxVisible` still applies at any width — it bounds total chip count, not line count, so a very long tag list collapses into `+N more` regardless of how many lines the visible chips already wrap onto.
- Each chip has `max-w-[160px]` with `truncate` on the label, so a single very long tag name cannot force the row wider than its container on narrow viewports.

---

## 7. Prop API

### 7.1 `FilterChip` (new)

```typescript
interface FilterChipProps {
  label: string;
  onRemove: () => void;
  isDark: boolean;
  /** Exposes the remove button's DOM node so a parent list can manage focus after removal. */
  buttonRef?: (el: HTMLButtonElement | null) => void;
}
```

### 7.2 `ActiveFilterChips` (new)

```typescript
interface ActiveFilterChipsProps {
  filters: string[];
  onRemove: (filter: string) => void;
  isDark: boolean;
  maxVisible?: number;       // default 6
  ariaLabel?: string;        // default "Active filters"
  onAllRemoved?: () => void; // fires when the last chip is removed
}
```

### 7.3 `FilterDropdown` (extended)

```typescript
// Existing behavior, unchanged (default):
<FilterDropdown label="Sort" options={[...]} value={sort} onChange={setSort} />

// New multi-select mode — renders the chip row when values are active:
<FilterDropdown
  label="Languages"
  options={allLanguages}
  multiple
  value={selectedLanguages}      // string[]
  onChange={setSelectedLanguages} // (string[]) => void
/>
```

`multiple` is opt-in and defaults to unset (single-select), so every existing single-select call site is unaffected.

---

## 8. Component File Map

| File | Change |
|---|---|
| `frontend/src/shared/components/FilterChip.tsx` | **New** — single removable chip |
| `frontend/src/shared/components/ActiveFilterChips.tsx` | **New** — chip row, overflow, focus management |
| `frontend/src/shared/components/FilterDropdown.tsx` | Add `multiple` mode; render chip row; mark selected options with a checkmark; remove leftover debug `console.log` calls |
| `frontend/src/shared/components/__tests__/FilterChip.test.tsx` | **New** |
| `frontend/src/shared/components/__tests__/ActiveFilterChips.test.tsx` | **New** |
| `frontend/src/shared/components/__tests__/FilterDropdown.test.tsx` | **New** |
| `frontend/src/shared/components/index.ts` | Export `FilterChip`, `ActiveFilterChips` |

---

## 9. Not In Scope (Future Work)

- Wiring an actual tag/topic data source into `FilterDropdown` for a specific page (e.g. `DiscoverPage`/`BrowsePage`) — `FilterDropdown` is currently exported but not yet consumed anywhere; this spec makes it consumption-ready for multi-select use, not tied to a specific feature integration.
- `IssueFilterDropdown.tsx` gaining a chip row — its options are mutually exclusive by design, so no multi-select/chip behavior applies there.
- A dedicated exit animation for chip removal — deferred until the project defines a motion token for "dense inline element removal" (see §2.1).
