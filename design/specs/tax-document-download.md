# Tax Document Download UX — Design Spec

**Issue:** [#1394 Design tax-document download UX (1099-style PDFs) on SettingsPage](https://github.com/AnnieIj/grainlify/issues/1394)  
**Branch:** `design/tax-document-download-settings-ux`  
**Status:** Implemented  
**Last updated:** 2026-06-25

---

## Overview

Contributors who earn above reporting thresholds ($600 USD annually) need a way to download annual tax summaries directly from SettingsPage. This spec covers the end-to-end UX for the new **Tax Documents** tab including the documents list, year-range selector, PDF preview modal, download flow, and empty state.

---

## Screens in scope

| Screen | Breakpoints |
|---|---|
| Tax Documents tab — list view | 1440px, 375px |
| PDF preview modal | 1440px, 375px |
| Empty state (below threshold) | 1440px, 375px |

---

## 1. Tax Documents tab — list view

### Location
`frontend/src/features/settings/pages/SettingsPage.tsx` → tab `tax-documents`  
Component: `frontend/src/features/settings/components/tax-documents/TaxDocumentsTab.tsx`

### 1440px layout

```
┌────────────────────────────────────────────────────────────────┐
│  Tax Documents                              [Year range ▾]      │
│  Annual tax summaries for contributors…                         │
└────────────────────────────────────────────────────────────────┘
┌────────────────────────────────────────────────────────────────┐
│  📄  Tax Year 2024                [● Available]  [Preview] [↓] │
│      Generated Jan 31, 2025 · $12,450.00                       │
├────────────────────────────────────────────────────────────────┤
│  📄  Tax Year 2023                [● Available]  [Preview] [↓] │
│      Generated Jan 31, 2024 · $8,200.50                        │
├────────────────────────────────────────────────────────────────┤
│  📄  Tax Year 2022                [⏳ Pending]                   │
└────────────────────────────────────────────────────────────────┘
ℹ  Tax documents are issued to contributors whose annual earnings…
```

### 375px layout

- Header card stacks title/description above the year selector
- Each document row stacks the meta block and action controls vertically
- Status badge remains inline with document title
- Preview and Download buttons remain full-width on small screens

### Component structure

```
TaxDocumentsTab
├── Section header card (title, description, YearRangeSelector)
├── Document list card
│   ├── DocumentRow × N  (or EmptyState)
│   │   ├── FileText icon
│   │   ├── Year label + generated date + earnings
│   │   ├── StatusBadge
│   │   ├── Preview button → opens TaxDocumentPreviewModal
│   │   └── Download button (with Loader2 spinner while in progress)
│   └── EmptyState (no documents in range)
└── Below-threshold notice
```

---

## 2. Year-range selector

### Interaction

- Dropdown trigger shows selected range, e.g. `2021 – 2025` or single year `2024`
- Opens a listbox with per-year options for **From** year
- Options outside the valid range are rendered disabled with reduced opacity
- Keyboard accessible: `Tab` to focus, `Enter`/`Space` to open, arrow keys implied via button list, `Escape` closes
- Closes on outside click

### States

| State | Appearance |
|---|---|
| Default | Outlined pill, no fill |
| Hover | Subtle background fill (`bg-white/[0.12]` dark, `bg-white/[0.25]` light) |
| Open | Chevron rotates 180°; dropdown renders below trigger |
| Year selected | Year label in `#c9983a` (brand gold), font-semibold |
| Year disabled | Muted color, `cursor-not-allowed` |
| Focus | `ring-2 ring-[#c9983a]/40` |

---

## 3. Availability status badges

| Status | Icon | Label | Light bg | Dark bg |
|---|---|---|---|---|
| `available` | `CheckCircle2` | Available | `bg-green-50 text-green-700` | `bg-green-900/30 text-green-400` |
| `pending` | `Clock` | Pending | `bg-yellow-50 text-yellow-700` | `bg-yellow-900/30 text-yellow-400` |
| `not-applicable` | `AlertCircle` | Not Applicable | `bg-neutral-100 text-neutral-500` | `bg-white/[0.05] text-[#b8a898]` |

---

## 4. PDF preview modal

### Location
`frontend/src/features/settings/components/tax-documents/TaxDocumentPreviewModal.tsx`

### 1440px layout

```
╔══════════════════════════════════════════════════════════╗
║  Tax Document Preview — 2024                          ✕  ║
╠══════════════════════════════════════════════════════════╣
║                                                          ║
║  ┌──────────────────────────────────────────────────┐   ║
║  │  [iframe PDF preview / branded summary card]      │   ║
║  └──────────────────────────────────────────────────┘   ║
║                                                          ║
╠══════════════════════════════════════════════════════════╣
║  [↓ Download PDF]   [Cancel]                             ║
╚══════════════════════════════════════════════════════════╝
```

### Fallback: branded summary card (when no `pdfUrl`)

Renders when the document URL is not yet available (e.g. pre-signed URL not provisioned). Shows:

- Grainlify branded circular icon (gradient `#c9983a → #a2792c`)
- "Annual Tax Summary" sub-header
- Earnings table: Tax Year, Total Earnings, Stellar Address, Generated date
- Disclaimer footer: _"This document is generated for informational purposes only…"_

### Accessibility

- `role="dialog"`, `aria-modal="true"`, `aria-labelledby="tax-preview-title"`
- Focus moves to Close button on open; returns to trigger on close
- Focus trapped: `Tab`/`Shift+Tab` cycles within modal
- `Escape` closes modal
- Backdrop click closes modal
- `body.overflow = hidden` while modal is open; restored on unmount

### Modal tokens (from `design-tokens.json`)

| Token | Value |
|---|---|
| `modal.zIndexBase` | `10000` |
| `modal.overlayOpacity.base` | `0.50` |
| `modal.borderRadius` | `24px` |
| `modal.shadow` | `0 8px 32px rgba(0,0,0,0.24)` |
| `modal.animationDuration` | per motion-spec |

---

## 5. Empty state

Shown when no documents exist for the selected year range.

```
        📄
  No documents found
  No tax documents are available for the selected year range.
  Documents are generated for contributors who earn above the
  reporting threshold ($600 USD).
```

- Centered vertically and horizontally within the list card
- Icon container: `w-14 h-14 rounded-full` with `bg-[#c9983a]/[0.1]` (light) / `bg-white/[0.06]` (dark)
- Icon: `FileText` in `text-[#c9983a]`

---

## 6. Download progress indicator

- Button text changes from "Download" → "Downloading…"
- `Download` icon replaced by `Loader2` with `animate-spin`
- Button is `disabled` during download; reduced opacity (`disabled:opacity-60`) + `cursor-not-allowed`
- Preview modal closes before download starts

---

## 7. PDF template spec (grainlify-branded)

For the server-generated PDF, the template must include:

### Branded header
- Grainlify wordmark (SVG, `frontend/src/assets/grainlify_log.svg`)
- Title: "Annual Tax Summary" · Sub-title: tax year in large type
- Brand gold rule below header (`#c9983a`)

### Earnings table
| Column | Value |
|---|---|
| Tax Year | 20XX |
| Contributor | GitHub username |
| Stellar Address | `G…` full address |
| Total Earnings | $X,XXX.XX USD |
| Payment Period | Jan 1 – Dec 31, 20XX |

### Footer
- "Grainlify, Inc. · [address]"
- "This document is provided for informational purposes only and does not constitute tax advice. Consult a qualified tax professional for reporting guidance."
- Generation timestamp (ISO 8601)

---

## 8. Design tokens used

| Token | Usage |
|---|---|
| `color.primary.600` (`#c9983a`) | Accent, badge backgrounds, icon tints |
| `color.primary.700` (`#a2792c`) | Active tab, gradient end, download button |
| `color.neutral.*` | Text hierarchy, borders |
| `color.semantic.success.*` | Available badge |
| `color.semantic.warning.*` | Pending badge |

---

## 9. Security notes

- PDF URLs should be pre-signed with short TTLs (≤15 min) — never expose long-lived storage tokens client-side
- Tax documents must only be served to the authenticated owner; backend must enforce user-scoped access
- Stellar address displayed in the document is read-only; no mutation endpoint is exposed via this UI
- No PII is stored in frontend state beyond the active session

---

## 10. QA checklist

### Year selector
- [ ] Keyboard: `Tab` focuses selector, `Enter` opens dropdown
- [ ] Selecting a year updates the document list
- [ ] Disabled years cannot be selected
- [ ] Dropdown closes on outside click and `Escape`

### Document list
- [ ] Available documents show Preview and Download buttons
- [ ] Pending documents show only the Pending badge (no action buttons)
- [ ] Download spinner appears and button is disabled during download
- [ ] Empty state renders when no documents match the selected range
- [ ] Earnings amounts formatted with currency symbol and comma separators

### PDF preview modal
- [ ] Opens on Preview button click
- [ ] Focus moves to Close button on open
- [ ] `Tab` / `Shift+Tab` cycle stays within modal
- [ ] `Escape` and backdrop click both close the modal
- [ ] Focus returns to trigger button on close
- [ ] Body scroll is locked while modal is open
- [ ] Download button inside modal triggers download and closes modal
- [ ] Mobile: modal scrolls internally and does not exceed 90vh

### Accessibility
- [ ] All interactive elements have accessible names (`aria-label` where needed)
- [ ] Status badges have meaningful text (not icon-only)
- [ ] Color contrast meets WCAG 2.1 AA (4.5:1 for text, 3:1 for UI components)

### Responsive (375px)
- [ ] Header card stacks correctly
- [ ] Document rows stack vertically
- [ ] Modal fits within viewport; inner content scrolls

---

## 11. Implementation files

| File | Purpose |
|---|---|
| `frontend/src/features/settings/types/index.ts` | `TaxDocument`, `TaxDocumentStatus`, `TaxDocumentYearRange` types; `SettingsTabType` updated |
| `frontend/src/features/settings/components/tax-documents/TaxDocumentsTab.tsx` | Main tab: list, year selector, empty state, download |
| `frontend/src/features/settings/components/tax-documents/TaxDocumentPreviewModal.tsx` | PDF preview modal with focus trap |
| `frontend/src/features/settings/pages/SettingsPage.tsx` | Tab registered: import, tab entry, render branch |
| `design/specs/tax-document-download.md` | This spec |
