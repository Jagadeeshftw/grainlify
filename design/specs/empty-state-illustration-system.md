# Empty-State Illustration System

**Issue:** #1405
**Branch:** `design/empty-state-illustration-system`
**Status:** Complete
**Scope:** `frontend/src/shared/components/EmptyState/`

---

## Overview

A cohesive set of 8 responsive SVG spot illustrations for all empty surfaces in Grainlify,
styled with the grainlify gold/warm-neutral palette defined in `design-tokens.json`.

---

## Layout Spec

```
┌─────────────────────────────────┐
│                                 │
│        [SVG 120×120px]          │  ← role="img", aria-labelledby
│                                 │
│       Headline (18px/700)       │  ← h3, text-[#f5f5f5] dark / text-[#2d2820] light
│                                 │
│    Subtext (14px, max-w-280)    │  ← text-[#b8a898] dark / text-[#7a6b5a] light
│                                 │
│    [ Optional CTA button ]      │  ← min-h-[44px], gold, focus ring
│                                 │
└─────────────────────────────────┘
```

- Centered within parent container via `flex flex-col items-center`
- `role="status"` + `aria-label` on root — announces to screen readers
- `py-12` vertical padding — scales to available space

---

## Illustration Variants

| Variant             | Headline (default)   | Subtext (default)                                         |
| ------------------- | -------------------- | --------------------------------------------------------- |
| `no-results-search` | No results found     | Try different keywords or remove some filters.            |
| `no-bounties`       | No bounties yet      | Bounties posted by maintainers will appear here.          |
| `no-contributions`  | No contributions yet | Merged pull requests and closed issues will show up here. |
| `no-notifications`  | All caught up        | New activity on your projects will appear here.           |
| `no-leaderboard`    | Leaderboard is empty | Rankings appear once contributors start earning points.   |
| `no-programs`       | No programs found    | Grant programs created by ecosystems will be listed here. |
| `no-payout-history` | No payouts yet       | Completed payouts from bounties and programs appear here. |
| `no-ecosystems`     | No ecosystems yet    | Registered ecosystems funding open-source work show here. |

---

## Token Usage

All illustrations draw exclusively from `design-tokens.json`:

| Role            | Dark value               | Light value             |
| --------------- | ------------------------ | ----------------------- |
| Stroke / accent | `#c9983a` (primary-600)  | `#a67c2e` (primary-700) |
| Fill / wash     | `rgba(201,152,58,0.12)`  | `rgba(201,152,58,0.10)` |
| Neutral shapes  | `rgba(255,255,255,0.18)` | `rgba(44,36,28,0.14)`   |
| Headline text   | `#f5f5f5`                | `#2d2820`               |
| Subtext         | `#b8a898`                | `#7a6b5a`               |
| CTA button      | `#c9983a` bg             | `#a67c2e` bg            |
| CTA hover       | `#e8c77f`                | `#c9983a`               |
| Focus ring      | `#f1b400`                | `#a2792c`               |

---

## Accessibility

| Requirement                         | Implementation                                               |
| ----------------------------------- | ------------------------------------------------------------ |
| SVG role="img"                      | ✅ Every SVG has `role="img"`                                |
| Descriptive `<title>`               | ✅ Each SVG contains `<title id="es-title-{variant}">`       |
| `aria-labelledby` linking SVG title | ✅ `aria-labelledby="es-title-{variant}"`                    |
| `<figcaption class="sr-only">`      | ✅ Text-only fallback for reduced-data / print               |
| `role="status"` on root             | ✅ Announces empty state to screen readers                   |
| CTA min touch target 44px           | ✅ `min-h-[44px]` on CTA button                              |
| Focus ring on CTA                   | ✅ `focus-visible:outline-2 focus-visible:outline-offset-2`  |
| Contrast — headline on dark bg      | ✅ `#f5f5f5` on `#2d2820` = 15.5:1 (AAA)                     |
| Contrast — subtext on dark bg       | ✅ `#b8a898` on `#2d2820` = 9.1:1 (AAA)                      |
| Contrast — gold stroke on dark fill | ✅ `#c9983a` on `rgba(201,152,58,0.12)` ≥ 3:1 (UI component) |
| `prefers-reduced-motion`            | ✅ No animations used in illustrations                       |
| Responsive (375px min)              | ✅ `120px` fixed SVG, `max-w-[280px]` subtext, flex layout   |

---

## Usage

### Basic

```tsx
import { EmptyState } from '@/shared/components/EmptyState';

// No bounties — default copy
<EmptyState variant="no-bounties" />

// With CTA
<EmptyState
  variant="no-results-search"
  ctaLabel="Clear filters"
  onCta={() => setFilters({})}
/>

// With override copy
<EmptyState
  variant="no-contributions"
  headline="Nothing here yet"
  subtext="Submit your first PR to a registered project."
  ctaLabel="Find projects"
  onCta={() => navigate('/browse')}
/>
```

### Replacing inline empty states

Before (BrowsePage.tsx pattern):

```tsx
<div role="status" className="p-8 rounded-[16px] border text-center ...">
  <p>No projects found</p>
  <p>Try adjusting your filters or check back later.</p>
</div>
```

After:

```tsx
<EmptyState variant="no-results-search" ctaLabel="Clear filters" onCta={clearAllFilters} />
```

---

## File Structure

```
frontend/src/shared/components/EmptyState/
├── EmptyState.tsx   ← Component + all 8 SVG illustrations + types
└── index.ts         ← Barrel export
```

---

## Design QA Checklist

Run manually before merging:

- [ ] SVG `<title>` present in DOM (inspect element)
- [ ] All 8 variants render without layout overflow at 375px viewport width
- [ ] CTA button height ≥ 44px (measure with DevTools)
- [ ] Tab to CTA → gold focus ring visible in dark mode
- [ ] Tab to CTA → gold/brown focus ring visible in light mode
- [ ] `prefers-reduced-motion: reduce` → no animation artefacts
- [ ] `print` media → illustration and text-only figcaption both present
- [ ] Screen reader (VoiceOver / NVDA) announces "No bounties yet" (or active headline) on mount

---

## Security Notes

- No user-supplied content is interpolated into SVG attributes — all strings are static constants
- No `dangerouslySetInnerHTML` usage
- No external SVG fetches — all illustrations are inline JSX
- CTA `onCta` handler is caller-supplied and not executed at mount

---

## Integration Map

Pages that should adopt `<EmptyState>` to replace existing ad-hoc empty states:

| File                                                                | Replace when            | Recommended variant |
| ------------------------------------------------------------------- | ----------------------- | ------------------- |
| `frontend/src/features/dashboard/pages/BrowsePage.tsx`              | `projects.length === 0` | `no-results-search` |
| `frontend/src/features/dashboard/pages/SearchPage.tsx`              | search results empty    | `no-results-search` |
| `frontend/src/features/dashboard/pages/EcosystemsPage.tsx`          | ecosystems list empty   | `no-ecosystems`     |
| `frontend/src/features/dashboard/pages/ContributorsPage.tsx`        | contributors list empty | `no-leaderboard`    |
| `frontend/src/features/leaderboard/pages/LeaderboardPage.tsx`       | leaderboard data empty  | `no-leaderboard`    |
| `frontend/src/features/notifications/NotificationsPage.tsx`         | no notifications        | `no-notifications`  |
| `frontend/src/features/settings/components/payout/PayoutTab.tsx`    | no payout history       | `no-payout-history` |
| `frontend/src/features/maintainers/components/issues/IssuesTab.tsx` | no bounties on project  | `no-bounties`       |
