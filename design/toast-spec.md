# Toast Notification Specification

## Overview

This document defines the Grainlify toast notification system for the frontend application. The goal is consistency, accessibility, and predictable behavior across all user-triggered and system event messages.

The implementation is centered on `frontend/src/shared/components/Toast.tsx` and uses the Sonner toast library already in the repo. The new system standardizes:

- Visual variants
- Positioning and safe-area handling
- Stacking and queue overflow
- Auto-dismiss timing and progress
- Mobile-specific behavior
- Accessibility and reduced-motion support

## Toast Variants

Each toast variant is designed for a clear intent and a distinct visual tone.

### Variants

- `success`
  - Positive confirmation for completed user actions
  - Example: "Profile updated successfully"
- `error`
  - Urgent failures that require user attention
  - Example: "Failed to update avatar. Please try again."
- `warning`
  - Soft caution for recoverable issues or important reminders
  - Example: "Your subscription expires in 3 days"
- `info`
  - Neutral updates and non-critical system messages
  - Example: "New feature available in beta"
- `loading`
  - In-progress operations that should remain visible until completion
  - Example: "Uploading file..."
- `action`
  - Notifications that include a CTA button
  - Example: `toast.success('File saved', { action: { label: 'Undo', onClick: ... } })`

### Visual states

Each variant includes support for:

- default
- hover
- focus
- active
- disabled (where applicable for action buttons)
- loading
- error
- empty / zero data state (via content, not toast styling)

### Design tokens

New toast tokens have been added to `design-tokens.json` for:

- `toast.maxVisible`
- `toast.position.desktop`
- `toast.position.mobile`
- `toast.offset`
- `toast.borderRadius`
- `toast.shadow`
- `toast.duration`

Semantic token support was expanded to include `info` and `loading`.

## Positioning

### Desktop

- `bottom-right`
- offset from edges using safe-area insets
- consistent with persistent status notifications on large screens

### Mobile

- `bottom-center`
- centered beneath the viewport edge
- width limited to `min(92vw, 24rem)` for comfortable reading
- safe-area bottom inset applied for notched devices

### Implementation

`frontend/src/shared/components/Toast.tsx` now uses:

- `position="bottom-right"`
- responsive wrapper CSS for mobile center alignment
- `containerAriaLabel="Application notifications"`
- `closeButtonAriaLabel="Dismiss notification"`

## Stacking and queue behavior

- Maximum of **3 visible** toasts at a time
- Overflow is queued by Sonner automatically
- A collapse badge is rendered when additional toasts are waiting:
  - `+N` indicator in the toast overlay
  - badge is non-interactive and visually distinct

## Timing and progress

- Default duration: **5000ms**
- Recommended error duration: **8000ms**
- Action/confirmation toast duration: **0** when the user must interact with a CTA
- Progress bar styling is supported through Sonner's loader class

## Accessibility

- Non-urgent toasts should use `toast.success`, `toast.info`, `toast.warning`, or plain `toast()`
- Error toasts should use `toast.error(...)` to ensure stronger semantic urgency
- `containerAriaLabel="Application notifications"` is applied
- `closeButtonAriaLabel="Dismiss notification"` is applied
- Reduced motion is honored via CSS when `prefers-reduced-motion: reduce`

## Responsive breakpoints

Tested layout expectations:

- `sm` 640px — mobile and small-tablet toast centering
- `md` 768px — transition point to desktop aligned to bottom-right
- `lg` 1024px — desktop layout
- `xl` 1280px — desktop layout

## Trigger sites

Current toast trigger sites discovered in the frontend include:

- `frontend/src/features/admin/pages/AdminPage.tsx`
- `frontend/src/features/settings/components/profile/ProfileTab.tsx`

These pages use `toast.success(...)` and `toast.error(...)`. The new sequence should be used consistently:

- success for positive completion
- error for failed conditions
- info for neutral system updates
- warning for recoverable or cautionary conditions
- loading for async operations that remain in progress
- action for confirmable interactions with a CTA button

## Quality assurance checklist

- [ ] Contrast ratios meet WCAG 2.1 AA: text >= 4.5:1, UI >= 3:1
- [ ] Keyboard-only dismissal and CTA focus styles are visible
- [ ] Responsive layout verifies bottom-center on small screens and bottom-right on desktop
- [ ] Long text wraps cleanly and does not overflow the toast container
- [ ] Empty/zero state toasts are not shown; use page-level empty state UI instead
- [ ] Error and success toasts display consistent variant styling
- [ ] Reduced motion disables transition animation

## Implementation notes

- The toast wrapper is mounted in `frontend/src/app/App.tsx` via `<Toast />`
- Design tokens are kept in `design-tokens.json` for reuse across UI specs
- The toast component is intentionally unstyled by Sonner and styled via Tailwind utility classes

## Future handoff

For Figma/spec handoff, export:

- One toast sheet for each variant
- A mobile vs desktop viewport state
- A stacked toast set showing 3 visible toasts plus a queued badge
- Hover/focus/active examples for CTA buttons and close icons

The current implementation is ready for review and can be validated with design QA across the listed breakpoints.