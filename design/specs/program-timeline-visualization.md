# Design Specification: Program Timeline Milestone Visualization

This document details the UI/UX, motion behavior, accessibility features, and technical specifications for the visual release schedule timeline component (`ProjectReleaseTimeline`) in the `ProjectDetailPage`.

---

## 1. Overview
The visual milestone timeline displays the distribution track for bounty and program payouts based on the `ProgramReleaseSchedule` contract structure from `contracts/program-escrow/src/lib.rs`. It replaces a plain text list of milestones with an interactive, responsive track.

### Data Shape Integration
The timeline maps directly to the following Soroban smart contract structure:
```rust
pub struct ProgramReleaseSchedule {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_timestamp: u64,
    pub released: bool,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}
```

---

## 2. Visual Layout & Mockups

### Milestone Node States
Milestones can exist in one of four states, derived from contract properties relative to the current timestamp (`currentTime`):

| State | Color Token (Dark) | Color Token (Light) | Logic Criteria |
|-------|--------------------|---------------------|----------------|
| **Released** | `#c9983a` (Gold) | `#b8872f` (Ochre) | `released === true` |
| **Skipped** | `#ef4444` (Red) | `#dc2626` (Crimson) | `released === false && currentTime >= release_timestamp && exists(M_later.released === true)` |
| **Unlocked** | `#22c55e` (Green) | `#16a34a` (Forest) | `released === false && currentTime >= release_timestamp && !exists(M_later.released === true)` |
| **Upcoming** | `#b8a898` (Muted) | `#7a6b5a` (Warm Gray) | `released === false && currentTime < release_timestamp` |

---

### Desktop Layout Mockup (1440px Horizontal Track)

Horizontal scrolling container (`overflow-x-auto`) with a solid connecting line running through the center of interactive milestone node buttons.

```text
+--------------------------------------------------------------------------------------------------------+
|  ✦ Milestone Release Schedule                                                                           |
|                                                                                                        |
|  ===================================(●)===================(●)===================(●)==================  |
|                                  Milestone 1           Milestone 2           Milestone 3               |
|                                    1,000 USDC            2,500 USDC            5,000 USDC              |
|                                    [Released]             [Skipped]            [Unlocked]              |
|                                                                                                        |
|                                                  Active Popover (Node 2)                               |
|                                                  +----------------------------------+                  |
|                                                  | status: [ Skipped ]              |                  |
|                                                  | amount: 2,500.00 USDC [CoinIcon] |                  |
|                                                  | release: 2026-06-15 14:00        |                  |
|                                                  | explorer: [View Transaction ↗]   |                  |
|                                                  +----------------------------------+                  |
+--------------------------------------------------------------------------------------------------------+
```

---

### Mobile Layout Mockup (375px Vertical Track)

Vertical layout (`flex-col`) with a vertical connector line on the left. Popovers render inline right below/beside their respective node elements.

```text
+---------------------------------------+
|  ✦ Milestone Release Schedule         |
|                                       |
|  |                                    |
|  +---(●) Milestone 1: Released        |
|  |      Amount: 1,000 USDC            |
|  |                                    |
|  |                                    |
|  +---(●) Milestone 2: Skipped         |
|  |   |  [Popover Opened Inline]       |
|  |   |  +--------------------------+  |
|  |   |  | status: [ Skipped ]      |  |
|  |   |  | amount: 2,500 USDC       |  |
|  |   |  | release: 2026-06-15      |  |
|  |   |  | explorer: [View Tx ↗]    |  |
|  |   |  +--------------------------+  |
|  |                                    |
|  +---(●) Milestone 3: Unlocked        |
|  |      Amount: 5,000 USDC            |
|  |                                    |
|  +---(○) Milestone 4: Upcoming        |
|  |      Amount: 2,000 USDC            |
|  V                                    |
+---------------------------------------+
```

---

## 3. Node Popover Specification

The popover provides contextual information for a milestone when clicked or focused.

1. **Amount & Token Icon**: Displays amount divided by `10^7` (to account for Stellar's 7 decimal places) and appends the token code (e.g. `USDC` / `XLM`). Renders a token coin icon.
2. **Ledger Date**: Renders the human-readable UTC date calculated from `release_timestamp` (in seconds). Format: `YYYY-MM-DD HH:mm:ss UTC`.
3. **Stellar Explorer Link**:
   - If a transaction hash is available (or using the `released_by` address as a fallback), renders a link pointing to:
     - `https://stellar.expert/explorer/testnet/tx/{tx_hash}` or `https://stellar.expert/explorer/testnet/account/{recipient}`
4. **Status Badge**: Pill-shaped badge styled according to the active milestone state (colors correspond to node state colors).

---

## 4. Accessibility and Keyboard Focus

### ARIA Roles and Attributes
- **Timeline Container**: `role="list"`, `aria-label="Milestone Release Schedule Timeline"`
- **Timeline Node**: `role="listitem"`, `aria-label="Milestone {id} - {status} - {amount} {token}"`
- **Interactive Node Buttons**: `<button>` element with `aria-expanded={isOpen}`, `aria-haspopup="dialog"`, and `aria-controls={`popover-${id}`}`.
- **Popover Dialog**: `role="dialog"`, `aria-modal="false"`, `aria-label="Milestone Details"`

### Keyboard Focus Management
- Nodes are fully tabbable (`tabIndex={0}`).
- **Left / Right arrow key navigation** in desktop view moves focus to the previous/next milestone node.
- **Up / Down arrow key navigation** in mobile view moves focus between milestone nodes.
- **Escape Key**: Closes any active popover and returns focus to the corresponding milestone node button.
- Focus is trapped within the active popover using a keyboard trap helper or standard element navigation if interactive links (like Explorer link) are present.

### Color Contrast Auditing (WCAG AA Compliance)
- Nodes and text colors must satisfy at least **4.5:1** contrast ratio against the parent page background:
  - Dark theme background: `#1a1612` (Ochre-tinted dark)
  - Light theme background: `#fdfcfb` (Warm cream)
  - Released Node Text (`#c9983a` Dark theme vs background): Contrast >= 4.6
  - Unlocked Node Text (`#22c55e` Dark theme vs background): Contrast >= 4.8
  - Skipped Node Text (`#ef4444` Dark theme vs background): Contrast >= 5.1
  - Upcoming Node Text (`#b8a898` Dark theme vs background): Contrast >= 4.5

---

## 5. Motion and Animation Specifications

### Scroll-Into-View Reveal
Timeline nodes reveal using a staggered horizontal/vertical sweep as they scroll into view.
- **Library**: `motion` (Framer Motion v12)
- **Viewport Trigger**: `whileInView` with `viewport={{ once: true, margin: "-100px" }}`
- **Transition Duration**: `motionConfig.durations.normal` (300ms) per item.
- **Stagger Delay**: 75ms delay between consecutive nodes.

### Reduced-Motion Fallback
When user preference indicates `prefers-reduced-motion: reduce`:
- Set all animation durations to `0`.
- All states enter instantly (`opacity: 1`, `scale: 1`, `y: 0`, `x: 0`).
- Fallback logic checks standard `window.matchMedia('(prefers-reduced-motion: reduce)')` matches.

---

## 6. Security Invariants
- **XSS Prevention**: Recipient addresses, transaction links, and amounts are strictly sanitized and processed as plain text strings.
- **Safe External Navigation**: Explorer links are generated with `rel="noopener noreferrer"` and `target="_blank"` to protect user identity and context.
- **Mathematical Invariants**: Token division logic uses `BigInt` or safe precision division to prevent overflows/floating-point rounding issues when formatting standard i128 values.
