# Error Boundary Fallback UI

**Issue:** #1535
**Branch:** `design/error-boundary-fallback`
**Status:** Complete
**Scope:** `frontend/src/shared/components/ErrorBoundary/`

---

## Overview

A production-grade React error boundary that catches rendering errors and displays
a themed fallback UI with distinct experiences for development (visible stack trace)
and production (friendly message) environments. Supports two visual variants — full-page
for top-level crashes and widget-level for isolated component failures — and includes
retry, navigation, and issue-reporting actions.

---

## State Machine

```
                    ┌──────────────┐
         error ──▶  │  has-error   │ ◀── retry succeeds
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
         retry ──▶  │ retry-pending│ ──▶ retry succeeds → normal
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │retry-failed  │ ──▶ retry again
                    └──────────────┘
```

| State | UI |
|---|---|
| `normal` (no error) | `children` rendered transparently |
| `full-page-error` | Full-viewport fallback: illustration, heading, actions, focus on heading |
| `widget-level-error` | Compact inline card: small icon, short message, retry button |
| `retry-pending` | Fallback still visible; retry button shows spinner |
| `retry-failed-again` | Fallback remains; retry button still available |

---

## Variants

### Full-page (top-level boundary)

Designed to wrap `<Routes>` or the main app content. Covers the entire viewport.

```
┌──────────────────────────────────────────┐
│                                          │
│                                          │
│            [SVG 96×96px]                 │  ← role="img"
│                                          │
│         Something went wrong             │  ← h1, 24px/700
│                                          │
│    An unexpected error occurred. Our     │
│    team has been notified. You can try   │  ← p, 14px, max-w-[400px]
│    again or go back to the homepage.     │
│                                          │
│    ┌──────────┐  ┌──────────────────┐    │
│    │ Try again │  │ Go to homepage  │    │  ← buttons
│    └──────────┘  └──────────────────┘    │
│                                          │
│    Report this issue ──────────────────▶ │  ← link, opens context
│                                          │
│    ─── [ Development — Stack trace ] ─── │  ← collapsible (dev only)
│                                          │
└──────────────────────────────────────────┘
```

### Widget-level (local boundary)

Designed to wrap individual widgets, cards, or sections. Compact and inline.

```
┌──────────────────────────────────┐
│ ⚠ Widget failed to load         │
│ ┌────────┐                      │
│ │ Retry  │                      │  ← small card
│ └────────┘                      │
└──────────────────────────────────┘
```

---

## Token Usage

| Role | Dark value | Light value | High-contrast value |
|---|---|---|---|
| Page background | `#1a1714` | `#f5f0ea` | `#000000` |
| Card background | `#2d2820` | `#ffffff` | `#0d0d0d` |
| Card border | `rgba(255,255,255,0.10)` | `rgba(44,36,28,0.12)` | `#888888` |
| Heading | `#f5f5f5` | `#2d2820` | `#ffffff` |
| Subtext | `#d4d4d4` | `#7a6b5a` | `#ebebeb` |
| Stack trace bg | `#1a1714` | `#f5f0ea` | `#000000` |
| Stack trace text | `#b8a898` | `#7a6b5a` | `#c8c8c8` |
| Retry CTA bg | `#c9983a` | `#a67c2e` | `#f5c842` |
| Retry CTA text | `#ffffff` | `#ffffff` | `#000000` |
| Home link (text) | `#c9983a` | `#a67c2e` | `#f5c842` |
| Report link (text) | `#b8a898` | `#9f8b74` | `#c8c8c8` |
| Focus ring | `#f1b400` | `#a2792c` | `#ffff00` |
| SVG stroke | `#c9983a` | `#a67c2e` | `#f5c842` |
| SVG fill | `rgba(201,152,58,0.12)` | `rgba(201,152,58,0.10)` | `rgba(245,200,66,0.20)` |

---

## Accessibility

| Requirement | Implementation |
|---|---|
| `role="alert"` on error region | ✓ Announced on mount |
| Focus moves to error heading | ✓ `autoFocus` via `tabIndex={-1}` on heading |
| Keyboard-navigable actions | ✓ Retry, Home, Report all reachable by Tab |
| Min touch target 44px | ✓ `min-h-[44px]` on retry button |
| Focus ring visible | ✓ 2px gold/brown (`#f1b400` / `#a2792c`), 3px yellow (`#ffff00`) in high-contrast |
| Stack trace collapsible | ✓ `aria-expanded` / `aria-controls` pattern |
| High-contrast variant | ✓ Opaque backgrounds, yellow focus ring, solid borders |
| Reduced-motion | ✓ No animations; opacity-only fades ≤ 150ms |
| Responsive min 375px | ✓ Stack trace scrollable, no overflow at 375px |
| Contrast — heading on dark | ✓ `#f5f5f5` on `#2d2820` = 15.5:1 (AAA) |
| Contrast — heading on light | ✓ `#2d2820` on `#ffffff` = 13.8:1 (AAA) |

---

## Usage

### Full-page (recommended for App.tsx)

```tsx
import { ErrorBoundary } from '@/shared/components/ErrorBoundary';

<ErrorBoundary>
  <Routes>
    <Route path="/" element={<LandingPage />} />
    {/* ... */}
  </Routes>
</ErrorBoundary>
```

### Widget-level (for isolated components)

```tsx
<ErrorBoundary variant="widget" onReset={() => refetchWidget()}>
  <AnalyticsChart />
</ErrorBoundary>
```

### With custom report handler

```tsx
<ErrorBoundary
  onReportIssue={(error, errorInfo) => {
    window.open(
      `https://github.com/Jagadeeshftw/grainlify/issues/new?title=${encodeURIComponent('[Error] ' + error.message)}`,
      '_blank'
    );
  }}
>
  <Dashboard />
</ErrorBoundary>
```

### Dev-only stack trace

```tsx
// The stack trace section is automatically shown/hidden based on
// process.env.NODE_ENV === 'development'. No configuration needed.
```

---

## File Structure

```
frontend/src/shared/components/ErrorBoundary/
├── ErrorBoundary.tsx     ← Class component + fallback UI component
├── ErrorBoundary.test.tsx ← Test suite
└── index.ts              ← Barrel export
```

---

## Design QA Checklist

- [ ] Full-page fallback fills viewport with no overflow at 375px
- [ ] Widget fallback stays within parent container bounds
- [ ] Focus lands on error heading on mount (Tab or programmatic)
- [ ] Tab order: heading → Retry → Home → Report (→ Stack trace toggle in dev)
- [ ] Retry button min 44px height
- [ ] `role="alert"` present in DOM after error
- [ ] Stack trace toggle has correct `aria-expanded` state in dev
- [ ] High-contrast: opaque backgrounds, solid borders, yellow focus ring
- [ ] `prefers-reduced-motion: reduce` → no animation artefacts
- [ ] Report-issue link encodes error message + URL

---

## Security Notes

- No `dangerouslySetInnerHTML` — error message is rendered as text content
- Stack trace is only rendered in `process.env.NODE_ENV === 'development'`
- No user-supplied content interpolated into SVG attributes
- Report-issue link opens externally; no data exfiltration
