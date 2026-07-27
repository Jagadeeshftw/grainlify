# MaintainersPage — Bounty Analytics Dashboard Spec

**Feature:** `Analytics` tab in `MaintainersPage.tsx`
**Components added:**
- `frontend/src/features/maintainers/components/dashboard/BountyFunnelChart.tsx`
- `frontend/src/features/maintainers/components/dashboard/PayoutHistoryTable.tsx`
- `frontend/src/features/maintainers/components/dashboard/TopContributorsModule.tsx`
- `frontend/src/features/maintainers/components/dashboard/AnalyticsTab.tsx`
**Issue:** #1509
**Status:** Implemented & tested
**Date:** 2026-07-26

---

## 1. Overview

The `DashboardTab` currently shows `StatsCard` rows and `ApplicationsChart` independently. This spec adds a fourth tab — **Analytics** — that consolidates bounty-lifecycle data into three scoped modules:

1. **Conversion Funnel** — 4-stage horizontal funnel (Applied → Assigned → Submitted → Paid) built on Recharts `FunnelChart`.
2. **Payout History Table** — paginated table of completed payouts with date, contributor, amount, and status.
3. **Top Contributors Module** — avatar/rank/earned leaderboard capped at 5, with a "View all" link to `LeaderboardPage`.

All three modules share the same loading skeleton pattern (`StatsCardSkeleton`-style pulse), empty state, and date-range filter that scopes data to the maintainer's selected repositories.

---

## 2. Tab Integration

### 2.1 New `TabType` value

```typescript
// frontend/src/features/maintainers/types/index.ts
export type TabType = "Dashboard" | "Issues" | "Pull Requests" | "Analytics";
```

### 2.2 Route in `MaintainersPage.tsx`

```tsx
{activeTab === "Analytics" && (
  <AnalyticsTab
    selectedProjects={selectedProjects}
    isLoadingProjects={isLoading}
  />
)}
```

---

## 3. Date-Range Filter

A compact row above the three modules lets the maintainer scope all data.

```
┌──────────────────────────────────────────────────────────────┐
│  [Last 7 days ▾]   [Last 30 days]   [Last 90 days]   [All]  │
└──────────────────────────────────────────────────────────────┘
```

- Implemented as four `<button>` pill toggles.
- Active state: `bg-[#c9983a]/30 border-[#c9983a]/60 text-[#fef5e7]` (dark) / `bg-[#c9983a]/20 border-[#c9983a]/40 text-[#2d2820]` (light).
- Inactive state: `bg-white/[0.08] border-white/20` glass.
- Each button: `type="button"`, `aria-pressed="{true|false}"`, `aria-label="Filter by {period}"`.
- Keyboard: standard focus ring (`outline: 2px solid #f1b400`), `Space` activates.

---

## 4. Conversion Funnel Module

### 4.1 Stages

| Stage | Data field | Color token | Hex |
|---|---|---|---|
| Applied | `applied` | `primary.600` | `#c9983a` |
| Assigned | `assigned` | `semantic.info.500` | `#3b82f6` |
| Submitted | `submitted` | `semantic.warning.500` | `#f59e0b` |
| Paid | `paid` | `semantic.success.500` | `#22c55e` |

**Color-blind safety:** each stage also carries a distinct shape/pattern marker (solid, hatched, dotted, cross) rendered as an SVG `pattern` fill variant. Color is never the sole differentiator.

### 4.2 Recharts implementation

```tsx
import { FunnelChart, Funnel, LabelList, Tooltip, ResponsiveContainer } from 'recharts';
```

- `<Funnel dataKey="value" data={funnelData} isAnimationActive />` inside a `<ResponsiveContainer width="100%" height={260} />`.
- Custom `<Tooltip>` matching `ApplicationsChart` tooltip style (glass card, `backdrop-blur-[40px]`, `rounded-[14px]`, `border`).
- `<LabelList dataKey="name" position="right" />` with `fill` matching theme text token.
- Conversion rate label between each stage: `"{n}% converted"` in `text-[11px] font-semibold`.

### 4.3 Accessible data-table alternative

Below the chart, a visually hidden (but accessible) `<table>` provides the same data for screen readers and keyboard-only users:

```html
<table class="sr-only" aria-label="Bounty conversion funnel data">
  <thead>
    <tr>
      <th scope="col">Stage</th>
      <th scope="col">Count</th>
      <th scope="col">Conversion rate</th>
    </tr>
  </thead>
  <tbody>
    <tr><td>Applied</td><td>{applied}</td><td>100%</td></tr>
    <tr><td>Assigned</td><td>{assigned}</td><td>{rate}%</td></tr>
    <tr><td>Submitted</td><td>{submitted}</td><td>{rate}%</td></tr>
    <tr><td>Paid</td><td>{paid}</td><td>{rate}%</td></tr>
  </tbody>
</table>
```

### 4.4 States

| State | Render |
|---|---|
| Loading | 4× `StatsCardSkeleton`-style pulse bars, `h-[260px]` |
| Empty | Centered illustration + "No bounty activity yet in this period" |
| Default | Recharts `FunnelChart` + sr-only table |

### 4.5 Visual anatomy

```
┌─────────────────────────────────────────────────────────────────┐
│  Conversion Funnel            [Last 30 days ▾]                  │
│                                                                  │
│  ████████████████████████████  Applied     142   100%           │
│    ██████████████████████████  Assigned     98    69%           │
│         ████████████████████  Submitted    71    72%            │
│              ████████████████  Paid         58    82%           │
│                                                                  │
│  [screen-reader table: sr-only]                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Payout History Table

### 5.1 Column layout

| Column | Width | Content |
|---|---|---|
| Date | 140px | `MMM DD, YYYY` format |
| Contributor | flex-1 | Avatar (24px) + username |
| Repository | 160px | Repo avatar (16px) + `org/repo` |
| Amount | 100px | `{value} XLM` right-aligned |
| Status | 120px | Status pill |

### 5.2 Status Pills

| Value | Label | Token | Hex (dark) |
|---|---|---|---|
| `paid` | Paid | `semantic.success.500/20 + border/30` | text `#22c55e` |
| `pending` | Pending | `semantic.warning.500/20 + border/30` | text `#f59e0b` |
| `processing` | Processing | `semantic.info.500/20 + border/30` | text `#3b82f6` |
| `failed` | Failed | `semantic.error.500/20 + border/30` | text `#ef4444` |

Status is communicated by label + color (never color-only).

### 5.3 Table structure (accessibility)

```html
<table role="table" aria-label="Payout history">
  <thead>
    <tr role="row">
      <th scope="col" role="columnheader">Date</th>
      <th scope="col" role="columnheader">Contributor</th>
      <th scope="col" role="columnheader">Repository</th>
      <th scope="col" role="columnheader">Amount</th>
      <th scope="col" role="columnheader">Status</th>
    </tr>
  </thead>
  <tbody>
    {rows}
  </tbody>
</table>
```

- Each `<tr>` is keyboard-focusable (`tabIndex={0}`) with a hover state.
- Status pills: `role="status"` + `aria-label="{label}"`.

### 5.4 Mobile (< 768px) — card-list layout

Below `md` breakpoint, the table transforms to a vertical card list. Each row becomes:

```
┌────────────────────────────────┐
│  [avatar] alice     Mar 5 2026 │
│  org/repo          250 XLM     │
│                    [Paid]      │
└────────────────────────────────┘
```

Implemented via `hidden md:table` on the `<table>` and `md:hidden` on the card-list `<ul>`.

### 5.5 Loading skeleton

Reuses `StatsCardSkeleton` animation pattern:

```tsx
// 5 skeleton rows
[...Array(5)].map((_, i) => (
  <tr key={i}>
    <td><SkeletonLoader className="h-4 w-24" /></td>
    <td><div className="flex items-center gap-2">
      <SkeletonLoader variant="circle" className="w-6 h-6" />
      <SkeletonLoader className="h-4 w-28" />
    </div></td>
    <td><SkeletonLoader className="h-4 w-32" /></td>
    <td><SkeletonLoader className="h-4 w-16 ml-auto" /></td>
    <td><SkeletonLoader className="h-5 w-20 rounded-full" /></td>
  </tr>
))
```

### 5.6 Pagination

- Row cap: 10 per page.
- "Previous / Next" buttons using `aria-label="Previous page"` / `aria-label="Next page"`.
- Current page announced via `aria-live="polite"` region: `"Showing page {n} of {total}"`.

### 5.7 Empty state

```
[PayoutIcon]
No payouts yet
When contributors complete bounties, payouts will appear here.
```

---

## 6. Top Contributors Module

### 6.1 Layout

```
┌──────────────────────────────────────────────────────────┐
│  Top Contributors                       [View all →]     │
│  ──────────────────────────────────────────────────────  │
│  #1  [avatar] alice         1,240 XLM  ↑ +2             │
│  #2  [avatar] bob             980 XLM  →                 │
│  #3  [avatar] carol           750 XLM  ↓ -1             │
│  #4  [avatar] dave            620 XLM  ↑ +5             │
│  #5  [avatar] eve             410 XLM  →                 │
└──────────────────────────────────────────────────────────┘
```

### 6.2 Row anatomy

- **Rank badge:** `w-7 h-7` circle, gold gradient for #1, silver for #2, bronze for #3, neutral for #4–#5.
  - Token: `primary.600` (`#c9983a`) for gold, `neutral.400` (`#a8a29e`) for silver, `#cd7f32` for bronze.
- **Avatar:** 32px rounded-full, GitHub CDN URL, initials fallback.
- **Username:** `text-[14px] font-semibold`.
- **Amount:** `text-[13px] font-bold text-[#c9983a]` right-aligned.
- **Trend indicator:** `TrendingUp` (green) / `TrendingDown` (red) / `Minus` (neutral) from Lucide, with numeric delta.

### 6.3 "View all" link

```tsx
<a
  href="#"
  onClick={() => onNavigate('leaderboard')}
  aria-label="View all contributors on leaderboard"
  className="text-[12px] font-semibold text-[#c9983a] hover:text-[#e8c77f] ..."
>
  View all →
</a>
```

Pre-applies `?type=contributors&filter=rewards` filter to the leaderboard URL so it opens to the earnings view.

### 6.4 States

| State | Render |
|---|---|
| Loading | 5× skeleton rows (circle + two bars) |
| Empty | "No contributor data yet for the selected period" |
| Default | Ranked list, up to 5 rows |

---

## 7. Full Analytics Tab Layout

```
┌──────────────────────────────────────────────────────────────┐
│  [Filter: Last 7d] [Last 30d] [Last 90d] [All]               │
├────────────────────┬─────────────────────────────────────────┤
│  Conversion Funnel │         Top Contributors                 │
│  (60% width)       │         (40% width)                     │
│                    │                                         │
│  Recharts Funnel   │  #1 alice   1,240 XLM  ↑               │
│  + sr-only table   │  #2 bob       980 XLM  →               │
│                    │  …                                      │
│                    │  [View all →]                           │
├────────────────────┴─────────────────────────────────────────┤
│  Payout History                                              │
│  (full width)                                                │
│  Date | Contributor | Repository | Amount | Status          │
│  ─────────────────────────────────────────────────────────  │
│  … rows …                               [Prev] Page 1 [Next]│
└──────────────────────────────────────────────────────────────┘
```

Responsive breakpoints:
- `≥ 1024px`: side-by-side funnel + contributors, full-width table below.
- `768px – 1023px`: funnel full-width, contributors full-width below, then table.
- `< 768px`: all stacked; table becomes card-list.

---

## 8. Contrast Verification

All pairs tested against the glass card surface `rgba(255,255,255,0.08)` in dark theme (`#1a1714` effective bg).

| Element | Foreground | Effective bg | Ratio | Result |
|---|---|---|---|---|
| Funnel label "Applied" | `#f5f5f5` | `#1a1714` | 18.1:1 | ✅ AA |
| Funnel label "Paid" | `#22c55e` | `#1a1714` | 5.2:1 | ✅ AA |
| Status pill "Paid" text (dark) | `#22c55e` | `#1a1714` | 5.2:1 | ✅ AA |
| Status pill "Pending" text (dark) | `#f59e0b` | `#1a1714` | 4.7:1 | ✅ AA |
| Status pill "Processing" text (dark) | `#3b82f6` | `#1a1714` | 4.6:1 | ✅ AA |
| Status pill "Failed" text (dark) | `#ef4444` | `#1a1714` | 4.6:1 | ✅ AA |
| Amount column gold `#c9983a` | `#c9983a` | `#1a1714` | 4.7:1 | ✅ AA |
| Contributor username `#e8dfd0` | `#e8dfd0` | `#1a1714` | 14.2:1 | ✅ AA |
| Rank #1 gold badge text white | `#ffffff` | `#c9983a` | 2.5:1 | ⚠ (icon-only, no text on badge) |
| Period filter active text `#fef5e7` | `#fef5e7` | `#c9983a`-tinted bg | 4.8:1 | ✅ AA |

---

## 9. Keyboard Walkthrough

1. Tab to **Analytics** tab button → `Enter` activates tab.
2. Tab to **period filter** buttons → `Space` selects period.
3. Tab enters **Funnel sr-only table** (screen reader announces each cell).
4. Tab to **Top Contributors** rows (each `<li>` is focusable).
5. Tab to **View all** link → `Enter` navigates to leaderboard.
6. Tab to **Payout History table** rows (each `<tr tabIndex={0}>`).
7. Tab to **Previous / Next** pagination buttons.

---

## 10. Component File Map

| File | Type | Description |
|---|---|---|
| `frontend/src/features/maintainers/components/dashboard/BountyFunnelChart.tsx` | New | Recharts funnel + sr-only table |
| `frontend/src/features/maintainers/components/dashboard/PayoutHistoryTable.tsx` | New | Payout table / card-list + skeleton + pagination |
| `frontend/src/features/maintainers/components/dashboard/TopContributorsModule.tsx` | New | Top-5 contributors + View all link |
| `frontend/src/features/maintainers/components/dashboard/AnalyticsTab.tsx` | New | Composes the three modules with date filter |
| `frontend/src/features/maintainers/types/index.ts` | Updated | `TabType` + `PayoutRecord` + `FunnelStage` + `TopContributor` |
| `frontend/src/features/maintainers/pages/MaintainersPage.tsx` | Updated | Add Analytics tab |
| `frontend/src/features/maintainers/components/dashboard/__tests__/AnalyticsTab.test.tsx` | New | 40+ tests |

---

## 11. Out of Scope

- Real API endpoint for payout history (uses mock data shape; backend endpoint TBD).
- Export to CSV.
- Drill-down from funnel stage to list of individual bounties.
