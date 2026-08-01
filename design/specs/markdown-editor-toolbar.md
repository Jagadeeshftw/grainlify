# Markdown Editor Toolbar Spec

## Overview
This specification details the UI/UX design for a compact markdown formatting toolbar to be used across comment and description inputs in Grainlify.

## Target Inputs
- `frontend/src/features/dashboard/pages/IssueDetailPage.tsx`
- `frontend/src/features/maintainers/components/issues/IssueCard.tsx`

## Design System Validation (from `/design-tokens.json`)
- **Icon Colors**: 
  - Light mode: `neutral.600` (`#57534e`) against `neutral.50` (`#fafaf9`) background (Contrast > 4.5:1).
  - Dark mode: `text.secondary` (`#d4d4d4`) against `background.surfacePrimary` (`#1a1714`) or `background.surfaceSecondary` (`#2d2820`) (Contrast > 4.5:1).
- **Divider Colors**:
  - Light mode: `neutral.200` (`#e7e5e4`).
  - Dark mode: `border.subtle` (`rgba(255, 255, 255, 0.08)`).
- **Focus Rings**:
  - Light mode focus ring color: `interactive.focus` (`#0066cc`).
  - Dark mode focus ring color: `interactive.focusRing` (`#f1b400`).

## Button Grid and Grouping

Icons are sourced from `lucide-react`.

### Group 1: Text Style
| Action | Icon (`lucide-react`) | Keyboard Shortcut | Shortcut Hint Tooltip |
|--------|-----------------------|-------------------|-----------------------|
| Bold   | `Bold`                | Cmd/Ctrl + B      | Bold (Cmd+B)          |
| Italic | `Italic`              | Cmd/Ctrl + I      | Italic (Cmd+I)        |

*(Divider)*

### Group 2: Insert
| Action | Icon (`lucide-react`) | Keyboard Shortcut | Shortcut Hint Tooltip |
|--------|-----------------------|-------------------|-----------------------|
| Code   | `Code`                | Cmd/Ctrl + E      | Code (Cmd+E)          |
| Link   | `Link`                | Cmd/Ctrl + K      | Link (Cmd+K)          |

*(Divider)*

### Group 3: Lists & Quotes
| Action | Icon (`lucide-react`) | Keyboard Shortcut | Shortcut Hint Tooltip |
|--------|-----------------------|-------------------|-----------------------|
| List   | `List`                | Cmd/Ctrl + Shift + 8 | Bulleted List (Cmd+Shift+8) |
| Quote  | `Quote`               | Cmd/Ctrl + Shift + 9 | Blockquote (Cmd+Shift+9) |

## States
- **Default**: Icon color per theme (e.g., `text.secondary` in dark mode). Transparent background.
- **Hover**: Background changes to `interactive.hover` (`rgba(255, 255, 255, 0.10)` in dark mode).
- **Button-Active**: Button is highlighted (e.g., cursor is inside bold text). Background changes to `interactive.active` (`rgba(255, 255, 255, 0.15)` in dark mode), or text color shifts to `text.primary`. `aria-pressed="true"`.
- **Disabled**: E.g., no text selection for the link button. Icon opacity reduced, color shifts to `text.disabled` (`#6b5d4d` in dark mode). `aria-disabled="true"`.
- **Mobile-Overflow-Open**: The "More" overflow menu is toggled open, displaying collapsed buttons in a dropdown.

## Mobile Condensed Row (Below 480px)
At screen widths below 480px (e.g. 375px responsive review):
- The toolbar collapses to save horizontal space.
- **Visible Buttons**: Bold, Italic, Link.
- **Collapsed Buttons**: Code, List, Quote.
- **More Button**: A generic `MoreHorizontal` (or `MoreVertical`) icon button appears at the end of the row. Tapping it opens a dropdown/popover menu containing the collapsed actions.

## Accessibility Annotations
- **Toolbar Role**: The container uses `role="toolbar"`.
- **Labels**: Every button has a descriptive `aria-label` (e.g., `aria-label="Bold"`).
- **State Properties**: Toggle buttons (like Bold/Italic) use `aria-pressed="true"` when active and `aria-pressed="false"` when inactive.
- **Keyboard Navigation**:
  - Implements **roving tabindex** for the toolbar items. 
  - Only one button is in the document tab sequence (`tabindex="0"`), while the rest are `tabindex="-1"`.
  - Arrow keys (Left/Right) move focus between buttons within the toolbar.
- **Shortcuts**: Shortcuts must fire correctly when the cursor is focused inside the associated `<textarea>`.

## QA Requirements
- **Contrast**: Toolbar icon contrast against input background must meet WCAG 2.1 AA 4.5:1 in both light and dark themes.
- **Keyboard-only**: Users must be able to Tab into the toolbar, use Left/Right arrow keys to navigate (roving tabindex), activate buttons with Space/Enter, and fire shortcuts from the textarea.
- **Responsive**: Ensure the overflow menu correctly houses Code, List, and Quote buttons at 375px.
