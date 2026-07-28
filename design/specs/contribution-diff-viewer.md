# Contribution Diff Viewer Spec

**Component:** `frontend/src/shared/components/ui/ContributionDiffViewer.tsx`
**Integration:** `frontend/src/features/dashboard/pages/IssueDetailPage.tsx` and `frontend/src/features/maintainers/components/issues/IssuesTab.tsx`
**Design tokens:** [`design-tokens.json`](../../design-tokens.json)
**Status:** Frontend contract and UI implementation
**Breakpoint:** `768px`

## Overview

Issue detail can receive an optional pull-request diff payload without coupling the page to a new API shape. When the payload is available, the viewer shows changed files inline with the issue context. When it is absent, the page does not invent or render sample content.

The component is intentionally prop-driven. A future API adapter can map GitHub patches to the documented contract without changing the layout or interaction model.

## Component Anatomy

```text
ContributionDiffViewer
├── Toolbar
│   ├── Pull request title, number, author, and additions/deletions
│   ├── Open on GitHub link
│   └── View mode toggle: Side-by-side | Inline
└── DiffFile[]
    ├── File path header
    │   ├── file icon and path
    │   ├── renamed-from path when present
    │   └── additions/deletions summary
    ├── Hunk header: @@ -old +new @@
    └── Diff rows
        ├── Side-by-side: old line number + marker + code | new line number + marker + code
        └── Inline: one line-number + marker + code row per changed side
```

### Side-by-side anatomy

- The file path header spans the full viewer width and stays above the hunk rows.
- Each row has four visual columns: old line number, old code, new line number, and new code.
- Context rows repeat the same line on both sides.
- Removed-only rows leave the new side empty. Added-only rows leave the old side empty.
- Replacement pairs render the removed line on the left and added line on the right.
- Long code lines use horizontal scrolling instead of wrapping so line alignment remains stable.
- Side-by-side is the automatic default at `768px` and wider.

### Inline anatomy

- Inline mode uses one line-number gutter, one marker column, and one code column.
- A replacement pair renders the removed line first, followed by the added line.
- Context rows render once, not twice.
- The inline layout is the automatic default below `768px` and remains available at every width through the toggle.
- If a user explicitly selects side-by-side on a compact viewport, the diff keeps its minimum readable width and scrolls horizontally. The automatic default is not overridden silently.

### View-mode toggle

- Placement: top-right of the viewer toolbar, after diff stats and before the external GitHub link when space allows.
- Control: two adjacent buttons in a labeled group, each using `aria-pressed`.
- Labels: `Side-by-side view` and `Inline view`.
- The selected mode has a solid accent treatment and a visible text label. The inactive mode retains a visible border and text label.
- Keyboard order follows the toolbar: metadata, view toggle buttons, then external link.

## State Model

| State | Trigger | UI behavior |
|---|---|---|
| `side-by-side` | Effective mode at `>=768px`, or user selection | Four-column old/new table with synchronized row heights. |
| `inline` | Effective mode below `768px`, or user selection | Single-column chronological diff with explicit add/remove markers. |
| `collapsed-hunk-expanded` | User activates an unchanged-hunk marker | The marker is replaced by its omitted rows. The button changes to `Collapse unchanged lines`. |
| `binary-file` | `file.isBinary === true` | Show file path and `Binary file preview is not supported.` Do not render unreadable bytes. |
| `loading-diff` | `status === 'loading-diff'` | Show a labeled skeleton for toolbar, file header, and rows. Set `aria-busy="true"`. |
| `unsupported-preview` | No diff payload or upstream response cannot provide patches | Keep the host page free of fake content. If the component is mounted in this state, announce that the preview is unavailable and link to the external PR when present. |

## Large-Diff Handling

- Unchanged runs of more than 8 rows are collapsed by the data adapter into a `collapsed-hunk` row.
- The marker text is `+{count} lines unchanged` and includes a down-chevron icon.
- The marker is a real button with `aria-expanded="false"` and `aria-controls` pointing to the hidden row group.
- Expanding replaces the marker in place, preserves the surrounding hunk order, and sets `aria-expanded="true"`.
- A partial file shows a `Load full file` action in the file footer. It is only rendered when `file.isPartial` is true and calls `onLoadFullFile(file.path)`.
- Loading the full file keeps the current hunk visible and changes the action label to `Loading full file` with `aria-busy="true"`.
- There is no automatic full-file fetch on scroll. This avoids large DOM work and keeps the contributor in control of expensive requests.

## Accessibility

- Each file is a `role="region"` with `aria-label="Diff for {file path}"`.
- Each hunk header is visible text and is not conveyed by color alone.
- Added lines expose `Added line` text and a plus icon. Removed lines expose `Removed line` text and a minus icon. The row background is supplemental only.
- Line numbers are visible code-navigation aids and are labelled by their associated row. They are not the only identifier for a change.
- The collapsed-hunk marker and `Load full file` action are native buttons in the tab order.
- View-mode buttons use `aria-pressed`; no positive `tabindex` is used.
- Loading updates a polite status region and marks the viewer busy.
- Focus indicators use the existing theme focus ring. Reduced motion is inherited from the global theme stylesheet.
- The code surface supports horizontal scrolling without trapping keyboard focus.

## Token and Contrast Validation

The viewer uses the existing neutral, semantic, and high-contrast token values. Marker colors are paired with icons and labels for color-blind safety.

| Element | Light theme | Dark theme | Contrast |
|---|---|---|---:|
| Added marker | `#15803d` on `#f0fdf4` | `#22c55e` on `#2d2820` | `4.79:1` / `6.42:1` |
| Removed marker | `#b91c1c` on `#fef2f2` | `#ff6e6e` on `#2d2820` | `5.91:1` / `5.37:1` |
| Code text | `#292524` on `#fafaf9` | `#f5f5f5` on `#2d2820` | `14.52:1` / `13.41:1` |
| Line numbers | `#78716c` on `#fafaf9` | `#b8a898` on `#2d2820` | `4.59:1` / `6.33:1` |

The dark removed marker uses the high-contrast error token `#ff6e6e` because the regular `#ef4444` token is only `3.89:1` on the dark code surface.

## Contract

```ts
interface ContributionDiffViewerProps {
  diff?: ContributionDiff | null;
  status?: 'ready' | 'loading-diff' | 'unsupported-preview';
  defaultViewMode?: 'side-by-side' | 'inline';
  onLoadFullFile?: (path: string) => void | Promise<void>;
}
```

The full TypeScript contract is exported from the component file. `ContributionDiff` is intentionally independent of GitHub response fields so a future API adapter remains replaceable.

## Annotated Redlines

```text
Toolbar                                           [1] view toggle
┌──────────────────────────────────────────────────────────────────────┐
│ #42 Improve wallet flow   +12 -4   [Side-by-side] [Inline]  GitHub > │
└──────────────────────────────────────────────────────────────────────┘
                         [1] top-right, always reachable by keyboard

File header                                        [2] region label
┌──────────────────────────────────────────────────────────────────────┐
│ * frontend/src/routes.tsx                              +8  -2         │
└──────────────────────────────────────────────────────────────────────┘
                         [2] path is visible and announced by region

Side-by-side row
┌────────┬──────────────┬────────┬─────────────────────────────────────┐
│  41    │ -  old code  │        │                                     │
├────────┼──────────────┼────────┼─────────────────────────────────────┤
│        │              │  41    │ +  new code                         │
└────────┴──────────────┴────────┴─────────────────────────────────────┘
 [3] marker icon + text prefix; background color is never the only signal

Collapsed unchanged hunk
┌──────────────────────────────────────────────────────────────────────┐
│              [v] +42 lines unchanged                                 │
└──────────────────────────────────────────────────────────────────────┘
                         [4] button expands in place, no scroll jump
```

## QA Checklist

- [ ] At `>=768px`, a fresh viewer defaults to side-by-side.
- [ ] Below `768px`, a fresh viewer defaults to inline.
- [ ] Toggle is reachable with Tab and changes `aria-pressed` with Enter or Space.
- [ ] Added and removed rows expose text plus icon semantics in both themes.
- [ ] Added/removed marker and line-number colors meet `4.5:1` in both themes.
- [ ] Collapsed hunk marker expands and collapses with keyboard activation.
- [ ] `Load full file` calls the supplied callback and exposes a loading label when controlled by the host.
- [ ] Binary files render the unsupported message without attempting to display patch bytes.
- [ ] Loading state exposes `aria-busy` and a polite status message.
- [ ] Horizontal scrolling is available for long side-by-side lines.
- [ ] Focus remains visible in light, dark, high-contrast, and reduced-motion themes.

## Logic Tracking

- To find view-mode selection and responsive defaults visit [ContributionDiffViewer.tsx](file:///C:/Stellar%20Contributions/Grainlify/frontend/src/shared/components/ui/ContributionDiffViewer.tsx).
- To find issue-detail prop plumbing visit [IssueDetailPage.tsx](file:///C:/Stellar%20Contributions/Grainlify/frontend/src/features/dashboard/pages/IssueDetailPage.tsx) and [IssuesTab.tsx](file:///C:/Stellar%20Contributions/Grainlify/frontend/src/features/maintainers/components/issues/IssuesTab.tsx).
- To find interaction coverage visit [ContributionDiffViewer.test.tsx](file:///C:/Stellar%20Contributions/Grainlify/frontend/src/shared/components/ui/__tests__/ContributionDiffViewer.test.tsx).
