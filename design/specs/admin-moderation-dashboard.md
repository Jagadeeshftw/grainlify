# Admin Moderation Dashboard — Design Specification

## Overview

The Admin Moderation Dashboard provides a dedicated interface for administrators to review and act on programs flagged by community reports or automated risk scoring. It follows the glassmorphism aesthetic defined in [`design-tokens.json`](../../design-tokens.json) and is fully WCAG 2.1 AA compliant.

### File Manifest

| File | Purpose |
|---|---|
| `AdminPage.tsx` | Feature integration — hosts moderation queue, drawer, bulk actions, and audit log |
| `components/types.ts` | Shared TypeScript type definitions for all moderation entities |
| `components/SeverityBadge.tsx` | Severity indicator badge (low/medium/high/critical) |
| `components/ModerationQueue.tsx` | Flagged program list with search, filter, sort, and selection |
| `components/ProgramModerationDrawer.tsx` | Single-program slide-over drawer with flag history, evidence, and action buttons |
| `components/BulkActionToolbar.tsx` | Bulk select/action toolbar with Radix AlertDialog confirmation |
| `components/ActionHistoryTable.tsx` | Audit log table with action/category filtering |

---

## 1. Flagged-Program List View (1440px mockup)

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│  ⚑ Moderation Queue                    [Queue] [Audit Log] │
│  Review and manage flagged programs...                      │
├─────────────────────────────────────────────────────────────┤
│  [Search...               ] [All] [Low] [Med] [Hi] [Crit]  │
├─────────────────────────────────────────────────────────────┤
│  ☐ 0 of 5 selected  Clear    [Warn] [Pause] [Terminate]    │
├─────────────────────────────────────────────────────────────┤
│  ☐ ┌──────────────────────────────────────────────────┐    │
│    │ [icon] quantum-consensus    [Skull Critical 12]  │    │
│    │        Multiple users reported...  92 Risk  👤.. │    │
│    └──────────────────────────────────────────────────┘    │
│  ☐ ┌──────────────────────────────────────────────────┐    │
│    │ [icon] green-energy-dao     [⚠ High 5]           │    │
│    │        Unverified third-party...  74 Risk  👤..  │    │
│    └──────────────────────────────────────────────────┘    │
│  ...                                                       │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

- **Glass cards**: Each program row uses `backdrop-blur-[25px]` with `rgba(255,255,255,0.06)` (dark) or `rgba(255,255,255,0.12)` (light) backgrounds
- **Severity color mapping**:
  - Low: Blue (`bg-blue-500/20`)
  - Medium: Amber (`bg-amber-500/20`)
  - High: Orange (`bg-orange-500/20`)
  - Critical: Red (`bg-red-500/20`)
- **Risk score indicator**: Circular badge colored by threshold (≥80 red, ≥50 orange, <50 amber)
- **Hover state**: Card scales to 1.01 with increased background opacity
- **Selected state**: Gold border accent (`border-[#c9983a]/30`) with tinted background
- **Accessibility**: Each row is a focusable `listitem` with keyboard activation (Enter/Space)

### Search and Filtering

| Control | Behavior |
|---|---|
| Search input | Filters by program name or flag reason (case-insensitive) |
| Severity pills | `All`, `Low`, `Medium`, `High`, `Critical` — active state shown with gold accent |
| Sort dropdown | `Severity` (default, descending), `Risk Score` (descending), `Date` (newest first) |

---

## 2. Severity Badge Specification

### Variants

| Severity | Icon | Dark Mode | Light Mode | Contrast Ratio |
|---|---|---|---|---|
| Low | `Info` (blue) | `text-blue-300` / `bg-blue-500/20` | `text-blue-700` / `bg-blue-500/10` | ≥4.5:1 |
| Medium | `AlertCircle` (amber) | `text-amber-300` / `bg-amber-500/20` | `text-amber-700` / `bg-amber-500/10` | ≥4.5:1 |
| High | `AlertTriangle` (orange) | `text-orange-300` / `bg-orange-500/20` | `text-orange-700` / `bg-orange-500/10` | ≥4.5:1 |
| Critical | `Skull` (red) | `text-red-300` / `bg-red-500/20` | `text-red-700` / `bg-red-500/10` | ≥4.5:1 |

### Accessible Design

- Icon + text label ensures color is not the sole differentiator (WCAG 1.4.1)
- `role="status"` and `aria-label` includes severity level and flag count
- Minimum 4.5:1 contrast ratio against all backgrounds per WCAG 2.1 AA
- Touch target ≥44px when used as interactive element

---

## 3. Single-Program Moderation Drawer (1440px mockup)

### Layout

```
┌──────────────────────────────────────────────────┐
│  ╳  Shield Moderation Review                     │
├──────────────────────────────────────────────────┤
│                                                  │
│  quantum-consensus            [Skull Crit 12]    │
│  A quantum-resistant consensus protocol...       │
│  👤 by quantum_labs  🕐 Created Nov 15, 2025    │
│  🔗 View Program                                │
│  ─────────────────────────────────────           │
│                                                  │
│  ⚑ Flag History (3)                             │
│  ┌──────────────────────────────────────────┐   │
│  │ [Skull Crit] [Automated]  Jun 24, 2026   │   │
│  │ Suspicious token distribution pattern...  │   │
│  │ Reported by: automated-scanner           │   │
│  └──────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────┐   │
│  │ [⚠ High]              Jun 23, 2026       │   │
│  │ Community report: misleading docs...     │   │
│  │ Reported by: trusted_user_42  📄 Evidence│   │
│  └──────────────────────────────────────────┘   │
│  ...                                            │
│  ─────────────────────────────────────           │
│                                                  │
│  Actions                                         │
│  ┌─ [⚠ Send Warning] ──────────────────────┐   │
│  │  This will notify the program owner...   │   │
│  │  [Confirm Send Warning] [Cancel]         │   │
│  └──────────────────────────────────────────┘   │
│  [⏸ Pause Program]                              │
│  [⛔ Terminate Program]                          │
│                                                  │
└──────────────────────────────────────────────────┘
```

### Key Design Decisions

- **Full-height drawer**: 540px max-width slides in from right with `translate-x` transition (300ms, ease-out)
- **Z-index stacking**: `z-50` with overlay backdrop handled by parent
- **Scrollable content**: Uses `ScrollArea` component (Radix ScrollArea primitive) for inner scrolling
- **In-drawer confirmation**: Action buttons expand inline confirmation with description and confirm/cancel — no separate modal needed for single actions
- **Evidence links**: External link icon with `FileText` icon, opens in new tab with `rel="noopener noreferrer"`
- **Flag history**: Reverse chronological order, each card shows severity badge, date, reason, reporter, and optional evidence link
- **Automated flags**: Purple badge `[Automated]` distinguishes bot-detected from community-reported flags

### Action Handling

| Action | Effect | Destructive | Confirmation Required |
|---|---|---|---|
| Send Warning | Notifies owner, logs audit entry | No | Yes (inline) |
| Pause Program | Halts contributions until admin re-enables | No | Yes (inline) |
| Terminate Program | Permanently removes program | Yes | Yes (inline, red styling) |

---

## 4. Bulk-Action Toolbar Specification

### Behavior

1. Appears when ≥1 program is selected via checkbox
2. Shows count: "X of Y selected"
3. "Select all" / "Deselect all" toggle
4. Three action buttons: Warn, Pause, Terminate

### Confirmation Dialog (Radix AlertDialog)

```
┌──────────────────────────────────────┐
│         [⚠]                          │
│       Terminate Selected             │
│                                      │
│  Permanently terminate 3 selected   │
│  program(s). This action cannot be  │
│  undone. All associated data will   │
│  be removed.                        │
│                                      │
│        [Cancel]  [Yes, Terminate]    │
└──────────────────────────────────────┘
```

- Uses `AlertDialog` from `@radix-ui/react-alert-dialog` (wrapped in `app/components/ui/alert-dialog.tsx`)
- Destructive actions use red icon + red button styling
- Non-destructive actions use gold accent
- `AlertDialogDescription` for full explanation
- `AlertDialogCancel` (outline) and `AlertDialogAction` (primary/destructive) for dual-button layout
- Focus trapped within dialog per Radix default behavior

---

## 5. Action-History Audit Log Table

### Layout

```
┌────────────────────────────────────────────────────────┐
│  [Search...         ] [All] [Warn] [Pause] [Term] [Res]│
├────────────────────────────────────────────────────────┤
│  Action    │ Program           │ Reason       │ By   │ Date   │
│  ─────────│───────────────────│──────────────│──────│────────│
│  ⚠ Warning │ zk-rollup-explorer│ Data...      │ alice│Jun 22  │
│  ⛔ Term.  │ quantum-consensus  │ Confirmed... │ bob  │Jun 20  │
│  ✅ Res.   │ nft-marketplace-v2 │ Access...    │ bob  │Jun 18  │
└────────────────────────────────────────────────────────┘
```

### Column Definitions

| Column | Content | Width |
|---|---|---|
| Action | Colored pill with icon + label | 120px |
| Program | Program name (linked) | 200px |
| Reason | Moderation reason text | auto |
| Performed By | Admin username | 140px |
| Date | Relative/absolute date with clock icon | 120px |

### Empty State

When no entries match filters, a centered illustration with `Shield` icon, "No audit entries found" heading, and "No moderation actions have been recorded yet" subtext is shown.

---

## 6. Glassmorphism Implementation

All containers follow the glassmorphism pattern from `design-tokens.json`:

```css
/* Dark mode */
backdrop-blur-[40px] bg-white/[0.08] border-white/10

/* Light mode */
backdrop-blur-[40px] bg-white/[0.15] border-white/20
```

- **Blur radius**: 25px (cards) to 40px (section containers)
- **Background opacity**: 0.06–0.15 depending on element hierarchy
- **Border opacity**: 0.10–0.25 with white tint
- **Elevation**: Level 2 (`0 8px 32px rgba(0,0,0,0.08)`) for section containers

---

## 7. Accessibility Compliance

| Criterion | Implementation |
|---|---|
| 1.4.1 Use of Color | Severity conveyed via icon + text label, not color alone |
| 1.4.3 Contrast (AA) | All text meets ≥4.5:1 against glass backgrounds |
| 1.4.11 Non-text Contrast | UI components (badges, buttons, borders) ≥3:1 |
| 2.1.1 Keyboard | All interactive elements focusable and activatable |
| 2.4.3 Focus Order | Logical tab order: toolbar → cards → drawer |
| 2.4.7 Focus Visible | 2px solid focus ring with 4px offset |
| 3.3.1 Error Identification | Form validation with `aria-describedby` |
| 4.1.2 Name, Role, Value | `role="list"`, `role="listitem"`, `aria-label` on all controls |
| 4.1.3 Status Messages | Toast notifications for action confirmations |

### Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  .drawer-transition { transition-duration: 0.01ms; }
  .card-hover { transform: none; }
}
```

All transitions degrade gracefully — drawer becomes instant, hover transforms become opacity-only.

---

## 8. Security Considerations

| Threat | Mitigation |
|---|---|
| XSS via evidence URLs | URLs open with `rel="noopener noreferrer"`, rendered as external links |
| Unauthorized moderation actions | All actions routed through `handleModerationAction` which can be guarded by RBAC |
| Bulk action abuse | Confirmation dialog requires explicit user acknowledgment |
| Data leakage | Drawer content scoped to single program; drawer closes on action |
| CSRF | All API endpoints assumed to have CSRF tokens (backend concern) |
| Audit tampering | Audit log is append-only on backend; UI displays what server provides |

---

## 9. Component States

### SeverityBadge
- **Normal**: Icon + label + optional count pill
- **Accessible**: `role="status"` with descriptive `aria-label`

### ModerationQueue
- **Loading**: Skeleton cards matching glass card dimensions (deferred to parent)
- **Empty**: Centered `Shield` icon with descriptive message
- **Error**: N/A (data is local state; API errors handled by parent)
- **Selected**: Gold border accent on row
- **Filtered**: Empty state when no results match filters

### ProgramModerationDrawer
- **Closed**: `translate-x-full` (offscreen right)
- **Open**: `translate-x-0` with 300ms ease-out transition
- **No program**: Returns `null`
- **Action confirmation**: Inline expandable section per action

### BulkActionToolbar
- **Hidden**: `display: none` when `selectedCount === 0`
- **Active**: Shows count, select/deselect, action buttons
- **Confirming**: AlertDialog open with destructive/non-destructive styling

### ActionHistoryTable
- **Loading**: Skeleton rows (deferred to parent)
- **Empty**: Centered message when no entries exist
- **Filtered**: Empty state when no matches
- **Error**: N/A (data is local state)

---

## 10. Testing Checklist

### Unit Tests
- `SeverityBadge` renders correct icon and colors for each severity
- `ModerationQueue` filters by search text and severity
- `ModerationQueue` sorts by severity, date, risk score
- `BulkActionToolbar` appears only when items selected
- `BulkActionToolbar` confirmation dialog triggers alert
- `ActionHistoryTable` filters by action type and search
- `ProgramModerationDrawer` calls `onAction` with correct params

### Integration Tests
- Selecting a program row opens the drawer
- Bulk select toggles all checkboxes
- Audit log tab shows entries with correct columns
- Queue tab shows filtered results

### Accessibility Tests
- All interactive elements keyboard accessible
- Focus traps in AlertDialog
- Screen reader announcements for dynamic content
- Color contrast ratios meet AA standards

---

## 11. File Map

```
frontend/src/features/admin/
├── index.ts                              # Barrel export
├── pages/
│   └── AdminPage.tsx                     # Feature integration
└── components/
    ├── index.ts                          # Component barrel export
    ├── types.ts                          # Shared type definitions
    ├── SeverityBadge.tsx                 # Severity badge component
    ├── ModerationQueue.tsx               # Flagged program list
    ├── ProgramModerationDrawer.tsx       # Single-program drawer
    ├── BulkActionToolbar.tsx             # Bulk action toolbar
    └── ActionHistoryTable.tsx            # Audit log table

design/
└── specs/
    └── admin-moderation-dashboard.md     # This document
```
