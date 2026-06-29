# ProfilePage Certificate Downloads & Preview Modal — Design Specification

This document details the frontend UI/UX specifications for integrating contributor certificates into `frontend/src/features/ProfilePage`.

---

## 1. ProfilePage Layout Integration

Certificates are integrated into the ProfilePage under the **Completed Programs** section. Each completed program card or table row contains specialized triggers for viewing and downloading certificates.

### Layout Placement
```
┌────────────────────────────────────────────────────────────────────────┐
│ Completed Programs & Rewards                                           │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │  Cairo Quests Protocol Development      $2,500 USD   Completed     │ │
│ │  Ecosystem: Stellar Development Fdn.    Issued: Jun 28, 2026       │ │
│ │                                                                    │ │
│ │  [ Download Certificate (Button) ]   [ Preview Certificate (Eye) ] │ │  ← Main UX Triggers
│ └────────────────────────────────────────────────────────────────────┘ │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │  Soroban SDK Optimization Program       $1,000 USD   KYC Pending   │ │
│ │  Ecosystem: Stellar Development Fdn.    Status: Verified           │ │
│ │                                                                    │ │
│ │  [ Download (Disabled) ]             [ Preview (Disabled) ]        │ │  ← Disabled State
│ └────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Interactive States & Buttons

All triggers follow Grainlify's button tokens for size, contrast, borders, and animations.

### 2.1 Download Certificate Button
- **Default State**: `bg-[#c9983a]/15 border border-[#c9983a]/30 text-[#c9983a]`. Includes an SVG download icon on the left.
- **Hover State**: `bg-[#c9983a]/25 border border-[#c9983a]/50 text-[#e8c77f]`. Transition duration: `150ms`.
- **Active State**: Scale `98%` on-click.
- **Focus State**: Outline `2px solid #f1b400`, outline offset `2px`.
- **Loading State**: Swaps the download icon with a spinning loader and shows text "Generating PDF...".

### 2.2 Preview Certificate Action
- **Default State**: Transparent button with an SVG "eye" icon. Text color: `text-[#d4c5b0]` (dark theme) / `text-[#7a6b5a]` (light theme).
- **Hover State**: `text-[#c9983a]` with a light underline decoration.
- **Focus State**: Outlined with a focus ring.

### 2.3 Disabled State (Unavailable)
- **Condition**: Certificate is unavailable if:
  1. Contributor KYC verification is incomplete.
  2. The on-chain payout transaction is pending.
- **Visuals**: `opacity-40 bg-gray-400/10 border-gray-400/20 text-gray-400 cursor-not-allowed`.
- **Interaction**: Clicking does not trigger action. Hovering displays a tooltip indicating the reason (e.g. *"Complete KYC to download certificate"* or *"Transaction processing on Stellar"*).

---

## 3. Responsive Certificate Preview Modal

The preview modal provides a high-fidelity visual check before download, along with quick actions.

### 3.1 Desktop Layout (1440px)
- **Dimensions**: Center-aligned dialog, width `1024px`, height `640px`.
- **Elevation**: Level 4 Overlay (`shadow-[0_20px_25px_-5px_rgba(0,0,0,0.5)]`).
- **Layout**: 2-Pane Split:
  - **Left Pane (65% width)**: Renders the certificate preview inside an aspect-ratio-locked container (`aspect-[1.414/1]`). The canvas is scaled down responsively (using CSS transforms or flex flex-shrink) to fit the modal height.
  - **Right Pane (35% width)**: Contains certificate metadata and actions:
    - **Header**: Certificate Title & Recipient Name.
    - **Metadata List**: Certificate ID, Issue Date, Program Name, and Stellar Transaction Hash.
    - **Actions Stack**:
      - **"Download PDF" (Primary Button)**: Gold solid gradient button.
      - **"Copy Link" (Secondary Button)**: Copies verification URL to clipboard.
      - **"Verify on Stellar" (External Link)**: Opens the ledger transaction details in a new tab with `target="_blank" rel="noopener noreferrer"`.
      - **"Close" (Top Right X)**: Compact close button.

### 3.2 Mobile Layout (375px)
- **Dimensions**: Full screen overlay (`inset-0`).
- **Layout**: Vertical stack:
  - **Header**: Accessible Title ("Certificate Preview") and close button (`44px` minimum height).
  - **Body**: The certificate canvas is rotated or scaled using CSS to fit the viewport width. Aspect ratio is locked; scroll overflow is enabled vertically.
  - **Sticky Action Bar**: Positioned at the bottom of the screen (`fixed bottom-0 left-0 right-0`). Contains a full-width gold **"Download PDF"** button.

---

## 4. Component Structure & Data Props

The implementation is broken down into a template component (for drawing the certificate) and a container component (for rendering the download experience and modal).

### Props Interface (RewardCertificateTemplate)
```typescript
interface RewardCertificateTemplateProps {
  /** Recipient name (e.g. "Amara Nwosu") */
  displayName: string;
  /** Name of the reward program */
  programName: string;
  /** Amount awarded (e.g. "$2,500 USD" or "5,000 XLM") */
  amount: string;
  /** Issue Date (ISO string or formatted date) */
  issueDate: string;
  /** Unique Certificate Hash ID */
  certId: string;
  /** On-chain Stellar Transaction Hash */
  stellarTxHash: string;
  /** Style Variant Theme */
  variant: 'gold' | 'blue' | 'silver';
  /** Optional Custom Partner/Sponsor Logo URL */
  sponsorLogoUrl?: string;
}
```
