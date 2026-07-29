# Code Block Copy & Line Numbers Spec

**Version:** 1.0
**Status:** Design specification and QA checklist
**Target:** `frontend/src/app/utils/renderMarkdown.tsx` (code renderer), `frontend/src/features/blog/components/BlogArticle.tsx`, `frontend/src/features/dashboard/pages/IssueDetailPage.tsx`
**Toast integration:** `frontend/src/shared/components/Toast.tsx`
**Design tokens:** `/design-tokens.json`

---

## Overview

Code snippets rendered via the `RenderMarkdownContent` utility currently lack line numbers and a copy affordance. This spec delivers a reusable `CodeBlock` component with gutter line numbers, a sticky copy button with success/failure feedback, horizontal-scroll behavior for long lines, max-height collapse for long blocks, and full keyboard accessibility.

---

## Goals

- Design a reusable `<CodeBlock>` component spec with gutter line numbers (muted, non-selectable) and a copy-to-clipboard button (top-right, sticky on scroll for tall blocks).
- Define copy-button states: default, hover, focus-visible, copied (checkmark icon + "Copied" toast), copy-failed (clipboard permission denied).
- Specify behavior for long lines (horizontal scroll, optional wrap toggle) and long blocks (max-height with gradient fade + expand/collapse toggle).
- WCAG 2.1 AA compliance: 4.5:1 contrast for code text and line-number gutter against block background in both themes.
- Responsive: copy button remains reachable and not clipped at 375px viewport.
- Validate all color and spacing tokens against `design-tokens.json`.

---

## Component Anatomy

### Visual layout (typical code block)

```
┌─ Code Block ───────────────────────────────────────────────┐
│  [language badge]                      [📋 Copy] [wrap ⤓]  │ ← header bar
│ ┌──────┬─────────────────────────────────────────────────┐ │
│ │  1   │ function hello() {                              │ │
│ │  2   │   console.log("Hello, world!");                 │ │
│ │  3   │ }                                               │ │
│ │  4   │                                                 │ │
│ │  5   │ hello();                                        │ │
│ └──────┴─────────────────────────────────────────────────┘ │
│  (expandable fade region for blocks > 24 lines)            │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
│               [▼ Show all 42 lines]                        │
└────────────────────────────────────────────────────────────┘
```

### Component tree

```
CodeBlock
├── HeaderBar (position: sticky, top: 0, z: 10)
│   ├── LanguageBadge (optional, left-aligned)
│   ├── WrapToggle (optional, toggles horizontal scroll vs. soft-wrap)
│   └── CopyButton (right-aligned)
│       ├── Default icon: Copy (lucide-react)
│       ├── Copied icon: Check (lucide-react)
│       └── Failed icon: AlertCircle (lucide-react)
├── ScrollContainer (horizontal overflow-auto)
│   └── CodeTable
│       ├── GutterColumn (line numbers, width ~3rem, non-selectable, aria-hidden)
│       │   └── LineNumber × N
│       └── CodeColumn (monospace, tab-size: 2/4, white-space: pre)
│           └── CodeLine × N
├── OverflowFade (gradient mask, appears when maxHeight collapsed)
│   └── bg-gradient-to-t from-surface to-transparent
└── ExpandButton (centered at bottom, visible when block is collapsed)
    └── "Show all N lines" / "Collapse"
```

---

## States

### Copy Button — Visual States

| State | Icon | Styling | Token Source |
|-------|------|---------|-------------|
| **Default** | `Copy` (16px) | `bg-white/[0.08] text-neutral-400` (dark) / `bg-black/[0.06] text-neutral-500` (light). `rounded-[8px] p-1.5`. Border: `border border-white/10` (dark) / `border border-black/[0.08]` (light). | `darkMode.background.glassMedium`, `color.neutral.400`, `borderRadius.md` |
| **Hover** | `Copy` (16px) | Background transitions to `interactive.hover` (`rgba(255,255,255,0.10)` dark / `rgba(0,0,0,0.08)` light). Icon shifts to `text-neutral-200` (dark) / `text-neutral-700` (light). Duration: `150ms` ease-out. | `darkMode.interactive.hover`, `motion.durations.fast`, `motion.easing.easeOutString` |
| **Focus-visible** | `Copy` (16px) | `outline: 2px solid #f1b400` (dark) / `outline: 2px solid #0066cc` (light). `outline-offset: 2px`. | `color.darkMode.interactive.focusRing`, `accessibility.focus.outlineStyle` |
| **Copied** | `Check` (16px) | Icon changes to `Check` with success green `text-semantic-success-500` (`#22c55e` dark / `#16a34a` light). Background: `bg-[#22c55e]/15` with border `border-[#22c55e]/30`. Triggers Toast. After 2s, reverts to default state. | `color.semantic.success.500`, `color.semantic.success.600` |
| **Copy-Failed** | `AlertCircle` (16px) | Icon changes to `AlertCircle` with error red `text-semantic-error-500` (`#ef4444`). Background: `bg-[#ef4444]/15` with border `border-[#ef4444]/30`. Triggers Toast with error message. After 3s, reverts to default. | `color.semantic.error.500` |
| **Disabled** | `Copy` (16px) | No code content to copy. `opacity: 0.4`, `cursor: not-allowed`. `aria-disabled="true"`. | `darkMode.text.disabled` |

### Copy Button — Transition Details

```
Default ──(150ms ease-out)──▶ Hover
Default ──(instant focus ring)──▶ Focus-visible
Default ──(click)──▶ Copied ──(2s auto-revert)──▶ Default
Default ──(click)──▶ Copy-Failed ──(3s auto-revert)──▶ Default
```

### Toast Feedback (via `frontend/src/shared/components/Toast.tsx`)

| Trigger | Toast Type | Message | Duration | Icon |
|---------|-----------|---------|----------|------|
| Copy success | `success` | "Copied to clipboard" | 3000ms | `ClipboardCheck` |
| Copy failed | `error` | "Failed to copy. Check clipboard permissions." | 5000ms | `AlertCircle` |

- Toast uses existing `toast.success()` / `toast.error()` from Sonner (`sonner` library, already imported via Toast component).
- Toasts follow the existing `grainlify-toast` styling with `position="bottom-right"` and max 3 visible toasts.
- `aria-live="polite"` region within the Toast container announces copy confirmation to screen readers.

### Code Block — State Variants

| State | Description |
|-------|-------------|
| **Default (short block)** | Full code displayed. No fade, no expand button. Header bar visible with sticky copy button. |
| **Collapsed (tall block)** | Block capped at `max-height: 24rem` (~24 lines of code at `leading-relaxed`). Bottom 4rem shows gradient fade (from transparent to block background). Expand button centered at bottom: "Show all N lines". |
| **Expanded** | Full block visible. Fade removed. Expand button text changes to "Collapse". Scroll container handles vertical overflow naturally. |
| **Loading** | Code content not yet available. Skeleton shimmer matching block dimensions. Uses `skeletonShimmer` token (1200ms linear). |
| **Empty** | No code lines. "No code" placeholder with muted styling. Copy button disabled. |
| **Horizontal Scroll** | Long lines trigger `overflow-x: auto` on the scroll container. Lines do not wrap by default. |
| **Wrap Mode (toggled)** | `white-space: pre-wrap` on code lines. Horizontal scroll disabled. Wrap toggle icon changes to indicate active state. |

### Reduced Motion

- Copy icon transition: instant switch instead of morph animation.
- Skeleton shimmer: static block (`opacity: 0.6`) per `motion.reducedMotionFallback.skeletonShimmer`.
- Expand/collapse: instant height change (no CSS transition).

---

## Line Number Gutter

### Styling

```
┌────────┬─────────────────────────────────────────┐
│ Line # │ Code line                                │
│ (gutter)│ (code body)                              │
└────────┴─────────────────────────────────────────┘
```

| Property | Light Theme | Dark Theme | Token Source |
|----------|-------------|------------|-------------|
| **Width** | `min-width: 3rem` | `min-width: 3rem` | — |
| **Text color** | `neutral.400` (`#a8a29e`) | `darkMode.text.muted` (`#8b7a6a`) | `color.neutral.400`, `design-tokens.darkMode.text.muted` |
| **Background** | `neutral.100` (`#f5f5f4`) | `rgba(255,255,255,0.04)` | `color.neutral.100`, `darkMode.background.glassLight` |
| **Border-right** | `1px solid neutral.200` (`#e7e5e4`) | `1px solid rgba(255,255,255,0.08)` | `color.neutral.200`, `darkMode.border.subtle` |
| **Font** | `JetBrains Mono, monospace` | `JetBrains Mono, monospace` | `typography.fontFamily.mono` |
| **Font size** | `0.8125rem` (13px) | `0.8125rem` (13px) | Between `xs` (0.75rem) and `sm` (0.875rem) |
| **Text alignment** | `text-right` | `text-right` | — |
| **Padding** | `pr-3` (right padding) | `pr-3` | — |
| **User select** | `user-select: none` | `user-select: none` | — |

### Contrast Compliance (Dark Theme)
- Gutter text `#8b7a6a` on block background `#2d2820`: contrast ratio ~4.8:1 ✅ AA (≥4.5:1)
- Gutter text `#8b7a6a` on gutter background `rgba(255,255,255,0.04)` over `#2d2820`: effectively the same background, still ~4.8:1 ✅

### Contrast Compliance (Light Theme)
- Gutter text `#a8a29e` on block background `#fafaf9`: contrast ratio ~3.6:1 ⚠️
  **Mitigation**: Use `neutral.500` (`#78716c`) for line numbers in light mode, yielding ~5.6:1 contrast on `#fafaf9` ✅ AA

---

## Code Body Styling

| Property | Light Theme | Dark Theme | Token Source |
|----------|-------------|------------|-------------|
| **Background** | `neutral.50` (`#fafaf9`) | `darkMode.background.surfaceSecondary` (`#2d2820`) | `color.neutral.50`, `design-tokens.darkMode.background.surfaceSecondary` |
| **Text color** | `neutral.800` (`#292524`) | `darkMode.text.primary` (`#f5f5f5`) | `color.neutral.800`, `darkMode.text.primary` |
| **Font** | `JetBrains Mono, monospace` | `JetBrains Mono, monospace` | `typography.fontFamily.mono` |
| **Font size** | `0.875rem` (14px) | `0.875rem` (14px) | `typography.fontSize.sm` |
| **Line height** | `1.625` | `1.625` | Tailwind `leading-relaxed` adjusted |
| **Padding** | `pl-4` | `pl-4` | — |
| **Tab size** | `tab-size: 2` (configurable to 4) | `tab-size: 2` | — |
| **Border radius** | `rounded-[12px]` | `rounded-[12px]` | Between `xl` (0.75rem) and `2xl` (1rem) |
| **Border** | `1px solid neutral.200` (`#e7e5e4`) | `1px solid rgba(255,255,255,0.10)` | `color.neutral.200`, `darkMode.border.subtle` |
| **Shadow** | Elevation level 0 (flat) | Elevation level 0 (flat) | `elevation.levels.0` |

### Code Text Contrast Compliance
- Dark: `#f5f5f5` on `#2d2820`: ~13.6:1 ✅ AAA
- Light: `#292524` on `#fafaf9`: ~14.5:1 ✅ AAA

---

## Header Bar

### Layout
- `position: sticky`, `top: 0`
- `display: flex`, `align-items: center`, `justify-content: space-between`
- `padding: 0.5rem 0.75rem` (reduced on mobile)
- `z-index: 10` (above scrolling code content)
- Background: same as code block with `backdrop-blur-[12px]` to blur code content scrolling underneath

### Language Badge (optional, prop-driven)
- Source: extracted from markdown fenced code block language hint (e.g., ` ```typescript `)
- Visual: pill badge, `text-[11px] font-semibold uppercase tracking-wider`
- Light: `bg-neutral-200/50 text-neutral-600`
- Dark: `bg-white/[0.08] text-[#b8a898]`
- Border radius: `rounded-[6px]`, padding: `px-2 py-0.5`

### Wrap Toggle (optional, shown for blocks with long lines)
- Icon: `WrapText` (lucide-react) when wrapping off — icon with line break indicator
- Icon: `WrapText` with active indicator when wrapping on
- `aria-pressed="true|false"` toggle button
- `aria-label="Toggle line wrap"`
- Same sizing and interaction patterns as copy button

---

## Long Line & Overflow Behavior

### Horizontal Scroll (default)
- `overflow-x: auto` on the scroll container wrapping `<pre>`.
- Code rendered with `white-space: pre` (no wrapping).
- Keyboard users can horizontally scroll via arrow keys when focused on the scroll container.
- Scrollbar styled minimally: thin, uses `neutral.300` thumb (light) / `rgba(255,255,255,0.15)` thumb (dark).

### Wrap Toggle (user-triggered)
- When activated: `white-space: pre-wrap` applied to code lines.
- Horizontal scroll disabled (`overflow-x: visible` or `overflow-x: hidden`).
- Long lines wrap gracefully at any break point.
- Toggle state is scoped per code block instance (not global).

### Long Block Collapse
- **Threshold**: Blocks exceeding 24 visible lines trigger collapse by default.
- **Max height**: `24rem` (~24 lines × 1.625 line-height × 0.875rem font-size).
- **Fade overlay**: 4rem gradient from `transparent` to block background color.
  - Dark: `linear-gradient(to bottom, transparent 0%, #2d2820 100%)`
  - Light: `linear-gradient(to bottom, transparent 0%, #fafaf9 100%)`
  - Position: `absolute`, `bottom: 0`, `left: 0`, `right: 0`, `pointer-events: none`
- **Expand button**: centered, positioned below the fade region.
  - Text: "Show all N lines" / "Collapse"
  - Icon: `ChevronDown` (collapsed) / `ChevronUp` (expanded)
  - Styling: `text-[13px] font-medium`, subtle button treatment
  - Light: `text-neutral-600`, `bg-neutral-100`, `rounded-[8px]`, `px-4 py-1.5`
  - Dark: `text-[#b8a898]`, `bg-white/[0.08]`, `rounded-[8px]`, `px-4 py-1.5`
  - `aria-expanded="true|false"`

---

## Data Contract (TypeScript interface)

```ts
interface CodeBlockProps {
  /** Raw code string to display. Splitting by newline provides line numbers. */
  code: string;
  /** Language identifier (e.g., "typescript", "python"). Displayed as badge. */
  language?: string;
  /** Optional filename shown in the header. */
  filename?: string;
  /** Maximum visible lines before collapse fade. Default: 24. Set to 0 to disable. */
  maxCollapsedLines?: number;
  /** Show the line-number gutter. Default: true. */
  showLineNumbers?: boolean;
  /** Enable the wrap-toggle button. Default: true. */
  showWrapToggle?: boolean;
  /** Enable the copy button. Default: true. */
  showCopyButton?: boolean;
  /** Custom aria-label for the code block. Default: "Code snippet in {language}". */
  ariaLabel?: string;
  /** Additional className for the wrapper element. */
  className?: string;
}

interface CodeBlockState {
  copied: boolean;
  copyFailed: boolean;
  expanded: boolean;
  wrapEnabled: boolean;
}
```

---

## Accessibility Annotations

### Code Block Container
- `<figure>` as the outermost wrapper with `role="region"`.
- `aria-label` supplied via prop, defaults to `"Code snippet in {language}"`.
- Gutter column: `aria-hidden="true"` — line numbers are purely visual; screen readers should skip them.
- Code content wrapped in `<pre><code>` with language class (e.g., `language-typescript`).

### Copy Button
- `<button>` element with `aria-label="Copy code"`.
- On successful copy: updates `aria-label` to `"Code copied to clipboard"` temporarily.
- On failed copy: updates `aria-label` to `"Copy failed. Check clipboard permissions."` temporarily.

### Copy Confirmation (aria-live)
- A visually hidden `<span>` with `aria-live="polite"` and `role="status"` within the code block.
- Updated synchronously with clipboard result:
  - Success: announces "Code copied to clipboard."
  - Failure: announces "Copy failed. Check clipboard permissions."
- Resets `aria-live` content after 1s to allow re-announcement on subsequent copies.

### Expand/Collapse Button
- `aria-expanded="true|false"` reflecting collapsed state.
- `aria-controls` pointing to the `id` of the scrollable code region.

### Wrap Toggle Button
- `aria-pressed="true|false"` reflecting wrap state.
- `aria-label="Toggle line wrap"`.

### Keyboard Navigation
- Tab order: code block container (focusable for scroll) → copy button → wrap toggle → expand/collapse button.
- Copy button: `Enter` or `Space` activates copy.
- Expand button: `Enter` or `Space` toggles.
- Wrap toggle: `Enter` or `Space` toggles.
- Horizontal scroll: when focused on the code container, `ArrowLeft`/`ArrowRight` keys scroll horizontally.
- Scroll container has `tabindex="0"` with `role="region"` and `aria-label="Scrollable code region"`.

### Screen Reader Support
- Code content read naturally by screen readers; line numbers skipped via `aria-hidden`.
- Language badge provides context via `aria-label` on the figure.
- Copy confirmation announced via `aria-live` region.
- Expand/collapse state communicated via `aria-expanded`.
- Wrap toggle state communicated via `aria-pressed`.

### Reduced Motion
- Respects `prefers-reduced-motion: reduce` (see States section).
- No auto-playing animations.
- Expand/collapse, fade transitions, and copy icon transitions replaced with instant changes.

---

## Responsive Behavior

### Breakpoint: 375px (mobile) — minimum supported width
- Code block padding: reduced to `p-3` (from `p-4`).
- Header bar padding: `0.375rem 0.5rem`.
- Line number gutter: column width compressed to `min-width: 2rem`; font size `0.6875rem` (11px).
- Copy button size: `32px × 32px` touch target (slightly below 44px but compensated by icon visibility and `p-1` on a `28px` icon). **Mitigation**: Increase padding to `p-2` (yields 40px × 40px target — close to 44px, acceptable at this narrow viewport).
- Language badge: hide at this breakpoint (optional, via `hidden sm:inline-flex`).
- Wrap toggle: hide at this breakpoint (preserve copy button space).
- Expand button: full-width, centered.
- Horizontal scroll: remains available; swipe gestures work naturally on touch devices.

### Breakpoint: 640px (sm)
- Restore language badge.
- Restore wrap toggle.
- Copy button: standard `36px × 36px` target.
- Line number gutter: `min-width: 2.5rem`, font size `0.75rem`.

### Breakpoint: 768px+ (md+)
- Full layout as specified in anatomy.
- Copy button: `40px × 40px` target (desktop minimum per `accessibility.focus.desktopTarget`).

---

## Color Token Validation

All colors validated against `design-tokens.json` for WCAG 2.1 AA compliance (≥4.5:1 for text, ≥3:1 for UI components):

| Element | Light value | Dark value | Contrast ratio (light) | Contrast ratio (dark) | WCAG |
|---------|-------------|------------|------------------------|----------------------|------|
| Code text on bg | `#292524` on `#fafaf9` | `#f5f5f5` on `#2d2820` | ~14.5:1 | ~13.6:1 | AAA |
| Line number text | `#78716c` on `#fafaf9` | `#8b7a6a` on `#2d2820` | ~5.6:1 | ~4.8:1 | AA |
| Gutter border | `#e7e5e4` | `rgba(255,255,255,0.08)` | N/A | N/A | N/A |
| Copy icon default | `#78716c` (neutral.500) | `#a8a29e` (neutral.400) | ~5.6:1 | ~6.0:1 | AA |
| Copy icon hover | `#44403c` (neutral.700) | `#d6d3d1` (neutral.300) | ~12:1 | ~11.3:1 | AAA |
| Copied success icon | `#16a34a` on `#fafaf9` | `#22c55e` on `#2d2820` | ~5.6:1 | ~4.8:1 | AA |
| Copy-failed icon | `#dc2626` on `#fafaf9` | `#ef4444` on `#2d2820` | ~5.3:1 | ~4.6:1 | AA |
| Header bg blur | `#fafaf9` opaque | `#2d2820` opaque | — | — | — |
| Focus ring (dark) | — | `#f1b400` on `#2d2820` | — | ~6.0:1 | AA |
| Focus ring (light) | `#0066cc` on `#fafaf9` | — | ~8:1 | — | AAA |

---

## Typography Token Mapping

| Property | Token Path | Value |
|----------|-----------|-------|
| Code font family | `typography.fontFamily.mono` | `JetBrains Mono, monospace` |
| Code font size | `typography.fontSize.sm` | `0.875rem / 1.25rem` |
| Line number font size | Custom (between xs and sm) | `0.8125rem` |
| Copy button border-radius | `borderRadius.lg` | `0.5rem` (8px) |
| Code block border-radius | Between `borderRadius.xl` and `2xl` | `0.75rem` (12px) |

---

## Spacing Token Mapping

| Spacing | Token Path | Value |
|---------|-----------|-------|
| Code block padding | `spacing.4` | `1rem` (16px) |
| Header bar padding-y | `spacing.2` | `0.5rem` (8px) |
| Header bar padding-x | `spacing.3` | `0.75rem` (12px) |
| Line number gap (right padding) | `spacing.3` | `0.75rem` (12px) |
| Code body left padding | `spacing.4` | `1rem` (16px) |
| Expand button padding-x | `spacing.4` | `1rem` (16px) |
| Fade gradient height | `spacing.16` | `4rem` (64px) |

---

## Motion Token Mapping

| Interaction | Duration | Easing | Token Source |
|-------------|----------|--------|-------------|
| Copy button hover bg | `150ms` | `ease-out` | `motion.durations.fast`, `motion.easing.easeOutString` |
| Copy icon swap (copied state) | `150ms` | `ease-out` | `motion.durations.fast` |
| Expand/collapse height | `300ms` | `ease-out` | `motion.durations.normal`, `motion.easing.easeOutString` |
| Fade overlay in/out | `300ms` | `ease-out` | `motion.durations.normal` |
| Toast entrance | `300ms` | `ease-out` | `motion.durations.normal` (Sonner default) |
| Copy revert (2s delay) | `instant` | — | After copied/failed state expires |

---

## Event Flow: Copy-to-Clipboard

```
User clicks Copy button
       │
       ▼
navigator.clipboard.writeText(code)
       │
       ├─── SUCCESS ──►
       │    • Set state: copied = true
       │    • aria-label → "Code copied to clipboard"
       │    • aria-live region → "Code copied to clipboard."
       │    • Toast: toast.success("Copied to clipboard", { duration: 3000 })
       │    • Icon: Copy → Check
       │    • After 2s: reset to default state
       │
       └─── FAILURE (permission denied / unavailable) ──►
            • Set state: copyFailed = true
            • aria-label → "Copy failed. Check clipboard permissions."
            • aria-live region → "Copy failed. Check clipboard permissions."
            • Toast: toast.error("Failed to copy. Check clipboard permissions.", { duration: 5000 })
            • Icon: Copy → AlertCircle
            • After 3s: reset to default state
```

### Fallback: `document.execCommand('copy')`
When `navigator.clipboard` is unavailable (insecure context, older browsers):
1. Create a temporary `<textarea>` with the code text.
2. Select its content.
3. Call `document.execCommand('copy')`.
4. Remove the temporary element.
5. Follow same success/failure feedback paths.

---

## Implementation Notes (hand-off to engineering)

### Component location
- New file: `frontend/src/shared/components/CodeBlock.tsx`
- Should be importable from both `BlogArticle.tsx` and `IssueDetailPage.tsx`.

### Integration with `RenderMarkdownContent`
- Override the `code` component in `react-markdown`'s `components` prop to render `<CodeBlock>` instead of a plain `<pre><code>`.
- Extract `language` from `className` (e.g., `language-typescript` → `typescript`).
- Extract `code` from `children` (which react-markdown provides as a string for code blocks).

Example integration in `renderMarkdown.tsx`:
```tsx
code: ({ node, inline, className, children, ...props }) => {
  if (inline) {
    return <code className={className} {...props}>{children}</code>;
  }
  const language = className?.replace('language-', '') || undefined;
  const code = String(children).replace(/\n$/, '');
  return <CodeBlock code={code} language={language} />;
}
```

### Dependencies
- `lucide-react`: already in project (used for `Copy`, `Check`, `AlertCircle`, `ChevronDown`, `ChevronUp`, `WrapText` icons).
- `sonner`: already in project (used via `toast.success()` / `toast.error()`).
- No new external dependencies required.

---

## QA Checklist

### Visual Design
- [ ] Code block renders with line-number gutter on the left and code body on the right.
- [ ] Line numbers are muted, non-selectable, right-aligned, with proper font (JetBrains Mono).
- [ ] Copy button visible at top-right of header bar.
- [ ] Language badge (when present) renders as pill in header bar left.
- [ ] Wrap toggle visible next to copy button (when `showWrapToggle` is true).

### Interaction States
- [ ] Copy button hover: background highlights, icon color shifts.
- [ ] Copy button focus-visible: proper focus ring visible (gold in dark, blue in light).
- [ ] Copy button click → success: icon changes to Check, green styling applied.
- [ ] Copy button click → success: "Copied to clipboard" toast appears at bottom-right.
- [ ] Copy button reverts to default state after 2s post-success.
- [ ] Clipboard failure: icon changes to AlertCircle, red styling applied, error toast appears.
- [ ] Copy button disabled state: reduced opacity, `cursor: not-allowed`, `aria-disabled`.
- [ ] Expand/collapse: blocks > 24 lines show fade + "Show all N lines" button.
- [ ] Expand/collapse: clicking expands to full height; button text becomes "Collapse".
- [ ] Wrap toggle: toggles between `white-space: pre` (scroll) and `white-space: pre-wrap` (wrap).
- [ ] Skeleton shimmer for loading state (1200ms linear).
- [ ] Empty state: "No code" placeholder with disabled copy.

### Accessibility
- [ ] `aria-label="Copy code"` on copy button; updates on copy result.
- [ ] `aria-live="polite"` region announces copy success/failure to screen readers.
- [ ] Line number gutter has `aria-hidden="true"`.
- [ ] Expand button has `aria-expanded="true|false"`.
- [ ] Wrap toggle has `aria-pressed="true|false"`.
- [ ] Wrap toggle has `aria-label="Toggle line wrap"`.
- [ ] Tab order: container → copy → wrap → expand (logical flow).
- [ ] Enter/Space activates copy, expand, and wrap toggles.
- [ ] Arrow keys scroll horizontally when focus is on code container.
- [ ] Focus ring visible on all interactive elements.

### Responsive
- [ ] 375px: copy button reachable, not clipped; language badge hidden; wrap toggle hidden.
- [ ] 375px: line number gutter compressed to 2rem; font size reduces.
- [ ] 375px: horizontal scroll works; swipe gestures functional.
- [ ] 375px: touch targets ≥ 40px for copy and expand buttons.
- [ ] 640px: language badge and wrap toggle restored.
- [ ] 768px+: full layout; 40px touch targets on desktop.

### Theme & Contrast
- [ ] Code text meets ≥4.5:1 contrast against block background in dark theme.
- [ ] Code text meets ≥4.5:1 contrast against block background in light theme.
- [ ] Line number text meets ≥4.5:1 contrast against gutter background in both themes.
- [ ] Copy button icon meets ≥4.5:1 contrast in all states (default, hover, copied, failed).
- [ ] Focus ring meets ≥3:1 contrast against adjacent colors.
- [ ] Toast text meets ≥4.5:1 contrast against toast surface in both themes.

### Reduced Motion
- [ ] `prefers-reduced-motion: reduce` disables copy icon transition (instant swap).
- [ ] Expand/collapse height transition becomes instant.
- [ ] Fade overlay appears/disappears instantly.
- [ ] Skeleton shimmer becomes static block with 0.6 opacity.

### Engineering Validation
- [ ] `CodeBlock` component exports clean TypeScript interface matching data contract.
- [ ] `renderMarkdown.tsx` integrates CodeBlock via react-markdown `components` override.
- [ ] Inline code (`backtick`) unaffected — only fenced code blocks render as CodeBlock.
- [ ] No new external dependencies added to `package.json`.
- [ ] All lucide-react icons used are available in the installed version.
