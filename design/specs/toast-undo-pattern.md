# Toast Undo Pattern Specification

## Overview

This document extends the [Toast Notification Specification](../toast-spec.md) to define a standard pattern for reversible destructive actions (e.g., removing a wallet card, deleting a draft). 
This introduces a 5-second undo-window toast variant that reverses the action if the user acts in time, and finalizes it otherwise.

## Undo-Toast Anatomy

The undo toast is a specialized version of the `action` variant:

- **Action Confirmation Copy**: Clear, concise text describing the action that just occurred (e.g., "Draft deleted").
- **"Undo" Button**: A primary call-to-action (CTA) button within the toast.
- **Visual Countdown**: A progress bar (shrinking bar) at the bottom of the toast, indicating the remaining time in the 5-second window. The progress bar styling leverages Sonner's loader class, matching existing toast token aesthetics.
- **Stacking & Positioning**: Follows the existing `toast-spec.md` conventions (Desktop: `bottom-right`, Mobile: `bottom-center`, max 3 visible toasts).

## Interaction States & Behavior

### 1. Undo-Active (Countdown Running)
- **Duration**: 5000ms.
- **Visuals**: The countdown bar shrinks from 100% to 0%.
- **Action Pending**: The destructive action is held in a pending/optimistic state on the client.

### 2. Undo-Triggered (Restored)
- **Trigger**: User clicks the "Undo" button or presses `Enter`/`Space` while it is focused.
- **Result**: The destructive action is reversed.
- **Feedback**: The undo toast is immediately replaced with a standard `success` toast (e.g., "Restored") that auto-dismisses after the default duration.

### 3. Countdown-Expired (Finalized)
- **Trigger**: The 5000ms duration elapses without interaction.
- **Result**: The destructive action is finalized and sent to the server.
- **Feedback**: The toast dismisses smoothly. No further feedback is required.

### 4. Stacked with Another Undo Toast
- **Behavior**: If a user performs multiple destructive actions in rapid succession, the undo toasts **stack** (up to the maximum of 3 visible toasts) rather than replace each other.
- **Reasoning**: Each destructive action is independent and must have its own distinct 5-second reversal window.

## Accessibility Annotations

- **Role**: The undo toast container must use `role="alert"` (with `aria-live="assertive"`) to ensure screen readers immediately announce the destructive action and the availability of the undo option.
- **Focus & Keyboard Navigation**: The "Undo" button must be reachable via `Tab` immediately after the toast appears, before the auto-dismiss timer expires. Focus should pause the countdown if possible (following the `pauseOnHover` token pattern).
- **Time Conveyance**: The remaining time must not be conveyed by color or animation alone. The toast must ensure the 5-second window is sufficient for interaction, and the screen reader announcement must clearly imply the temporary nature of the undo action.
- **Reduced Motion**: If `prefers-reduced-motion: reduce` is active, the shrinking progress bar animation is disabled and replaced with a static indicator or opacity transition.
- **Contrast**: The undo toast text and CTA button contrast must meet the WCAG 2.1 AA requirement of >= 4.5:1 against the toast surface in both light and dark themes.

## Validation against Design Tokens

This pattern utilizes existing tokens from `design-tokens.json`:
- `toast.duration`: Overridden locally for the undo pattern to strictly 5000ms.
- `toast.position.desktop` and `toast.position.mobile`: Inherited.
- `semantic.warning` or `semantic.info`: Can be used for the base background or border depending on the severity of the action, matching the `action` toast variant.
- `accessibility.screenReaderSupport.ariaLive.assertive`: Applied for urgent updates.
