# Sidebar Collapsed Micro-Icon State

**Branch:** `design/sidebar-collapsed-icons`
**Status:** Spec
**WCAG target:** 2.1 AA
**Last updated:** 2026-07-27

---

## 1. Overview

`AppShell.tsx` supports a collapsed icon-only sidebar mode (64px rail), but the collapsed state's micro-icon treatment — sizing, spacing, hover-label tooltip, active-route indicator, and the expand/collapse toggle — are not formally specified beyond the general `design/navigation-spec.md`. This spec defines the collapsed micro-icon state in isolation, providing redlines, token references, states, and accessibility annotations for hand-off to implementation.

### Scope

- Collapsed rail icon sizing, vertical spacing, and rhythm
- Hover/focus tooltip revealing the full label (positioned right, not obscuring adjacent content)
- Active-route indicator treatment distinct from expanded mode's text-based active state
- Expand/collapse toggle control's icon and position within the collapsed rail

### Out of scope

- Expanded sidebar state (covered by `design/navigation-spec.md`)
- Mobile drawer behaviour (covered by `design/navigation-spec.md`)
- Sidebar floating/inset variant (separate concern)

---

## 2. Collapsed Rail Anatomy

The collapsed rail is a 64px-wide vertical container fixed to the left edge of the viewport.

```
┌──────┐
│  [▶] │  ← expand toggle (44×44px hit target)
├──────┤
│      │
│  📊  │  ← nav icon (20×20px visual, 44×44px hit target)
│      │
│  📦  │  ← nav icon — badge appended to tooltip
│      │
│  🏆  │
│      │
│  ⚙️  │
│      │
│  📖  │  ← external link icon
│      │
└──────┘
```

### 2.1 Rail dimensions

| Property | Value | Token ref |
|---|---|---|
| Rail width | 64px (`w-16`) | `navigation.sidebar.widthCollapsed` |
| Rail background | `bg-gray-900` (#111827) | `color.neutral.900` |
| Rail border-right | 1px solid `border-gray-800` (#1f2937) | `color.neutral.800` |
| Rail z-index | 20 | `navigation.zIndex.sidebar` |

### 2.2 Vertical spacing rhythm

| Element | Top padding | Bottom padding | Gap between items |
|---|---|---|---|
| Expand toggle | `py-4` (16px from rail top) | — | — |
| First nav icon | 8px below toggle separator | — | — |
| Nav icon stack | — | — | `gap-1` (4px) between `<li>` elements |
| Rail bottom | — | `p-2` (8px) | — |

The nav container uses `flex flex-col gap-1 p-2 items-center` — consistent with the existing `AppShell.tsx` implementation.

---

## 3. Icon Sizing

| Property | Value | Notes |
|---|---|---|
| Icon visual size | 20×20px (`size={20}`) | Lucide icons, `aria-hidden="true"` |
| Icon hit target | 44×44px (`h-11 w-11`) | WCAG 2.5.5 minimum touch target |
| Icon container shape | Rounded square | `rounded-md` (6px border-radius) |
| Icon container padding | 0px (icon fills hit target) | `flex items-center justify-center` |

### 3.1 Icon colour mapping

| State | Icon colour | Background | Token ref |
|---|---|---|---|
| Default | `text-gray-300` (#d1d5db) | transparent | `color.neutral.300` |
| Hover | `text-white` (#ffffff) | `bg-gray-700` (#374151) | `color.neutral.700` |
| Focus-visible | `text-white` (#ffffff) | `bg-gray-700` (#374151) | `color.neutral.700` |
| Active (current page) | `text-gray-900` (#111827) | `bg-white` (#ffffff) | `color.neutral.900` on `#ffffff` |
| Active + hover | `text-gray-900` (#111827) | `bg-white` (#ffffff) | Same as active |
| Disabled | `text-gray-300` at 40% opacity | transparent | Exempt (disabled) |

---

## 4. Hover/Focus Tooltip

When the sidebar is collapsed, hovering or focusing a nav icon reveals a tooltip to the right of the icon. The tooltip is supplementary — the icon's `aria-label` is the primary accessible name.

### 4.1 Tooltip anatomy

```
                    ┌──────────────────────┐
                    │  Dashboard            │  ← tooltip content
                    └──────────────────────┘
                           ↑
  ┌──────┐  8px  ─────────┘
  │  📊  │  ← icon hit target (44×44px)
  └──────┘
```

### 4.2 Tooltip dimensions and positioning

| Property | Value | Notes |
|---|---|---|
| Position | `absolute left-full top-1/2 -translate-y-1/2 ml-2` | Right of icon, vertically centered |
| z-index | 60 | Above sidebar (20), below skip-nav (100) |
| Background | `bg-gray-800` (#1f2937) | `color.neutral.800` |
| Text colour | `text-white` (#ffffff) | 14.7:1 contrast on #1f2937 |
| Font size | `text-xs` (12px) | `typography.fontSize.xs` |
| Font weight | `font-medium` (500) | `typography.fontWeight.medium` |
| Padding | `px-2.5 py-1.5` (10px × 6px) | — |
| Border radius | `rounded-md` (6px) | `borderRadius.md` |
| Shadow | `shadow-lg` | `elevation.levels.3` |
| White space | `whitespace-nowrap` | Prevents line wrapping |
| Arrow | None | Clean flat tooltip |

### 4.3 Tooltip visibility trigger

| Trigger | Behaviour |
|---|---|
| `:hover` on icon wrapper | Tooltip fades in (`opacity-0 → opacity-100`, `transition-opacity`) |
| `:focus-within` on icon wrapper | Tooltip fades in (same transition) |
| Both removed | Tooltip fades out |
| Sidebar expanded | Tooltip hidden (`hidden={state !== "collapsed"}`) |
| Mobile viewport | Tooltip hidden (`hidden={isMobile}`) |

### 4.4 Tooltip content rules

| Nav item | Tooltip text |
|---|---|
| Standard item | Item name (e.g. "Dashboard") |
| Disabled item | `"ItemName (Soon)"` (e.g. "Programs (Soon)") |
| External link | Item name only (e.g. "Docs") — the `↗` indicator is omitted in collapsed mode |

---

## 5. Active-Route Indicator

In expanded mode, the active route is indicated by `bg-white text-gray-900 shadow-sm` on the full-width nav row. In collapsed mode, a distinct visual treatment is needed because the icon container is small and the full-row background change alone may not be sufficiently prominent.

### 5.1 Chosen treatment: Filled icon container + left accent bar

The active-route indicator in collapsed mode uses **two** signals:

1. **Filled background**: `bg-white text-gray-900` on the 44×44px icon container (existing behaviour)
2. **Gold accent bar**: A 3px-wide vertical bar on the left edge of the icon container, coloured with the brand gold accent

```
  ┃📊 ┃  ← active: gold bar (3px, #f1b400) + white bg + dark icon
  │📦│  ← inactive: no bar, transparent bg
  │🏆│
```

### 5.2 Accent bar specification

| Property | Value | Token ref |
|---|---|---|
| Width | 3px | — |
| Height | 20px (matches icon visual height) | — |
| Colour (light) | `#f1b400` | `color.primary.500` |
| Colour (dark) | `#c9983a` | `darkMode.semantic.accentPrimary` |
| Position | Left edge of icon container, vertically centred | `absolute left-0 top-1/2 -translate-y-1/2` |
| Border-radius | `rounded-full` (pill shape) | `borderRadius.full` |
| Contrast on `bg-gray-900` | 4.8:1 | Passes 3:1 for UI components (WCAG 1.4.11) |

### 5.3 Why not a gold dot

A gold dot (circle indicator) was considered but rejected because:
- It would occupy space within the 44×44px hit target, reducing the icon's visual breathing room
- The filled background + accent bar provides a clearer, more conventional "selected tab" pattern
- The accent bar aligns with the brand gold accent (`primary.500`) while remaining subtle

### 5.4 Active state visual comparison

| Mode | Active treatment |
|---|---|
| Expanded | Full-width row: `bg-white text-gray-900 shadow-sm` |
| Collapsed | 44×44px container: `bg-white text-gray-900` + 3px gold accent bar on left edge |

---

## 6. Expand/Collapse Toggle Control

The toggle button sits at the top of the collapsed rail, above the nav icons. It allows the user to expand the sidebar back to 288px.

### 6.1 Toggle anatomy

```
┌──────┐
│  [▶] │  ← ChevronRight icon, 18×18px
├──────┤  ← border separator (border-gray-800)
│  📊  │
```

### 6.2 Toggle specification

| Property | Value | Notes |
|---|---|---|
| Hit target | 44×44px (`h-11 w-11`) | WCAG 2.5.5 |
| Icon (collapsed) | `ChevronRight` (18×18px, `size={18}`) | Points right → "expand" |
| Icon (expanded) | `ChevronLeft` (18×18px, `size={18}`) | Points left → "collapse" |
| Icon colour | `text-gray-400` (#9ca3af) | 3.2:1 on `#111827` — meets 3:1 UI (1.4.11) |
| Hover colour | `text-white` (#ffffff) | — |
| Hover background | `bg-gray-700` (#374151) | — |
| Focus ring | `ring-2 ring-inset ring-gray-400` | 3.2:1 on `#111827` |
| `aria-label` | `"Expand sidebar"` (collapsed) / `"Collapse sidebar"` (expanded) | — |
| `aria-expanded` | `false` (collapsed) / `true` (expanded) | — |
| Position | Top of rail, centred horizontally | Inside `<div className="flex items-center border-b border-gray-800 px-3 py-4 justify-center">` |
| Transition | Background and colour transitions inherited from `transition-colors` | — |

### 6.3 Toggle placement in the rail

The toggle is the first focusable element in the collapsed rail. It sits inside the sidebar header `<div>`, which has `justify-center` when collapsed and a bottom border (`border-b border-gray-800`) separating it from the nav icons below.

When expanded, the toggle aligns to the right edge of the header row (next to the workspace name).

---

## 7. Design Token Mapping

All values cross-referenced against `design-tokens.json` for WCAG 2.1 AA compliance.

### 7.1 Colour tokens

| Element | Light token | Dark token | Contrast ratio |
|---|---|---|---|
| Rail background | `color.neutral.900` (#111827) | — | N/A (container) |
| Rail border | `color.neutral.800` (#1f2937) | — | N/A (decorative) |
| Default icon | `color.neutral.300` (#d1d5db) on #111827 | — | 9.5:1 ✅ |
| Hover icon | `#ffffff` on `color.neutral.700` (#374151) | — | 13.1:1 ✅ |
| Active icon | `color.neutral.900` (#111827) on `#ffffff` | — | 16:1 ✅ |
| Active accent bar | `color.primary.500` (#f1b400) on #111827 | `darkMode.semantic.accentPrimary` (#c9983a) on #111827 | 4.8:1 ✅ (UI component ≥3:1) |
| Tooltip text | `#ffffff` on `color.neutral.800` (#1f2937) | — | 14.7:1 ✅ |
| Toggle icon | `color.neutral.400` (#9ca3af) on #111827 | — | 3.2:1 ✅ (UI component ≥3:1) |
| Focus ring | `color.neutral.400` (#9ca3af) on #111827 | — | 3.2:1 ✅ (UI component ≥3:1) |

### 7.2 Spacing tokens

| Property | Token | Value |
|---|---|---|
| Rail width | `navigation.sidebar.widthCollapsed` | 64px |
| Icon hit target | `accessibility.focus.minTouchTarget` | 44×44px |
| Nav item gap | `spacing.1` | 4px |
| Rail padding | `spacing.2` | 8px |
| Tooltip offset from icon | `spacing.2` | 8px |
| Tooltip padding-x | `spacing[2.5]` | 10px |
| Tooltip padding-y | `spacing[1.5]` | 6px |

### 7.3 Border-radius tokens

| Element | Token | Value |
|---|---|---|
| Icon container | `borderRadius.md` | 6px |
| Tooltip | `borderRadius.md` | 6px |
| Accent bar | `borderRadius.full` | Pill (9999px) |

### 7.4 Typography tokens

| Element | Token | Value |
|---|---|---|
| Tooltip text | `typography.fontSize.xs` | 0.75rem / 12px |
| Tooltip line-height | `typography.fontSize.xs[1].lineHeight` | 1rem / 16px |
| Tooltip font-weight | `typography.fontWeight.medium` | 500 |

---

## 8. Accessibility Annotations

### 8.1 ARIA roles and attributes

| Element | Attribute | Value | Rationale |
|---|---|---|---|
| Icon `<Link>` / `<a>` | `aria-label` | Item name (e.g. "Dashboard") | Primary accessible name — always present regardless of visual label visibility |
| Icon `<Link>` / `<a>` | `aria-current` | `"page"` (when active) | Identifies current page to screen readers |
| Tooltip `<span>` | `role="tooltip"` | — | Declared tooltip role for assistive tech |
| Tooltip `<span>` | `aria-hidden` | Not set | Tooltip is visible to screen readers as supplementary context |
| Disabled icon wrapper | `aria-disabled` | `"true"` | Communicates disabled state |
| External link `<a>` | `aria-label` | `"ItemName (opens in new tab)"` | Includes context about new tab opening |
| Expand toggle `<button>` | `aria-label` | `"Expand sidebar"` | Describes action |
| Expand toggle `<button>` | `aria-expanded` | `"false"` | Communicates collapsed state |

### 8.2 Critical accessibility rule

**The tooltip is supplementary, not the only label source.** Every collapsed icon link retains its full `aria-label` attribute even when the visual label text is hidden. Screen reader users will hear the item name directly from the `aria-label` without needing the tooltip to appear.

### 8.3 Keyboard interaction

| Key | Action |
|---|---|
| `Tab` | Move focus to next interactive element (toggle → nav icons in source order) |
| `Shift+Tab` | Move focus to previous element |
| `Enter` / `Space` | Activate focused link or button |
| `Escape` | No action (tooltip is not dismissible — it follows focus/hover) |

- Focus order in collapsed rail: Toggle button → Dashboard icon → Programs icon → Bounties icon → Settings icon → Docs icon
- A visible focus ring (`ring-2 ring-inset ring-gray-400`) appears on each focused element
- The tooltip appears on `:focus-within` of the icon wrapper, ensuring keyboard users see the label

### 8.4 Contrast verification checklist

| Pair | Foreground | Background | Ratio | Requirement | Pass |
|---|---|---|---|---|---|
| Default icon on rail | #d1d5db | #111827 | 9.5:1 | 4.5:1 text | ✅ |
| Active icon on white | #111827 | #ffffff | 16:1 | 4.5:1 text | ✅ |
| Hover icon on gray-700 | #ffffff | #374151 | 13.1:1 | 4.5:1 text | ✅ |
| Tooltip text on gray-800 | #ffffff | #1f2937 | 14.7:1 | 4.5:1 text | ✅ |
| Gold accent bar on rail | #f1b400 | #111827 | 4.8:1 | 3:1 UI component | ✅ |
| Toggle icon on rail | #9ca3af | #111827 | 3.2:1 | 3:1 UI component | ✅ |
| Focus ring on rail | #9ca3af | #111827 | 3.2:1 | 3:1 UI component | ✅ |

### 8.5 Reduced motion

- Tooltip appearance uses `transition-opacity` only (no transform animation)
- Rail width collapse/expand uses `transition-[width]` — respected by `prefers-reduced-motion` via CSS
- Gold accent bar has no animation; it is a static visual indicator

---

## 9. States Reference

### 9.1 Default icon

```
  ┌──────────────┐
  │              │
  │     📊      │  ← text-gray-300, transparent bg
  │              │
  └──────────────┘
```

- Icon: `text-gray-300` (#d1d5db)
- Background: transparent
- No accent bar

### 9.2 Hover-with-tooltip

```
  ┌──────────────┐  ┌──────────────┐
  │              │  │  Dashboard   │  ← tooltip
  │     📊      │──│              │
  │              │  └──────────────┘
  └──────────────┘
```

- Icon: `text-white` (#ffffff)
- Background: `bg-gray-700` (#374151)
- Tooltip: visible, `opacity-100`, positioned `left-full ml-2`
- No accent bar

### 9.3 Focus-with-tooltip

- Same visual as hover-with-tooltip
- Additionally: `ring-2 ring-inset ring-gray-400` focus ring on the icon container
- Tooltip appears via `:focus-within` selector

### 9.4 Active-route

```
  ┌──────────────┐
  │              │
  │┃    📊      │  ← bg-white, text-gray-900, gold accent bar
  │              │
  └──────────────┘
```

- Icon: `text-gray-900` (#111827)
- Background: `bg-white` (#ffffff)
- Accent bar: 3px × 20px, `bg-primary-500` (#f1b400), left edge, vertically centred
- `aria-current="page"` set on the link

### 9.5 Active-route + hover

- Same as active-route (background remains white, icon remains dark)
- No additional visual change on hover for active item (consistent with expanded mode)

### 9.6 Disabled

```
  ┌──────────────┐
  │              │
  │     📦      │  ← text-gray-300 at 40% opacity
  │              │
  └──────────────┘
```

- Icon: `text-gray-300` at 40% opacity
- Background: transparent
- `aria-disabled="true"` on wrapper
- Tooltip shows `"Programs (Soon)"`

---

## 10. Responsive Behaviour

| Viewport | Collapsed rail visible? | Tooltip behaviour | Notes |
|---|---|---|---|
| `< 768px` (sm) | No — mobile drawer replaces sidebar | N/A | Collapsed rail is `hidden md:flex` |
| `768px–1023px` (md) | Yes | Appears on hover/focus | 64px rail, all states apply |
| `1024px+` (lg/xl) | Yes | Appears on hover/focus | 64px rail, all states apply |

### Tap target verification

- All icon hit targets are 44×44px — meets WCAG 2.5.5 at all viewport widths
- At `md` (768px), the 64px rail leaves 704px for main content — sufficient for dashboard layouts
- The tooltip extends 8px + content width to the right of the rail — does not obscure the rail itself

---

## 11. QA and Test Plan

### Design QA

- [ ] Icon fill/stroke contrast meets 4.5:1 against `bg-gray-900` in both light and dark themes
- [ ] Gold accent bar contrast meets 3:1 against `bg-gray-900` in both themes
- [ ] Tooltip contrast meets 4.5:1 in both themes
- [ ] Toggle icon contrast meets 3:1 against `bg-gray-900`
- [ ] All icon containers are exactly 44×44px (verify in DevTools)
- [ ] Rail width is exactly 64px
- [ ] Tooltip appears to the right of the icon, not overlapping adjacent content
- [ ] Tooltip does not overflow viewport on the rightmost sidebar item

### Keyboard walkthrough

- [ ] `Tab` moves focus through toggle → Dashboard → Programs → Bounties → Settings → Docs
- [ ] Tooltip appears on focus for each icon (not hover-only)
- [ ] Focus ring visible on every focused element (`ring-2 ring-inset ring-gray-400`)
- [ ] `Enter`/`Space` activates the focused link or button
- [ ] Toggle button expands sidebar on activation
- [ ] Focus is not trapped in the collapsed rail

### Responsive review

- [ ] Collapsed rail renders at 768px (md breakpoint) — verify `hidden md:flex`
- [ ] Collapsed rail is hidden below 768px — mobile drawer is used instead
- [ ] All tap targets remain 44×44px at 768px
- [ ] Tooltip does not clip at 768px viewport width
- [ ] Rail + main content layout is functional at 768px (64px rail + 716px content)

### Accessibility audit

- [ ] Every collapsed icon link has an `aria-label` attribute
- [ ] Active link has `aria-current="page"`
- [ ] Tooltip has `role="tooltip"`
- [ ] External links have `aria-label` including "(opens in new tab)"
- [ ] Disabled items have `aria-disabled="true"`
- [ ] Toggle has `aria-label` and `aria-expanded`
- [ ] Screen reader announces nav items correctly in collapsed mode (VoiceOver / NVDA)
- [ ] Focus order matches visual top-to-bottom order

---

## 12. Files Changed

| File | Change |
|---|---|
| `design/specs/sidebar-collapsed-micro-icons.md` | This document |
| `design/navigation-spec.md` | No changes — this spec cross-references it |
| `design-tokens.json` | No changes — all values already defined |
| `frontend/src/app/components/layout/AppShell.tsx` | No changes — this is a design spec only |
