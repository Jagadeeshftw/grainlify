# LeaderboardPage Design Spec

## Overview

The leaderboard is a high-visibility community engagement page showing top contributors and projects ranked by contribution score. This spec covers the visual layout, filter interaction, and real-time update animation system.

---

## Podium Component (Top 3 Contributors)

### Layout
- **1st Place (Gold)**: Center, elevated `-mt-8`, 170px card width, golden gradient border, animated glow/rays/particles
- **2nd Place (Silver)**: Left, same baseline, 150px card width, silver medal badge
- **3rd Place (Bronze)**: Right, same baseline, 150px card width, bronze medal badge
- Placement order in DOM: 2nd → 1st → 3rd (visual center alignment)

### States

| State | Visual |
|-------|--------|
| **Default** | Glassmorphism card, backdrop-blur, subtle border, medal icon with rank |
| **Hover** | `scale(1.05)` card transform, avatar `rotate(12deg)`, sparkle icon appears |
| **Focus** | `outline-[#c9983a]` around card via keyboard navigation |
| **Loading** | `ContributorsPodiumSkeleton` — shimmer placeholders for 3 cards |
| **Empty** | Centered text "No contributors yet" (no podium rendered) |
| **1st special** | Animated crown icon, rotating rays, floating particles, pulsing ring, number glow |

### Accessibility
- `role="group"` with `aria-label="Top contributors podium"`
- Each podium card: `role="article"` with `aria-label="{rank} place"`
- `sr-only` span inside rank badge for screen reader text
- Rank numbers rendered as text (not color-only)

---

## Rank-Change Animation & Delta Indicators

### Behavior
- On data refresh (polling every 30s), `previousRank` is computed from previous fetch
- Each row compares `previousRank` vs `currentRank` to show a delta badge

### Delta Badge Visual

| Change | Badge | Animation |
|--------|-------|-----------|
| **Up** | Green badge: `△ +N` | `animate-delta-up` — spring-bounce from below |
| **Down** | Red badge: `▽ -N` | `animate-delta-down` — spring-bounce from above |
| **Same** | Gray badge: `─ 0` | No animation |

### CSS Keyframes

```css
@keyframes delta-up {
  0%   { transform: translateY(8px); opacity: 0; }
  60%  { transform: translateY(-2px); opacity: 1; }
  100% { transform: translateY(0); opacity: 1; }
}

@keyframes delta-down {
  0%   { transform: translateY(-8px); opacity: 0; }
  60%  { transform: translateY(2px); opacity: 1; }
  100% { transform: translateY(0); opacity: 1; }
}
```

### Reduced Motion
All rank-change animations respect `prefers-reduced-motion: reduce` — animations are disabled, delta badges appear static.

---

## Filter Bar

### Time Period Tabs
- `role="tablist"` with three tabs: **Weekly**, **Monthly**, **All Time**
- Active tab: gold gradient background, elevated shadow
- Inactive: translucent with hover darken
- Styled as a segmented control within a pill container

### Sort Filter Dropdown
- Label shows current selection (default: "Overall Leaderboard")
- Options: Overall Leaderboard, Total Rewards, Total Contributions
- `role="listbox"` with `aria-selected` on options
- Click-outside closes dropdown

### Role Filter Dropdown
- Icon changes based on selection (Users / Sparkles / Code)
- Options: All Roles, Core, Contributor, First Timer
- Same dropdown pattern as sort filter

### Ecosystem Filter Dropdown
- Fetches from `GET /ecosystems` API endpoint
- "All Ecosystems" default + dynamically loaded active ecosystems
- Loading spinner while fetching
- Same dropdown pattern as others

### States

| State | Visual |
|-------|--------|
| **Default** | Translucent glassmorphism bar, dropdowns closed |
| **Hover** | Buttons: `scale(1.05)`, slightly brighter background |
| **Focus** | `focus-visible:outline-[#c9983a]` with 2px offset |
| **Active/Dropdown Open** | Chevron rotates 180°, dropdown appears with `animate-dropdown-in` |
| **Disabled** | Not applicable (filters remain interactive) |
| **Loading (ecosystem)** | Spinner in dropdown menu |
| **Error (ecosystem fetch)** | Fallback to "All Ecosystems" only; error logged to console |

### Accessibility
- `role="region" aria-label="Leaderboard filters"`
- All dropdown buttons: `aria-haspopup="listbox"` and `aria-expanded`
- All dropdown options: `role="option"` with `aria-selected`
- Time period tabs: `role="tablist"` and `aria-selected` on each tab
- Keyboard navigation: Tab between filters, Enter/Space to open dropdown, arrow keys (future)

---

## Responsive Breakpoints

| Breakpoint | Behavior |
|------------|----------|
| **xl (1280px)** | Full layout — 3-column podium, 12-col table grid |
| **lg (1024px)** | Table grid shrinks gaps, text sizes remain |
| **md (768px)** | Podium cards reduce width (150px → 130px), filter bar wraps to 2 rows |
| **sm (640px)** | Podium stacks vertically (1st elevated), filter dropdowns go full-width, table becomes horizontal scroll, "View Profile" button hidden, rank/trend columns collapse |

### Table Responsive Strategy
- On `sm`, the table container overflows with `overflow-x: auto`
- Rank and Trend columns collapse into a single combined cell
- "View Profile" button hidden (avatar + username remain clickable)
- Ecosystem tags wrap naturally

---

## Contributing States (Table Rows)

| State | Visual |
|-------|--------|
| **Default** | Translucent row, divide-white/10 separator |
| **Hover** | `bg-white/[0.08]` background, avatar scales up, score badge scales up and brightens |
| **Focus** | `focus-visible:outline-[#c9983a]` outline on row |
| **Active/Press** | Not explicitly styled (uses click for navigation) |
| **Loading** | `ContributorsTableSkeleton` — 5 shimmer rows |
| **Empty** | Centered text: "No contributors found" + suggestion to adjust filters |
| **Error** | Red alert banner with retry button (page-level, not per-row) |
| **Disabled** | Not applicable |

### Accessibility
- `role="region" aria-label="Leaderboard rankings"`
- Each row: `role="listitem"` with `tabIndex={0}` for keyboard
- `aria-label` on each row: `"Rank {n}: {username}, score {score} points"`
- `aria-live="polite"` wrapping the table for real-time update announcements
- Last updated timestamp with live indicator dot

---

## Design Tokens Used

All tokens reference `theme.css` CSS custom properties:

| Token | Value |
|-------|-------|
| `--color-primary-600` | `#c9983a` (gold) |
| `--color-primary-700` | `#a67c2e` (dark gold) |
| `--background` | Light: `#e8dfd0` / Dark: `#0c0a09` |
| `--foreground` | Light: `#2d2820` / Dark: `#fafaf9` |
| `--glass-bg` | `rgba(255,255,255,0.35)` |
| `--glass-blur` | `40px` |
| `--radius-2xl` | `1rem` |
| `--glow-primary` | `0 0 20px rgba(201,152,58,0.3)` |

---

## Real-Time Update System

### Polling
- Interval: **30 seconds** via `setInterval`
- Only active on "contributors" tab
- On poll: `fetchLeaderboard(true)` — does not show loading skeleton
- `prevDataRef` (Map) stores previous ranks for delta computation
- `lastUpdated` state displays "Last updated: HH:MM:SS" with pulsing dot

### Rank Persistence
- Ranks tracked per `username` in a `Map<string, number>`
- On each poll, `previousRank` is set from the Map, then Map is updated
- If a user is new (no previous entry), `previousRank` equals `currentRank` (no delta)

### ARIA Live Region
- Table wrapper: `aria-live="polite" aria-atomic="false"` — announces new ranks without resetting
- Last updated text: `aria-live="polite" aria-atomic="true"` — announces timestamp changes

---

## Keyboard Walkthrough

1. **Tab** enters the filter bar → time period tabs
2. **Arrow keys** (future) switch between time period tabs
3. **Tab** moves to sort dropdown → Enter/Space opens it → Tab moves through options → Enter selects
4. **Tab** moves to role dropdown → same interaction
5. **Tab** moves to ecosystem dropdown → same interaction
6. **Tab** moves to first table row → Enter/Space triggers profile navigation
7. **Tab** moves through subsequent rows

---

## Error Handling & Edge Cases

| Scenario | Handling |
|----------|----------|
| Empty leaderboard | "No contributors yet" text in podium area, empty table state |
| API failure | Error alert banner with retry button; polling silently fails |
| Long usernames | `truncate` with `max-w-[140px]` on podium, `min-w-0 truncate` in table |
| Zero scores | Displayed as `0` — valid state |
| Single contributor | Only 1st place podium shown (no 2nd/3rd) |
| Image load failure | OnError fallback to `github.com/{username}.png` |
| Ecosystems fetch failure | Default to "All Ecosystems" only |
| Reduced motion | All animations disabled, falling petals hidden |

---

## Before/After Summary

| Aspect | Before | After |
|--------|--------|-------|
| Podium | 3 cards with basic styling | Animated gold/silver/bronze cards with elevation, crown, particles |
| Rank change | Only trend icon (up/down/same) | Trend icon + animated delta badge (+N / -N / 0) |
| Filter: Time | None | Weekly/Monthly/All Time segmented tabs |
| Filter: Sort | Dropdown (3 options) | Same, with better a11y |
| Filter: Ecosystem | Dropdown | Same, with better a11y |
| Filter: Role | None | All Roles / Core / Contributor / First Timer |
| Real-time updates | None | 30s polling with rank persistence and delta computation |
| A11y | No ARIA regions | Full ARIA with live regions, roles, labels, keyboard nav |
| Error state | No visible error UI | Alert banner with retry |
| Empty state | Inline text | Dedicated empty state in table |
| Reduced motion | Not respected | `prefers-reduced-motion: reduce` disables animations |
| Responsive | Basic | Explicit breakpoint behavior for sm/md/lg/xl |
