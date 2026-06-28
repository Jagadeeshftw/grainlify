# URL-Persistent Filter & Sort State — LeaderboardPage

## Overview

The leaderboard filter bar currently uses local React state (`useState`) that is lost on navigation and cannot be shared via URL. This spec defines a URL-persistent filter system that syncs all filter dimensions to the URL query string and restores them on mount.

---

## Contract: Types & Interfaces

### Updated `FilterState` in `frontend/src/features/leaderboard/types/index.ts`

```typescript
// ——— Current (unchanged) ———
export type FilterType = 'overall' | 'rewards' | 'contributions' | 'ecosystems';
export type LeaderboardType = 'contributors' | 'projects';
export type TimePeriod = 'weekly' | 'monthly' | 'all-time';
export type RoleFilter = 'all' | 'core' | 'contributor' | 'first-timer';
export type RankDirection = 'up' | 'down' | 'same';

// ——— New / updated ———

export type LeaderboardSort = 'earnings' | 'contributions' | 'reputation';

/** Serializable filter state that maps 1:1 to URL search params */
export interface URLFilterState {
  period?: TimePeriod;
  ecosystem?: string;
  role?: RoleFilter;
  sort?: LeaderboardSort;
  type?: LeaderboardType;
  filter?: FilterType;
}
```

### GlassDropdown Adapter Generic

The existing `GlassDropdown<T extends string>` component accepts bare string values. For label-value options (e.g. ecosystem choices), a thin adapter is needed:

```typescript
/** Adapter to bridge label-value options into GlassDropdown */
export interface DropdownOption<T extends string = string> {
  label: string;
  value: T;
}
```

---

## URL Parameter Schema

### Query Parameters

| Param      | Type              | Default        | Example Values              |
|------------|-------------------|----------------|-----------------------------|
| `period`   | `TimePeriod`      | `"all-time"`   | `weekly`, `monthly`         |
| `ecosystem`| `string \| "all"` | `"all"`        | `stellar`, `web3`, `ai`     |
| `role`     | `RoleFilter`      | `"all"`        | `core`, `contributor`       |
| `sort`     | `LeaderboardSort` | `"earnings"`   | `contributions`, `reputation`|
| `type`     | `LeaderboardType` | `"contributors"`| `projects`                  |
| `filter`   | `FilterType`      | `"overall"`    | `rewards`, `contributions`  |

### Encoding Rules

1. **Only non-default values are serialised** — keeps URLs short. Default-only state yields a clean URL with no query string.
2. **Values are URI-encoded** (`encodeURIComponent` / `decodeURIComponent`).
3. **Unknown or invalid values** are silently ignored and the default is used.
4. **Order is not significant** — params are parsed as a flat map.
5. **Unrecognised params** (e.g. `?utm_source=twitter`) are preserved but ignored — they pass through without affecting filter state.

### Examples

```
/leaderboard
/leaderboard?period=weekly
/leaderboard?period=monthly&ecosystem=stellar&sort=contributions
/leaderboard?period=weekly&ecosystem=web3&role=core&sort=reputation&type=projects
```

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  LeaderboardPage                                     │
│  ┌─────────────────────────────────────────────────┐ │
│  │  useLeaderboardSearchParams()                   │ │
│  │  ┌───────────────────────────────────────────┐  │ │
│  │  │  useSearchParams (react-router-dom)       │  │ │
│  │  └───────────────────────────────────────────┘  │ │
│  │  Provides:                                      │ │
│  │  • filters: URLFilterState                      │ │
│  │  • setFilters(partial) → updates URL + state    │ │
│  │  • resetFilters() → clears all params           │ │
│  │  • shareURL: string (fully qualified)           │ │
│  │  • isDefault: boolean                           │ │
│  └─────────────────────────────────────────────────┘ │
│                                                       │
│  FiltersSection receives filters from hook            │
│  (no more local useState for filter values)           │
└─────────────────────────────────────────────────────┘
```

### `useLeaderboardSearchParams` Hook (Contract)

A new hook `frontend/src/features/leaderboard/hooks/useLeaderboardSearchParams.ts`:

```typescript
interface UseLeaderboardSearchParamsReturn {
  /** Current parsed filter values (always valid — defaults fill gaps) */
  filters: URLFilterState;
  /** Update one or more filters. Omitted keys retain current values. */
  setFilters: (partial: Partial<URLFilterState>) => void;
  /** Clear all filter params → reset to defaults */
  resetFilters: () => void;
  /** Absolute shareable URL reflecting current filter state */
  shareURL: string;
  /** True when every dimension is at its default */
  isDefault: boolean;
  /** Copy shareURL to clipboard. Returns promise<boolean> for success. */
  copyShareLink: () => Promise<boolean>;
}
```

---

## Filter Bar State Restoration Animation on Mount

When the page loads with URL params (e.g. `?period=weekly&ecosystem=stellar`):

1. **Search params are parsed on mount** before the first render.
2. **Filters are set from URL** — no flash of default state.
3. **The exit animation from default-to-restored** is a subtle **shimmer sweep** across the active filter chips:
   - A golden gradient overlay (`from-[#c9983a]/0 via-[#c9983a]/20 to-[#c9983a]/0`) sweeps left-to-right over each non-default chip.
   - Duration: 600ms, delay: staggered 100ms per chip, ease: `ease-out`.
   - This runs once on mount, then the overlay is removed.
4. **Respects `prefers-reduced-motion`** — shimmer is replaced by an instant `opacity` transition.

### CSS Implementation

```css
@keyframes filter-shimmer {
  0%   { translate: -100%; }
  100% { translate: 200%; }
}

.filter-shimmer::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(201, 152, 58, 0.15) 50%,
    transparent 100%
  );
  translate: -100%;
  pointer-events: none;
}

.filter-shimmer.active::after {
  animation: filter-shimmer 0.6s ease-out;
}
```

---

## Shareable-Link Copy Button

### Placement

A small **link icon button** placed at the **right edge of the filter bar**, aligned with the dropdown row. On mobile (768px), it sits at the end of the filter row, after all dropdowns wrap.

### Visual (1440px)

```
┌─────────────────────────────────────────────────────────────┐
│ [Weekly] [Monthly] [All Time]                               │
│ ───────────────────────────────────────────────────────────  │
│ [Overall Leaderboard ▼]  [All Roles ▼]  [All Ecosystems ▼]  │  🔗  ✕
└─────────────────────────────────────────────────────────────┘
```

### Visual (768px)

```
┌─────────────────────────────────────────────────┐
│ [Weekly] [Monthly] [All Time]                    │
│ ───────────────────────────────────────────────  │
│ [Overall Leaderboard ▼]   [All Roles ▼]          │
│ [All Ecosystems ▼]                         🔗 ✕  │
└─────────────────────────────────────────────────┘
```

### Interaction States

| State | Visual |
|-------|--------|
| **Default** | Ghost icon button: `w-9 h-9 rounded-[12px]`, `backdrop-blur-[30px]`, `border border-white/20`, link icon (`lucide Link`) |
| **Hover** | Background `bg-white/[0.12]`, scale 1.05, tooltip "Copy shareable link" appears above (delayed 300ms) |
| **Focus** | `focus-visible:outline-[#c9983a]` |
| **Click / Copied** | Icon swaps to `lucide Check` for 2 seconds, tooltip changes to "Link copied!", then reverts |
| **Error** | Icon swaps to `lucide X`, tooltip "Failed to copy", auto-dismiss after 3s |

### Tooltip Design

```tsx
<div className="absolute -top-10 left-1/2 -translate-x-1/2 px-3 py-1.5 rounded-[8px] 
            bg-[#2d2820] text-white text-[12px] font-semibold whitespace-nowrap
            shadow-[0_4px_12px_rgba(0,0,0,0.2)] animate-tooltip-in">
  Copy shareable link
  <div className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-[#2d2820] rotate-45" />
</div>
```

### Clipboard Confirmation

```typescript
const [copied, setCopied] = useState(false);

const copyShareLink = useCallback(async () => {
  try {
    await navigator.clipboard.writeText(shareURL);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    return true;
  } catch {
    // Fallback for insecure contexts
    const textarea = document.createElement('textarea');
    textarea.value = shareURL;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand('copy');
    document.body.removeChild(textarea);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    return true;
  }
}, [shareURL]);
```

---

## Reset Filters Affordance

### Placement

A **"Reset" ghost button** (`lucide RotateCcw`) positioned **to the left of the share link button**. It is **only visible when at least one filter is non-default** (`isDefault === false`).

### Visual (1440px)

```
┌─────────────────────────────────────────────────────────────┐
│ [Weekly] [Monthly] [All Time]                               │
│ ───────────────────────────────────────────────────────────  │
│ [Overall Leaderboard ▼]  [All Roles ▼]  [All Ecosystems ▼]  │  ↺  🔗
└─────────────────────────────────────────────────────────────┘
```

### Interaction States

| State | Visual |
|-------|--------|
| **Default** | Ghost icon button: `w-9 h-9 rounded-[12px]`, `backdrop-blur-[30px]`, `border border-white/20`, rotate icon (`lucide RotateCcw`). Tooltip: "Reset filters" |
| **Hover** | Background `bg-white/[0.12]`, slight rotate animation on icon |
| **Focus** | `focus-visible:outline-[#c9983a]` |
| **Click** | All filters reset to defaults, URL clears params, any active dropdowns close. Icon does a brief `rotate(360deg)` animation (200ms) |

### Filter State: Default vs Non-Default

| Filter | Default Value |
|--------|---------------|
| `period` | `"all-time"` |
| `ecosystem` | `"all"` |
| `role` | `"all"` |
| `sort` | `"earnings"` |
| `type` | `"contributors"` |
| `filter` | `"overall"` |

`isDefault` is `true` when every dimension equals its default.

---

## GlassDropdown Integration

The existing `GlassDropdown<T>` (`frontend/src/shared/components/GlassDropdown.tsx`) must be adapted for label-value pairs.

### Option 1: Extend GlassDropdown

Add an optional `getLabel` render prop:

```typescript
interface GlassDropdownProps<T extends string> {
  value: T;
  onChange: (value: T) => void;
  options: T[];
  /** Optional: map value to display label. Falls back to value string. */
  getLabel?: (value: T) => string;
  isOpen: boolean;
  onToggle: () => void;
  onClose: () => void;
}
```

Usage inside `FiltersSection`:

```tsx
<GlassDropdown
  value={selectedEcosystem.value}
  onChange={(val) => setFilters({ ecosystem: val === 'all' ? undefined : val })}
  options={ecosystemOptions.map(e => e.value)}
  getLabel={(val) => ecosystemOptions.find(e => e.value === val)?.label ?? val}
  isOpen={showEcosystemDropdown}
  onToggle={onToggleEcosystemDropdown}
  onClose={() => setShowEcosystemDropdown(false)}
/>
```

### Option 2: GlassDropdownSelect (wrapper)

A wrapper component that accepts `DropdownOption[]` instead of `string[]`:

```tsx
interface GlassDropdownSelectProps<T extends string> {
  value: T;
  onChange: (value: T) => void;
  options: DropdownOption<T>[];
  isOpen: boolean;
  onToggle: () => void;
  onClose: () => void;
}
```

**Recommendation**: Option 1 (minimal API change, backward compatible).

---

## Responsive Behavior

### 1440px (desktop)

- All filter dropdowns in a single row, right-aligned
- Share + Reset buttons at the far right
- No wrapping

### 768px (tablet)

- Filter dropdowns wrap to two rows when space is tight
  - Row 1: sort dropdown + role dropdown
  - Row 2: ecosystem dropdown + reset + share
- Time period tabs remain in a single row
- Share tooltip: appears **below** button (avoids viewport clipping)

### < 480px (mobile)

- Time period tabs collapse to a `select` dropdown behind a "Period" label button
- Sort, role, ecosystem dropdowns go full-width (`w-full`)
- Share + Reset buttons stack vertically below filters
- Share link button label shows "Copy link" text instead of icon-only (for tap target clarity)

---

## Security Considerations

| Concern | Mitigation |
|---------|------------|
| **XSS via URL params** | All param values are decoded via `decodeURIComponent` then validated against an allowlist of known enum values. Unknown values are dropped. Values are never interpolated into HTML directly — only used as React state. |
| **Clipboard poisoning** | The share URL is constructed on the client side using `window.location.origin + pathname + search`. No external input is concatenated into URLs beyond the allowlisted enum values. |
| **Referrer leakage** | The share URL is a same-origin URL. No third-party domains appear in the query string. |
| **CSRF** | Filter state is read-only and has no side effects on the server. The API calls for leaderboard data are authenticated separately via bearer token. |

---

## Accessibility

| Requirement | Implementation |
|-------------|----------------|
| **Share button label** | `aria-label="Copy shareable link to current filters"` |
| **Copied confirmation** | `aria-live="polite"` region announces "Link copied" for screen readers |
| **Reset button label** | `aria-label="Reset all filters to defaults"` |
| **Tooltips** | `role="tooltip"` with `aria-describedby` on the trigger button |
| **Keyboard** | Tab order: time tabs → sort → role → ecosystem → reset → share → table rows |
| **Reduced motion** | All entry/exit animations + shimmer sweep disabled when `prefers-reduced-motion: reduce` |
| **Focus management** | After reset, focus moves to the first time period tab |

---

## Edge Cases

| Scenario | Handling |
|----------|----------|
| **Malformed URL** (`?period=invalid`) | Unknown values replaced with defaults; console.warn logged |
| **Empty query string** (`?`) | Treated as no params; defaults used |
| **Hash fragment present** (`#section`) | Preserved; only search params are read/written |
| **Ecosystem slug no longer active** | Recognised ecosystem but removed from dropdown; filter remains applied until user changes it |
| **Concurrent tab edits** | Each tab has its own `useSearchParams` instance; last-write-wins (acceptable for filters) |
| **Very long ecosystem names** | Truncated in dropdown button with `max-w-[120px] truncate`; full label in tooltip |
| **Clipboard API unavailable** (HTTP) | Fallback to `document.execCommand('copy')`; user sees tooltip confirmation |
| **No JavaScript** | URL params are present but cannot be read — page renders with defaults. Graceful degradation. |

---

## Implementation Plan

### Phase 1: Contract & Types

1. `frontend/src/features/leaderboard/types/index.ts` — add `URLFilterState`, `LeaderboardSort`, `DropdownOption`
2. `frontend/src/features/leaderboard/hooks/useLeaderboardSearchParams.ts` — new hook

### Phase 2: Hook Implementation

1. Read `useSearchParams` from `react-router-dom`
2. Parse each key with fallback to default
3. `setFilters` applies partial update, omits undefined keys
4. `resetFilters` deletes all known keys from search params
5. `isDefault` compares each dimension to default
6. `shareURL` constructs `window.location.origin + pathname + search`
7. `copyShareLink` uses `navigator.clipboard` with fallback

### Phase 3: UI Integration

1. Update `LeaderboardPage.tsx` — replace `selectedEcosystem`, `timePeriod`, etc. `useState` with hook values
2. Update `FiltersSection.tsx` — wire props to hook values; add share + reset buttons
3. Adopt `GlassDropdown` in place of inline dropdown implementations
4. Add shimmer animation on mount when non-default params are detected
5. Add responsive wrapping for < 768px

### Phase 4: GlassDropdown Enhancement

1. Add `getLabel?: (value: T) => string` prop to `GlassDropdown`
2. Ensure backward compatibility (when `getLabel` is omitted, display `value` as before)

---

## Files to Create / Modify

| File | Action |
|------|--------|
| `frontend/src/features/leaderboard/types/index.ts` | Modify — add `URLFilterState`, `LeaderboardSort`, `DropdownOption` |
| `frontend/src/features/leaderboard/hooks/useLeaderboardSearchParams.ts` | **Create** — new hook |
| `frontend/src/features/leaderboard/pages/LeaderboardPage.tsx` | Modify — integrate hook |
| `frontend/src/features/leaderboard/components/FiltersSection.tsx` | Modify — add share + reset buttons, use GlassDropdown |
| `frontend/src/shared/components/GlassDropdown.tsx` | Modify — add `getLabel` prop |
| `design/specs/leaderboard-url-filter-state.md` | **Create** — this document |

---

## Design QA Checklist

### Filter Bar Keyboard Operability
- [ ] Tab enters filter bar → time period tabs receive focus
- [ ] Left/Right arrow keys switch between time period tabs (role="tablist" with aria-orientation="horizontal")
- [ ] Tab moves to sort dropdown → Enter/Space opens it
- [ ] Down arrow moves through options → Enter selects
- [ ] Escape closes open dropdown
- [ ] Tab moves to role dropdown → same interaction
- [ ] Tab moves to ecosystem dropdown → same interaction
- [ ] Tab moves to reset button → Enter activates
- [ ] Tab moves to share button → Enter copies link
- [ ] After reset, focus returns to first time period tab

### URL Parameter Encoding
- [ ] Non-default values appear as query params
- [ ] All URL values are URI-encoded
- [ ] Invalid/unknown param values fall back to defaults
- [ ] Empty URL (`?`) or no URL renders defaults
- [ ] Share button reflects current URL state
- [ ] Copied URL, when pasted into new tab, restores same filter state

### Mobile Filter Collapse
- [ ] At 768px, dropdowns wrap to two rows
- [ ] At 480px, time period tabs collapse to a select
- [ ] Share tooltip appears below button on mobile
- [ ] Reset button visible only when non-default
- [ ] No horizontal overflow on filter bar
- [ ] Touch targets minimum 44×44px

### Animation & Motion
- [ ] Shimmer sweep plays once on mount when URL params present
- [ ] `prefers-reduced-motion: reduce` disables all filter animations
- [ ] Copy confirmation (check icon + tooltip) animates smoothly
- [ ] Reset button rotates on click

### Security
- [ ] No XSS vectors via query string values
- [ ] Clipboard operations use secure `navigator.clipboard` API
- [ ] Fallback `execCommand` only used in insecure contexts
- [ ] No external URL construction from user input
- [ ] CSP does not need modification

---

## Security Notes

1. **Input sanitisation**: All URL param values are validated against strict enum allowlists before being applied to state. Unknown values are discarded.
2. **Clipboard API**: The primary path uses `navigator.clipboard.writeText()` (requires Secure Context). The fallback `document.execCommand('copy')` is used only on HTTP or non-secure origins. No user data touches the clipboard beyond the application-origin URL.
3. **No stored state**: The filter state is ephemeral — stored in URL only. No localStorage, sessionStorage, or cookies are used for filter persistence.
4. **No server-side impact**: The URL filters are client-only. The server receives the same API calls regardless of URL state. Authentication is handled separately via bearer token.
