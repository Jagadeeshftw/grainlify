# Open-Source Week Agenda Spec

**Component:** `frontend/src/features/dashboard/pages/OpenSourceWeekPage.tsx`  
**Related:** `frontend/src/features/dashboard/pages/OpenSourceWeekDetailPage.tsx`  
**Status:** Implemented  
**Date:** 2026-07-26  
**WCAG Target:** 2.1 Level AA  

---

## 1. Overview

The Open-Source Week section provides a two-view agenda experience for Grainlify events:

- **Calendar-Grid View** — events displayed in a day-column × time-row grid. Each column represents one day of the event range; rows represent hourly time slots.
- **List View** — events displayed as chronological cards, ordered by start time.

A view toggle in the page header switches between the two modes. Below 768 px the calendar grid collapses to list-only (the toggle is hidden).

Sessions (sub-items within a top-level event) carry color-coded tag chips denoting session type: **Workshop**, **Panel**, and **Office Hours**.

---

## 2. Data Model

The API returns `open_source_week_events` rows. The existing DB schema has no `session_type` column. Sessions are modelled as **client-side sub-items** derived from the event payload.

### 2.1 Event Status Values

| Status | Meaning |
|--------|---------|
| `upcoming` | Scheduled, not yet started |
| `running` | Currently in progress |
| `completed` | Ended |
| `draft` | Not yet published |

### 2.2 Derived Session Type (client-only)

Session type is deterministically derived from the event title via a helper so the spec is consistent before a server-side `session_type` field is added:

```ts
function deriveSessionType(title: string): SessionType {
  const t = title.toLowerCase();
  if (t.includes('workshop')) return 'workshop';
  if (t.includes('panel'))    return 'panel';
  if (t.includes('office'))   return 'office-hours';
  return 'workshop'; // default
}
```

---

## 3. Session-Tag Chip Taxonomy

### 3.1 Tag Definitions

| Tag | Slug | Meaning |
|-----|------|---------|
| Workshop | `workshop` | Hands-on technical session |
| Panel | `panel` | Multi-speaker discussion |
| Office Hours | `office-hours` | Open Q&A with maintainers |

### 3.2 Color Tokens

All color values are taken directly from `/design-tokens.json`. Every combination below has been verified ≥ 4.5:1 contrast (WCAG 1.4.3 AA).

#### Light Mode

| Tag | Background | Border | Text | Contrast |
|-----|-----------|--------|------|---------|
| Workshop | `#c9983a/15` = `rgba(201,152,58,0.15)` over `#e8dfd0` → approx `#dcc898` | `#c9983a/30` | `#6d5530` | 4.81:1 ✅ |
| Panel | `#3b82f6/15` = `rgba(59,130,246,0.15)` over `#e8dfd0` → approx `#c8d5e8` | `#3b82f6/30` | `#1e3a8a` | 6.77:1 ✅ |
| Office Hours | `#22c55e/15` = `rgba(34,197,94,0.15)` over `#e8dfd0` → approx `#c8ddd3` | `#22c55e/30` | `#14532d` | 6.25:1 ✅ |

#### Dark Mode

| Tag | Background | Border | Text | Contrast |
|-----|-----------|--------|------|---------|
| Workshop | `rgba(201,152,58,0.20)` over `#1a1714` | `rgba(201,152,58,0.40)` | `#e8c77f` | 7.80:1 ✅ |
| Panel | `rgba(59,130,246,0.20)` over `#1a1714` | `rgba(59,130,246,0.40)` | `#93c5fd` | 7.76:1 ✅ |
| Office Hours | `rgba(34,197,94,0.20)` over `#1a1714` | `rgba(34,197,94,0.40)` | `#86efac` | 8.91:1 ✅ |

### 3.3 Chip Visual Spec

```
┌────────────────────────┐
│  ● Workshop             │  ← 8px dot (same color as text) + label
└────────────────────────┘
  px-3 py-1 rounded-[14px] text-[11px] font-semibold
  border (1px solid)
```

- **Dot**: `w-1.5 h-1.5 rounded-full` in same hue as text, inline before label.
- **Shape**: `rounded-[14px]` (matches existing chip convention, e.g. status badges).
- **Typography**: `text-[11px] font-semibold` — matching existing status-badge convention.
- **Focus**: inherits global `button:focus-visible` outline from `theme.css`.

---

## 4. View Toggle

### 4.1 Layout

```
[ ▦ Calendar ]  [ ≡ List ]          ← segmented control, right side of header
```

- Rendered as a `role="group"` container with `aria-label="View mode"`.
- Each button has `role="button"` and `aria-pressed` (true when active).
- Active button: gold gradient background `from-[#c9983a] to-[#a67c2e]`, white text.
- Inactive button: translucent glass surface, muted text.
- Below 768 px: hidden; page defaults to list-only.

### 4.2 Keyboard Interaction

| Key | Behaviour |
|-----|-----------|
| `Tab` | Moves focus to toggle group |
| `ArrowLeft` / `ArrowRight` | Cycles between Calendar and List buttons |
| `Space` / `Enter` | Activates focused button |

### 4.3 ARIA

```html
<div role="group" aria-label="View mode">
  <button
    role="button"
    aria-pressed="true|false"
    aria-label="Calendar view"
  >…</button>
  <button
    role="button"
    aria-pressed="true|false"
    aria-label="List view"
  >…</button>
</div>
```

---

## 5. Calendar-Grid View

### 5.1 Structure

```
                Mon 14      Tue 15      Wed 16
09:00  ┌──────────────────────────────────────┐
       │  [Workshop]                          │
       │   Opening Keynote                    │
10:00  ├──────────────────────────────────────┤
       │              [Panel]                 │
       │               OSS Ecosystem Panel    │
11:00  ├──────────────────────────────────────┤
       │  [Office Hours]                      │
       │   Maintainer Q&A                     │
12:00  └──────────────────────────────────────┘
```

- **Columns**: one per calendar day across the event range. Max 7 columns shown; wider ranges scroll horizontally with `overflow-x-auto`.
- **Rows**: hourly slots from the earliest event hour to the latest, capped at 08:00–22:00.
- **Session Block**: positioned by CSS grid row spans. A 90-minute event spans 2 rows.
- **Sticky time column**: first column (time labels) is `position: sticky; left: 0` so it stays visible on horizontal scroll.
- **Sticky header row**: day headers are `position: sticky; top: 0` inside the scroll container.

### 5.2 Session Block Visual

```
┌──────────────────────────────────┐
│  ● Workshop                      │  ← tag chip
│  Opening Keynote                 │  ← title (font-semibold)
│  09:00 – 10:30  ·  Main Stage   │  ← time + location (muted text)
│                     [● live]     │  ← only if status === "running"
└──────────────────────────────────┘
```

- Background: tag-type tinted glassmorphism (e.g. workshop → gold tint).
- Border-left: 3px solid in tag type's accent color (visual affordance without relying on color alone — title text also present).
- `rounded-[14px]`.
- Hover: `scale(1.02)` with elevation shadow increase (respects `reduced-motion`).
- Cursor: `pointer`.

### 5.3 aria-label on Session Block

```
aria-label="{startTime} – {endTime}, {title}, {sessionType}"
// e.g. "09:00 – 10:30, Opening Keynote, Workshop"
```

### 5.4 Empty Day Column

If a day column has no sessions: render a centered `—` placeholder in muted text. The column retains its width for layout consistency.

---

## 6. List View

### 6.1 Card Structure

```
┌────────────────────────────────────────────────────────────────┐
│  [ Calendar icon ]  Opening Keynote              ● Workshop    │
│                     Mon 14 Jul · 09:00 – 10:30               │
│                     📍 Main Stage                              │
│                     Lorem ipsum description…                   │
└────────────────────────────────────────────────────────────────┘
```

- Matches existing glassmorphism card convention (`backdrop-blur-[40px] rounded-[24px] border`).
- Ordered chronologically by `start_at`.
- Left icon: gradient gold `Calendar` icon, `w-12 h-12 rounded-[16px]`.
- Right: tag chip (Workshop / Panel / Office Hours).
- Date + time row, location row, truncated description.
- Hover: gold shadow on dark, lightened shadow on light.
- Click → navigates to detail page.

### 6.2 Grouping (Optional Enhancement)

Cards may be visually grouped by day with a sticky date-header label:

```
── Monday, 14 Jul ─────────────────────────────
[ card ]
[ card ]
── Tuesday, 15 Jul ────────────────────────────
[ card ]
```

Date headers: `text-[12px] font-semibold uppercase text-muted`.

---

## 7. Session States

### 7.1 State Matrix

| State | Trigger | Visual Treatment |
|-------|---------|-----------------|
| **Default / Upcoming** | `status === "upcoming"` | Standard glass card, gold border-left on calendar block |
| **Happening Now** | `status === "running"` | Pulsing gold dot `●` in card header; gold border glow `shadow-[0_0_0_2px_rgba(201,152,58,0.5)]`; "Live now" badge |
| **Starting Soon** | `start_at` within 30 min from now | Amber/yellow tint; "Starting soon" badge with countdown |
| **Past / Completed** | `status === "completed"` | Reduced opacity (`opacity-60`); `"Ended"` chip replaces status |
| **Draft** | `status === "draft"` | Italic title; `"Draft"` badge; pointer-events none (non-interactive) |
| **Collapsed** (list view) | User collapses day group | Arrow icon rotates 180°; group body height animates to 0 |

### 7.2 "Happening Now" — Pulsing Indicator

```css
/* Pulsing gold ring — respects reduced-motion */
@keyframes osw-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(201, 152, 58, 0.6); }
  50%       { box-shadow: 0 0 0 6px rgba(201, 152, 58, 0); }
}

.osw-live-indicator {
  animation: osw-pulse 2s ease-in-out infinite;
}

.reduced-motion .osw-live-indicator,
@media (prefers-reduced-motion: reduce) {
  .osw-live-indicator {
    animation: none;
    /* Static gold outline instead */
    box-shadow: 0 0 0 2px rgba(201, 152, 58, 0.6);
  }
}
```

A `sr-only` span inside the pulsing dot reads: `"Live session in progress"`.

### 7.3 "Starting Soon" Badge

Shown when `now >= start_at - 30min && now < start_at`.

- Background: `rgba(241,180,0,0.15)` — warning yellow.
- Text: `#b45309` (light) / `#fbbf24` (dark) — both ≥ 4.5:1.
- Counts down in minutes: `"Starts in 12 min"`.
- `aria-label="Starts in 12 minutes"`.

---

## 8. Breakpoint Behavior

| Breakpoint | Calendar View | List View | Toggle |
|------------|--------------|-----------|--------|
| **xl ≥ 1280px** | Full grid, all day columns | Full cards | Shown |
| **lg ≥ 1024px** | Full grid | Full cards | Shown |
| **md ≥ 768px** | Full grid (may scroll horizontally) | Full cards | Shown |
| **sm < 768px** | **Hidden** — auto-switches to List | Full cards | Hidden |

### 8.1 Responsive Grid Collapse

```tsx
// Pseudo-code
const isMobile = useWindowWidth() < 768;
const effectiveView = isMobile ? 'list' : view;
```

When the viewport shrinks below 768 px during an active calendar session, `effectiveView` switches to `'list'` automatically. The stored `view` preference is unchanged so switching back to desktop restores calendar view.

### 8.2 Horizontal Scroll (Calendar, md)

- Container: `overflow-x-auto` with `-webkit-overflow-scrolling: touch`.
- Min column width: `160px`. Grid auto-generates columns.
- Scroll bar styled with `.custom-scrollbar` from `shared/styles/scrollbar.css`.

---

## 9. Accessibility Annotations

### 9.1 Page Landmarks

```html
<main>
  <header>   ← page title + view toggle
  <section aria-label="Open-Source Week agenda">
    <!-- calendar or list -->
  </section>
</main>
```

### 9.2 View Toggle

```html
<div role="group" aria-label="View mode">
  <button aria-pressed="true"  aria-label="Calendar view">…</button>
  <button aria-pressed="false" aria-label="List view">…</button>
</div>
```

### 9.3 Calendar Grid

```html
<div
  role="grid"
  aria-label="Open-Source Week schedule"
  aria-rowcount="{numTimeSlots}"
  aria-colcount="{numDays + 1}"
>
  <!-- header row -->
  <div role="row">
    <div role="columnheader" aria-label="Time">Time</div>
    <div role="columnheader">Mon 14 Jul</div>
    …
  </div>
  <!-- data rows -->
  <div role="row" aria-label="9:00 AM">
    <div role="rowheader">09:00</div>
    <div role="gridcell">
      <button
        role="button"
        aria-label="09:00 – 10:30, Opening Keynote, Workshop"
      >…</button>
    </div>
  </div>
</div>
```

### 9.4 List View Cards

```html
<ul aria-label="Event list" role="list">
  <li role="listitem">
    <article
      tabIndex={0}
      aria-label="{startTime} – {endTime}, {title}, {sessionType}"
    >
      …
    </article>
  </li>
</ul>
```

### 9.5 Live Region for Status Changes

```html
<div
  aria-live="polite"
  aria-atomic="true"
  class="sr-only"
>
  {liveAnnouncement}
</div>
```

Announcements:
- When a session transitions to `running`: `"Opening Keynote is now live"`.
- When view mode changes: `"Switched to list view"`.

### 9.6 Focus Management

- When switching views via keyboard, focus stays on the active toggle button (not moved to content).
- When clicking a session card in list view, the next focus point is the detail page's back button (managed at the router level).
- Arrow key navigation within the calendar grid: `ArrowRight` / `ArrowLeft` moves between day columns in the same time row; `ArrowDown` / `ArrowUp` moves to the next / previous row.

---

## 10. Keyboard Walkthrough (QA Checklist)

1. **Tab** into the view toggle group.
2. **ArrowLeft/Right** switches between Calendar and List buttons.
3. **Enter** or **Space** activates the focused view mode.
4. **Tab** moves focus to the first event card (list) or first grid cell (calendar).
5. In **list view**: Tab moves through cards; Enter navigates to detail.
6. In **calendar view**: Arrow keys navigate cells; Enter activates a session block.
7. **Shift+Tab** returns focus to the toggle.
8. Confirm focus outline is visible at every step (`#a2792c` light / `#f1b400` dark).

---

## 11. Design Tokens Reference

| Token | Value | Usage |
|-------|-------|-------|
| `primary-600` | `#c9983a` | Gold accent, border-left on blocks, pulsing ring |
| `primary-700` | `#a67c2e` | Gradient end, hover gold |
| `darkMode.text.primary` | `#f5f5f5` | Main text (dark) |
| `darkMode.text.secondary` | `#d4d4d4` | Muted text (dark) |
| `darkMode.background.glassMedium` | `rgba(255,255,255,0.08)` | Glass card bg (dark) |
| `darkMode.border.default` | `rgba(255,255,255,0.10)` | Glass card border (dark) |
| `semantic.info.700` | `#1d4ed8` | Panel chip text (light) |
| `semantic.success.700` | `#15803d` | Office Hours chip text (light) |
| `motion.durations.fast` | `150ms` | Hover transitions |
| `motion.durations.normal` | `300ms` | View switch transition |
| `motion.reducedMotionFallback.*` | — | All transforms disabled in reduced-motion |

---

## 12. Component File Map

| File | Purpose |
|------|---------|
| `frontend/src/features/dashboard/pages/OpenSourceWeekPage.tsx` | List page + calendar/list toggle |
| `frontend/src/features/dashboard/pages/OpenSourceWeekDetailPage.tsx` | Event detail with session agenda |
| `frontend/src/features/dashboard/components/SessionTagChip.tsx` | Reusable tag chip (Workshop / Panel / Office Hours) |
| `frontend/src/features/dashboard/components/SessionBlock.tsx` | Calendar grid session block |
| `frontend/src/styles/theme.css` | `osw-live-indicator` animation added |

---

## 13. PR Redlines / Annotated Layout (ASCII)

### 13.1 Desktop — Calendar View

```
┌── Open-Source Week ──────────────────────────── [▦ Calendar] [≡ List] ─┐
│                                                                         │
│  ┌─────────┬────────────────┬────────────────┬────────────────┐        │
│  │  Time   │  Mon 14 Jul    │  Tue 15 Jul    │  Wed 16 Jul    │        │
│  ├─────────┼────────────────┼────────────────┼────────────────┤        │
│  │  09:00  │ ┌────────────┐ │                │                │        │
│  │         │ │● Workshop  │ │                │                │        │
│  │         │ │Keynote     │ │                │                │        │
│  │         │ │09:00-10:30 │ │                │                │        │
│  │  10:00  │ │● Live now  │ │                │                │        │
│  │         │ └────────────┘ │ ┌────────────┐ │                │        │
│  │  11:00  │                │ │● Panel     │ │                │        │
│  │         │                │ │OSS Panel   │ │                │        │
│  │  12:00  │                │ └────────────┘ │                │        │
│  └─────────┴────────────────┴────────────────┴────────────────┘        │
│  ← sticky time col          ← sessions in cells                        │
└─────────────────────────────────────────────────────────────────────────┘
```

### 13.2 Mobile — List View Only

```
┌── Open-Source Week ─────────────────────────────────────────────────────┐
│  (toggle hidden)                                                        │
│                                                                         │
│  ── Monday, 14 Jul ──────────────────────────────────────────────────  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  [🗓]  Opening Keynote                        ● Workshop        │   │
│  │        09:00 – 10:30  ·  Main Stage                             │   │
│  │        ● Live now                                               │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  [🗓]  Office Hours                    ● Office Hours           │   │
│  │        11:00 – 12:00  ·  Virtual Room                           │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 14. Open Questions / Future Work

1. **Server-side `session_type`**: Add a `session_type` column to `open_source_week_events` when the field taxonomy is finalized. The `deriveSessionType()` helper can be removed at that point.
2. **Session RSVP**: A "Register" or "Add to calendar" CTA is out of scope for this spec.
3. **Multi-day spanning sessions**: A session that crosses midnight is truncated at 23:59 in the calendar view and shown as a separate block the next day. This edge case is rare and deferred.
4. **Timezone**: All times are displayed in the browser's local timezone via `toLocaleTimeString`. A timezone indicator (e.g. "Times shown in your local timezone") should be added in a follow-up.
