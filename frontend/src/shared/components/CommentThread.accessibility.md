# CommentThread Accessibility Documentation

## Overview

The `CommentThread` component provides a fully accessible comment interface with keyboard navigation, screen reader support, and proper ARIA attributes. This document outlines the accessibility features and expected behavior.

## Keyboard Navigation

### Reply Composer
- **Escape**: Closes the reply composer without submitting
- **Ctrl/Cmd + Enter**: Submits the reply
- **Tab**: Navigates between form elements
- **Shift + Tab**: Navigates backwards through form elements
- **Auto-focus**: Textarea is automatically focused when reply composer opens

### Edit Mode
- **Escape**: Cancels edit mode and reverts to original content
- **Tab**: Navigates between textarea and action buttons
- **Shift + Tab**: Navigates backwards through form elements

### Reaction Picker
- **Arrow Left/Right**: Navigates between reaction options in the listbox
- **Enter/Space**: Selects the currently focused reaction
- **Escape**: Closes the reaction picker without selection
- **Tab**: Moves focus out of the picker (closes it)
- **Focus management**: Focus moves to selected option when using arrow keys

### Collapsed Replies
- **Enter/Space**: Expands collapsed replies
- **Tab**: Moves focus to next interactive element

## Screen Reader Behavior

### Announcements
- **Comment count**: Total number of comments is announced at the thread level
- **Reply count**: Collapsed replies button announces the number of hidden replies (e.g., "View 4 more replies")
- **Edit context**: Edit textarea announces which comment is being edited (e.g., "Edit comment by alice")
- **Reply context**: Reply composer announces the target author (e.g., "Write a reply to alice")
- **Action buttons**: All buttons have descriptive aria-labels (e.g., "Reply to alice", "Edit comment", "Delete comment")
- **Cancel buttons**: Cancel buttons in edit/delete modes have clear labels (e.g., "Cancel editing comment", "Cancel delete comment")
- **Submit buttons**: Submit buttons announce their state (e.g., "Post reply", "Posting reply", "Save comment edit", "Saving comment")

### ARIA Attributes

#### CommentThread Component
- `aria-label="Comments"`: Main section label
- `role="list"`: Top-level comment list
- `aria-label="Comment thread"`: Comment list label

#### Replies List
- `aria-label="Replies to {username}"`: Identifies which comment replies belong to
- `aria-expanded`: Indicates whether replies are collapsed or expanded
- `role="list"`: Semantic list structure

#### Collapsed Replies Button
- `aria-expanded="false"`: Indicates collapsed state
- `aria-label="View {count} more repl{ies/y}"`: Describes action and count

#### CommentCard Component
- Avatar button: `aria-label="{username}'s avatar"`
- Reply button: `aria-label="Reply to {username}"`
- Edit button: `aria-label="Edit comment"`
- Delete button: `aria-label="Delete comment"`
- Edit textarea: `aria-label="Edit comment by {username}"`

#### ReplyComposer Component
- `role="form"`: Semantic form structure
- `aria-label="Reply to {username}"`: Form context
- Textarea: `aria-label="Write a reply to {username}"`
- Cancel button: `aria-label="Cancel reply"`
- Submit button: `aria-label="Post reply"` or `aria-label="Posting reply"` (when submitting)

#### ReactionBar Component
- `role="group"`: Groups related reaction buttons
- `aria-label="Reactions"`: Group label
- Reaction buttons: `aria-pressed` indicates user's reaction state
- Reaction buttons: `aria-label` includes emoji name, count, and toggle instruction
- Add reaction button: `aria-haspopup="listbox"` and `aria-expanded` indicates picker state
- Reaction picker: `role="listbox"` with `role="option"` for each emoji
- Reaction options: `aria-selected` indicates focused/selected state

## Focus Management

### Focus Indicators
- All interactive elements have `focus-visible:outline-none` and `focus-visible:ring-1` classes
- Custom focus ring color (`#c9983a`) for consistent visibility
- Delete confirmation uses red focus ring (`focus-visible:ring-red-500`) for destructive actions

### Focus Traps
- Reaction picker: Focus is contained within the picker when open
- Escape key closes picker and returns focus to trigger button
- Blur outside picker closes it automatically

### Focus Movement
- Reply composer: Auto-focuses textarea when opened
- Reaction picker: Arrow keys move focus between options
- Edit mode: Focus moves to textarea when edit is activated

## Dynamic Content

### State Changes
- **Reply expansion**: `aria-expanded` updates from `false` to `true`
- **Reaction picker**: `aria-expanded` updates when opened/closed
- **Edit mode**: Textarea appears/disappears with proper ARIA labels
- **Delete confirmation**: Confirmation buttons appear with descriptive labels
- **Loading states**: Button text changes (e.g., "Posting...", "Saving...") with updated aria-labels

### Live Regions
- Currently, the component does not use live regions for dynamic content updates
- Future enhancement: Consider `aria-live` for reply submission confirmation

## Testing

The accessibility behavior is covered by comprehensive tests in `CommentThread.test.tsx`:

### Keyboard Navigation Tests
- Escape key to close edit mode
- Escape key to cancel reply composer
- Ctrl/Cmd+Enter to submit reply

### ARIA Attribute Tests
- `aria-expanded` on collapsed replies button
- `aria-expanded` on replies list
- `aria-label` on all action buttons
- `aria-label` on cancel buttons in edit mode
- `aria-label` on cancel buttons in delete confirmation
- `aria-label` on reply composer buttons
- `aria-haspopup` and `aria-expanded` on reaction picker

### Screen Reader Behavior Tests
- Reply count announcement in collapsed state
- Edit textarea context announcement
- Reply composer context announcement

### Focus Management Tests
- Auto-focus of reply composer textarea
- Focus-visible styles on interactive elements

## Browser Compatibility

The accessibility features use standard ARIA attributes and keyboard events supported by:
- Modern browsers (Chrome, Firefox, Safari, Edge)
- Screen readers (NVDA, JAWS, VoiceOver, TalkBack)
- Keyboard navigation across all platforms

## Known Limitations

1. **No live regions**: Dynamic content changes are not announced via aria-live
2. **No focus trap in edit mode**: Focus can leave edit mode without explicit action
3. **No skip links**: No mechanism to skip to main content or skip navigation
4. **Reaction picker**: Focus management relies on React state, may have edge cases with rapid keyboard navigation

## Future Enhancements

1. Add `aria-live` regions for reply submission confirmations
2. Implement focus trap for edit mode
3. Add skip links for keyboard users
4. Consider adding `aria-describedby` for additional context
5. Implement proper focus restoration after closing dialogs/pickers
