# Modal Interaction Specification

## Overview

This specification describes the accessibility-focused interaction model for `frontend/src/shared/components/ui/Modal.tsx` and all modal usage patterns across the frontend.

Goals:
- Define focus entry, trap cycling, and return-focus behavior
- Specify scroll lock behavior and inner content scrolling
- Standardize modal stacking and backdrop layering
- Document responsive and WCAG 2.1 AA-compliant modal states
- Capture all current modal usage sites for consistent review

## Current modal usage sites

The shared modal component is used in:
- `frontend/src/app/pages/DashboardComplete.tsx`
- `frontend/src/features/dashboard/pages/EcosystemsPage.tsx`
- `frontend/src/features/admin/pages/AdminPage.tsx`
- `frontend/src/features/maintainers/components/issues/IssuesTab.tsx`

Additional modal-like overlay patterns exist in:
- `frontend/src/features/maintainers/components/AddRepositoryModal.tsx`
- `frontend/src/features/maintainers/components/NewProjectSetupModal.tsx`
- `frontend/src/features/maintainers/components/InstallGitHubAppModal.tsx`

These pages should follow the same interaction model for focus, keyboard dismissal, scroll lock, and nested overlay behavior.

## Accessibility interaction model

### Dialog semantics
- `role="dialog"`
- `aria-modal="true"`
- `aria-labelledby` pointing to the modal title
- `aria-describedby` can be added for longer content descriptions or error details

### Focus behavior
- Focus entry: first focusable element inside modal content
- If no focusable elements exist, the modal container itself receives focus
- Focus trap: `Tab` and `Shift+Tab` cycle through modal focusable elements only
- Escape closes the modal and returns focus to the previously focused element
- Keyboard-only users can access the close button and any modal actions

### Backdrop and dismissal
- Backdrop click closes the modal unless the click is inside modal content
- Escape key closes the modal
- The close button always remains visible and keyboard-accessible

### Scroll lock and content scrolling
- Body scroll is disabled while any modal is open via `body.modal-open`
- Inner modal content remains vertically scrollable
- Use visible scrollbar styling (`scrollbar-custom`) for clarity
- Modal content should not exceed `90vh` when `maxHeight=true`

## Stacking and nested modal rules

- Base z-index: `10000`
- Each nested modal increases stack depth by `20`
- Backdrop opacity increases for nested modals to preserve visual depth:
  - base overlay opacity: `0.50`
  - nested increment: `0.04`
  - capped at `0.62`
- Inner modal content always appears above its backdrop layer
- When nested modals are open, the body scroll lock remains enabled until all modals are closed

## Design states

Each modal supports the following states:
- default
- hover (primary and secondary buttons, close button)
- focus (buttons and input controls)
- active (pressed button state)
- disabled (button disabled state)
- loading-within (async state inside modal content)
- error state (form validation or server error state)
- empty content (modal should still display title and action area)

## Responsive behavior

Breakpoint expectations:
- `sm` (640px): modal width uses `95vw` and remains centered
- `md` (768px): modal scales to the defined width class and remains centered
- `lg` (1024px): modal uses `max-w-[90vw]` and flex layout for desktop
- `xl` (1280px): modal remains centered and constrained by `max-w-[650px]`

## Design tokens and implementation

New modal tokens were added to `design-tokens.json`:
- `modal.zIndexBase`
- `modal.overlayOpacity.base`
- `modal.overlayOpacity.nestedIncrement`
- `modal.overlayOpacity.max`
- `modal.borderRadius`
- `modal.shadow`
- `modal.maxHeight`
- `modal.focusRing`
- `modal.animationDuration`

These tokens ensure the modal system can be reviewed and reused consistently.

## Implementation notes

- The shared modal has been updated for explicit dialog semantics and focus management
- The body lock is controlled with `body.modal-open`
- Nested modals now track stack order and increase overlay opacity accordingly
- Accessibility improvements include keyboard trap support and focus restoration
- Modal content is scrollable, with the page background locked while modal is open

## QA checklist

- [ ] Verify focus enters the modal when opened and returns to the trigger on close
- [ ] Confirm `Tab` and `Shift+Tab` stay within the modal
- [ ] Confirm `Escape` closes the modal
- [ ] Confirm backdrop click closes the modal
- [ ] Confirm body cannot scroll while the modal is open
- [ ] Confirm the modal content scrolls independently when long
- [ ] Confirm nested modal backdrop opacity and stacking behavior
- [ ] Confirm color contrast meets WCAG 2.1 AA
- [ ] Confirm modal works on `sm`, `md`, `lg`, and `xl` breakpoints
- [ ] Confirm keyboard-only walkthrough includes close button and action buttons

## Hand-off and review

For design handoff, provide:
- modal open/close and overlay examples
- focus trap cycles with keyboard flow
- nested modal stacking examples
- mobile and desktop responsive designs
- accessible roles and dismissal behavior
- error, loading, and empty-content states
