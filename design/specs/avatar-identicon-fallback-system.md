# Avatar and Identicon Fallback System Spec

## Overview
This document specifies the design for a deterministic identicon fallback system for user avatars across Grainlify. It ensures that users without a profile photo or those experiencing image load failures receive a consistent, accessible, and aesthetically pleasing fallback (gradient + initials) based on their user ID or address.

## States
The avatar component supports the following states:
1. **loading-shimmer**: A slow-load placeholder displaying a shimmer effect before the real image or fallback resolves.
2. **image-loaded**: The user's provided avatar URL successfully loads and renders.
3. **image-error (fallback shown)**: The provided avatar URL fails to load, triggering the deterministic fallback.
4. **fallback-generated**: The user has no avatar URL provided, triggering the deterministic fallback immediately.

## Deterministic Gradient Generation
To ensure the same user always renders the same fallback, the background gradient is generated deterministically from the user's ID or wallet address.

### Hash-to-Gradient Rule
- **Input**: User ID or Wallet Address (string).
- **Hashing**: Use a simple string hashing function (e.g., DJB2 or a fast hash) to generate a numeric value.
- **Palette Family**: The gradient must use hue pairs exclusively from the Grainlify **gold/warm-neutral** palette family defined in `/design-tokens.json` to maintain stellar alignment and brand credibility.
  - *Primary (Gold)*: `#f1b400` (500) to `#3a2710` (950)
  - *Neutral (Warm Neutral)*: `#78716c` (500) to `#0c0a09` (950)
- **Gradient Construction**: 
  - Hash modulo determines the start and end color stops.
  - Angle is also determined by modulo (e.g., `hash % 360` degrees).
  - *Contrast Rule*: The generated gradient hues must stay within a contrast-safe range to ensure the text (initials) overlay maintains a minimum of 4.5:1 WCAG AA contrast ratio. Since we use dark/warm tones, initials should generally be white (`#ffffff`) or a very light neutral (e.g., Neutral 50 `#fafaf9`).

## Initials Placement
- **Content**: 1 to 2 letters extracted from the user's display name or username. If not available, use the first 2 characters of their address.
- **Styling**: Centered vertically and horizontally within the avatar container.
- **Typography**: Uses the primary sans-serif font, semi-bold to bold weight, scaled proportionally to the avatar size.

## Size Variants
The system supports the following standard sizes used across `IssueCard.tsx`, `UserProfileDropdown.tsx`, and comment threads:

| Size | Usage Context | Initials Font Size |
|---|---|---|
| **16px** | Inline mentions, dense tables | 8px (or hidden if illegible at 375px viewport) |
| **24px** | IssueCard assignees | 10px |
| **32px** | Standard comment threads | 14px |
| **40px** | UserProfileDropdown trigger | 16px |
| **64px** | User Profile header | 24px |

*Note on Responsive Review*: The smallest 16px variant must remain legible at a 375px viewport width. If 1-2 letters cannot be legibly rendered at 16px, the initials may be omitted, leaving only the deterministic gradient to convey identity.

## Accessibility (a11y) Annotations
- **WCAG 2.1 AA Compliance**: All text (initials) over gradients must meet the 4.5:1 contrast requirement.
- **Alt Text**: The avatar `<img>` or fallback container must always have its `aria-label` or `alt` text set to the user's username/display name.
- **Decorative Elements**: The background gradient is purely decorative. When initials convey identity and the container has an appropriate `aria-label`, the internal gradient and initials elements should be marked `aria-hidden="true"` to prevent redundant screen reader announcements.
- **Keyboard Navigation**: When the avatar acts as an interactive link (e.g., navigating to a user's profile), it must be fully keyboard reachable (`tabindex="0"`) and properly labeled for screen readers. Focus rings must be visible according to `design-tokens.json` focus outline styles.

## Quality Assurance & Testing Requirements
1. **Design QA**: Verify that initials text meets a 4.5:1 contrast ratio against *every* possible generated gradient combination. Spot-check a representative hue sample from the hash algorithm.
2. **Keyboard-only Walkthrough**: Confirm that any avatar functioning as a link is reachable via `Tab` and properly reads out the user's name.
3. **Responsive Review**: Confirm the 16px variant remains legible on small screens (e.g., 375px width).
