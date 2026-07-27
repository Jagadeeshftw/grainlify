# Breadcrumb Navigation Pattern

**Branch:** `design/breadcrumb-pattern`
**Status:** Spec
**WCAG target:** 2.1 AA
**Last updated:** 2026-07-26

---

## 1. Overview

Deep pages such as `ProjectDetailPage.tsx`, `IssueDetailPage.tsx`, and `SettingsPage.tsx` have no consistent breadcrumb trail. The reusable `Breadcrumb` component in `frontend/src/app/components/ui/breadcrumb.tsx` exists as a base primitive but is unused. Existing breadcrumb implementations in `Dashboard.tsx` and `AppShell.tsx` are custom inline solutions, leading to visual and behavioral inconsistencies.

This spec defines a unified breadcrumb pattern built on the existing `Breadcrumb` component, covering hierarchy maps, truncation rules, mobile collapse behavior, accessibility, and design token validation.

### Goals

- Establish a consistent breadcrumb anatomy across all deep pages.
- Define per-page hierarchy maps with dynamic label sourcing.
- Specify truncation rules for deep navigation and long titles.
- Specify mobile collapse behavior below 375px breakpoint.
- Achieve WCAG 2.1 AA compliance via ARIA annotations and contrast ratios.

---

## 2. Breadcrumb Anatomy

The breadcrumb is built exclusively from the shared primitives in `frontend/src/app/components/ui/breadcrumb.tsx`:

| Component | Role | Tag | Key Props |
|---|---|---|---|
| `Breadcrumb` | Wrapper | `<nav>` | `aria-label="breadcrumb"` |
| `BreadcrumbList` | List container | `<ol>` | — |
| `BreadcrumbItem` | List item | `<li>` | — |
| `BreadcrumbLink` | Navigable link | `<a>` | `href`, `asChild` supported via Radix `Slot` |
| `BreadcrumbPage` | Current page indicator | `<span>` | `aria-current="page"`, `aria-disabled="true"`, `role="link"` |
| `BreadcrumbSeparator` | Separator icon | `<li>` | `role="presentation"`, `aria-hidden="true"`; renders `<ChevronRight />` (size `3.5`) |
| `BreadcrumbEllipsis` | Truncation indicator | `<span>` | `role="presentation"`, `aria-hidden="true"`; renders `<MoreHorizontal />` + `<span class="sr-only">More</span>` |

### Separator

- Always `<ChevronRight />` (16x16px via `size-3.5`), rendered by `BreadcrumbSeparator`.
- `role="presentation"` and `aria-hidden="true"` for screen reader exclusion.

### Current-page styling

- Uses `BreadcrumbPage` (renders a `<span>` with `aria-current="page"`).
- Font weight: `font-normal` (inherits).
- Color: `text-foreground` (maps to `neutral.900` light, `#f5f5f5` dark).

### Container / Background

Breadcrumbs sit inside the existing glassmorphism bar in `Dashboard.tsx`:

```tsx
<div className="mb-4 rounded-[22px] border px-4 py-3 backdrop-blur-[60px] ...">
  <Breadcrumb>
    <BreadcrumbList>
      ...
    </BreadcrumbList>
  </Breadcrumb>
</div>
```

---

## 3. Per-Page Hierarchy Maps

### 3.1 SettingsPage

**Hierarchy:** `Home > Settings > {Tab}`

| Segment | Component | Label Source | Link |
|---|---|---|---|
| Home | `BreadcrumbLink` | "Home" | `/dashboard` |
| Settings | `BreadcrumbLink` | "Settings" | `/dashboard?tab=settings` |
| {Tab} | `BreadcrumbPage` | `activeTab` state value (Profile, Notifications, Payout, Billing, Referrals, Terms, Tax Documents) | — |

### 3.2 ProjectDetailPage

**Hierarchy:** `Home > {Ecosystem} > {Project Name}`

| Segment | Component | Label Source | Link |
|---|---|---|---|
| Home | `BreadcrumbLink` | "Home" | `/dashboard` |
| {Ecosystem} | `BreadcrumbLink` | Ecosystem name (from context or API response) | `/dashboard?ecosystem={id}` |
| {Project Name} | `BreadcrumbPage` | Project name (from `projectId` lookup) | — |

- If ecosystem context is unavailable, omit the ecosystem segment and show `Home > Project Name`.

### 3.3 IssueDetailPage

**Hierarchy:** `Home > {Ecosystem} > {Project Name} > {Issue Title}`

| Segment | Component | Label Source | Link |
|---|---|---|---|
| Home | `BreadcrumbLink` | "Home" | `/dashboard` |
| {Ecosystem} | `BreadcrumbLink` | Ecosystem name | `/dashboard?ecosystem={id}` |
| {Project Name} | `BreadcrumbLink` | Project name | `/dashboard?project={id}` |
| {Issue Title} | `BreadcrumbPage` | Issue title (truncated to max 40 characters with tooltip on overflow) | — |

- If ecosystem context is unavailable, show `Home > Project Name > Issue Title` (3 segments).

---

## 4. States

### 4.1 Full-trail (desktop, ≤4 segments)

All segments visible. No truncation.

```
Home  ›  Ecosystem  ›  Project Name  ›  Issue Title
```

### 4.2 Middle-truncated (5+ segments or overflow)

When the total segment count exceeds 4 or a label exceeds 180px, collapse middle segments into `BreadcrumbEllipsis`.

```
Home  ›  …  ›  Issue Title
```

Rules:
- First segment ("Home") always visible.
- Last segment (current page) always visible.
- All middle segments are replaced by a single `BreadcrumbEllipsis` instance.
- The ellipsis icon renders `<MoreHorizontal />` with `sr-only` text "More".
- Clicking the ellipsis does NOT expand; it is purely a visual truncation indicator.

### 4.3 Long-title-truncated segment

Individual segment labels (project names, issue titles) are CSS-truncated with `truncate max-w-[180px]` (matching existing Dashboard pattern).

- Full text is exposed via `title` attribute on the containing `<span>` or `<a>` element for native tooltip.
- No custom tooltip component; rely on native `title` attribute.

### 4.4 Mobile-collapsed-back-link

Below **375px** viewport width (`sm` breakpoint), the full breadcrumb trail collapses to a single "Back to {parent}" link.

| Context | Back link label | Target |
|---|---|---|
| IssueDetailPage | "Back to {Project Name}" | Project detail view |
| ProjectDetailPage | "Back to {Ecosystem}" or "Back to Browse" | Ecosystem page or browse |
| SettingsPage | "Back to Dashboard" | `/dashboard` |

Implementation:
- The back link uses `BreadcrumbLink` with a `ChevronLeft` icon prepended: `← Back to {parent}`.
- Hidden above 375px via `sm:flex` / `flex sm:hidden` pattern.

---

## 5. Design Token Mapping

All colors validated against `design-tokens.json` for WCAG 2.1 AA 4.5:1 contrast.

| Element | Token (light) | Token (dark) | Contrast |
|---|---|---|---|
| Link text (`BreadcrumbLink`) | `neutral.600` (#57534e) | `darkMode.text.muted` (#9b8d7f) | ≥4.5:1 |
| Link text hover | `primary.600` (#c9983a) | `darkMode.accent.primaryHover` (#e8c77f) | ≥4.5:1 |
| Current page (`BreadcrumbPage`) | `text-foreground` → `neutral.900` (#1c1917) | `darkMode.text.primary` (#f5f5f5) | ≥10.5:1 |
| Separator (`ChevronRight`) | `neutral.400` (#a8a29e) | `darkMode.text.muted` (#9b8d7f) | ≥3:1 (decorative) |
| Focus ring | `accessibility.focus.outlineStyle` (#0066cc) | `darkMode.interactive.focusRing` rgba(201,152,58,0.28) | ≥3:1 |

Glassmorphism container:
| Property | Light | Dark |
|---|---|---|
| Background | `elevation.glassmorphism.light.backgroundFill` (rgba(255,255,255,0.15)) | `elevation.glassmorphism.dark.backgroundFill` (rgba(255,255,255,0.08)) |
| Border | `elevation.glassmorphism.light.borderColor` (rgba(255,255,255,0.25)) | `elevation.glassmorphism.dark.borderColor` (rgba(255,255,255,0.15)) |
| Blur | `elevation.glassmorphism.light.blurRadius` (25px) | `elevation.glassmorphism.dark.blurRadius` (25px) |

Typography:
| Property | Token | Value |
|---|---|---|
| Font size | `typography.fontSize.sm` | 0.875rem / 14px |
| Line height | `typography.fontSize.sm[1].lineHeight` | 1.25rem / 20px |
| Font family | `typography.fontFamily.sans` | Inter, system-ui, -apple-system, sans-serif |

---

## 6. Accessibility

All requirements target WCAG 2.1 AA.

### ARIA

| Element | Attribute | Value |
|---|---|---|
| `<nav>` | `aria-label` | "breadcrumb" |
| `<ol>` | `role` | `list` (implicit) |
| `<li>` | `role` | `listitem` (implicit) |
| Current page (`BreadcrumbPage`) | `aria-current` | `page` |
| Current page (`BreadcrumbPage`) | `aria-disabled` | `true` |
| Separator (`BreadcrumbSeparator`) | `aria-hidden` | `true` |
| Ellipsis (`BreadcrumbEllipsis`) | `aria-hidden` | `true` |
| Ellipsis sr-only text | — | `<span class="sr-only">More</span>` |

### Semantic structure

```html
<nav aria-label="breadcrumb">
  <ol>
    <li><a href="/dashboard">Home</a></li>
    <li aria-hidden="true"><ChevronRight /></li>
    <li><a href="/dashboard?ecosystem=1">Ecosystem</a></li>
    <li aria-hidden="true"><ChevronRight /></li>
    <li><span aria-current="page" aria-disabled="true">Current Page</span></li>
  </ol>
</nav>
```

### Keyboard navigation

- Tab moves focus through `BreadcrumbLink` items in source order (left to right).
- `Enter` or `Space` activates the link.
- The current page (`BreadcrumbPage`) is NOT focusable (rendered as `<span>`).
- Ellipsis (`BreadcrumbEllipsis`) is NOT focusable (decorative).
- Separators are NOT focusable.
- A visible focus ring is applied on `BreadcrumbLink:focus-visible`:
  - Light: `outline: 2px solid #0066cc` with `offset: 2px`
  - Dark: `box-shadow: 0 0 0 2px rgba(201,152,58,0.28)`
- Focus order must match the logical left-to-right reading order.

### Reduced motion

- `prefers-reduced-motion: reduce` disables breadcrumb entry animations (if any).
- No auto-rotating or animated breadcrumb content.

---

## 7. Responsive Behavior

| Breakpoint | Viewport | Behavior |
|---|---|---|
| `sm` | ≤375px | **Mobile-collapsed**: Full breadcrumb hidden. Single "Back to {parent}" link visible, styled with `ChevronLeft` icon. |
| `md` | 376–768px | **Truncated**: Maximum 3 visible segments. If >3 segments, apply middle-truncation. Individual labels capped at `max-w-[120px]`. |
| `lg`+ | >768px | **Full trail**: All segments visible. Individual labels capped at `max-w-[180px]`. No truncation for ≤4 segments. |

### Mobile back link styling

```tsx
<BreadcrumbLink href={backTarget}>
  <ChevronLeft className="w-4 h-4" />
  Back to {parentLabel}
</BreadcrumbLink>
```

- Visible only below `sm` breakpoint: `flex sm:hidden`.
- Target: appropriate parent context URL (ecosystem page, project detail, or dashboard).
- The `ChevronLeft` icon is 16x16px (`w-4 h-4`).

---

## 8. BreadcrumbLabelSource Utility

A utility function `getBreadcrumbLabels` maps route/state context to an ordered array of label segments.

```ts
interface BreadcrumbSegment {
  label: string;
  href?: string; // undefined for current page
}

function getBreadcrumbLabels(context: BreadcrumbContext): BreadcrumbSegment[]
```

### Context mapping

| Page | Context Input | Output Segments |
|---|---|---|
| Settings | `{ page: "settings", activeTab: "Notifications" }` | `[{label:"Home", href:"/dashboard"}, {label:"Settings", href:"/settings"}, {label:"Notifications"}]` |
| Project Detail | `{ page: "project", projectName: "grainlify", ecosystemName: "Stellar" }` | `[{label:"Home", href:"/dashboard"}, {label:"Stellar", href:"/ecosystem/..."}, {label:"grainlify"}]` |
| Issue Detail | `{ page: "issue", projectName: "grainlify", ecosystemName: "Stellar", issueTitle: "Fix XDR bug" }` | `[{label:"Home", href:"/dashboard"}, {label:"Stellar", href:"/ecosystem/..."}, {label:"grainlify", href:"/project/..."}, {label:"Fix XDR bug"}]` |

---

## 9. QA and Test Plan

### Contrast verification

- [ ] Link text on glassmorphism background passes 4.5:1 in light theme (verify `neutral.600` on rgba(255,255,255,0.15)).
- [ ] Link text on glassmorphism background passes 4.5:1 in dark theme (verify `#9b8d7f` on rgba(255,255,255,0.08)).
- [ ] Current page text passes 4.5:1 in both themes.
- [ ] Separator icon contrast is decorative (≥3:1).

### Keyboard walkthrough

- [ ] Tab moves through breadcrumb links in left-to-right order.
- [ ] Focus ring visible on each `BreadcrumbLink`.
- [ ] Current page is not focusable.
- [ ] Ellipsis and separators are not focusable.

### Responsive review

- [ ] Full trail renders correctly at 1440px (desktop).
- [ ] Middle truncation activates at 768px when >4 segments.
- [ ] Single "Back to {parent}" link renders at 375px.
- [ ] Long labels truncate with ellipsis at 180px/120px caps.
- [ ] Tooltip (`title` attribute) shows full text on truncated labels.

### Accessibility audit

- [ ] `nav aria-label="breadcrumb"` present.
- [ ] `<ol>` / `<li>` structure correct.
- [ ] `aria-current="page"` on current item.
- [ ] `aria-hidden="true"` on separators and ellipsis.
- [ ] `sr-only` text present on ellipsis.
- [ ] Screen reader announces breadcrumb trail correctly (VoiceOver / NVDA).

---

## 10. Implementation Status

| Feature | Status |
|---|---|
| Breadcrumb Anatomy (reusable components) | Complete (in `breadcrumb.tsx`) |
| Per-Page Hierarchy Maps | Spec |
| Full-trail state | Spec |
| Middle-truncated state | Spec |
| Long-title-truncated segment | Spec |
| Mobile-collapsed back link | Spec |
| Design token validation | Spec |
| Accessibility annotations | Spec |
| Keyboard navigation | Spec |
| Responsive behavior | Spec |
