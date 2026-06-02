# BrowsePage Responsive Grid & Collapsible Filter Drawer

## Overview

Redesign the BrowsePage filter sidebar and project grid layout to be fully responsive across all breakpoints. The current persistent horizontal filter row collapses poorly on small screens, leading to horizontal overflow and unusable controls on mobile.

---

## Breakpoints & Grid Spec

### Column Definitions

| Breakpoint | Min Width | Grid Columns | Gap | Max Card Width |
|------------|-----------|--------------|-----|----------------|
| **sm**     | 0         | 1            | 16px | 100%          |
| **md**     | 768px     | 2            | 20px | ~360px        |
| **lg**     | 1024px    | 4            | 24px | ~280px        |
| **xl**     | 1280px    | 5            | 24px | ~240px        |

### Grid CSS (Tailwind v4)

```tsx
<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 xl:grid-cols-5 gap-4 md:gap-5">
```

---

## Filter Layout Strategy

### Desktop (lg+ / ≥1024px) — Inline Dropdown Row
- Filter dropdowns displayed as a horizontal row (current behavior, works well at ≥1024px)
- Each dropdown opens a popover/menu below the trigger
- 4 filter types: Languages, Ecosystems, Categories, Tags

### Tablet & Mobile (sm–md / <1024px) — Collapsible Drawer
- Filters hidden behind a **floating filter trigger button** (FAB)
- Tapping opens a **slide-in drawer** from the right (overlay, not push)
- Same 4 filter sections shown in an accordion layout inside the drawer
- Drawer has a semi-transparent backdrop to maintain context
- Scrollable independently if content overflows

#### Drawer Spec
| Property | Value |
|----------|-------|
| Width | 85vw (max 400px) |
| Backdrop | rgba(0,0,0,0.5), blur(4px) |
| Entry animation | slide-in-right (300ms ease-out) |
| Exit animation | slide-out-right (200ms ease-in) |
| Z-index | 50 |
| Close triggers | X button, backdrop click, Escape key |

#### Filter Trigger Button (FAB)
| Property | Value |
|----------|-------|
| Position | Fixed bottom-right (bottom: 24px, right: 24px) |
| Size | 56×56px (w-14 h-14) |
| Icon | SlidersHorizontal (or Filter) from lucide-react |
| Badge | Shows count of active filters when > 0 |
| Shadow | shadow-xl + hover:shadow-2xl |
| Visible | Only on sm and md breakpoints (hidden on lg+) |
| Z-index | 40 |

---

## Active Filter Chips Strip

**Visible on all breakpoints** below the search bar.

### Behavior
- Each selected filter value renders as a chip/tag with an X dismiss button
- Wraps to multiple lines when needed (flex-wrap)
- On mobile, chips are also shown so user can see and clear active filters without opening the drawer
- Same visual style as current implementation

---

## Component States

### Filter Dropdown Trigger Button (desktop) / Drawer Sections (mobile)

| State | Visual |
|-------|--------|
| Default | Glassmorphism bg, 1.5px border, gold accent on hover |
| Hover | scale(1.05), slightly brighter bg, shadow increase |
| Focus (keyboard) | 3px ring `rgba(201,152,58,0.3)`, outline-offset-2 |
| Active | Selected bg `#a17932`/`#b8872f` (dark/light), white text |
| Disabled | opacity-50, cursor-not-allowed, no hover effects |
| Loading | Skeleton shimmer placeholder |
| Empty | "No options found" message centered |
| Error | Toast notification (not inline) |

### Project Card States

| State | Visual |
|-------|--------|
| Default | Glassmorphism card, 18px radius, 1.5px border |
| Hover | bg brightens 4%, shadow `0 8px 24px rgba(201,152,58,0.15)`, cursor pointer |
| Focus (keyboard) | 3px gold ring, outline-offset-2 |
| Active | scale(0.98) on click, duration 100ms |
| Loading | SkeletonLoader component (currently exists) |
| Empty | Full-width centered message "No projects found" |
| Error | Full-width error state with retry button |

---

## Accessibility (WCAG 2.1 AA)

### Contrast Ratios
| Element | Ratio | Check |
|---------|-------|-------|
| Body text (#f5f5f5 on dark / #2d2820 on light) | ≥ 7:1 | AAA |
| Gold accent (#c9983a) on dark bg (#1a1512) | ~4.8:1 | AA |
| Muted text (#d4d4d4 on dark / #7a6b5a on light) | ≥ 4.5:1 | AA |
| UI borders (3:1 minimum) | ≥ 3:1 | AA |

### Keyboard Navigation
- All filter triggers are `<button>` elements (natively focusable)
- **Tab order**: Search bar → Filter FAB/dropdowns → Grid → Pagination (if any)
- Drawer traps focus when open (focus stays within drawer)
- Escape key closes drawer
- Arrow keys navigate dropdown option lists
- Each chip's X button is a `<button>` with `aria-label="Remove {filterName}"`

### ARIA Attributes
| Element | Attribute | Value |
|---------|-----------|-------|
| Filter drawer | `role="dialog"` | — |
| Filter drawer | `aria-modal="true"` | — |
| Filter drawer | `aria-label="Filters"` | — |
| Filter trigger FAB | `aria-expanded` | `true`/`false` |
| Filter trigger FAB | `aria-controls` | `filter-drawer` |
| Drawer close button | `aria-label` | `"Close filters"` |
| Filter chips (dismiss) | `aria-label` | `"Remove {value}"` |
| Dropdown listbox | `role="listbox"` | — |
| Dropdown option | `role="option"` | — |
| Dropdown option | `aria-selected` | `true`/`false` |
| Loading skeleton | `aria-hidden="true"` | — |
| Loading grid container | `aria-busy="true"` | — |
| Empty state | `role="status"` | — |

### Focus Order Diagram (Mobile)
```
1. Search input
   ↓ Tab
2. Filter FAB (or dropdowns on desktop)
   ↓ Tab (or Arrow keys inside drawer)
3. Project Grid cards (first card)
   ↓ Tab through remaining cards
4. (Optional) Load more / pagination
```

---

## Implementation Plan

### Files to Modify
1. **`frontend/src/features/dashboard/pages/BrowsePage.tsx`**
   - Add responsive grid classes
   - Add filter drawer state & logic (isFilterDrawerOpen)
   - Render filter FAB (hidden on lg+)
   - Render filter drawer as a portal (visible on sm/md)
   - Move filter dropdown rendering into conditional: inline row (lg+) / drawer (sm/md)
   - Keep active filter chips strip unconditional

2. **`frontend/src/shared/components/ui/Dropdown.tsx`**
   - Add `variant` prop: `"popover"` (desktop, current) or `"accordion"` (mobile, inside drawer)
   - Accordion variant: click header to expand/collapse, inline checkbox options
   - No change to filtering logic — only presentation

3. **`frontend/src/features/dashboard/components/ProjectCard.tsx`**
   - Ensure card content doesn't overflow on narrow screens (use `min-w-0`, `truncate`, responsive font sizes)
   - Make stats row (contributors/issues/PRs) responsive — switch to horizontal or compact layout on smallest sizes
   - Already uses `line-clamp-2` for description — verify it works at 1-col

4. **`frontend/src/features/dashboard/components/ProjectCardSkeleton.tsx`**
   - No changes needed — responsive via parent grid

### New Code Structure

#### Filter Drawer Component (inside BrowsePage.tsx or extracted)
```tsx
function FilterDrawer({ isOpen, onClose, sections, selectedFilters, onToggle, searchTerms, onSearchChange, openDropdown, setOpenDropdown }: FilterDrawerProps) {
  // Accordion-style filter sections
  // Renders as a portal when isOpen === true
  // Traps focus
  // Closes on Escape, backdrop click, or X button
}
```

#### Filter FAB
```tsx
{/* Visible below lg breakpoint */}
<div className="fixed bottom-6 right-6 z-40 lg:hidden">
  <button
    onClick={() => setIsFilterDrawerOpen(true)}
    className="w-14 h-14 rounded-full bg-gradient-to-br from-[#c9983a] to-[#b8872f] shadow-xl hover:shadow-2xl flex items-center justify-center transition-all hover:scale-105 active:scale-95"
    aria-label="Open filters"
    aria-expanded={isFilterDrawerOpen}
    aria-controls="filter-drawer"
  >
    <Filter className="w-6 h-6 text-white" />
    {activeFilterCount > 0 && (
      <span className="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-red-500 text-white text-[10px] font-bold flex items-center justify-center">
        {activeFilterCount}
      </span>
    )}
  </button>
</div>
```

### Edge Cases
| Case | Handling |
|------|----------|
| Zero filters selected | FAB shows no badge; drawer shows all sections collapsed initially |
| Many filters (overflow) | Chips wrap; drawer accordion sections scrollable; max-h with overflow-y |
| Empty projects list | Full-width centered message independent of breakpoint |
| API error | Error state with retry button; filters still operable |
| Long filter option text | text-ellipsis overflow-hidden in option display |
| Reduced motion | Respect `prefers-reduced-motion`: slide animations become instant |
| Touch targets | All interactive elements ≥44×44px (WCAG 2.5.8) |
| RTL | Not required for v1 (no current RTL support) |

---

## Testing / QA Checklist

### Responsive
- [ ] 375px (mobile): 1-col grid, FAB visible, drawer opens/closes, chips wrap
- [ ] 640px (sm): 1-col grid, FAB visible
- [ ] 768px (md): 2-col grid, FAB visible
- [ ] 1024px (lg): 4-col grid, inline filter row, no FAB
- [ ] 1280px (xl): 5-col grid
- [ ] Verify no horizontal scroll at any breakpoint
- [ ] Verify grid gap is consistent

### Accessibility
- [ ] Tab through all controls in logical order
- [ ] Enter/Space open dropdowns and the drawer
- [ ] Escape closes dropdowns and drawer
- [ ] Focus trapped inside open drawer
- [ ] Focus returns to trigger button when drawer closes
- [ ] Screen reader reads drawer title, filter options, and chip dismissals
- [ ] Contrast ratio ≥ 4.5:1 for all text elements
- [ ] Contrast ratio ≥ 3:1 for all UI components and borders

### States
- [ ] Loading: skeleton cards shown
- [ ] Empty: "No projects found" message
- [ ] Error: error message + retry
- [ ] Filter active: chips shown, FAB shows badge count
- [ ] Filter drawer open/close animation fluid

---

## Design Tokens Reference

All colors, spacing, border-radius, shadows, and animation durations should use the existing tokens from:
- `frontend/src/styles/theme.css` (CSS custom properties)
- `design-tokens.json` (canonical JSON)

Key token mappings for this feature:

| Context | Variable | Tailwind Class |
|---------|----------|---------------|
| Card bg (dark) | `--card` | `bg-card` |
| Card bg (light) | `rgba(255, 255, 255, 0.35)` | `bg-white/[0.35]` |
| Accent | `--color-primary-600` | `bg-[#c9983a]` |
| Gold text | `--color-primary-600` | `text-[#c9983a]` |
| Border | `--border` | `border-border` |
| Shadow md | `--shadow-md` | `shadow-md` |
| Shadow xl | `--shadow-xl` | `shadow-xl` |
| Duration fast | `--duration-fast` (150ms) | — |
| Duration normal | `--duration-normal` (300ms) | — |
| Radius card | `--radius-2xl` (16px) | `rounded-[18px]` |
| Backdrop blur | `--glass-blur` (40px) | `backdrop-blur-[40px]` |

---

## Changelog

| Date | Version | Change |
|------|---------|--------|
| 2026-06-01 | 1.0 | Initial responsive grid spec, filter drawer pattern, a11y annotations |
