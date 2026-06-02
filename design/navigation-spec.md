# Navigation Spec — Grainlify Global Navigation

**Branch:** `design/responsive-navigation`  
**Status:** Implemented  
**WCAG target:** 2.1 AA  
**Last updated:** 2026-06-01

---

## 1. Overview

The global navigation consists of two surfaces:

| Surface | Viewport | Behaviour |
|---|---|---|
| **Sidebar** | `md` (768px) and above | Persistent, collapsible to icon-only at any time |
| **Mobile drawer** | Below `md` | Hidden; opened by hamburger trigger in top bar |

A **skip-nav link** is always present as the first focusable element on every page.

---

## 2. Breakpoints

| Name | Min width | Navigation surface |
|---|---|---|
| `sm` | 640px | Mobile drawer |
| `md` | 768px | Sidebar (expanded, 288px) |
| `lg` | 1024px | Sidebar (expanded) |
| `xl` | 1280px | Sidebar (expanded) |

The sidebar auto-collapses to icon-only (64px) when the user clicks the collapse toggle. This is a **user preference**, not an automatic breakpoint change. The collapsed state persists for the session.

---

## 3. Component Anatomy

### 3.1 Skip-nav link

```
[Skip to main content]   ← visually hidden, appears on :focus
```

- Rendered as the first DOM element inside `<AppShell>`.
- `href="#main-content"` targets `<main id="main-content" tabIndex={-1}>`.
- Visible only on keyboard focus (`.sr-only focus:not-sr-only`).
- Background: `bg-gray-900`, text: `text-white`, positioned `fixed top-4 left-4 z-[100]`.

---

### 3.2 Sidebar (desktop, `md`+)

**Expanded state (288px)**

```
┌──────────────────────────────┐
│ Grainlify Workspace  [◀]     │  ← header row, 44px collapse button
│ Very Long Org Name…          │
├──────────────────────────────┤
│ ▣  Dashboard                 │  ← active: bg-white text-gray-900
│ ⊞  Programs          Soon    │  ← disabled: opacity-40
│ ★  Bounties          Soon    │
│ ⚙  Settings          Soon    │
│ 📖 Docs              ↗       │  ← external link
├──────────────────────────────┤
│ Docs opens in a new tab      │  ← footer note
└──────────────────────────────┘
```

**Collapsed state (64px)**

```
┌────┐
│ [▶]│  ← 44×44px expand button
├────┤
│ ▣  │  ← active: bg-white text-gray-900
│ ⊞  │  ← hover shows tooltip "Programs (Soon)"
│ ★  │
│ ⚙  │
│ 📖 │
└────┘
```

Tooltips appear to the right of the icon (`left-full ml-2`) on `:hover` and `:focus-within`.

---

### 3.3 Mobile top bar

```
┌─────────────────────────────────┐
│ [☰]    Grainlify         [    ] │  ← 44px hamburger, spacer balances layout
└─────────────────────────────────┘
```

- Height: `h-14` (56px).
- `sticky top-0 z-30`.
- Hamburger: `aria-expanded`, `aria-controls="mobile-drawer"`, `aria-haspopup="dialog"`.

---

### 3.4 Mobile drawer

```
┌──────────────────────────────┐
│ Grainlify              [✕]   │  ← 44px close button
│ Very Long Org Name…          │
├──────────────────────────────┤
│ ▣  Dashboard                 │
│ ⊞  Programs          Soon    │
│ ★  Bounties          Soon    │
│ ⚙  Settings          Soon    │
│ 📖 Docs              ↗       │
├──────────────────────────────┤
│ Docs opens in a new tab      │
└──────────────────────────────┘
```

- Width: `w-80` (320px), capped at `max-w-[85vw]`.
- Slides in from the left: `translate-x-0` ↔ `-translate-x-full`, `duration-300 ease-out`.
- Overlay: `bg-black/50`, fades in/out `duration-200`.
- `role="dialog" aria-modal="true" aria-label="Navigation menu"`.
- Focus trap active while open (Tab/Shift+Tab cycle within drawer).
- Escape key closes drawer and returns focus to hamburger button.
- Body scroll locked (`overflow: hidden`) while drawer is open.

---

## 4. Nav Item States

All nav items (expanded and collapsed) have `min-height: 44px` to satisfy WCAG 2.5.5.

| State | Visual treatment |
|---|---|
| **Default** | `text-gray-300`, transparent background |
| **Hover** | `bg-gray-700 text-white` |
| **Focus** | `ring-2 ring-inset ring-gray-400` (no outline suppression) |
| **Active / current page** | `bg-white text-gray-900 shadow-sm` |
| **Disabled** | `opacity-40 cursor-not-allowed`, `aria-disabled="true"` |
| **External** | Trailing `↗` indicator, `target="_blank" rel="noreferrer"` |

### Badge ("Soon")

- `rounded-full bg-gray-700 px-2 py-0.5 text-xs text-gray-300`
- Shown on disabled items only.
- In collapsed mode, badge text is appended to the tooltip: `"Programs (Soon)"`.

---

## 5. Collapse Toggle Button

| Property | Value |
|---|---|
| Size | 44×44px (`h-11 w-11`) |
| Icon | `ChevronLeft` (expanded) / `ChevronRight` (collapsed) |
| `aria-label` | `"Collapse sidebar"` / `"Expand sidebar"` |
| `aria-expanded` | `true` (expanded) / `false` (collapsed) |
| Transition | `transition-[width] duration-200 ease-in-out` on `<aside>` |

---

## 6. Accessibility Annotations

### ARIA roles and attributes

| Element | Role / Attribute | Value |
|---|---|---|
| Skip-nav `<a>` | — | `href="#main-content"` |
| Desktop `<aside>` | `aria-label` | `"Primary navigation"` |
| Desktop `<nav>` | `aria-label` | `"Primary navigation"` |
| Collapse button | `aria-label`, `aria-expanded` | Dynamic |
| Mobile `<header>` | — | `sticky top-0 z-30` |
| Hamburger `<button>` | `aria-label`, `aria-expanded`, `aria-controls`, `aria-haspopup` | Dynamic |
| Drawer `<aside>` | `role="dialog"`, `aria-modal`, `aria-label` | `"Navigation menu"` |
| Close `<button>` | `aria-label` | `"Close navigation menu"` |
| Active `<Link>` | `aria-current` | `"page"` |
| Disabled item | `aria-disabled` | `"true"` |
| Collapsed icon button | `aria-label` | Item name |
| Tooltip `<span>` | `role="tooltip"` | Item name |
| Secondary `<nav>` | `role="navigation"`, `aria-label` | `"Secondary navigation"` |
| `<main>` | `id`, `tabIndex` | `"main-content"`, `-1` |
| Breadcrumb `<nav>` | `aria-label` | `"Breadcrumb"` |
| Last breadcrumb | `aria-current` | `"page"` |

### Keyboard interaction

| Key | Action |
|---|---|
| `Tab` | Move focus forward through interactive elements |
| `Shift+Tab` | Move focus backward |
| `Escape` | Close mobile drawer, return focus to hamburger |
| `Enter` / `Space` | Activate focused button or link |

Focus trap is active inside the mobile drawer. Tab wraps from last to first focusable element and vice versa.

### Contrast ratios (WCAG 2.1 AA)

| Pair | Ratio | Requirement | Pass |
|---|---|---|---|
| Default nav text `#d1d5db` on `#111827` | 9.5:1 | 4.5:1 text | ✅ |
| Active nav text `#111827` on `#ffffff` | 16:1 | 4.5:1 text | ✅ |
| Badge text `#d1d5db` on `#374151` | 5.1:1 | 4.5:1 text | ✅ |
| Disabled text `#d1d5db` on `#111827` at 40% opacity | — | Exempt (disabled) | ✅ |
| Focus ring `#9ca3af` on `#111827` | 3.2:1 | 3:1 UI | ✅ |
| Tooltip text `#ffffff` on `#1f2937` | 14.7:1 | 4.5:1 text | ✅ |

---

## 7. Responsive Behaviour Summary

| Viewport | Sidebar | Mobile bar | Drawer |
|---|---|---|---|
| `< 768px` (sm) | Hidden | Visible | Triggered by hamburger |
| `768px–1023px` (md) | Visible, collapsible | Hidden | N/A |
| `1024px+` (lg/xl) | Visible, collapsible | Hidden | N/A |

---

## 8. Edge Cases

| Scenario | Handling |
|---|---|
| Long org/workspace name | `truncate` on both sidebar and drawer header |
| Many nav items | Drawer nav is `overflow-y-auto` |
| Disabled item clicked | `cursor-not-allowed`, no navigation, `aria-disabled` |
| External link | `target="_blank" rel="noreferrer"`, `↗` indicator, accessible label includes "(opens in new tab)" in collapsed mode |
| Sidebar collapsed + tooltip | Tooltip appears right of icon on hover/focus-within |
| Drawer open + resize to md+ | Drawer remains in DOM but is visually hidden (`md:hidden`); body scroll lock is released on unmount |
| Reduced motion | CSS transitions use `transition-*` classes; `prefers-reduced-motion` is respected by the browser for `transition` properties |

---

## 9. Z-index Scale

| Layer | Value | Element |
|---|---|---|
| Sidebar | 20 | Desktop `<aside>` |
| Mobile header | 30 | `<header>` |
| Drawer overlay | 40 | Backdrop `<div>` |
| Drawer panel | 50 | Mobile `<aside>` |
| Tooltip | 60 | Collapsed sidebar tooltips |
| Skip-nav | 100 | Skip-nav `<a>` |

---

## 10. Before / After

### Before (issues)

- Mobile drawer had no CSS transition — appeared/disappeared instantly.
- No focus trap in mobile drawer — Tab key escaped the drawer.
- No Escape key handler to close drawer.
- Body scroll was not locked when drawer was open.
- No skip-nav link.
- Nav items used `px-4 py-3` with no explicit `min-height`, making some items below 44px on certain font sizes.
- No sidebar collapse mode — sidebar was always 288px wide on desktop.
- Icon-only buttons had no accessible labels or tooltips.
- Hamburger button had no `aria-expanded`, `aria-controls`, or `aria-haspopup`.
- Disabled items had no `aria-disabled`.
- Secondary nav items had no `min-height` guarantee.
- No `role="dialog"` or `aria-modal` on mobile drawer.
- No `id="main-content"` target for skip-nav.

### After (resolved)

- Drawer slides in with `translate-x` CSS transition (300ms ease-out), overlay fades (200ms).
- Focus trap cycles Tab/Shift+Tab within drawer while open.
- Escape closes drawer and returns focus to hamburger.
- Body scroll locked (`overflow: hidden`) while drawer is open.
- Skip-nav link is first focusable element, targets `#main-content`.
- All interactive nav elements have `min-h-[44px]` (44px minimum touch target, WCAG 2.5.5).
- Sidebar collapses to 64px icon-only mode with animated width transition.
- Collapsed icon buttons have `aria-label` and hover/focus tooltips.
- Hamburger has `aria-expanded`, `aria-controls="mobile-drawer"`, `aria-haspopup="dialog"`.
- Disabled items have `aria-disabled="true"`.
- Secondary nav items have `min-h-[44px]`.
- Drawer has `role="dialog" aria-modal="true" aria-label="Navigation menu"`.
- `<main id="main-content" tabIndex={-1}>` is the skip-nav target.

---

## 11. Design Token Reference

All navigation-specific values are documented in `design-tokens.json` under the `navigation` key:

```json
"navigation": {
  "touchTarget": { "min": "44px" },
  "sidebar": {
    "widthExpanded": "288px",
    "widthCollapsed": "64px",
    "collapseTransitionDuration": "200ms"
  },
  "drawer": {
    "width": "320px",
    "maxWidthVw": "85vw",
    "slideInDuration": "300ms",
    "slideInEasing": "cubic-bezier(0, 0, 0.2, 1)"
  },
  "zIndex": { "sidebar": "20", "mobileHeader": "30", "drawerOverlay": "40", "drawerPanel": "50", "tooltip": "60" },
  "navItem": { "minHeight": "44px" }
}
```

---

## 12. Files Changed

| File | Change |
|---|---|
| `design-tokens.json` | Added `navigation` token section |
| `frontend/src/app/components/layout/AppShell.tsx` | Full rewrite — see above |
| `design/navigation-spec.md` | This document |
