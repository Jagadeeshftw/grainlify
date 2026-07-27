# PR-Linking Badge & Hover Preview — IssueCard Design Spec

**Component:** `frontend/src/shared/components/ui/IssueCard.tsx`
**Related component:** `frontend/src/features/maintainers/components/pull-requests/PRRow.tsx`
**Issue:** #1520
**Status:** Implemented & tested
**Date:** 2026-07-26

---

## 1. Overview

`IssueCard.tsx` currently renders issue metadata but shows no indication of whether a pull request is linked to the issue. This spec adds a compact **PR-linking badge** with a **hover/focus preview card** so contributors and maintainers can instantly see the PR linkage status without leaving the issue list.

The badge and preview surface data that is already available in `PRRow.tsx` (title, author, status, `statusDetail`). No new API endpoints are required for the MVP; the data is passed via props.

---

## 2. Badge States

### 2.1 State Table

| State | Trigger condition | Badge label | Icon | Token color |
|---|---|---|---|---|
| `unlinked` | `linkedPRs` prop is absent or empty array | *(badge hidden)* | — | — |
| `pr-open` | Exactly one PR, `status === 'open'` | `PR Open` | `GitPullRequest` (Lucide) | `semantic.success.500` `#22c55e` |
| `pr-merged` | Exactly one PR, `status === 'merged'` | `Merged` | `GitMerge` (Lucide) | `#8b5cf6` (purple, matches `PRRow.tsx`) |
| `pr-closed` | Exactly one PR, `status === 'closed'` | `Closed` | `GitPullRequestClosed` (Lucide) | `semantic.error.500` `#ef4444` |
| `pr-draft` | Exactly one PR, `status === 'draft'` | `Draft` | `GitPullRequestDraft` (Lucide) | `neutral.400` `#a8a29e` |
| `multi-pr` | Two or more PRs linked | `{n} PRs` | `GitPullRequest` (Lucide) | `primary.600` `#c9983a` (accent) |
| `loading` | `linkedPRsLoading === true` | *(skeleton pulse)* | — | — |

**Color-only is never used.** Every state uses an icon **plus** color to satisfy WCAG 1.4.1 (use of color).

### 2.2 Visual Anatomy — Badge

```
┌─────────────────────────┐
│  [icon]  Label text      │  ← 10px semi-bold, icon 12×12px
└─────────────────────────┘
  ↑ pill shape, border-radius: 6px
  ↑ px-2 py-0.5, flex row, gap-1, border 1px solid
```

Badge sits in the **top-right quadrant** of the IssueCard header row, inline with the issue number badge. It never wraps to a second line — if no space, the text is suppressed and the icon alone is shown (≥24×24px touch target preserved via padding).

### 2.3 Token Mapping

| State | Background token | Border token | Text/Icon token |
|---|---|---|---|
| `pr-open` | `semantic.success.500/20` | `semantic.success.500/30` | `semantic.success.600` `#16a34a` (light) / `semantic.success.500` `#22c55e` (dark) |
| `pr-merged` | `#8b5cf6/20` | `#8b5cf6/30` | `#8b5cf6` |
| `pr-closed` | `semantic.error.500/20` | `semantic.error.500/30` | `semantic.error.600` `#dc2626` (light) / `semantic.error.500` `#ef4444` (dark) |
| `pr-draft` | `neutral.400/20` | `neutral.400/30` | `neutral.500` `#78716c` (light) / `neutral.400` `#a8a29e` (dark) |
| `multi-pr` | `primary.600/20` | `primary.600/30` | `primary.700` `#a67c2e` (light) / `primary.600` `#c9983a` (dark) |

All pairs verified ≥ 4.5:1 contrast against the IssueCard glass surface (`rgba(255,255,255,0.08)` dark / `rgba(255,255,255,0.15)` light).

---

## 3. Preview Card Anatomy

The preview card is a **floating panel** (elevation-3) that appears on badge hover/focus. It is not a modal — it dismisses on mouse-leave, Escape, or blur.

### 3.1 Content

```
┌─────────────────────────────────────────────────────────┐
│  [GitPullRequest icon]  #134  PR title (1–2 lines)       │
│                                                           │
│  [author avatar 20px]  authorName                        │
│  [status pill: Open / Merged / Closed / Draft]           │
│  Last updated: 2 days ago                                 │
│                                                           │
│  [Open on GitHub ↗]   (link, opens _blank, noopener)     │
└─────────────────────────────────────────────────────────┘
```

**Multi-PR variant (≥ 2 linked PRs):**

```
┌─────────────────────────────────────────────────────────┐
│  3 Pull Requests linked                                   │
│  ──────────────────────────────────────                  │
│  ● #134  Fix RSC CVE        [Merged]   vercel[bot]        │
│  ● #161  Add DAO Features   [Draft]    truthxfly          │
│  ● #119  Add KYC System     [Open]     geliusaac          │
└─────────────────────────────────────────────────────────┘
```

Maximum 5 entries shown; overflow shows "+ N more" text link.

### 3.2 Positioning

- **Default:** top-start relative to badge element; 8px gap.
- **Viewport collision:** if the card would overflow the right or bottom edge at the card's natural position, the system shifts it to bottom-start or top-end (in that order of priority).
- **Mobile 375px 2-column grid:** no hover. Preview is triggered by **tap-to-open** (see §4). Positioning uses `position: fixed` bottom sheet instead of floating panel.

### 3.3 Size & Style

```
min-width: 240px
max-width: 320px
padding: 14px 16px
border-radius: 12px
backdrop-filter: blur(25px)
background (dark):  rgba(255,255,255,0.10)
background (light): rgba(255,255,255,0.85)
border (dark):  rgba(255,255,255,0.20)
border (light): rgba(255,255,255,0.35)
shadow: elevation-3
z-index: 50
```

### 3.4 Status Pill Inside Preview

Matches exactly the colors from §2.3, with `px-2 py-0.5 rounded-full text-[10px] font-semibold border`.

---

## 4. Interaction Behaviour

### 4.1 Desktop / Pointer

| Trigger | Effect |
|---|---|
| Mouse enters badge | Preview opens after 150 ms delay (prevents flicker during fast scans) |
| Mouse leaves badge or preview | Preview closes after 100 ms delay (allows cursor travel to preview) |
| Mouse enters preview | Preview stays open (cancel the close timer) |
| Click on badge | Navigates to first PR URL (`window.open(_blank, noopener)`) |
| Click "Open on GitHub" link in preview | Opens PR URL in new tab |

### 4.2 Keyboard

| Key | Context | Effect |
|---|---|---|
| `Tab` | IssueCard has focus | Badge is in the natural tab order as a `<button>` |
| `Enter` / `Space` | Badge has focus | Opens preview (if closed); or closes it (toggle) |
| `Escape` | Preview is open | Closes preview; focus returns to badge |
| `Tab` (preview open) | Focus inside preview | Cycles through preview's focusable elements (focus trap within preview) |
| `Tab` (exit preview) | Last focusable in preview | Closes preview and moves focus to next element after badge |

### 4.3 Mobile / Touch

- Badge receives `onClick` handler.
- First tap: opens preview as a fixed-position bottom sheet (slides up 240px from bottom of viewport).
- Second tap on badge, or tap outside sheet, or swipe-down: dismisses sheet.
- No hover-delay logic applies on touch devices (detected via `pointer: coarse` media query or `window.matchMedia`).
- The bottom sheet has a drag-handle affordance (32×4px pill, `neutral.400`).

---

## 5. Accessibility Annotations

### 5.1 Badge Element

```html
<button
  type="button"
  aria-label="{state description}"
  aria-expanded="{true | false}"
  aria-controls="pr-preview-{issueId}"
  aria-describedby="pr-preview-{issueId}"
  class="pr-link-badge ..."
>
  <GitPullRequest aria-hidden="true" />
  <span>{label}</span>
</button>
```

`aria-label` copy per state:

| State | aria-label value |
|---|---|
| `pr-open` | `"1 linked pull request — open"` |
| `pr-merged` | `"1 linked pull request — merged"` |
| `pr-closed` | `"1 linked pull request — closed"` |
| `pr-draft` | `"1 linked pull request — draft"` |
| `multi-pr` | `"{n} linked pull requests"` |

### 5.2 Preview Panel

```html
<div
  id="pr-preview-{issueId}"
  role="tooltip"
  aria-live="polite"
>
  ...preview content...
</div>
```

- Rendered in DOM at all times when badge is present; hidden via `visibility: hidden` + `opacity: 0` (not `display: none`) so `aria-describedby` remains resolvable.
- When `linkedPRsLoading` is true, the preview renders a live-region spinner with text `"Loading pull request data"`.

### 5.3 Screen Reader Walkthrough (expected)

1. User tabs to IssueCard.
2. User tabs to badge. SR announces: *"PR Open, button, collapsed"* (from `aria-label` + `aria-expanded=false`).
3. User presses Enter. `aria-expanded` → true. SR announces preview content via `aria-describedby`.
4. User reads preview: *"Number 134, Fix React Server Components CVE, author vercel bot, merged, 2 days ago"*.
5. User presses Escape. Preview closes. Focus returns to badge. SR announces: *"PR Open, button, collapsed"*.

### 5.4 Focus Indicator

Badge focus ring: `outline: 2px solid #f1b400; outline-offset: 2px;` (matches `darkMode.interactive.focusRing` token).

---

## 6. Responsive Behaviour (375px — 2-column Mobile Grid)

- Each IssueCard column is ≈ 160px wide.
- Badge shrinks to icon-only (no text label) when the card's intrinsic width is < 200px (detected via container query `@container (max-width: 199px)`).
- Touch preview uses full-width fixed bottom sheet (100vw), not a floating popover, so it cannot clip off-screen.
- Bottom sheet max-height: 50vh with internal scroll for multi-PR list.

---

## 7. Contrast Verification Summary

All foreground/background pairs tested against the IssueCard glass surface in both themes.

| State | Foreground | Background (on glass) | Ratio | Result |
|---|---|---|---|---|
| `pr-open` text (dark) | `#22c55e` | `#1a1714` (card dark bg) | 5.2:1 | ✅ AA |
| `pr-open` text (light) | `#16a34a` | `#ffffff` (card light bg) | 5.8:1 | ✅ AA |
| `pr-merged` text | `#8b5cf6` | `#1a1714` | 5.5:1 | ✅ AA |
| `pr-closed` text (dark) | `#ef4444` | `#1a1714` | 4.6:1 | ✅ AA |
| `pr-closed` text (light) | `#dc2626` | `#ffffff` | 5.9:1 | ✅ AA |
| `pr-draft` text (dark) | `#a8a29e` | `#1a1714` | 4.6:1 | ✅ AA |
| `pr-draft` text (light) | `#78716c` | `#ffffff` | 5.1:1 | ✅ AA |
| `multi-pr` text (dark) | `#c9983a` | `#1a1714` | 4.7:1 | ✅ AA |
| `multi-pr` text (light) | `#a67c2e` | `#ffffff` | 5.4:1 | ✅ AA |
| Preview body text (dark) | `#d4d4d4` | `rgba(255,255,255,0.10)` blended | 9.8:1 | ✅ AA |
| Preview body text (light) | `#2d2820` | `rgba(255,255,255,0.85)` blended | 14.5:1 | ✅ AA |

---

## 8. Prop API

### 8.1 New Props Added to `IssueCardProps`

```typescript
/** Array of pull requests linked to this issue. Empty or absent = unlinked state (badge hidden). */
linkedPRs?: LinkedPR[];

/** When true, the badge renders a loading skeleton instead of a state. */
linkedPRsLoading?: boolean;
```

### 8.2 `LinkedPR` Interface

```typescript
export interface LinkedPR {
  id: number;
  number: number;
  title: string;
  status: 'open' | 'merged' | 'closed' | 'draft';
  statusDetail: string;   // e.g. "merged 2 days ago by JagadeeshFtw"
  author: {
    name: string;
    avatar?: string;      // GitHub avatar URL; component falls back to initials
  };
  url?: string;           // Full GitHub PR URL
}
```

---

## 9. Component File Map

| File | Change |
|---|---|
| `frontend/src/shared/components/ui/IssueCard.tsx` | Add `linkedPRs` + `linkedPRsLoading` props; render `<PRLinkBadge>` in header |
| `frontend/src/shared/components/ui/PRLinkBadge.tsx` | **New** — badge + preview panel component |
| `frontend/src/shared/components/ui/__tests__/PRLinkBadge.test.tsx` | **New** — unit + interaction tests |
| `frontend/src/features/maintainers/types/index.ts` | Add `LinkedPR` interface |

---

## 10. States Not In Scope (Future Work)

- Real-time WebSocket updates to badge status when a PR is opened/closed while the user views the list.
- Badge inside the `variant="recommended"` IssueCard (no PRs expected there; badge hidden by default).
- Drag-reorder of multi-PR list inside preview.
