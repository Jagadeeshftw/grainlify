# Ecosystem Comparison View Design Spec

**Version:** 1.0
**Status:** Design specification and QA checklist
**Target:** `frontend/src/features/dashboard/pages/EcosystemsPage.tsx`

---

## Overview

The ecosystem comparison view enables users to compare two or three ecosystems side-by-side without leaving the EcosystemsPage list.

The comparison mode includes:
- ecosystem selection checkboxes on ecosystem cards
- maximum of three selected ecosystems
- selection counter badge
- side-by-side metrics table for desktop (`1440px`)
- horizontally scrollable comparison table for mobile (`375px`)
- delta indicators with arrows, percentage values, and semantic non-color-only encoding
- mini bar-chart row for visual comparison of selected metric values

This specification covers selection UX, data contract, interaction states, accessibility requirements, security assumptions, and QA testing.

---

## Goals

- Allow users to compare ecosystem metrics directly from the list.
- Keep a lightweight selection interface for multi-ecosystem comparison.
- Ensure the comparison table remains legible and accessible on mobile.
- Surface growth deltas clearly with both icon and text semantics.
- Provide a secure, testable design contract before implementation.

---

## Data Contract

### Ecosystem comparison model

```ts
interface EcosystemComparisonItem {
  id: string;
  name: string;
  slug: string;
  logo_url?: string | null;
  status: 'active' | 'inactive' | 'archived';
  active_bounties: number;
  total_rewards: number;
  contributor_count: number;
  yoy_growth_percent: number;
  metric_color?: string;
}

interface EcosystemComparisonMetrics {
  ecosystems: EcosystemComparisonItem[];
}
```

### Required API behavior

- `GET /ecosystems` should return ecosystem metrics required for comparison.
- `GET /ecosystems/:id` should return detailed ecosystem metrics.
- The client may compute comparison deltas locally from returned metric values.
- The server must validate that returned metrics are accurate and reflect the current authenticated environment.

---

## Ecosystem Selection Mechanism

### Selection behavior

- Each ecosystem card displays a checkbox in the top-right corner.
- Users may select up to **3** ecosystems.
- When 1–3 ecosystems are selected, a sticky comparison CTA appears at the top of the page.
- A counter badge indicates the number of selected ecosystems (`2 selected`).
- Selecting a fourth ecosystem is prevented with an inline message: `Maximum 3 ecosystems can be compared.`
- Deselection is allowed at any time.

### Selection states

- Default: empty checkbox.
- Selected: filled checkbox with accent border and check icon.
- Disabled: if 3 ecosystems are already selected, additional checkboxes remain enabled but clicking them shows inline feedback; alternatively they can be disabled once the maximum is reached.
- Hover/focus: subtle glow and accessible outline.

### Visual affordance

- Checkbox appears on each card with enough tap target for mobile.
- Selected cards use a faint accent border and background.
- The compare tray is visible once at least 2 ecosystems are selected.
- On desktop, the comparison CTA sits inline with search and filter controls.
- On mobile, the compare tray is fixed at the bottom or pinned within the page so it is always available.

---

## Comparison View Layout

### Desktop (`1440px`)

- Render a side-by-side table with 2–3 columns.
- Column headers contain ecosystem name, status chip, and logo.
- Rows include metrics:
  - Active bounties
  - Total rewards
  - Contributor count
  - YoY growth
  - Mini bar chart
- Each metric row has a delta indicator cell where applicable.
- The mini bar chart row uses normalized bars to compare values horizontally.

Example layout:
```
| Metric            | Ecosystem A | Ecosystem B | Ecosystem C |
|-------------------|-------------|-------------|-------------|
| Active bounties   | 28          | 19          | 36          |
| Total rewards     | $1.2M       | $860K       | $1.7M       |
| Contributor count | 432         | 298         | 512         |
| YoY growth        | +12.5% ▲   | -4.7% ▼    | +33.8% ▲   |
| Visual compare    | ██████      | ████        | ████████    |
```

### Mobile (`375px`)

- Use a horizontally scrollable table container.
- Keep metric labels in a fixed left column.
- Each ecosystem appears in a separate column.
- Use sticky table header row for metric labels on mobile if the layout uses a table element.
- Provide scroll hint text or gradient overlays on either edge.
- Ensure each ecosystem column remains readable with condensed metric formatting.

---

## Delta Indicators

### Visual design

- Use up/down arrow icons next to percentage values.
- Positive growth: green arrow pointing up and `+` prefix.
- Negative growth: red arrow pointing down and `-` prefix.
- Neutral change: horizontal dot or dash.
- Add text labels to avoid color-only encoding.
- Icons should include accessible descriptions via `aria-label`.

### Non-color-only encoding

- Use both icon direction and text sign.
- Add subtle background shapes or borders when space allows.
- Ensure label text reads `Up 12.5%` or `Down 4.7%`.
- Avoid relying on green/red alone for meaning.

---

## Interaction & UX

### Comparison CTA

- Appears when 2 or more ecosystems are selected.
- Shows selected count and `Compare selected` button.
- Disabled until at least 2 ecosystems are selected.
- Clicking the button opens the comparison view inline.
- On mobile, the CTA should be accessible from the bottom edge and remain visible while scrolling.

### Table interaction

- The comparison view can be dismissed with a close button.
- Users can add or remove ecosystems from the comparison directly in the tray.
- Table rows should be focusable if they contain interactive details.
- Action buttons must be operable with keyboard and screen readers.

---

## Accessibility Requirements

- Checkboxes must have clear `aria-label`s such as `Select <ecosystem name> for comparison`.
- The compare tray and comparison table must be keyboard-accessible.
- Use `aria-live="polite"` for selection count updates.
- Use `role="table"` or semantic `<table>` markup for comparison layout.
- Ensure mobile horizontal scrolling is manageable with keyboard and touch.
- Focus must move into the comparison view when opened and restore after close.
- Delta indicators must use `aria-label` when icons are present.
- Contrast ratios must meet WCAG 2.1 AA for text and table borders.

---

## Security Assumptions

- Comparison state is maintained client-side and does not affect data integrity.
- Only authorized users may access ecosystem metrics already exposed by the page.
- No sensitive user data is introduced by comparison mode.
- The application should not trust client selection state for backend operations.
- Comparison controls should not expose internal IDs or unauthorized API endpoints.

---

## QA Checklist

### Selection UI
- [ ] Each ecosystem card displays a comparison checkbox.
- [ ] Selection counter badge updates immediately.
- [ ] Maximum selection is enforced at 3 ecosystems.
- [ ] Inline feedback appears if the user tries to exceed 3 selections.
- [ ] Selected cards receive a clear visual state.

### Comparison view
- [ ] Desktop shows 2–3 side-by-side columns.
- [ ] Mobile renders a horizontally scrollable comparison table.
- [ ] Mini bar chart row renders a visual value comparison.
- [ ] Delta indicators use both icon and text semantics.
- [ ] `Compare selected` button is only enabled with 2+ selections.

### Accessibility
- [ ] Checkboxes and CTA are keyboard operable.
- [ ] Selection count uses `aria-live="polite"`.
- [ ] Comparison view has proper `role="dialog"` or semantic table markup.
- [ ] Focus is trapped in the comparison view when open.
- [ ] Horizontal scroll on mobile is accessible.
- [ ] Delta indicators are not color-only.

### Edge cases
- [ ] No ecosystems selected: comparison CTA is hidden.
- [ ] Exactly 2 ecosystems selected: comparison view works.
- [ ] Exactly 3 ecosystems selected: comparison view works.
- [ ] More than 3 ecosystems attempted: selection prevented.
- [ ] Ecosystem data missing metrics: fallback gracefully.
- [ ] Very large numeric values format cleanly.

---

## Notes for Implementation

- Build the comparison logic in a self-contained component for EcosystemsPage.
- Keep the compare tray separate from the list so it can be reused on mobile.
- Use normalized bar widths for the mini visual comparison row.
- Keep the top of page and search bar visible when comparison is active.
- Prefer semantic HTML tables where possible.

---

## Expected Artifacts

- Annotated mockups for desktop and mobile comparison view.
- NatSpec-style component contract in `frontend/src/features/dashboard/components/EcosystemComparisonSection.tsx`.
- Design docs and QA checklist in `design/specs/ecosystem-comparison-view.md`.
