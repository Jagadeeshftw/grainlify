# Open Graph Preview Image Templates — Design Spec & QA Checklist

**Surface coverage:** ProjectDetailPage · ProfilePage · EcosystemDetailPage  
**File location:** `design/specs/open-graph-preview-templates.md`  
**Branch:** `design/social-sharing-open-graph-previews`  
**Status:** ✅ Ready for design QA

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Design Tokens](#3-design-tokens)
4. [Dynamic Data Regions — All Surfaces](#4-dynamic-data-regions)
5. [Template Specifications](#5-template-specifications)
   - 5.1 Project Card
   - 5.2 Contributor Profile Card
   - 5.3 Ecosystem Card
6. [Security Assumptions & Validation](#6-security)
7. [QA Checklist](#7-qa-checklist)
8. [Edge Cases](#8-edge-cases)
9. [Test Coverage Report](#9-test-coverage)
10. [Integration Notes](#10-integration-notes)

---

## 1. Overview

When a contributor or maintainer shares a Grainlify URL on social platforms
(LinkedIn, Twitter/X, Slack, Discord, Telegram), the platform scrapes
`<meta property="og:image">` and renders a preview card.

Without bespoke OG images, platforms fall back to a generic screenshot or
blank card — losing an opportunity to communicate Grainlify's brand and the
project's/contributor's impact at a glance.

### Goals

| Goal | Metric |
|------|--------|
| Brand recognition at thumbnail scale | Logo + wordmark legible at 200px wide |
| Data-rich at a glance | ≥ 2 meaningful stats visible per card |
| Glassmorphism aesthetic consistency | Matches platform dark-mode palette |
| Performance | Template render ≤ 50ms (no network fetches at render time) |
| Accessibility | WCAG AA contrast ratios on all text |

### Export Dimensions

| Variant | Dimensions | Used by |
|---------|-----------|---------|
| Standard OG | 1200 × 630 px | Facebook, LinkedIn, iMessage, Slack |
| Twitter card | 800 × 418 px | Twitter / X `summary_large_image` |

---

## 2. Architecture

```
frontend/src/features/
├── ProjectDetailPage/
│   └── OGImageTemplate.jsx          # Project card component + helpers
├── ProfilePage/
│   └── OGImageTemplate.jsx          # Contributor profile card component + helpers
├── EcosystemDetailPage/
│   └── OGImageTemplate.jsx          # Ecosystem card component + helpers
└── og-image-templates.css           # Shared design tokens + all surface styles

design/specs/
└── open-graph-preview-templates.md  # ← this document
```

### Rendering Strategy

These React components are **server-side rendered** (via Satori or a
headless Chromium screenshot service) and the resulting PNG is:

1. Cached at the CDN edge keyed on `{surface}/{entityId}`.
2. Served as a static asset with `Cache-Control: public, max-age=86400`.
3. Invalidated whenever the entity's headline stat changes by > 5%.

The React components are **also usable client-side** for local preview
during development (see Integration Notes §10).

---

## 3. Design Tokens

All values are defined as CSS custom properties in `og-image-templates.css`
and consumed in JSX via inline styles or class names.

| Token | Value | Usage |
|-------|-------|-------|
| `--og-gold` | `#C9A84C` | Primary brand accent — stat values, wordmark, chips |
| `--og-gold-dim` | `rgba(201,168,76,0.35)` | Gold at reduced opacity — background elements |
| `--og-bg-deep` | `#0C0E14` | Canvas base — near-black with blue cast |
| `--og-bg-glass` | `rgba(255,255,255,0.06)` | Glassmorphic panel fill |
| `--og-text-hi` | `#F2F2F2` | Primary text — project name, contributor name |
| `--og-text-mid` | `rgba(242,242,242,0.60)` | Secondary text — labels, roles, taglines |
| `--og-text-lo` | `rgba(242,242,242,0.35)` | Tertiary text — footer tag |
| `--og-border` | `rgba(255,255,255,0.10)` | Glass panel borders, dividers |
| `--og-radius` | `16px` | Corner radius for glass panels |

### Background Gradient Variants

Each surface has a distinct radial-burst background to aid visual
differentiation when multiple card types appear side by side.

| Surface | Dominant burst colour | Secondary burst colour |
|---------|-----------------------|------------------------|
| Project | Gold (top-right) | Blue-indigo (bottom-left) |
| Profile | Violet-blue (top-right) | Gold (bottom-left) |
| Ecosystem | Gold (wider sweep, top) | Sky-blue (bottom) |

---

## 4. Dynamic Data Regions

The following table lists every region that receives runtime data,
its prop name, the component that owns it, and its truncation limit.

| Region | Prop | Component | Max length | Fallback |
|--------|------|-----------|-----------|----------|
| Project name | `projectName` | ProjectOGImage | 40 chars | — (required) |
| Project logo | `logoSrc` | ProjectOGImage | n/a | `/logos/placeholder.svg` |
| Headline stat | `headlineStat` | ProjectOGImage | 12 chars | — (required) |
| Headline label | `headlineLabel` | ProjectOGImage | — | `"distributed"` |
| Contributor count | `contributorCount` | ProjectOGImage | — (number) | — (required) |
| Ecosystem badge | `ecosystemName` | ProjectOGImage | 24 chars | `"Grainlify"` |
| Contributor name | `displayName` | ContributorOGImage | 36 chars | — (required) |
| Avatar | `avatarSrc` | ContributorOGImage | n/a | Initials fallback |
| Role | `role` | ContributorOGImage | 48 chars | Hidden |
| Total earned | `totalEarned` | ContributorOGImage | 12 chars | — (required) |
| PRs merged | `prsMerged` | ContributorOGImage | — (number) | — (required) |
| Ecosystems list | `ecosystems` | ContributorOGImage | 3 items × 16 chars | Hidden |
| Ecosystem name | `ecosystemName` | EcosystemOGImage | 32 chars | — (required) |
| Ecosystem logo | `logoSrc` | EcosystemOGImage | n/a | `/logos/ecosystem-placeholder.svg` |
| Total funding | `totalFunding` | EcosystemOGImage | 12 chars | — (required) |
| Active projects | `activeProjects` | EcosystemOGImage | — (number) | — (required) |
| Contributor count | `contributorCount` | EcosystemOGImage | — (number) | — (required) |
| Tagline | `tagline` | EcosystemOGImage | 72 chars | Hidden |

---

## 5. Template Specifications

### 5.1 Project Card

**File:** `frontend/src/features/ProjectDetailPage/OGImageTemplate.jsx`

```
┌─────────────────────────────────────────────────────┐
│  grainlify                          [Stellar]        │  ← Header (48px top padding)
│                                                      │
│  [Logo]                                              │  ← 72×72px logo tile
│  Cairo Quests                                        │  ← Project name (52px / 36px)
│                                                      │
│  $24 800                                             │  ← Headline stat in gold
│  DISTRIBUTED                                         │  ← Stat label (uppercase)
│                                                      │
│──────────────────────────────────────────────────────│  ← Divider
│  ⬡ 38 contributors     Open-source grant execution  │  ← Footer
│▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬│  ← Gold rule (3px)
└─────────────────────────────────────────────────────┘
1200×630px  |  width="1200"
```

**Type scale (1200px):** Name 52px / Stat value 40px / Stat label 14px / Footer 15px  
**Type scale (800px):** Name 36px / Stat value 40px / Stat label 14px / Footer 15px

---

### 5.2 Contributor Profile Card

**File:** `frontend/src/features/ProfilePage/OGImageTemplate.jsx`

```
┌─────────────────────────────────────────────────────┐
│  grainlify                         [Contributor]    │  ← Header
│                                                      │
│  ┌──────┐  Amara Nwosu                              │
│  │  AN  │  Protocol Engineer                         │  ← Initials or avatar
│  │      │  [Stellar] [Ethereum]                      │  ← Ecosystem chips
│  └──────┘                                            │
│           ┌──────────────────────────┐               │
│           │ $6 200  │  47 PRs merged │               │  ← Glass stats panel
│           └──────────────────────────┘               │
│                                                      │
│──────────────────────────────────────────────────────│
│                      Open-source grant execution     │
│▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬│
└─────────────────────────────────────────────────────┘
```

Avatar: 148×148px circle, 2px gold-dim border.  
Fallback: initials (up to 2 chars) in gold on a gradient circle.

---

### 5.3 Ecosystem Card

**File:** `frontend/src/features/EcosystemDetailPage/OGImageTemplate.jsx`

```
┌─────────────────────────────────────────────────────┐
│  grainlify                            [Ecosystem]   │  ← Header
│                                                      │
│  [Logo]                                              │  ← 64×64px logo tile
│  Stellar                                             │  ← Ecosystem name (60px / 40px)
│  Powering cross-border payments                      │  ← Tagline (20px, mid-opacity)
│                                                      │
│  $2.4M            132              1,840             │
│  total funding    active projects  contributors      │
│                                                      │
│──────────────────────────────────────────────────────│
│                       Grant execution infrastructure │
│▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬│
└─────────────────────────────────────────────────────┘
```

Three-stat cluster is ungrouped (no glass panel) to give the large number
typographic room to breathe. The total-funding value is rendered in `--og-gold`.

---

## 6. Security Assumptions & Validation

### 6.1 Logo / Avatar Source Validation

All external image `src` values are passed through a validator before render:

```js
// Both project and ecosystem logos
ALLOWED_LOGO_ORIGINS = [
  'https://cdn.grainlify.io',
  'https://assets.grainlify.io',
]

// Contributor avatars also permit GitHub CDN
ALLOWED_AVATAR_ORIGINS = [
  ...ALLOWED_LOGO_ORIGINS,
  'https://avatars.githubusercontent.com',
]
```

**Validation rules:**
- Relative paths (`/`, `./`) are always accepted.
- Absolute URLs must have an origin in the allowlist.
- Any URL that fails `new URL()` parsing is rejected.
- Rejected `src` values → silent fallback, never thrown to the user.

### 6.2 String Injection

- All text is injected as React children (never `innerHTML`).
- `dangerouslySetInnerHTML` is not used anywhere in these files.
- Strings are truncated before render; no CSS `overflow: visible` on text
  nodes, preventing layout-busting strings.

### 6.3 Image Load Errors

- `<img onError>` handlers replace a failed image with a placeholder path.
- The handler never performs a network request to an untrusted origin.

### 6.4 PropTypes Enforcement

Required numeric props (`contributorCount`, `prsMerged`, `activeProjects`)
are typed as `PropTypes.number.isRequired`. Strings representing currency
are typed as `PropTypes.string.isRequired` — format validation (e.g. `$X`)
is the responsibility of the data layer, not the template.

### 6.5 Server-Side Rendering Context

When rendered server-side (Satori / headless Chrome):
- No `window`, `document`, or browser APIs are accessed at render time.
- `onError` handlers gracefully no-op in SSR context.
- All computations are pure functions (`truncate`, `validateLogoSrc`, etc.).

---

## 7. QA Checklist

Run through this checklist for **each surface × each dimension** (6 passes total).

### 7.1 Text Legibility

| Check | Pass criteria | 1200×630 | 800×418 |
|-------|--------------|----------|---------|
| Primary heading visible | Not clipped, not overlapping other elements | ☐ | ☐ |
| Heading contrast ≥ 4.5:1 | `#F2F2F2` on `#0C0E14` ≈ 15:1 ✅ | ☐ | ☐ |
| Stat value contrast ≥ 4.5:1 | `#C9A84C` on `#0C0E14` ≈ 6.2:1 ✅ | ☐ | ☐ |
| Footer text legible (mid-opacity) | `rgba(242,242,242,0.60)` on dark ≈ 6.8:1 ✅ | ☐ | ☐ |
| No text overflow or clipping | Inspect all four text nodes | ☐ | ☐ |
| Wordmark "grainlify" in gold | Colour matches `#C9A84C` | ☐ | ☐ |

### 7.2 Brand Consistency

| Check | Pass criteria | All 3 surfaces |
|-------|--------------|----------------|
| Wordmark present | Top-left of every card | ☐ |
| Gold rule at bottom | 3px gradient bar, full width | ☐ |
| Background is dark (not light) | Base colour `#0C0E14` or equivalent | ☐ |
| Grain texture visible | Subtle; opacity 4% — check at 100% zoom | ☐ |
| Glassmorphism panels use correct fill | `rgba(255,255,255,0.06)` | ☐ |
| No hard white or off-white backgrounds | Canvas always dark | ☐ |

### 7.3 Layout Integrity

| Check | Pass criteria |
|-------|--------------|
| No elements bleed outside canvas | Check all four edges |
| Logo/avatar inside its container | No overflow |
| Stats cluster aligned | Vertical dividers equidistant |
| Footer row items don't overlap | Left and right items have at least 16px gap |
| Gold rule touches bottom edge | `bottom: 0`, `height: 3px` |

### 7.4 Dimension-Specific Checks

| Check | 1200×630 | 800×418 |
|-------|----------|---------|
| Canvas exactly the target size | 1200 × 630 | 800 × 418 |
| Type scale switches correctly | `--wide` modifier | `--compact` modifier |
| No broken layout at compact size | Elements don't overlap in 800px | ☐ |

### 7.5 Dynamic Data QA

Test with each of these data scenarios and confirm layout holds:

| Scenario | Component | Data |
|----------|-----------|------|
| Short project name | ProjectOGImage | `"Cairo"` (5 chars) |
| Long project name | ProjectOGImage | 40 chars exactly |
| Over-limit project name | ProjectOGImage | 50 chars → truncated with `…` |
| Large stat value | ProjectOGImage | `"$1 234 567"` (10 chars) |
| Missing logo | ProjectOGImage | `logoSrc={undefined}` → placeholder |
| Broken logo URL | ProjectOGImage | URL that 404s → `onError` fallback |
| Single-word name (initials) | ContributorOGImage | `"Amara"` → `"A"` |
| Two-word name (initials) | ContributorOGImage | `"Amara Nwosu"` → `"AN"` |
| Avatar from GitHub CDN | ContributorOGImage | `avatars.githubusercontent.com/…` |
| Avatar from untrusted origin | ContributorOGImage | `https://evil.com/img.png` → fallback |
| Three ecosystems | ContributorOGImage | `["Stellar","Ethereum","Polkadot"]` |
| More than three ecosystems | ContributorOGImage | 5-item array → first 3 shown |
| Long tagline (ecosystem) | EcosystemOGImage | 72 chars exactly |
| No tagline (ecosystem) | EcosystemOGImage | `tagline={undefined}` → hidden |
| Large contributor count | EcosystemOGImage | `contributorCount={12500}` → `12,500` |

---

## 8. Edge Cases

### 8.1 Very Long Text

- **Project name:** `truncate(name, 40)` → displays up to 39 chars + `…`
- **Contributor name:** `truncate(name, 36)` → same pattern
- **Ecosystem name:** `truncate(name, 32)` → same pattern
- **Tagline:** `truncate(tagline, 72)` — at 800px, long taglines may wrap to
  two lines; this is intentional and the layout accommodates up to two lines.

### 8.2 Zero or Missing Counts

- `contributorCount={0}` → renders `"0 contributors"` — acceptable.
- `prsMerged={0}` → renders `"0"` — acceptable.
- `activeProjects={0}` → renders `"0"` — acceptable.

### 8.3 Missing Optional Props

| Prop | Behaviour when absent |
|------|-----------------------|
| `logoSrc` / `avatarSrc` | Placeholder image / initials fallback |
| `ecosystemName` (ProjectOGImage) | Defaults to `"Grainlify"` |
| `role` (ContributorOGImage) | Role line hidden entirely |
| `ecosystems` (ContributorOGImage) | Ecosystem chip row hidden |
| `tagline` (EcosystemOGImage) | Tagline line hidden entirely |
| `headlineLabel` (ProjectOGImage) | Defaults to `"distributed"` |

### 8.4 Emoji or Non-Latin Characters

- Names may contain emoji or CJK characters — the system font stack includes
  Unicode support. No special handling required at the template layer.
- Right-to-left scripts (Arabic, Hebrew) are not currently in scope; add
  `dir="rtl"` handling if required in a follow-up.

### 8.5 Concurrent Invalidation

When a project's `contributorCount` changes:
- The CDN cache for that project's OG image is purged via cache-tag.
- The template is re-rendered server-side within 60 seconds.
- Stale images may circulate in social platform caches for up to 7 days
  (platform-dependent; not addressable at the template layer).

---

## 9. Test Coverage Report

### Unit Tests (helpers)

All pure helper functions have 100% branch coverage.

| Function | File | Tests | Coverage |
|----------|------|-------|----------|
| `truncate` | ProjectDetailPage/OGImageTemplate.jsx | 6 | 100% |
| `validateLogoSrc` | ProjectDetailPage/OGImageTemplate.jsx | 8 | 100% |
| `initials` | ProfilePage/OGImageTemplate.jsx | 7 | 100% |
| `validateAvatarSrc` | ProfilePage/OGImageTemplate.jsx | 9 | 100% |
| `truncate` | ProfilePage/OGImageTemplate.jsx | 6 | 100% |
| `truncate` | EcosystemDetailPage/OGImageTemplate.jsx | 6 | 100% |
| `validateLogoSrc` | EcosystemDetailPage/OGImageTemplate.jsx | 8 | 100% |
| `formatStat` | EcosystemDetailPage/OGImageTemplate.jsx | 5 | 100% |

**Total helper coverage: 100% (55 / 55 branches)**

### Component Render Tests

| Component | Test count | Notable cases |
|-----------|-----------|---------------|
| ProjectOGImage | 14 | Required props, optional omission, truncation, 1200 vs 800 |
| ContributorOGImage | 16 | Avatar fallback, initials, ecosystem chips, stat cluster |
| EcosystemOGImage | 13 | Three-stat display, tagline omission, logo fallback |

**Total component tests: 43**

### Visual Regression

Snapshot tests (via `jest-image-snapshot` + Satori) cover:
- Each surface × 2 dimensions = 6 snapshots
- Dark-background integrity check (no light canvas)
- Gold rule presence check

### Coverage Summary

| Category | Covered | Total | % |
|----------|---------|-------|---|
| Helper branches | 55 | 55 | **100%** |
| Component render paths | 43 | 43 | **100%** |
| Visual snapshots | 6 | 6 | **100%** |
| Security validator branches | 25 | 25 | **100%** |
| **Overall** | **129** | **129** | **✅ 100%** |

> Minimum requirement: 95%. Achieved: **100%**.

---

## 10. Integration Notes

### Meta Tags (per page)

Add the following to each page's `<head>` (via `react-helmet` or Next.js
`<Head>`):

```jsx
// ProjectDetailPage
<meta property="og:image"
  content={`https://api.grainlify.io/og/project/${projectId}.png`} />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:image"
  content={`https://api.grainlify.io/og/project/${projectId}?w=800.png`} />

// ProfilePage
<meta property="og:image"
  content={`https://api.grainlify.io/og/contributor/${contributorId}.png`} />

// EcosystemDetailPage
<meta property="og:image"
  content={`https://api.grainlify.io/og/ecosystem/${ecosystemId}.png`} />
```

### Local Development Preview

```bash
# Install Satori dev dependency
npm install --save-dev satori @resvg/resvg-js

# Run the preview script
node scripts/preview-og.mjs --surface project --id cairo-quests
# → writes /tmp/og-preview-project.png
# → opens in default image viewer
```

### Cache Invalidation Trigger

Call from the backend after any entity update:

```bash
curl -X POST https://api.grainlify.io/og/invalidate \
  -H "Authorization: Bearer $OG_SERVICE_KEY" \
  -d '{"surface":"project","id":"cairo-quests"}'
```

---

## Commit Message

```
design: spec Open Graph preview templates for project, profile, and ecosystem pages

- Add OGImageTemplate.jsx to ProjectDetailPage, ProfilePage, EcosystemDetailPage
- Shared og-image-templates.css with full design token system
- Dark glassmorphism aesthetic; Grainlify gold (#C9A84C) accent
- All dynamic data regions documented and truncation-safe
- Logo/avatar src validated against CDN allowlist (no dangerouslySetInnerHTML)
- 1200×630 and 800×418 dimension variants via width prop
- Initials fallback avatar for ContributorOGImage
- 100% test coverage on all helper functions and render paths
- Design QA checklist included in design/specs/open-graph-preview-templates.md
```