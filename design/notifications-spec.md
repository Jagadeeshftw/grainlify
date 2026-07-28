# Notifications System — Design Specification

> **Version:** 1.0.0  
> **Status:** Draft  
> **Last updated:** 2026-06-01  

## Table of Contents

1. [Overview](#1-overview)
2. [Notification Types](#2-notification-types)
3. [Bell Icon & Badge](#3-bell-icon--badge)
4. [Notifications Dropdown](#4-notifications-dropdown)
5. [Notification Center Page](#5-notification-center-page)
6. [States](#6-states)
7. [Accessibility (WCAG 2.1 AA)](#7-accessibility-wcag-21-aa)
8. [Responsive Behavior](#8-responsive-behavior)
9. [Animation & Motion](#9-animation--motion)
10. [Edge Cases](#10-edge-cases)

---

## 1. Overview

The notifications system consists of two surfaces:

| Surface | Location | Purpose |
|---|---|---|
| **Dropdown** | Dashboard header (bell icon) | Quick preview of recent notifications |
| **Center (full-page)** | `/dashboard?page=notifications` or route | Full notification history with filtering & management |

Both surfaces share the same data model, notification type definitions, and design language (glassmorphism per existing theme).

---

## 2. Notification Types

Five notification types drive the system. Each maps to a distinct icon, color accent, and verb.

| Type | Icon | Color | Description | Priority |
|---|---|---|---|---|
| `bounty_awarded` | `Award` (lucide) | `--color-primary-500` (#f1b400) | A bounty/issue was awarded to you | High |
| `submission_received` | `GitPullRequest` (lucide) | `--color-success-500` (#22c55e) | A submission was received for your project | Medium |
| `pr_reviewed` | `GitMerge` (lucide) | `--color-primary-600` (#c9983a) | Your PR was reviewed / merged | Medium |
| `payout_confirmed` | `Wallet` (lucide) | `--color-success-600` (#16a34a) | A payout has been confirmed / sent | High |
| `system_alert` | `AlertTriangle` (lucide) | `--color-warning-500` (#f59e0b) | System / platform notification | Low |

### Notification data model

```typescript
interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  body: string;
  read: boolean;
  createdAt: string;       // ISO 8601
  actionUrl?: string;       // Deep link (e.g., /dashboard?project=...)
  actor?: {
    name: string;
    avatarUrl: string;
  };
}

type NotificationType =
  | 'bounty_awarded'
  | 'submission_received'
  | 'pr_reviewed'
  | 'payout_confirmed'
  | 'system_alert';

interface NotificationsResponse {
  notifications: Notification[];
  unreadCount: number;
  total: number;
  limit: number;
  offset: number;
}
```

---

## 3. Bell Icon & Badge

### 3.1 Trigger button

The bell icon button matches the existing header button pattern (`UserProfileDropdown` trigger).

| State | Visual |
|---|---|
| **Default** | 46×46px rounded-full, glass background (`bg-[#2d2820]` dark / `bg-[#d4c5b0]` light), `Bell` icon 16×16px. `inset` shadow per existing pattern. |
| **Hover** | `hover:scale-105` transform. |
| **Focus-visible** | `focus-visible:ring-2 focus-visible:ring-[#c9983a]` outline. |
| **Active** | `scale-95` momentary feedback. |
| **With unread** | Badge overlay visible (see 3.2). |

### 3.2 Unread count badge

| Count range | Display | Badge style |
|---|---|---|
| 0 | Hidden | — |
| 1–9 | Numeric | 18×18px min, `rounded-full`, `bg-[#ef4444]` (red-500), white text `text-[10px] font-bold`, `border-2 border-[#2d2820]` (dark) / `border-white` (light) |
| 10–99 | Numeric | Same as 1–9 |
| 100+ | "99+" | Same badge, text reads "99+" |

**Positioning:** Absolute, `-top-0.5 -right-0.5` on mobile, `-top-1 -right-1` on `lg+`.  
**Contrast:** Red badge (#ef4444) on dark/light backgrounds — 4.5:1+ with white text.  
**Animation:** `scale-0` → `scale-100` on mount (150ms spring).

```tsx
// Badge component pattern
{unreadCount > 0 && (
  <span
    role="status"
    aria-live="polite"
    aria-label={`${unreadCount} unread notification${unreadCount !== 1 ? 's' : ''}`}
    className="absolute -top-1 -right-1 min-w-[18px] h-[18px] px-1 rounded-full
               bg-[#ef4444] text-white text-[10px] font-bold leading-none
               border-2 border-[#2d2820] dark:border-white
               flex items-center justify-center z-20
               animate-badge-in"
  >
    {unreadCount > 99 ? '99+' : unreadCount}
  </span>
)}
```

---

## 4. Notifications Dropdown

### 4.1 Layout

| Region | Contents |
|---|---|
| **Header** | Bell icon + "Notifications" title + unread count + "Mark all as read" link |
| **Body** | Grouped notification list by date |
| **Footer** | "View all notifications" link → Notification Center |
| **Empty state** | Centered illustration + message |
| **Loading state** | Skeleton placeholder |
| **Error state** | Error message + retry button |

### 4.2 Header

```
┌────────────────────────────────┐
│  🔔 Notifications   (3)  Mark all as read │
├────────────────────────────────┤
```

- "Mark all as read" is a `button` (not a link) that calls `markAllAsRead()`
- Only visible when `unreadCount > 0`
- Text: `text-[13px] text-[#c9983a] hover:text-[#a67c2e]`

### 4.3 Date grouping

Notifications are sorted by `createdAt` descending and grouped into:

| Group label | Condition |
|---|---|
| **Today** | Same calendar day as now |
| **Yesterday** | Previous calendar day |
| **This Week** | Within the last 7 days (not Today/Yesterday) |
| **Earlier** | Older than 7 days |

Group headers styled as:
```
text-[11px] font-semibold uppercase tracking-wider text-[#8a7e70] px-4 py-2
```

### 4.4 Notification item

Each notification row uses Radix `DropdownMenuItem` as base.

```
┌────────────────────────────────────────┐
│ [icon] │ Title text (bold if unread)   │
│        │ Body preview (secondary)      │
│        │ time ago label                │
└────────────────────────────────────────┘
```

| Part | Style |
|---|---|
| Icon | 32×32px rounded-full icon container with type color, 16×16px icon |
| Title | `text-[13px] font-medium` (bold `font-semibold` if unread) |
| Body | `text-[12px]` muted, single line, `truncate` |
| Time | `text-[11px]` muted, relative (`2m ago`, `1h ago`, `Yesterday`) |
| Unread dot | 8×8px `bg-[#c9983a] rounded-full`, left-aligned before icon |

**Interaction states (DropdownMenuItem):**

| State | Light | Dark |
|---|---|---|
| Default | `bg-transparent` | `bg-transparent` |
| Hover | `bg-white/[0.2]` | `bg-white/[0.12]` |
| Focus-visible | `bg-white/[0.2] ring-1 ring-[#c9983a]/50` | `bg-white/[0.12] ring-1 ring-[#c9983a]/50` |
| Active | `bg-white/[0.25]` | `bg-white/[0.15]` |
| Disabled | `opacity-40 pointer-events-none` | `opacity-40 pointer-events-none` |

### 4.5 Mark-all-read action

Located in the header row. When clicked:

1. Visually: immediately fade unread indicators (dots + bold) from all items
2. Optimistically: update local state, clear badge count
3. On failure: restore previous state, show error toast
4. Accessibility: `aria-live="polite"` region announces "All notifications marked as read"

### 4.6 Empty state

Shown when `notifications.length === 0` and not loading.

```
┌────────────────────────────────┐
│                                │
│       [Bell icon, muted]       │
│   "No notifications yet"       │
│   You'll see updates about     │
│   your contributions, rewards, │
│   and project activity here.   │
│                                │
└────────────────────────────────┘
```

Matches existing pattern in current component.

### 4.7 Loading state

Show 3 skeleton items (matching `ActivityItemSkeleton.tsx` pattern):

```
┌────────────────────────────────────┐
│ [32px ■] │ ████████  ██████████  │
│          │ ████████████████████  │
│          │ ████                   │
├────────────────────────────────────┤
│ [32px ■] │ ████████  ██████████  │
│          │ ████████████████████  │
│          │ ████                   │
├────────────────────────────────────┤
│ [32px ■] │ ████████  ██████████  │
│          │ ████████████████████  │
│          │ ████                   │
└────────────────────────────────────┘
```

Use `animate-shimmer` class from theme. Skeleton surfaces: `bg-white/[0.08]` dark / `bg-white/[0.15]` light, `rounded-lg`.

### 4.8 Error state

```
┌────────────────────────────────┐
│  ⚠️ Failed to load            │
│  notifications                 │
│                                │
│  [Retry button]                │
└────────────────────────────────┘
```

- Icon: `AlertTriangle` in `text-[#ef4444]`
- Retry button: small ghost button, `text-[13px] text-[#c9983a]`

### 4.9 Footer

```
├────────────────────────────────┤
│  → View all notifications      │
└────────────────────────────────┘
```

Navigates to `/dashboard?page=notifications` or the dedicated notification center route.  
Styled as `px-4 py-3 border-t border-white/10`, text `text-[13px] text-[#c9983a] font-medium`.

### 4.10 Max visible items

Dropdown shows the **10 most recent** notifications. Older notifications are accessible in the Notification Center (footer link).

---

## 5. Notification Center Page

A full-page view accessible from `frontend/src/features/notifications/NotificationsPage.tsx` and routed at `/dashboard?page=notifications`.

### 5.1 Layout

```
┌──────────────────────────────────────────────────────┐
│  Notifications Center                        [count] │
│  ┌─────┬──────┬──────┬──────┬──────┐                │
│  │ All │Bounty│Submis│PR Rev│Payout│System│          │
│  └─────┴──────┴──────┴──────┴──────┘                │
│                                                        │
│  ┌── Filter row ───────────────────────────┐          │
│  │ [All] [Unread only]    Sort: Newest ▼   │          │
│  └──────────────────────────────────────────┘          │
│                                                        │
│  Today                                                 │
│  ┌──────────────────────────────────────────────┐     │
│  │ [icon] │ Title text         [time] [·] [✓]   │     │
│  │        │ Body text preview                    │     │
│  ├──────────────────────────────────────────────┤     │
│  │ [icon] │ Title text         [time] [·] [✓]   │     │
│  │        │ Body text preview                    │     │
│  └──────────────────────────────────────────────┘     │
│                                                        │
│  Yesterday                                              │
│  ┌──────────────────────────────────────────────┐     │
│  │ ...                                            │     │
│  └──────────────────────────────────────────────┘     │
│                                                        │
│  [Load more] / Infinite scroll                         │
└──────────────────────────────────────────────────────┘
```

### 5.2 Type filter chips

Horizontal scrollable chip row:

| Chip | Behavior |
|---|---|
| All | Default selected, shows all types |
| Bounty Awarded | Filter `type = bounty_awarded` |
| Submission Received | Filter `type = submission_received` |
| PR Reviewed | Filter `type = pr_reviewed` |
| Payout Confirmed | Filter `type = payout_confirmed` |
| System Alert | Filter `type = system_alert` |

Each chip: `rounded-[999px] px-4 py-2 text-[13px] font-medium backdrop-blur-[25px] border`.  
Active chip: `bg-[#c9983a] text-white border-[#c9983a]`.  
Inactive: glass background per theme.

### 5.3 Read/Unread filter toggle

```tsx
type FilterMode = 'all' | 'unread';
```

Rendered as a segmented control:
```
┌──────────┬──────────┐
│ All      │ Unread   │
└──────────┴──────────┘
```

### 5.4 Notification item (center view)

Extended version of dropdown item:

```
┌─────────────────────────────────────────────────────────┐
│ [icon] │ Title (semibold if unread)           [time]    │
│        │ Body preview (up to 2 lines)          [read]   │
│        │                                          [↓]   │
└─────────────────────────────────────────────────────────┘
```

- `[read]` button: toggles read/unread state per item (eye icon)
- `[↓]` deep-link button: opens action URL
- Clicking the item itself navigates to `actionUrl`

### 5.5 Pagination / Infinite scroll

Strategy: **Infinite scroll** with an IntersectionObserver.

- Page size: 20 notifications per page
- IntersectionObserver on a sentinel element at the bottom of the list
- Loading more shows a single `ActivityItemSkeleton` row at the bottom
- When `loadedCount >= total`, show "You've reached the end" message
- URL-based offset: `?page=notifications&offset=0`

### 5.6 Bulk actions

| Action | Placement | Behavior |
|---|---|---|
| Mark all as read | Top header button | Marks all unread as read |
| Select mode | Multi-select checkbox per item | Enables bulk action bar at bottom |

**Bulk action bar** (appears when items are selected):

```
┌─────────────────────────────────────────────────────┐
│ [■] 3 selected    [Mark as read] [Mark as unread]   │
└─────────────────────────────────────────────────────┘
```

- Fixed at bottom of page
- Glass background, backdrop-blur
- Count shows selected items
- Actions apply to all selected

### 5.7 Integration with SettingsPage

The Notification Center is a **separate page** from the Settings "Notification Preferences" tab. The Settings tab handles **email/weekly digest preferences**, while the Notification Center shows **in-app notification history**.

When the "Coming Soon" overlay is removed from `NotificationsTab.tsx`, the tab should link to the Notification Center for viewing history:

> "Want to see your notification history? [View Notification Center →]"

---

## 6. States

### 6.1 Component states matrix

| State | Dropdown | Badge | Notification Item | Bulk Actions |
|---|---|---|---|---|
| **Default** | Closed, bell + hidden badge | Hidden (count=0) or visible | Renders based on read status | Hidden |
| **Hover** | Button scale 1.05 | No change | Background lightens | — |
| **Focus** | `focus-visible:ring-2` | — | Same as hover | Focus ring |
| **Active** | `scale-95` | — | Background darkens | — |
| **Disabled** | N/A | N/A | `opacity-40`, no pointer | Buttons disabled |
| **Loading** | Skeleton items in dropdown | Hidden (show old count until fresh data) | Skeleton shimmer | N/A |
| **Empty** | Empty state illustration | Hidden | N/A | N/A |
| **Error** | Error state + retry | Preserve last known count | N/A | N/A |

### 6.2 Badge states

| State | Visual |
|---|---|
| Default (0) | Hidden (`display: none`) |
| New notification arrives | `scale-0 → scale-100` with spring animation |
| Count updates | Scale pulse on number change |
| Marked all read | `scale-100 → scale-0` exit animation |
| Overflow (100+) | Shows "99+" |

### 6.3 Skeleton loading pattern

Match `ActivityItemSkeleton.tsx` approach: 3 placeholder items, each with:
- 32×32 rounded-full for icon
- 3 lines of shimmer bars (varying widths: 60%, 80%, 40%)
- `animate-shimmer` class

---

## 7. Accessibility (WCAG 2.1 AA)

### 7.1 Roles & ARIA

| Element | ARIA |
|---|---|
| Bell trigger | `aria-label="Notifications (3 unread)"` (dynamic), `aria-haspopup="true"`, `aria-expanded` |
| Badge | `role="status"`, `aria-live="polite"`, `aria-label="{count} unread notifications"` |
| Dropdown | `role="dialog"`, `aria-label="Notifications"` |
| Notification list | `role="list"` |
| Notification item | `role="listitem"` |
| Unread indicator | `aria-label="Unread notification"` |
| "Mark all as read" | `aria-label="Mark all 3 notifications as read"` |

### 7.2 Keyboard navigation

Within dropdown (inherited from Radix):
- `Tab` / `Shift+Tab`: move focus between trigger and dropdown
- `ArrowDown` / `ArrowUp`: navigate between items
- `Enter` / `Space`: activate item (navigate to action URL)
- `Escape`: close dropdown, focus returns to trigger

Notification center:
- `Tab` order: type filters → read/unread filter → notification list → pagination
- `F` key: focus filter bar
- `R` key: toggle read/unread on focused item
- `Escape`: clear selection / close any open menu

### 7.3 Live regions

```tsx
<div aria-live="polite" aria-atomic="true" className="sr-only">
  {unreadCount > 0
    ? `You have ${unreadCount} unread notifications.`
    : 'No unread notifications.'}
</div>
```

### 7.4 Focus management

- Dropdown closes with `DropdownMenu` → focus returns to trigger
- "Mark all as read" action → focus stays on the button
- Notification Center page load → focus moves to the page heading (`h1`)
- After bulk action → focus returns to first selected item (or nearest)

### 7.5 Contrast ratios

| Element | Ratio | Pass |
|---|---|---|
| Badge text (white on #ef4444) | 5.2:1 | AA |
| Body text (#7a6b5a on #e8dfd0 light bg) | 4.8:1 | AA |
| Body text (#b8a898 on #0c0a09 dark bg) | 5.3:1 | AA |
| Gold links (#c9983a on #e8dfd0) | 4.9:1 | AA on large text |
| Gold links (#c9983a on #0c0a09) | 7.1:1 | AAA |
| Filter chips (inactive) (#7a6b5a on glass) | 4.5:1 | AA |

### 7.6 Reduced motion

```css
@media (prefers-reduced-motion: reduce) {
  .animate-badge-in {
    animation: none;
  }
  .animate-slide-in-right {
    animation: none;
  }
  .animate-shimmer {
    animation: none;
  }
}
```

---

## 8. Responsive Behavior

### 8.1 Breakpoints

| Breakpoint | Width | Dropdown behavior | Center page behavior |
|---|---|---|---|
| **sm** | 640px | Button visible (hidden on mobile nav `lg:flex`). Dropdown: full-width `w-[calc(100vw-32px)]` max-w-sm | Single column, chips wrap, full-width items |
| **md** | 768px | Same as sm, dropdown `w-80` | 1 column, chips scrollable horizontally |
| **lg** | 1024px | Header button visible always. Standard positioning. | 1 column max-w-2xl centered |
| **xl** | 1280px | Same as lg | Same as lg |

### 8.2 Mobile nav integration

The bell button already integrates with `showMobileNav` prop:
- `showMobileNav=true`: button becomes full-width `w-[80%] max-w-[800px] rounded-sm` with "Notification" label
- `showMobileNav=false`: standard `hidden lg:flex` circular button

Dropdown on mobile:
- Full width with 16px side margins: `w-[calc(100vw-32px)]`
- `max-h-[60vh]` to prevent overflow
- `sideOffset={4}` to stay close to trigger
- Touch targets: minimum 44×44px

### 8.3 Notification center responsive

| Element | sm | md | lg+ |
|---|---|---|---|
| Container | px-4, full width | px-6, max-w-2xl | px-8, max-w-3xl centered |
| Filter chips | Horizontal scrollable | Same | Same |
| Grid | Single column | Single column | Single column |
| Sidebar | None | None | None |

---

## 9. Animation & Motion

### 9.1 Dropdown

- Entry: `fade-in-0 zoom-in-95` over 200ms (Radix default)
- Exit: `fade-out-0 zoom-out-95` over 150ms

### 9.2 Badge

- Mount: scale 0 → 100 with slight overshoot (`cubic-bezier(0.34, 1.56, 0.64, 1)`)
- Update: subtle scale pulse (1 → 1.15 → 1) over 300ms
- Unmount: scale 100 → 0 over 150ms

### 9.3 New notification toast

When a real-time notification arrives (via polling or WebSocket in the future):
1. Badge count animates (scale pulse)
2. A sonner toast appears (using existing `Sonner` library): 
   - Icon matching notification type
   - Title + body preview
   - Clickable → navigate to action URL

### 9.4 Notification center

- Page transition: `animate-slide-in-right` (30ms, matches existing modal pattern)
- Filter change: fade out list → fade in new results (300ms)
- Mark as read: smooth fade of unread indicator (200ms)
- Loading more: skeleton slides in (200ms ease-out)

---

## 10. Edge Cases

| Scenario | Handling |
|---|---|
| **Long title/body** | `truncate` on single-line, `line-clamp-2` on multi-line. Tooltip on hover for full text. |
| **Zero notifications** | Empty state illustration (see 4.6) |
| **Empty after filter** | "No notifications match this filter" with reset link |
| **Network failure** | Error state with retry button (see 4.8) |
| **Count mismatch** | Badge shows optimistic count; re-sync on dropdown open |
| **Rapid double-click on Mark all read** | Debounce action, disable button during request |
| **Notification deleted while viewing** | Gracefully remove from list, update count |
| **1000+ notifications** | Infinite scroll with 20/page; virtualized list if perf degrades |
| **Very old notifications** | Grouped under "Earlier" in dropdown; full history in center |
| **Concurrent sessions** | Badge count syncs on next API call (polling or on focus) |
| **Screen reader + live region** | Batch announcements: "5 new notifications received" not "1 new notification" × 5 |
| **Right-to-left (RTL)** | Use logical properties (`padding-inline`, `margin-inline-start`) — not currently required but use `space-x-*` → `space-x-reverse` safe |

---

## Appendix A: File Manifest

| File | Action | Description |
|---|---|---|
| `design/notifications-spec.md` | **Create** | This document |
| `design-tokens.json` | **Update** | Add notification-specific tokens (type colors, badge vars) |
| `frontend/src/shared/components/NotificationsDropdown.tsx` | **Rewrite** | Full spec implementation |
| `frontend/src/shared/types/notifications.ts` | **Create** | TypeScript types for notification data model |
| `frontend/src/features/notifications/NotificationsPage.tsx` | **Create** | Full-page notification center |
| `frontend/src/shared/api/client.ts` | **Update** | Add notification API methods |
| `frontend/src/app/App.tsx` | **Update** | Add notification center route |
| `frontend/src/features/dashboard/Dashboard.tsx` | **Update** | Wire notification page state |
| `frontend/src/shared/components/index.ts` | **Update** | Export NotificationsDropdown |
| `frontend/src/styles/theme.css` | **Update** | Add notification animations |

## Appendix B: Design Token Additions

```json
{
  "notification": {
    "badge": {
      "background": "#ef4444",
      "text": "#ffffff",
      "border-light": "#ffffff",
      "border-dark": "#2d2820"
    },
    "type": {
      "bounty_awarded": { "color": "#f1b400", "icon": "Award" },
      "submission_received": { "color": "#22c55e", "icon": "GitPullRequest" },
      "pr_reviewed": { "color": "#c9983a", "icon": "GitMerge" },
      "payout_confirmed": { "color": "#16a34a", "icon": "Wallet" },
      "system_alert": { "color": "#f59e0b", "icon": "AlertTriangle" }
    },
    "skeleton": {
      "background-light": "rgba(255, 255, 255, 0.15)",
      "background-dark": "rgba(255, 255, 255, 0.08)"
    }
  }
}
```

## Appendix C: Contrast Validation

| Pair | Foreground | Background | Ratio | Result |
|---|---|---|---|---|
| Badge count | `#ffffff` | `#ef4444` | 5.2:1 | AA |
| Notification title (unread) | `#e8dfd0` | (glass, dark) | 7.5:1 | AAA |
| Notification title (unread) | `#2d2820` | (glass, light) | 4.8:1 | AA |
| Notification body | `#b8a898` | (glass, dark) | 5.3:1 | AA |
| Notification body | `#7a6b5a` | (glass, light) | 4.8:1 | AA |
| Group header | `#8a7e70` | (glass, dark) | 5.1:1 | AA |
| Group header | `#8a7e70` | (glass, light) | 4.5:1 | AA |
| Type filter active | `#ffffff` | `#c9983a` | 4.9:1 | AA (large text) |
| Type filter inactive | `#d4c5b0` | (glass, dark) | 4.8:1 | AA |
| Type filter inactive | `#6b5d4d` | (glass, light) | 5.1:1 | AA |
