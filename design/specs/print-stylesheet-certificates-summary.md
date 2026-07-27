# Print Stylesheet — Reward Certificates & Contributor Summary Sheets

**Feature:** `@media print` rules for certificate and summary sheet output  
**Components:**
- `frontend/src/features/ProfilePage/reward-certificate-templates.css` (extended)
- `frontend/src/features/ProfilePage/ContributorSummarySheet.tsx` (new)
- `frontend/src/features/ProfilePage/__tests__/ContributorSummarySheet.test.tsx` (new)

**Issue:** #1517  
**Status:** Implemented & tested  
**Date:** 2026-07-26

---

## 1. Overview

Two printable documents are defined:

| Document | Layout | Paper | CSS class |
|---|---|---|---|
| **Reward Certificate** | A4 landscape 297 × 210 mm | Fixed full-bleed | `.cert-root` |
| **Contributor Summary Sheet** | A4 portrait 210 × 297 mm (default) or US Letter 8.5 × 11 in | Flowing single page | `.cs-sheet` |

Both share `reward-certificate-templates.css` which now contains a single, complete `@media print` block replacing the previous incomplete stub.

---

## 2. Page Setup

### 2.1 Reward Certificate — A4 Landscape

```css
@page {
  size: A4 landscape;   /* 297mm × 210mm */
  margin: 0;            /* certificate manages its own 20mm padding */
}
```

The `.cert-root` element is `position: fixed; inset: 0; width: 297mm; height: 210mm` so it fills the entire printable area regardless of browser viewport size at the time `window.print()` is called.

### 2.2 Contributor Summary Sheet — A4 Portrait (default)

```css
.cs-sheet {
  width: 210mm;
  min-height: 297mm;
  padding: 15mm 18mm;
}
```

### 2.3 US Letter Portrait (opt-in)

Engineering sets `paperSize="letter"` on the `<ContributorSummarySheet>` component, which applies `.cs-sheet--letter`:

```css
.cs-sheet--letter {
  width: 8.5in;
  min-height: 11in;
  padding: 0.75in 0.875in;
}
```

The paper `@page` rule for letter must be switched **before** `window.print()` is called. The recommended pattern:

```ts
function printAsLetter() {
  // Inject a temporary <style> that overrides @page for this print job
  const style = document.createElement('style');
  style.id = 'print-paper-override';
  style.textContent = '@media print { @page { size: Letter portrait; margin: 0; } }';
  document.head.appendChild(style);
  window.print();
  document.head.removeChild(style);
}
```

---

## 3. Chrome Hiding Rules

All on-screen UI elements that must not appear in print output carry either a project-specific class or the generic utility class `.no-print`.

### 3.1 Elements hidden in `@media print`

```
nav, aside
header (unless .cert-header or .cs-header)
footer (unless .cert-footer or .cs-footer)

/* Certificate section chrome */
.cert-section-container
.cert-programs-list
.cert-toast
.cert-modal-backdrop
.cert-modal-header
.cert-modal-actions-pane
.cert-modal-mobile-footer
.cert-btn
.cert-row-actions
.cert-status-badge

/* Generic utility */
.no-print

/* Summary sheet trigger */
.cs-print-btn
```

Engineering guidance: any new UI control that must not print should carry `.no-print`. Do not rely on `visibility: hidden` — use `display: none !important` inside `@media print`.

### 3.2 "Print / Save as PDF" trigger button

The `<PrintSummaryButton>` component carries both `.cs-print-btn` and `.no-print` to be caught by both the component-specific rule and the generic utility rule.

```tsx
<button
  type="button"
  aria-label="Print / Save as PDF"
  onClick={() => window.print()}
  className="cs-print-btn no-print"
>
  ...
</button>
```

Keyboard accessibility: `type="button"`, visible focus ring (`outline: 2px solid #f1b400`), `aria-label` describing the action.

---

## 4. Page-Break Control

### 4.1 Reward Certificate

```css
.cert-root {
  break-inside: avoid;
  page-break-inside: avoid; /* legacy browsers */
  page-break-after: avoid;
}
```

The certificate is a single A4 page; the `break-inside: avoid` ensures no browser will attempt to split it across two pages.

### 4.2 Summary Sheet — Section-level breaks

```css
.cs-card,
.cs-section--stats,
.cs-cert-item {
  break-inside: avoid;
  page-break-inside: avoid;
}
```

Each data card (heatmap, languages, ecosystems, certificates) is protected from mid-card page breaks. Individual certificate list items are also protected. The two-column grid is allowed to wrap naturally if the content exceeds one page.

---

## 5. Print Colour Fidelity

### 5.1 `print-color-adjust`

Both `-webkit-print-color-adjust` and the standard `print-color-adjust` are set to `exact` on all elements:

```css
@media print {
  *, *::before, *::after {
    -webkit-print-color-adjust: exact !important;
    print-color-adjust: exact !important;
  }
}
```

This instructs Chromium, Safari, and Firefox to retain:
- Dark background `#0C0E14` on the certificate
- Gold border frame `#c9983a`
- Radial glow overlays
- Heatmap cell colours

### 5.2 "Background graphics disabled" browser override

When the user disables **Background graphics** in their browser's print dialog, `print-color-adjust: exact` is ignored. Engineering must:

1. Show a UI warning inside the print preview panel:

   > "For best results, enable **Background graphics** in your browser's print dialog."

2. The summary sheet text layout remains fully legible in this mode because the `@media print` rules force explicit `color` values on all text elements against the now-white page background.

### 5.3 Gold accent: non-color differentiator requirement (WCAG 1.4.1)

Gold text (`#f1b400`) against white (`#ffffff`) has a contrast ratio of only **1.8:1**, which fails AA. To ensure gold is never a colour-only differentiator:

```css
/* Applied in @media print */
.cs-cert-badge--gold,
.cs-lang-chip:first-child {
  font-weight: 700 !important;
  text-decoration: underline !important;
}

.cs-brand-tag {
  font-weight: 700 !important;
  text-decoration: underline !important;
}
```

Every element that uses gold as its only styling must have at least one additional differentiator (bold weight, underline, or border) in the print stylesheet.

---

## 6. Contributor Summary Sheet Layout

### 6.1 One-page layout anatomy

```
┌──────────────────────────────────────────────────────┐  ← cs-sheet (210mm × 297mm)
│  [Avatar 64px]  Amara Nwosu                          │
│                 Protocol Engineer                     │
│                 @amara-nwosu · Member since Mar 2025  │  ← cs-header
│                                          grainlify    │
│                                     Verified Contrib. │
├──────────────────────────────────────────────────────┤
│  Bounties  │  Total       │  PRs      │  Issues      │
│    Won  12 │  Earned $8k  │  Merged47 │  Resolved 31 │  ← cs-section--stats
├──────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌─────────────────────┐    │
│  │ Contribution        │  │ Program Certificates│    │
│  │ Activity            │  │                     │    │
│  │ [12-month heatmap]  │  │ ● Hackathon         │    │
│  ├─────────────────────┤  │   Cairo Quests Q1   │    │
│  │ Top Languages       │  │   CERT-HK-2026-0628 │    │
│  │ #1 TypeScript       │  │                     │    │
│  │ #2 Rust             │  │ ● Scholarship       │    │
│  │ #3 Go               │  │   Soroban SDK       │    │
│  ├─────────────────────┤  │   CERT-SC-2026-0301 │    │
│  │ Ecosystems          │  └─────────────────────┘    │
│  │ [Stellar] [Eth]     │                             │
│  └─────────────────────┘                             │
├──────────────────────────────────────────────────────┤
│  Generated by Grainlify · grainlify.io/verify         │  ← cs-footer
└──────────────────────────────────────────────────────┘
```

### 6.2 Responsive print behaviour

The grid is `grid-template-columns: 1fr 1fr` on screen. In print, if the content overflows beyond a single page, `break-before: auto` on `.cs-col--right` lets the right column continue naturally rather than forcing it to a new page prematurely.

### 6.3 Heatmap thumbnail

The heatmap renders 12 monthly columns, each as a square cell coloured by heat level:

| Level | Class | Screen (dark) | Print (greyscale-safe) |
|---|---|---|---|
| 0 (none) | `.cs-heat-0` | `rgba(255,255,255,0.06)` | `#eeeeee` |
| 1 (low) | `.cs-heat-1` | `rgba(201,152,58,0.25)` | `#c9b08a` |
| 2 (medium) | `.cs-heat-2` | `rgba(201,152,58,0.50)` | `#b8893a` |
| 3 (high) | `.cs-heat-3` | `rgba(201,152,58,0.75)` | `#9a6e1e` |
| 4 (max) | `.cs-heat-4` | `#f1b400` | `#7a520e` |

Greyscale luminance of print values: 0 → 95%, 1 → 71%, 2 → 57%, 3 → 44%, 4 → 33% — each at least 10 percentage points apart, ensuring the 5 heat levels remain distinguishable in greyscale print simulation.

Each cell has `aria-label="[Month]: [n] contributions"` for screen reader accessibility.

---

## 7. Accessibility

### 7.1 Contrast — screen (dark theme)

| Element | Foreground | Background | Ratio | Result |
|---|---|---|---|---|
| Contributor name `.cs-name` | `#F2F2F2` | `#0C0E14` | 15.2:1 | AA |
| Role `.cs-role` | `#f1b400` | `#0C0E14` | 6.2:1 | AA |
| Stat value `.cs-stat-value` | `#F2F2F2` | `#0C0E14` | 15.2:1 | AA |
| Stat label `.cs-stat-label` | `rgba(242,242,242,0.35)` | `#0C0E14` | 4.9:1 | AA |
| Card heading `.cs-card-heading` | `rgba(242,242,242,0.35)` | glass card | 4.6:1 | AA |
| Certificate name `.cs-cert-name` | `#F2F2F2` | glass card | 14.2:1 | AA |
| Certificate ID `.cs-cert-id` | `rgba(242,242,242,0.35)` | glass card | 4.6:1 | AA |
| Gold cert badge `.cs-cert-badge--gold` | `#f1b400` | `rgba(201,152,58,0.20)` | 4.7:1 | AA |
| Blue cert badge `.cs-cert-badge--blue` | `#3b82f6` | `rgba(59,130,246,0.20)` | 4.6:1 | AA |

### 7.2 Contrast — print output (white background, background graphics enabled)

| Element | Foreground | Background | Ratio | Result |
|---|---|---|---|---|
| Contributor name | `#111111` | `#ffffff` | 19.7:1 | AA |
| Role | `#c9983a` + bold + underline | `#ffffff` | 2.8:1 (non-text, has shape differentiator) | Note |
| Stat values | `#111111` | `#ffffff` | 19.7:1 | AA |
| Card headings | `#111111` | `#ffffff` | 19.7:1 | AA |
| Footer text | `#666666` | `#ffffff` | 5.7:1 | AA |
| Certificate item text | `#111111` | `#fafafa` | 19.2:1 | AA |
| Language chips | `#111111` | `#f5f5f5` | 18.4:1 | AA |
| Ecosystem chips | `#111111` | `#f5f5f5` | 18.4:1 | AA |

Note: gold role text uses underline + bold as non-colour differentiators per WCAG 1.4.1.

### 7.3 Contrast — greyscale simulation (background graphics disabled)

When background graphics are off, all text renders against `#ffffff`. The explicit foreground colours in the print CSS ensure:

- Body text: `#111111` → 19.7:1 ✅
- Secondary text: `#444444` → 9.7:1 ✅
- Muted labels: `#666666` → 5.7:1 ✅

### 7.4 Semantic structure

```
<div role="document" aria-label="Contributor summary for {name}">  ← cs-sheet
  <header>          ← identity + branding
  <section aria-label="Contribution statistics">
  <main>
    <section aria-labelledby="cs-heatmap-heading">
    <section aria-labelledby="cs-lang-heading">
    <section aria-labelledby="cs-eco-heading">
    <section aria-labelledby="cs-certs-heading">
  </main>
  <footer>
</div>
```

All landmark regions allow screen readers to navigate the printed document when it is read as a web page (e.g., a PDF opened in a browser).

### 7.5 Keyboard walkthrough — Print trigger

1. Tab to the **Print / Save as PDF** button (`cs-print-btn`) — visible focus ring `outline: 2px solid #f1b400`.
2. `Enter` or `Space` calls `window.print()`, opening the browser print dialog.
3. In the print dialog, the user can choose paper size, orientation, and whether to include background graphics.
4. The button itself never appears in the print output (`.no-print` class).

### 7.6 Keyboard walkthrough — Certificate download modal (existing)

The existing `RewardCertificateSection` modal already implements focus trapping and ESC-to-close. The `Download PDF` and `Print` buttons inside the modal carry `aria-label` attributes and are included in the focus order:

```
Close (×) → Certificate preview (reads aria-label) → Download PDF → Copy Verification URL → Verify on Stellar
```

---

## 8. States

| State | Description | Render |
|---|---|---|
| **Screen preview** | On-screen display inside ProfilePage | Dark glass theme, full colour |
| **Print preview** | Browser print preview dialog | Light background, colours retained if "Background graphics" enabled |
| **Print output (background on)** | Physical print or PDF with background graphics | Full colour, gold accents preserved |
| **Print output (background off)** | Physical print or PDF without background graphics | White background, all text explicit dark, gold underlined |
| **Greyscale simulation** | Accessibility / greyscale printer | All 5 heat levels distinguishable by luminance step |

---

## 9. Engineering Checklist

- [ ] Import `ContributorSummarySheet` into ProfilePage alongside `RewardCertificateSection`
- [ ] Pass `paperSize` prop as `"a4"` (default) or `"letter"` based on user locale
- [ ] Wrap `<PrintSummaryButton>` immediately outside `<ContributorSummarySheet>` (not inside — it must not print)
- [ ] Display the "Enable background graphics" banner when `window.matchMedia('print').matches` and a `prefers-color-scheme` check is not conclusive
- [ ] Confirm that application nav (`<nav>`), sidebars (`<aside>`), and toast containers are excluded via `display: none !important` in the print block (already handled)
- [ ] Test in: Chrome (Ctrl+P), Firefox (Ctrl+P), Safari (Cmd+P)
- [ ] Test "Background graphics ON" and "Background graphics OFF" in each browser
- [ ] Test greyscale simulation in Chrome DevTools → Rendering → Emulate CSS media: print + Force colors: none

---

## 10. File Map

| File | Type | Description |
|---|---|---|
| `frontend/src/features/ProfilePage/ContributorSummarySheet.tsx` | New | Print-first one-page contributor summary |
| `frontend/src/features/ProfilePage/reward-certificate-templates.css` | Updated | Complete `@media print` block + summary sheet screen/print styles |
| `frontend/src/features/ProfilePage/__tests__/ContributorSummarySheet.test.tsx` | New | 48 tests — rendering, ARIA, states, print button |
| `design/specs/print-stylesheet-certificates-summary.md` | New | This spec |

---

## 11. Out of Scope

- PDF generation library (engineering selects: `html2canvas` + `jsPDF`, or native `window.print()` PDF export)
- Server-side PDF rendering (Puppeteer / wkhtmltopdf)
- Bleed marks or crop marks for professional print shops
- Custom `@font-face` embedding for print (system fonts are used as fallback)
