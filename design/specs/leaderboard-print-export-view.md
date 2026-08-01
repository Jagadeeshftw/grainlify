# Leaderboard print/export view spec

## Summary

This spec defines a dedicated, print-friendly export experience for the leaderboard tables on the live page. The export surface is intentionally flatter than the live glassmorphism treatment so it remains legible in print and PDF workflows. The export view inherits the existing ranking semantics, preserves the numeric rank column, and supports paginated output for long lists.

## Related spec

- Cross-reference: [design/leaderboard-spec.md](../../design/leaderboard-spec.md)
- Token source: [design-tokens.json](../../design-tokens.json)

## Layout goals

- Flattened backgrounds and borders for print/PDF output
- High-contrast typography with a condensed row height
- Header/footer metadata including the export date and filter context
- Repeated table headers on each printed page
- An on-screen export trigger that remains accessible and compact on mobile

## States

### Screen view
- The live page keeps the current glassmorphism experience.
- An export trigger appears above the table region and is labeled clearly as "Export ranking".
- The trigger is keyboard reachable via Tab and operable by Enter or Space.

### Print preview / export mode
- The printable surface uses white or neutral paper-like backgrounds.
- Borders are solid and dark enough to meet WCAG AA contrast targets.
- Tables use a condensed row height and reduce decorative effects.
- The header includes the page title, export date, and active filter context such as "Top Contributors — This Month".

### Paginated output
- Long tables are split into pages of 18 rows each.
- Table headers repeat at the top of each printed page.
- Page numbers appear in the footer along with the export date and filter context.

### Empty filtered result
- If no data matches the active filters, the export view renders a short empty-state summary instead of a blank page.
- The empty-state still reports the active filter context and includes a numeric rank column placeholder.

## Visual treatment

### Print palette
Use the neutral and primary tokens from [design-tokens.json](../../design-tokens.json) to keep the print palette accessible and predictable.

- Page background: neutral 50 (#fafaf9)
- Table background: neutral 50 (#fafaf9)
- Row borders: neutral 300 (#d6d3d1)
- Header/footer text: neutral 900 (#1c1917)
- Accent text and focus rings: primary 600 (#c9983a)
- Status badges: semantic success/error/warning at accessible combinations

### Typography
- Base text size: 10–11 pt for body rows
- Header text: 11–12 pt, semibold
- Title in header: 14–16 pt, bold
- Line height: 1.2–1.3 for rows to keep the page compact

## Export trigger UI

- Placement: anchored above the leaderboard table, aligned to the right on desktop and centered/left-aligned on small viewports.
- Control: single button labeled "Export ranking".
- Format choice: present a compact select labeled "Export format" with options for "Print" and "PDF". The default is "Print".
- On mobile (375px width), the control remains visible without overlapping the table and stacks cleanly.

## Accessibility annotations

- The export trigger uses an explicit accessible name, not only an icon.
- The numeric rank column remains visible in exported output; rank is never conveyed by color alone.
- Focus styling uses a visible outline and does not rely on color.
- The export surface supports reduced motion by avoiding decorative animation in print mode.

## Pagination rules

- For lists of 1–18 rows, export as a single page.
- For lists of 19–36 rows, export as two pages.
- Pages use 18 rows per page, with the final page containing the remainder.
- Table headers repeat on every page after the first.
- Page numbers appear in the footer and start at 1.

## QA expectations

- Confirm printed text and borders meet a minimum 4.5:1 contrast ratio in the flattened palette.
- Verify keyboard-only users can reach and activate the export trigger from the live LeaderboardPage.
- Review the trigger placement at 375px viewport width to ensure it does not obscure the table header or content.
- Ensure the export layout remains readable in print preview and PDF export without glassmorphism treatment or translucent overlays.
