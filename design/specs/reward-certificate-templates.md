# Contributor Reward Certificate Templates — Design Spec

**Surface coverage:** ProfilePage (Certificate Downloads) · Verification Portal  
**File location:** `design/specs/reward-certificate-templates.md`  
**Branch:** `design/reward-certificate-pdf-templates`  
**Status:** 📝 Under Design QA / Reference Spec  

---

## Table of Contents

1. [Overview & Goals](#1-overview--goals)
2. [Layout & Dimensions (A4 Print Specs)](#2-layout--dimensions-a4-print-specs)
3. [Design Tokens & Variants](#3-design-tokens--variants)
   - 3.1 Hackathon (Gold)
   - 3.2 Scholarship (Blue)
   - 3.3 Bounty (Silver)
4. [Certificate Anatomy](#4-certificate-anatomy)
5. [Print Specifications & CSS Media Queries](#5-print-specifications--css-media-queries)
6. [QR Code Verification Workflow](#6-qr-code-verification-workflow)
7. [Accessibility Guidance (WCAG 2.1 AA)](#7-accessibility-guidance-wcag-21-aa)
8. [Design QA Checklist](#8-design-qa-checklist)
9. [Key Assumptions](#9-key-assumptions)

---

## 1. Overview & Goals

Grainlify rewards outstanding contributions to open-source programs (Hackathons, Bounty Programs, and Scholarships) with official certificates of achievement. These certificates act as verified professional credentials, linked to on-chain Stellar transactions, and shareable on platforms like LinkedIn.

### Goals
- **Print Quality**: Designed for high-fidelity A4 physical printing (300 DPI) and high-resolution PDF download.
- **Verification Integrity**: Contains a verification QR code and a Stellar transaction hash linked directly to the Soroban contract escrow payout.
- **Aesthetic Excellence**: Follows Grainlify's dark glassmorphism styling, with dynamic typography, border framing, and harmonized color palettes.
- **Accessibility**: Passes WCAG 2.1 AA standards for color contrast, screen reader labels, and keyboard usability in the download interface.

---

## 2. Layout & Dimensions (A4 Print Specs)

Certificates are designed in A4 landscape format. 

| Metric | Millimeters | Pixels at 300 DPI | Aspect Ratio |
|--------|-------------|-------------------|--------------|
| **Width** | 297 mm | 3508 px | 1.414 : 1 (A4) |
| **Height** | 210 mm | 2480 px | 1.414 : 1 (A4) |
| **Safe Margins** | 20 mm | 236 px | Padding inset |
| **QR Code Box** | 45 × 45 mm | 530 × 530 px | 1 : 1 (Square) |

### Grid Layout Structure
```
┌────────────────────────────────────────────────────────────────────────┐
│ Margin: 20mm (236px) safe zone                                         │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │  grainlify [Logo]                         SPONSOR: [Stellar SDF]   │ │  ← Brand Header
│ │                                                                    │ │
│ │                      CERTIFICATE OF ACHIEVEMENT                    │ │  ← Subheading
│ │                                                                    │ │
│ │                              Awarded to                            │ │
│ │                             [Amara Nwosu]                          │ │  ← Contributor Name (Bold, 64px)
│ │                                                                    │ │
│ │      For outstanding contributions to the                          │ │
│ │      [Cairo Quests Protocol Development Program]                   │ │  ← Program Name
│ │                                                                    │ │
│ │      Awarded Amount: [ $2,500 USD ]    Issue Date: [June 28, 2026] │ │  ← Meta Stats
│ │                                                                    │ │
│ │      ┌──────────┐  ID: [CERT-HK-2026-0628]                         │ │
│ │      │          │  TX: [0x8b65...2efd]                              │ │  ← Verification Block
│ │      │ QR CODE  │                                  [ Signature ]   │ │
│ │      │          │                                  CEO, Grainlify  │ │  ← Signatures
│ │      └──────────┘                                                  │ │
│ └────────────────────────────────────────────────────────────────────┘ │
│  Gold/Blue/Silver Rule border                                          │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Design Tokens & Variants

All variants utilize Grainlify's background texture and fonts, with distinct variant themes defining the borders, badges, signatures, and highlight elements.

### 3.1 Hackathon (Gold)
Designed to represent ultimate competitive achievement. 
- **Accent Color (Primary Gold)**: `#f1b400` (Token primary-500)
- **Border Accent (Dark Gold)**: `#c9983a` (Token primary-600)
- **Background Gradient**: 
  - Radial-burst: `rgba(201, 152, 58, 0.15)` to transparent.
  - Base: `#0C0E14` (Deep Canvas).
- **Secondary Accent**: `#fae199` (Token primary-200) for text highlight.
- **Visual Vibe**: Bright gold double-ruled border framing, metallic gradient badges.

### 3.2 Scholarship (Blue)
Designed to represent educational excellence and ecosystem grants.
- **Accent Color (Semantic Info)**: `#3b82f6` (Token info-500)
- **Border Accent (Deep Blue)**: `#1d4ed8` (Token info-700)
- **Background Gradient**:
  - Radial-burst: `rgba(59, 130, 246, 0.15)` to transparent.
  - Base: `#0C0E14` (Deep Canvas).
- **Secondary Accent**: `#eff6ff` (Token info-50) for text highlight.
- **Visual Vibe**: Royal/deep blue framing, corporate trust styling.

### 3.3 Bounty (Silver)
Designed to represent verified technical task completions.
- **Accent Color (Neutral Silver)**: `#a8a29e` (Token neutral-400)
- **Border Accent (Dark Silver)**: `#57534e` (Token neutral-600)
- **Background Gradient**:
  - Radial-burst: `rgba(168, 162, 158, 0.15)` to transparent.
  - Base: `#0C0E14` (Deep Canvas).
- **Secondary Accent**: `#fafaf9` (Token neutral-50) for text highlight.
- **Visual Vibe**: Clean metallic borders, geometric industrial pattern details.

---

## 4. Certificate Anatomy

Each certificate variant contains the following structural components:

### 4.1 Header Branding
- **Logo**: Vector SVG of the Grainlify emblem. Sized at `48px` height.
- **Wordmark**: `grainlify` in lowercase, rendered in the variant accent color.
- **Sponsor Logo**: Sized at `48px` height maximum. Set on the top right, representing the funding entity (e.g. *Stellar Development Foundation*).

### 4.2 Achievement Statement
- **Heading**: "CERTIFICATE OF ACHIEVEMENT" (Inter Bold, 32px / 2.0rem, tracking `0.2em`, variant accent color).
- **Preposition**: "Awarded to" (Inter Medium, Italic, 18px / 1.1rem, `rgba(242,242,242,0.60)`).
- **Recipient**: Full Legal/Github Name (Inter ExtraBold, 64px / 4.0rem, `#F2F2F2`).
- **Description**: Centered text block stating the achievement. Maximum 150 characters.

### 4.3 Program Details
- **Program Name**: Rendered in a highlighted text block (Inter SemiBold, 22px / 1.4rem, variant accent color).
- **Award Metric**: "Awarded Amount" (Inter Regular, 14px, muted) paired with a high-contrast dollar/token display (e.g., `$5,000 USD` / `15,000 XLM`).

### 4.4 Verification Footer Block
- **QR Code Container**: Left-aligned, `150px` square on-screen, mapping to `45mm` in physical prints. Includes a `4px` solid border using the variant border token.
- **Metadata Text**:
  - **Certificate ID**: Unique alphanumeric code `CERT-{SURFACE}-{YEAR}-{HEX}` (JetBrains Mono, 14px, `#F2F2F2`).
  - **Stellar Transaction Hash**: First 10 and last 10 characters visible (e.g. `GBAB...2EFD`). Copy action available on hover in preview modes.
- **Digital Signatures**: 
  - CEO / Board Member signatures as high-resolution transparent PNG/SVGs.
  - Title text: `CEO, Grainlify` and `Program Director` (Inter Regular, 14px, muted).

---

## 5. Print Specifications & CSS Media Queries

To ensure the web layout converts perfectly to PDF, specialized styling rules apply to print contexts.

### 5.1 CSS Stylesheet Directives
```css
@media print {
  /* Set exact page size and orientation */
  @page {
    size: A4 landscape;
    margin: 0; /* Let print wrapper handle margins */
  }

  /* Force background colors and colors to render */
  body {
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
    background-color: #0C0E14 !important;
  }

  /* Hide screen-only elements (buttons, modals, headers) */
  .no-print, 
  .modal-actions, 
  .close-button {
    display: none !important;
  }

  /* Scale the certificate container to full print page */
  .certificate-print-wrapper {
    position: absolute;
    left: 0;
    top: 0;
    width: 297mm;
    height: 210mm;
    page-break-after: avoid;
    page-break-inside: avoid;
  }
}
```

### 5.2 Quality and Safe Zones
- **CMYK Conversion**: Ensure colors translate well (Gold maps to standard `#C9983A`, Blue maps to `#1D4ED8`).
- **Vector Assets**: All icons, QR codes, logos, and signature marks must be rendered as SVGs (or 300 DPI high-resolution PNGs at minimum) to avoid blurring when printed.

---

## 6. QR Code Verification Workflow

Each QR code contains a verified routing link which checks the on-chain authenticity of the award:

```
[ QR Code Scanned ] ──► [ URL: https://grainlify.io/verify/{certId} ]
                                       │
                                       ▼
                         [ Verification Landing Page ]
                                       │
            ┌──────────────────────────┴──────────────────────────┐
            ▼                                                     ▼
 [ Read Metadata from DB ]                               [ Lookup on Stellar ]
 - Recipient: Amara Nwosu                                - Tx Hash: 0x8b65...2efd
 - Amount: $2,500                                        - Ledger Sequence: #23481
 - Timestamp: 2026-06-28                                 - Status: SUCCESS
```

### QR Code Parameters
- **Data encoding**: URL format.
- **Error Correction**: Level Q (25% restoration rate). This ensures the QR code remains fully scannable even if the certificate is folded, scratched, or signed over.
- **Contrast**: Pure black modules (`#000000`) on a solid white square (`#FFFFFF`) background. Never use transparent backgrounds for QR codes to guarantee print readability.

---

## 7. Accessibility Guidance (WCAG 2.1 AA)

All certificate components and the download interface are designed to support accessible operation.

### 7.1 WCAG Color Contrast Matrix
Each variant's text combinations must pass WCAG AA:

| Element | Background | Text Color | Ratio | AA Pass |
|---------|------------|------------|-------|---------|
| **Gold Text** | `#0C0E14` (Deep Canvas) | `#f1b400` | 6.2:1 | Yes ✅ |
| **Blue Text** | `#0C0E14` (Deep Canvas) | `#3b82f6` | 4.8:1 | Yes ✅ |
| **Silver Text** | `#0C0E14` (Deep Canvas) | `#a8a29e` | 4.6:1 | Yes ✅ |
| **Primary White Text** | `#0C0E14` (Deep Canvas) | `#F2F2F2` | 15.2:1 | Yes ✅ |
| **Muted Labels** | `#0C0E14` (Deep Canvas) | `rgba(242,242,242,0.60)` | 6.8:1 | Yes ✅ |

### 7.2 Keyboard Navigation Sequence (Modal)
When the Preview modal is opened:
1. Focus is trapped inside the modal.
2. Initial focus rests on the **Close Preview** button.
3. Users tab sequentially: `Close Preview` ──► `Certificate Preview (Reads Alt)` ──► `Download PDF` ──► `Copy Verification Link` ──► `Verify on Stellar`.
4. Escape key closes the modal instantly and restores focus to the invoking button.

### 7.3 Screen Reader (ARIA) Labels
- **Trigger Button**: `aria-label="Download Hackathon Certificate for Cairo Quests"`
- **Preview Trigger**: `aria-label="Preview Certificate for Cairo Quests"`
- **QR Code Image**: `alt="QR Code verification link for certificate CERT-HK-2026-0628"`
- **Certificate Layout**: `role="img"` with `aria-label="Certificate of Achievement awarded to Amara Nwosu for the Cairo Quests program, amount $2,500 USD, issued June 28, 2026"`

---

## 8. Design QA Checklist

Before publishing, verify the layout against this checklist:

### 8.1 Resolution & Quality
- [ ] Certificate template dimensions match `3508px × 2480px` (A4 at 300 DPI).
- [ ] QR code is rendered in vector SVG (or high-res 1200px PNG) and measures at least `45mm` in physical prints.
- [ ] Background noise/grain texture does not obscure text blocks.

### 8.2 Variant Specifics
- [ ] Gold variant uses Primary accent colors (`#f1b400`).
- [ ] Blue variant uses Info accent colors (`#3b82f6`).
- [ ] Silver variant uses Neutral/Silver accent colors (`#a8a29e`).
- [ ] Sponsor logo appears in correct dimensions on top-right, aligned with the header baseline.

### 8.3 Interaction & Access
- [ ] Trigger buttons display loading spinner during generation state.
- [ ] Disabled state displays descriptive tooltip (e.g. "KYC Verification required to unlock certificates").
- [ ] ESC key closes preview modal.
- [ ] Copied link triggers an accessible toast notification ("Verification link copied to clipboard").

---

## 9. Key Assumptions

1. **Client-side PDF Library availability**: Assumes browser-level PDF generation is handled using standard SVG conversion and canvas printing, with an A4 print layout stylesheet standard.
2. **On-chain Metadata**: Assumes the Stellar transaction hash is generated and verified before the certificate becomes active and downloadable.
3. **Verification Page**: Assumes the routing URL structure `/verify/{certId}` is registered on the landing page router.
