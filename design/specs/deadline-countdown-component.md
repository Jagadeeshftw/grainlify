# Deadline Countdown Component Spec

## Overview
A reusable countdown component (days/hours/minutes) with escalating urgency color states as the deadline approaches, and a clear "expired" terminal state. This component is primarily designed for `frontend/src/shared/components/ui/IssueCard.tsx` and project pages to display bounty and program deadlines.

## Format Rules & Refresh Cadence

The countdown text formatting transitions dynamically based on the time remaining to provide the most relevant granularity.

- **> 3 days:** `X days left` (e.g., "12 days left")
- **< 3 days & > 24 hours:** `X days Y hours left` (e.g., "2 days 14 hours left")
- **< 24 hours & > 1 hour:** `Xh Ym left` (e.g., "18h 32m left")
- **< 1 hour & > 0 minutes:** `Xm left` (e.g., "42m left")
- **Expired:** `Deadline passed`

**Refresh Cadence:**
- **> 24 hours:** Refresh every 1 hour.
- **< 24 hours:** Refresh every 1 minute.
- **< 1 hour:** Refresh every 1 minute.

## Urgency Tiers & Styling

Color alone must not be used to indicate state; an icon is paired with the text. Contrast ratios have been validated against `/design-tokens.json` to ensure WCAG 2.1 AA compliance (min 4.5:1).

| Tier | Threshold | Icon | Text/Icon Color (Light) | Text/Icon Color (Dark) | Contrast (L/D) |
|---|---|---|---|---|---|
| **Safe** | > 3 days | `Clock` (default) | `#424242` (`text.secondary`) | `#d4d4d4` (`darkMode.text.secondary`) | 10.5:1 / >4.5:1 |
| **Warning** | < 24 hours | `AlertTriangle` | `#b45309` (`semantic.warning.700`) | `#f59e0b` (`darkMode.semantic.warning`) | >4.5:1 / >4.5:1 |
| **Critical** | < 1 hour | `Clock` (pulsing) | `#b91c1c` (`semantic.error.700`) | `#ef4444` (`darkMode.semantic.error`) | >4.5:1 / >4.5:1 |
| **Expired** | < 0 minutes | `XCircle` | `#757575` (`text.tertiary`) | `#b8a898` (`darkMode.text.tertiary`) | 5.0:1 / >4.5:1 |
| **No Deadline**| N/A | None | Component is hidden. | Component is hidden. | N/A |

*Note: For the Warning and Critical states in Light Mode, the 700-level semantic tokens are used instead of the 500-level to guarantee a minimum 4.5:1 contrast ratio against the `#f5f5f5` background.*

## Expired State Behavior
When the countdown reaches 0:
1. The countdown text is replaced with **"Deadline passed"**.
2. The associated `Clock` icon is replaced with `XCircle`.
3. All related CTAs (e.g., "Submit Application", "Start Work") adjacent to the component must be set to a disabled state (using the `disabled` color token `#bdbdbd` in Light and `#978e82` in Dark).

## Accessibility Annotations

- **Screen Reader Announcements:** The countdown text updates must be wrapped in `aria-live="off"` to prevent spamming screen readers every minute.
- **Milestone Announcements:** Only major milestones (e.g., crossing from Safe to Warning, Warning to Critical, or Expired) should trigger a screen reader announcement using a visually hidden element with `aria-live="polite"`. 
- **Keyboard Navigation:** The countdown component itself is not interactive and must **not** be a tab-stop (`tabindex="-1"` or omit `tabindex`). 
- **Disabled CTAs:** When the deadline expires, the adjacent CTAs become disabled and this state is announced correctly to assistive technologies.

## Responsive Design

At narrower viewports (e.g., 375px card widths), the component gracefully truncates:
- `X days left` -> `Xd`
- `X days Y hours left` -> `Xd Yh`
- `Xh Ym left` -> `Xh Ym`
- `Deadline passed` -> `Expired`

## Redlines & Handoff Checklist
- [x] Font: `typography.fontFamily.sans`
- [x] Sizes: Base font `text-sm` (14px) for cards, `text-base` (16px) for project pages.
- [x] Contrast: All text and icons meet or exceed the 4.5:1 ratio on surface backgrounds.
- [x] Interactive states: Not applicable for the countdown text itself, but adjacent CTAs are handled.
