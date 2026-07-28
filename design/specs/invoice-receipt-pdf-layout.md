# Invoice / Receipt PDF Layout — Design Spec

**Status:** Draft  
**Last updated:** 2026-07-26

---

## 1. Document Overview

**Purpose** – Produce a formal payout receipt/invoice PDF that contributors and program maintainers can download from the billing section of Settings. The PDF serves as a proof-of-payment record showing each line item, fee breakdown, and net settlement.

**Intended users**
- Contributors who receive payouts for completed work
- Program maintainers who manage payout records
- Accounting/finance teams who reconcile payments

**Print size** – A4 (210 × 297 mm). The PDF uses a 595 × 842 pt viewport at 72 DPI. All measurements in the redlines section are expressed in mm for print and px for screen preview.

**PDF vs in-app preview differences**

| Aspect | PDF | In-app preview |
|---|---|---|
| Format | Server-generated or client-rendered `Blob` | Rendered inside a `<div>` via the same React component |
| Interactivity | Static | Scrollable, zoomable, closable |
| Branding | Full colour, logos, decorative gold rule | Respects theme (dark/light) |
| Watermarks | Refunded/Void watermarks rendered | Same watermarks rendered |
| Output | `.pdf` download | Modal display with download CTA |

**Design principles**
- Use only existing tokens from `design-tokens.json`. No new tokens.
- Follow glassmorphism style for the preview modal to match the rest of the billing UI.
- Keep the PDF printable in grayscale — never rely on colour alone to convey information.
- Meet WCAG 2.1 AA (4.5:1 text contrast, 3:1 UI component contrast).
- Single source of truth: one set of components renders both the PDF and the preview.

---

## 2. PDF Layout

### Page dimensions

- **Format:** A4 (210 × 297 mm / 595 × 842 pt)
- **Margins:** 20 mm all sides (57 pt)
- **Content width:** 170 mm (481 pt)
- **Baseline grid:** 4 pt

### Document anatomy (top → bottom)

```
┌──────────────────────────────────────────────────┐
│  ┌──────────────────────────────────────────┐    │  ← 20 mm margin
│  │  HEADER                                  │    │
│  │  [Logo]  INVOICE  INV-2026-0042          │    │
│  │          Issued: Jul 15, 2026            │    │
│  │          Paid:   Jul 16, 2026   [Paid]   │    │
│  ├──────────────────────────────────────────┤    │
│  │  CONTRIBUTOR                             │    │
│  │  Jane Doe                     Wallet     │    │
│  │  jane@example.com             G…ABCD…    │    │
│  │  Tax ID: •••–••–1234                     │    │
│  ├──────────────────────────────────────────┤    │
│  │  PROGRAM / PROJECT                       │    │
│  │  Open Source Fund · Q3 Maintenance       │    │
│  │  Milestone: M2 – Security Audit          │    │
│  │  Period: Jul 1 – Jul 15, 2026  ·  USD    │    │
│  ├──────────────────────────────────────────┤    │
│  │  LINE ITEM TABLE                         │    │
│  │  ┌─────────────┬───┬──────┬──────┬────┐  │    │
│  │  │ Description │ Q │ Unit │Gross │Net │  │    │
│  │  ├─────────────┼───┼──────┼──────┼────┤  │    │
│  │  │ Item 1     │ 1 │ 500  │ 500  │475 │  │    │
│  │  │ Item 2     │ 2 │ 250  │ 500  │475 │  │    │
│  │  └─────────────┴───┴──────┴──────┴────┘  │    │
│  ├──────────────────────────────────────────┤    │
│  │  SUMMARY                                 │    │
│  │  Gross total               $1,000.00     │    │
│  │  Platform fee (5%)         –$50.00       │    │
│  │  ─────────────────────────────────────   │    │
│  │  Net paid                  $950.00       │    │
│  │  Outstanding balance       $0.00         │    │
│  ├──────────────────────────────────────────┤    │
│  │  FOOTER                                  │    │
│  │  USDC · Stellar · G…TXHASH…             │    │
│  │  support@grainlify.com                   │    │
│  │  Disclaimer text                   1/1   │    │
│  └──────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

---

### Header

| Element | Value / behaviour |
|---|---|
| **Logo** | Grainlify wordmark (`frontend/src/assets/grainlify_log.svg`). Left-aligned. Max height 32 pt. |
| **Title** | `"INVOICE"` or `"RECEIPT"` set in **bold** 18 px / 24 pt, colour `neutral.900` (`#1c1917`) or `darkMode.text.primary` (`#f5f5f5`) for dark. |
| **Invoice number** | Below title, text 11 px, colour `neutral.500` (`#78716c`). Format: `INV-YYYY-NNNN`. |
| **Issue date** | Same line as invoice number. Format: `Issued: MMM DD, YYYY`. |
| **Payment date** | Below issue date. Only shown when `status === 'paid'`. Format: `Paid: MMM DD, YYYY`. |
| **Status badge** | Right-aligned. Pill badge matching existing `InvoicesTab.tsx` token patterns. |

**Status badge tokens**

| Status | Background | Text | Border | Icon |
|---|---|---|---|---|
| `paid` | `semantic.success.50` / 20% opacity dark | `semantic.success.600` / `semantic.success.500` dark | `semantic.success.500` / 30% opacity | `CheckCircle2` |
| `pending` | `semantic.warning.50` / 20% opacity dark | `semantic.warning.600` / `semantic.warning.500` dark | `semantic.warning.500` / 30% opacity | `Clock` |
| `refunded` | `semantic.error.50` / 20% opacity dark | `semantic.error.600` / `semantic.error.500` dark | `semantic.error.500` / 30% opacity | `RotateCcw` |
| `void` | `neutral.100` / `darkMode.border.subtle` | `neutral.500` / `darkMode.text.muted` | `neutral.300` / `darkMode.border.default` | `Ban` |

**Decorative rule** – A 2 pt solid horizontal rule in `primary.600` (`#c9983a`) spans the full content width below the header block. 16 pt bottom margin.

---

### Contributor Block

Positioned below the header rule.

| Field | Typography | Colour | Notes |
|---|---|---|---|
| **Section label** | 9 px uppercase bold, tracking 0.5 px | `neutral.400` | Label: `"CONTRIBUTOR"` |
| **Contributor name** | 14 px bold | `neutral.900` / `darkMode.text.primary` | From billing profile `firstName` + `lastName` |
| **Email** | 11 px regular | `neutral.500` | Optional; show only if available |
| **Wallet / account** | 11 px mono | `neutral.500` | Right-aligned, truncate with ellipsis in PDF |
| **Tax ID** | 11 px regular | `neutral.500` | Show if `taxId` exists. Mask middle digits as `•••–••–1234` |

Layout: Two-column grid. Left column holds name + email + tax ID. Right column holds wallet label + address.

---

### Program / Project Block

Same two-column layout as Contributor Block.

| Field | Typography | Colour |
|---|---|---|
| **Section label** | 9 px uppercase bold, tracking 0.5 px | `neutral.400` |
| **Program name** | 14 px bold | `neutral.900` / `darkMode.text.primary` |
| **Project name** | 13 px regular | `neutral.600` |
| **Milestone / task ref** | 12 px regular | `neutral.500` |
| **Payment period** | 12 px regular | `neutral.500` |
| **Currency** | Right-aligned, 14 px bold | `neutral.900` |

---

### Line Item Table

Column definitions and widths (content area = 481 pt):

| Column | Alignment | Width | Header label |
|---|---|---|---|
| Description | Left | 45% (216 pt) | Description |
| Quantity | Right | 10% (48 pt) | Qty |
| Unit amount | Right | 15% (72 pt) | Unit Amount |
| Gross amount | Right | 15% (72 pt) | Gross |
| Net payout | Right | 15% (73 pt) | Net Payout |

**Table styling**
- Header row: 10 px bold uppercase, `neutral.500`, bottom border 1 px `neutral.300`
- Body rows: 12 px regular, `neutral.800` / `darkMode.text.primary`
- Alternating row background: every other row has `neutral.50` / `darkMode.background.surfaceSecondary` at 20% opacity
- Row height: 28 pt minimum
- Last row bottom border: 1.5 px `neutral.400`
- All monetary values formatted with currency symbol and thousands separator

**Fee breakdown (per line item, optional)**  
When a single item has a platform fee or deduction, show a sub-row:
```
│ Security audit       │  1 │ $1,000.00 │ $1,000.00 │ $950.00  │
│   └ Platform fee 5%  │    │           │  –$50.00  │          │
```

Sub-row: 10 px italic, `neutral.400`, indented 16 pt, no background.

**Platform fee and deductions**

| Concept | Type | Display |
|---|---|---|
| Platform fee | Percentage or flat | Sub-row under the item OR aggregated in Summary |
| Stellar network fee | Flat (≈0.00001 XLM) | Show in Summary only if > $0.01 equivalent |
| Other deductions | Named (e.g. "Withholding") | Sub-row under item + aggregated in Summary |

---

### Summary

Positioned below the line item table. Right-aligned block with the following rows:

| Row | Typography | Format |
|---|---|---|
| Gross Total | 13 px regular, `neutral.600` | `$X,XXX.XX` |
| Platform Fee | 13 px regular, `neutral.600` | `–$X,XXX.XX` |
| Other Deductions | 13 px regular, `neutral.600` | `–$X,XXX.XX` (hide if zero) |
| Horizontal rule | 1.5 px `neutral.400` | Full summary width |
| **Net Paid** | **16 px bold**, `neutral.900` / `darkMode.text.primary` | **`$X,XXX.XX`** |
| Outstanding Balance | 12 px regular, `semantic.error.600` | `$X,XXX.XX` (hide if zero) |

Outstanding balance only displays when there are pending amounts across the billing profile (e.g. partial payments).

---

### Footer

| Element | Details |
|---|---|
| **Payment method** | `"{cryptoType} on {ecosystem}"` e.g. `USDC on Stellar` |
| **Transaction hash** | Mono 9 px, truncated to last 8 chars with ellipsis on PDF. Full hash in tooltip (preview) or wrapped. Label: `"Tx: G…ABCD…"` |
| **Support contact** | `support@grainlify.com`, 10 px regular |
| **Legal disclaimer** | 8 px regular, `neutral.400`. Text: *"This document is provided for informational purposes only and does not constitute a tax invoice or legal receipt. Consult a qualified professional for reporting guidance."* |
| **Page number** | Right-aligned, 9 px regular, `neutral.400`. Format: `"Page X of Y"` |

Footer block sits 16 pt above the bottom margin.

---

## 3. Multi-page Behaviour

When the line item table exceeds the available vertical space, the PDF flows to subsequent pages.

| Element | Behaviour |
|---|---|
| **Repeating header** | Logo + title + invoice number repeated at the top of each continuation page. Status badge omitted after page 1. |
| **Repeating table header** | Table column headers repeat on every page. Same 10 px bold styling as page 1. |
| **Contributor / Program blocks** | Printed once on page 1 only. |
| **Summary** | Printed on the last page only, immediately after the final line item. |
| **Footer** | Repeats on every page: payment method, support email, disclaimer, page number. |
| **Page numbering** | `"Page X of Y"` right-aligned in footer. X = current page, Y = total pages. |
| **Row splitting** | Line item rows are treated as atomic units — never split a single row across pages. If a row does not fit, it starts on the next page. |
| **Bottom margin** | 40 pt reserved on each page for the footer. |

---

## 4. Invoice States

### Single-item invoice
Standard layout. One line item row, summary directly below.

### Multi-item invoice
Standard layout with 2+ line item rows. If items exceed page, flow to page 2+.

### Refunded invoice

**Status badge:** `refunded` (red tones, `RotateCcw` icon).

**Watermark treatment:**
- Diagonal "REFUNDED" text across the full content area
- Colour: `semantic.error.500` at 12% opacity
- Rotation: 45°
- Font: 48 px bold, uppercase
- The watermark must not obscure readability — it sits behind all content
- The header status badge also shows `Refunded`

**Summary changes:**
- "Net Paid" row replaced by "Amount Refunded" in `semantic.error.600`
- Original net payout shown as strikethrough for reference

### Void invoice

**Status badge:** `void` (neutral tones, `Ban` icon).

**Watermark treatment:**
- Diagonal "VOID" text across the full content area
- Colour: `neutral.400` at 10% opacity
- Rotation: 45°
- Font: 48 px bold, uppercase
- All monetary values display as `$0.00` or `"—"`
- Summary shows "Net Paid: $0.00" with a note: "This invoice has been voided."

**No outstanding balance row.**

---

## 5. In-App Preview Modal

### Modal structure

The preview modal reuses `frontend/src/shared/components/ui/Modal.tsx` with the following configuration:

| Prop | Value |
|---|---|
| `width` | `xl` (650 px) — but overridden to `w-[95vw] max-w-[900px]` for PDF preview |
| `maxHeight` | `true` |
| `showCloseButton` | `true` |
| `dimBackdrop` | `true` |

A new `width` option `full` may be added to the Modal component to support this use case, or a custom class override can be applied.

### Layout

```
╔══════════════════════════════════════════════════════════╗
║  Invoice Preview — INV-2026-0042           [✕]          ║
╠══════════════════════════════════════════════════════════╣
║  ┌────────────────────────────────────────────────────┐  ║
║  │  [Toolbar]  [🔍–  100%  🔍+]    [↓ Download PDF]  │  ║
║  ├────────────────────────────────────────────────────┤  ║
║  │                                                     │  ║
║  │  ┌──────────────────────────────────────────────┐  │  ║
║  │  │  PDF preview rendered as React component      │  │  ║
║  │  │  Scaled to fit within modal                   │  │  ║
║  │  └──────────────────────────────────────────────┘  │  ║
║  │                                                     │  ║
║  └────────────────────────────────────────────────────┘  ║
╠══════════════════════════════════════════════════════════╣
║  [Close]                                                 ║
╚══════════════════════════════════════════════════════════╝
```

### Toolbar

- **Zoom controls:** `–` button, percentage label, `+` button. Steps: 50%, 75%, 100%, 125%, 150%. Default: 100%.
- **Download PDF button:** Primary gradient button (`from-[#c9983a] to-[#a67c2e]`). Downloads the generated PDF blob.
- **Close button:** Reuses existing Modal close button pattern.

### Scroll behaviour

- The modal itself scrolls vertically if the content exceeds `max-h-[90vh]`.
- The PDF preview area has its own independent scroll if the scaled rendering overflows the visible area.
- Body scroll is locked while modal is open (inherited from Modal component).

### Zoom controls

- `aria-label="Zoom out"` / `Zoom in` on buttons
- `aria-live="polite"` on the percentage label announces zoom level
- Current zoom rendered as `"100%"` in 13 px font, `neutral.500`
- Zoom range: 50%–150%

### Download PDF CTA

- Triggers PDF generation and download
- Shows `Loader2` spinner during generation
- Disabled state with `opacity-50 cursor-not-allowed` while generating

### Close action

- Close button, `Escape` key, and backdrop click all close the modal.
- Focus returns to the "Download" or "Preview" trigger button in the invoices list.

---

## 6. Responsive Behaviour

The PDF container is fixed at A4 proportions. The preview scales to fit the viewport.

### Desktop (≥1024 px)

- Modal width: `max-w-[900px]`, centered.
- PDF preview renders at 100% scale, filling the available width.
- Toolbar visible at top of preview area.

### Tablet (768–1023 px)

- Modal width: `95vw`, centered.
- PDF preview scales to fit. Zoom default adjusts to 75% on open.
- Toolbar remains visible.

### Mobile (375 px)

- Modal width: `95vw` (≈356 px usable).
- PDF preview scales to fit. Zoom default adjusts to 50% on open.
- Toolbar collapses: zoom controls remain, "Download" button spans full width below preview.
- The preview container should not exceed the viewport height — internal scroll handles overflow.
- Horizontal scrolling is never required; the preview scales down as needed.

### Breakpoint zoom defaults

| Breakpoint | Default zoom |
|---|---|
| ≥1024 px | 100% |
| 768–1023 px | 75% |
| 375–767 px | 50% |

---

## 7. Accessibility

### Focus trapping
- Follows the existing `Modal.tsx` focus trap: `Tab` and `Shift+Tab` cycle within the modal.
- Focus lands on the Close button on open (first focusable element).
- Focus returns to the trigger element on close.

### Keyboard navigation
- All interactive elements in the toolbar are keyboard-focusable: zoom buttons, download button, close button.
- `Escape` closes the modal (handled by Modal component).
- Arrow keys are not used for zoom — explicit `+` and `–` buttons prevent confusion.

### Screen reader labels
- Modal: `role="dialog"`, `aria-modal="true"`, `aria-labelledby` referencing the invoice title.
- Close button: `aria-label="Close invoice preview"`.
- Zoom buttons: `aria-label="Zoom in"`, `aria-label="Zoom out"`.
- Download button: `aria-label="Download invoice PDF"`.
- Status badge: text label included (not icon-only).
- Watermark text: `aria-hidden="true"` (decorative; information conveyed by status badge).

### Reading order
- Matches visual order: header → contributor → program → table → summary → footer.
- PDF tag tree order for screen readers follows the same sequence.

### WCAG 2.1 AA compliance
- All text: minimum 4.5:1 contrast ratio on applicable backgrounds.
- UI components (borders, focus rings): minimum 3:1 contrast ratio.
- Print-safe colours: use `neutral` and `semantic` token values only. Avoid relying on `darkMode` specific colours in the PDF output.

### Print-safe colours
- PDF always renders in light mode colour set (`neutral.800` text on white background) regardless of the app's current theme.
- The in-app preview respects the current theme (dark/light) and uses the corresponding tokens from `design-tokens.json`.

---

## 8. Design Tokens

Only existing tokens from `design-tokens.json` are used. The following table maps each layout element to its token.

### Typography

| Token | Value | Usage |
|---|---|---|
| `typography.fontFamily.sans` | `Inter, system-ui, -apple-system, sans-serif` | Body text, headings |
| `typography.fontFamily.mono` | `JetBrains Mono, monospace` | Wallet addresses, tx hashes |
| `typography.fontWeight.bold` | `700` | Invoice title, summary net |
| `typography.fontWeight.semibold` | `600` | Section labels |
| `typography.fontWeight.medium` | `500` | Status badge text |
| `typography.fontWeight.normal` | `400` | Body, table data |

### Spacing

| Token | Value | Usage |
|---|---|---|
| `spacing.1` | 0.25 rem (4 pt) | Table cell padding |
| `spacing.2` | 0.5 rem (8 pt) | Vertical gap between summary rows |
| `spacing.3` | 0.75 rem (12 pt) | Section vertical spacing |
| `spacing.4` | 1 rem (16 pt) | Page margins, section spacing |
| `spacing.6` | 1.5 rem (24 pt) | Header bottom margin |
| `spacing.8` | 2 rem (32 pt) | Top margin of page |

### Colours (light mode — PDF output)

| Token | Value | Usage |
|---|---|---|
| `color.neutral.50` | `#fafaf9` | Alternating table row bg |
| `color.neutral.100` | `#f5f5f4` | Void badge bg |
| `color.neutral.300` | `#d6d3d1` | Table borders, void badge border |
| `color.neutral.400` | `#a8a29e` | Section labels, secondary text, disclaimer |
| `color.neutral.500` | `#78716c` | Body secondary text, table header |
| `color.neutral.600` | `#57534e` | Summary labels |
| `color.neutral.800` | `#292524` | Body text |
| `color.neutral.900` | `#1c1917` | Headings, bold summary |
| `color.primary.600` | `#c9983a` | Decorative rule, accent |
| `color.semantic.success.50` | `#f0fdf4` | Paid badge bg |
| `color.semantic.success.600` | `#16a34a` | Paid badge text |
| `color.semantic.warning.50` | `#fffbeb` | Pending badge bg |
| `color.semantic.warning.600` | `#d97706` | Pending badge text |
| `color.semantic.error.50` | `#fef2f2` | Refunded badge bg |
| `color.semantic.error.500` | `#ef4444` | Refunded badge text, watermark |
| `color.semantic.error.600` | `#dc2626` | Outstanding balance |

### Colours (dark mode — preview only)

| Token | Value | Usage |
|---|---|---|
| `darkMode.text.primary` | `#f5f5f5` | Headings, bold text |
| `darkMode.text.secondary` | `#d4d4d4` | Body text |
| `darkMode.text.tertiary` | `#b8a898` | Secondary labels |
| `darkMode.text.muted` | `#8b7a6a` | Void badge text |
| `darkMode.border.default` | `rgba(255,255,255,0.10)` | Table borders |
| `darkMode.background.surfacePrimary` | `#1a1714` | PDF preview bg |
| `darkMode.background.glassMedium` | `rgba(255,255,255,0.08)` | Table alt row |

### Borders

| Token | Value | Usage |
|---|---|---|
| `borderRadius.md` | 0.375 rem | Status badge corners |
| `borderRadius.lg` | 0.5 rem | Watermarked area |
| `borderRadius.xl` | 0.75 rem | — |

### Table styling

| Rule | Value |
|---|---|
| Header border bottom | 1 px `neutral.300` |
| Body row height | 28 pt min |
| Alternating row | `neutral.50` (light) / `glassMedium` (dark) |
| Last row border | 1.5 px `neutral.400` |

---

## 9. Redlines

### Page margins (A4 PDF)

| Edge | Value |
|---|---|
| Top | 57 pt (20 mm) |
| Bottom | 57 pt (20 mm) |
| Left | 57 pt (20 mm) |
| Right | 57 pt (20 mm) |
| Footer from bottom | 16 pt (above bottom margin) |

### Grid and column widths

The content area is 481 pt (170 mm) wide, divided into a single column for the document flow.

Two-column blocks (Contributor, Program) use:
- Left column: 60% (289 pt)
- Right column: 40% (192 pt)

### Line item table column widths

| Column | Width |
|---|---|
| Description | 216 pt (45%) |
| Qty | 48 pt (10%) |
| Unit Amount | 72 pt (15%) |
| Gross | 72 pt (15%) |
| Net Payout | 73 pt (15%) |

### Component sizing

| Element | Height / Size |
|---|---|
| Logo | ≤ 32 pt tall |
| Decorative rule | 2 pt × 481 pt |
| Status badge | 22 pt tall, 8 pt horizontal padding |
| Table header row | 24 pt |
| Table body row | 28 pt min |
| Watermark text | 48 pt bold |

### Spacing

| Between | Gap |
|---|---|
| Top of page → Header | 0 pt |
| Header → Decorative rule | 16 pt |
| Decorative rule → Contributor | 24 pt |
| Contributor → Program/Project | 24 pt |
| Program/Project → Table | 24 pt |
| Table → Summary | 20 pt |
| Summary → Footer | 24 pt |

### Alignment

- All monetary columns in the table: right-aligned
- Description column: left-aligned
- Header logo: left-aligned
- Status badge: right-aligned
- Summary block: right-aligned
- Footer content: left-aligned, page number right-aligned
- Section labels (`CONTRIBUTOR`, etc.): left-aligned, 9 pt uppercase

### Overflow behaviour

| Content type | Handling |
|---|---|
| Long project names | Truncate with ellipsis at 60 chars |
| Transaction hashes | Show first 2 + `…` + last 6 chars: `GB…ABCDEF` |
| Long wallet addresses | Same truncation as tx hashes |
| Multi-word descriptions | Wrap to next line within the Description column |
| Tax ID | Mask centre digits, show last 4 |
| Empty optional fields | Omit the row entirely (do not show blank fields) |

---

## 10. QA Checklist

### PDF layout
- [ ] A4 dimensions (595 × 842 pt) confirmed
- [ ] All margins within spec (20 mm / 57 pt)
- [ ] Header, contributor, program, table, summary, footer render in correct order
- [ ] Decorative gold rule renders at 2 pt
- [ ] Status badge renders with correct icon, text, and colour per status

### Print readability
- [ ] All text ≥ 8 pt
- [ ] Grayscale printable — no information conveyed by colour alone
- [ ] 4.5:1 minimum contrast for all text elements
- [ ] Status badges include text label (not icon-only)

### Contrast
- [ ] WCAG 2.1 AA: 4.5:1 for all text
- [ ] WCAG 2.1 AA: 3:1 for UI components (borders, badges)
- [ ] Watermark text does not reduce readability of foreground content

### Keyboard navigation (preview modal)
- [ ] Tab moves through close → zoom– → zoom label → zoom+ → download → back
- [ ] Shift+Tab reverses the order
- [ ] Escape closes the modal
- [ ] Focus does not leave the modal while open
- [ ] Focus returns to trigger on close

### Responsive preview
- [ ] Desktop (≥1024 px): modal ≤ 900 px, centered
- [ ] Tablet (768–1023 px): modal 95vw, default zoom 75%
- [ ] Mobile (375 px): modal 95vw, default zoom 50%, no horizontal scroll
- [ ] Toolbar collapses gracefully on mobile
- [ ] Zoom controls update the preview scale
- [ ] Preview area scrolls independently when zoomed

### Pagination
- [ ] Header + table header repeat on page 2+
- [ ] Summary only on last page
- [ ] Row splitting: single rows never straddle a page boundary
- [ ] Footer repeats on every page
- [ ] Page number format: `"Page X of Y"`

### Watermarks
- [ ] "REFUNDED" diagonal watermark renders correctly
- [ ] "VOID" diagonal watermark renders correctly
- [ ] Watermark does not obscure content
- [ ] Watermark is `aria-hidden="true"`
- [ ] Monetary values correctly zeroed on void invoices

### Long content
- [ ] Long project names truncate with ellipsis
- [ ] Long transaction hashes truncate correctly
- [ ] Empty optional fields are omitted (not rendered as blank rows)

---

## 11. Developer Notes

### PDF rendering approach

Use `@react-pdf/renderer` (if already in the project dependency tree) or generate via a server-side Node.js service using `pdfkit` or `puppeteer`. The preferred approach:

1. Build a shared `InvoiceDocument` React component that renders the entire PDF layout.
2. Use this component both for PDF generation (via `@react-pdf/renderer`'s `PDFDownloadLink` or `pdf()` helper) **and** for in-app preview (rendered into a regular `<div>` with identical markup and CSS).
3. The shared component receives an `InvoiceData` prop object containing all the data described in this spec.

### Preview rendering

The preview modal wraps the `InvoiceDocument` component inside a scrollable container with a `transform: scale(N)` applied to the wrapper. The zoom level is stored in state and applied as a CSS transform.

```
<div class="preview-container" style="overflow: auto;">
  <div class="preview-scaler" style="transform: scale(0.75); transform-origin: top left;">
    <InvoiceDocument data={invoiceData} />
  </div>
</div>
```

Width of the scaler container should be computed as `containerWidth = modalContentWidth / zoom` to maintain proportions.

### Shared components

Build the following shared components under `frontend/src/features/settings/components/invoice/`:

```
InvoiceDocument.tsx        # Root component — renders full document
InvoiceHeader.tsx          # Logo, title, number, dates, status badge
InvoiceContributor.tsx     # Contributor details block
InvoiceProgram.tsx         # Program/project information block
InvoiceTable.tsx           # Line item table with headers and rows
InvoiceSummary.tsx         # Gross, fees, net, outstanding
InvoiceFooter.tsx          # Payment method, tx hash, disclaimer, page number
InvoiceWatermark.tsx       # Diagonal watermark overlay for refunded/void
InvoicePreviewModal.tsx    # Modal wrapper with toolbar, zoom, download
InvoiceToolbar.tsx         # Zoom controls + download button
```

### Data model extension

The current `Invoice` interface in `frontend/src/features/settings/types/index.ts` needs extension to support the full receipt layout:

```typescript
export interface InvoiceLineItem {
  id: string;
  description: string;
  quantity: number;
  unitAmount: number;
  grossAmount: number;
  currency: string;
  platformFee?: number;
  otherDeductions?: { label: string; amount: number }[];
  netPayout: number;
}

export type InvoiceStatus = 'paid' | 'pending' | 'refunded' | 'void';

export interface Invoice {
  // existing fields
  id: string;
  invoiceNumber: string;
  date: string;
  amount: number;
  currency: string;
  status: InvoiceStatus;
  description: string;
  billingPeriod: string;
  // new fields
  paymentDate?: string;
  lineItems: InvoiceLineItem[];
  platformFee: number;
  netPayout: number;
  outstandingBalance: number;
  contributor: {
    name: string;
    email?: string;
    walletAddress: string;
    taxId?: string;
  };
  program: {
    name: string;
    projectName: string;
    milestone: string;
    periodStart: string;
    periodEnd: string;
  };
  payment: {
    method: string;
    transactionHash: string;
  };
}
```

### Future extensibility

- The `InvoiceDocument` component should accept an optional `variant` prop: `'pdf'` | `'preview'` to toggle between light-mode-only (PDF) and theme-aware (preview) colour tokens.
- Add `'payment-slip'` or `'tax-summary'` as future variant types that reuse the same block components with different headers or summary layouts.
- Page numbering should be handled by the PDF renderer's built-in pagination system. For `@react-pdf/renderer`, use the `render prop` pattern with `<Page />` wrapping.
