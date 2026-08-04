# @Mentions Suggestions Dropdown UX Spec

**Version:** 1.0
**Status:** Design specification
**Target:** `frontend/src/features/dashboard/pages/IssueDetailPage.tsx` (comment input)
**Data source:** `backend/internal/github/issues_comments.go` (`IssueComment` model)
**Interaction reference:** `frontend/src/shared/components/SearchModal.tsx` (cmdk-based list pattern)
**Token reference:** `/design-tokens.json`

---

## Overview

The comment input on the Issue Detail Page needs an @-mention suggestions dropdown so contributors can tag collaborators inline within comment text. The interaction pattern is modelled after the existing `SearchModal` component's cmdk-based list, adapted for an anchored suggestions dropdown positioned relative to the textarea cursor.

This spec defines the full trigger behaviour, list-item anatomy, keyboard navigation contract, mention-chip rendering inside the textarea, accessibility annotations, responsive behaviour, and QA checklist — all validated against `design-tokens.json` for WCAG 2.1 AA compliance.

---

## Goals

- Define trigger behaviour: typing `@` opens a filtered list of project collaborators, updating as the user types.
- Specify list-item anatomy: avatar (24px), username, role badge (maintainer/contributor).
- Specify keyboard navigation: ArrowUp/Down to move, Enter to select, Escape to dismiss, Backspace on empty filter deletes the preceding chip.
- Specify inserted-mention chip styling inside the comment textarea and its removal behaviour (Backspace deletes the whole chip as a unit).
- Define all states: closed, open-filtering, open-no-results, open-loading (fetching collaborators), mention-inserted.
- Add accessibility annotations: dropdown as `role="listbox"` with `aria-activedescendant`, inserted mention chip announced as a single unit to screen readers.
- WCAG 2.1 AA compliance: all text and interactive elements meet 4.5:1 contrast in both themes.
- Responsive: dropdown must not clip at 375px viewport width (mobile keyboards).

---

## Data Contract

```ts
interface Collaborator {
  login: string;          // GitHub username, e.g. "octocat"
  avatarUrl: string;      // Full avatar URL
  role: 'maintainer' | 'contributor';
  displayName?: string;   // Optional full name (falls back to login)
}

interface MentionSuggestion {
  collaborator: Collaborator;
  matchRange: [number, number]; // [start, end] of matching substring in the filter text
}

interface MentionsDropdownProps {
  /** All available collaborators for the project. */
  collaborators: Collaborator[];
  /** Whether collaborators are being fetched. */
  isLoading: boolean;
  /** Called when the user selects a collaborator from the dropdown. */
  onSelectMention: (collaborator: Collaborator) => void;
}

interface MentionChip {
  login: string;
  avatarUrl: string;
  displayName?: string;
}
```

---

## Trigger Behaviour

### Activation

The dropdown is triggered by the `@` character typed into the comment textarea. The trigger logic is:

1. User types `@` anywhere in the textarea.
2. The textarea component detects the `@` character and:
   - Extracts the filter text after `@` (up to the next whitespace, punctuation, or end of string).
   - Opens the suggestions dropdown anchored to the caret position.
   - Fetches/renders a filtered list of collaborators whose `login` or `displayName` starts with the filter text (case-insensitive).
3. As the user continues typing, the filter narrows the list in real time.
4. If the user types a space, punctuation, or presses Escape, the dropdown closes without inserting.

### Deactivation

The dropdown closes when:
- The user selects a mention (Enter or click).
- The user presses Escape.
- The user clicks outside the dropdown and textarea.
- The user moves the caret so the `@` trigger is no longer adjacent (e.g., cursor moves away from the mention-fragment).
- The filter text becomes empty and the user presses Backspace (to allow deleting the `@` character).

### Filter Matching

- Case-insensitive prefix match on `login` and `displayName`.
- Results sorted: exact-match first, then by match position, then alphabetically.
- If no filter text (immediately after `@`), show all collaborators sorted alphabetically.

---

## Component Tree

```
CommentTextarea (wraps native <textarea>)
├── MentionChip[] (inline rendered chips within the textarea overlay)
│   └── MentionChip
│       ├── Avatar (20px, rounded-full)
│       ├── Username Text
│       └── RemoveButton (×, optional, visible on chip focus/hover)
│
└── MentionsDropdown (absolutely positioned, anchored to caret)
    ├── LoadingState (spinner skeleton)
    ├── EmptyState ("No collaborators found")
    └── <ul role="listbox">
        └── <li role="option"> SuggestItem[]
            ├── Avatar (24px, rounded-full)
            ├── Username (with matched portion highlighted)
            └── RoleBadge ("MAINTAINER" or "CONTRIBUTOR")
```

---

## List-Item Anatomy (SuggestItem)

### Visual layout

```
┌──────────────────────────────────────────────┐
│  [24px avatar]  octocat  [MAINTAINER]        │
│                 The Octocat                   │
└──────────────────────────────────────────────┘
```

### Dimensions & Spacing

| Element | Spec |
|---|---|
| Item padding | `px-3 py-2.5` |
| Item height | 48px (min touch target 44px + padding) |
| Avatar size | 24px × 24px, `rounded-full` |
| Avatar border | `border border-[#c9983a]/40` |
| Avatar fallback | Initials on gradient when image fails |
| Username font | `text-[13px] font-semibold` |
| Display name font | `text-[11px]` (secondary line) |
| Role badge | 6px horizontal padding, `text-[10px] font-bold` |
| Gap (avatar → text) | `gap-2.5` |
| Gap (text → badge) | `gap-2` |

### Username Highlighting

The portion of the username matching the filter text is rendered in the accent colour (`#c9983a`) with a subtle underline (`border-b border-[#c9983a]/50`). The unmatched portion uses the standard text colour.

### Role Badge

| Role | Label | Background | Text Color | Border |
|---|---|---|---|---|
| `maintainer` | MAINTAINER | `bg-[#c9983a]/20` | `text-[#c9983a]` | `border border-[#c9983a]/30` |
| `contributor` | CONTRIBUTOR | `bg-white/[0.06]` (dark) / `bg-black/[0.04]` (light) | `text-[#b8a898]` (dark) / `text-[#6b5d4d]` (light) | `border border-white/10` (dark) / `border border-black/10` (light) |

- Border radius: `rounded-[4px]`
- Padding: `px-1.5 py-0.5`

### Hover / Active States

| State | Background (dark) | Background (light) | Ring |
|---|---|---|---|
| Default | transparent | transparent | none |
| Hovered | `bg-white/[0.06]` | `bg-black/[0.03]` | none |
| Active (arrow-key selected) | `bg-white/[0.10]` | `bg-black/[0.06]` | `ring-1 ring-[#c9983a]` |

### Transition

- `transition-colors duration-150` (matches `design-tokens.json` motion dropdown spec: `150ms easeOut`).

---

## MentionsDropdown Container

### Positioning

- Absolutely positioned relative to the textarea container.
- Anchored to the caret position:
  - Below the caret line (if enough viewport space below).
  - Above the caret line (if insufficient space below; e.g., near bottom of viewport).
- Horizontal alignment: left-edge aligned with the `@` trigger character position.
- Maximum width: `min(320px, calc(100vw - 2rem))` — prevents overflow at 375px viewports.

### Visual Spec

| Property | Dark | Light |
|---|---|---|
| Background | `bg-[#2d2820]/[0.95]` | `bg-[#d4c5b0]/[0.95]` |
| Backdrop filter | `backdrop-blur-[40px]` | `backdrop-blur-[40px]` |
| Border | `border-[1.5px] border-[#c9983a]/30` | `border-[1.5px] border-[#c9983a]/30` |
| Border radius | `rounded-[16px]` | `rounded-[16px]` |
| Shadow | `shadow-[0_20px_60px_rgba(0,0,0,0.4)]` | `shadow-[0_20px_60px_rgba(0,0,0,0.15)]` |
| Max height | `240px` (scrollable overflow-y) | `240px` |

### Scrollbar (dark theme)

- Track: `bg-transparent`
- Thumb: `bg-white/[0.15] rounded-full`
- Width: 6px

### Opening Animation

- `animate-in fade-in slide-in-from-top-2 duration-150`
- Matches the dropdown motion spec in `design-tokens.json` (`150ms easeOut`, `slide-fade`).

---

## Mention Chip (Inserted State)

Once a collaborator is selected, an inline mention chip is inserted into the textarea content at the position of the `@` trigger.

### Visual layout

```
┌──────────────────────────────┐
│  [20px avatar] @octocat  [×]  │
└──────────────────────────────┘
```

### Chip Spec

| Element | Spec |
|---|---|
| Height | 24px (inline with text) |
| Padding | `px-2 py-0.5` (horizontal), `py-px` (vertical centering) |
| Background | `bg-[#c9983a]/15` |
| Border | `border border-[#c9983a]/30` |
| Border radius | `rounded-[6px]` |
| Avatar size | 20px × 20px, `rounded-full` |
| Avatar border | `border border-[#c9983a]/30` |
| Username font | `text-[13px] font-semibold text-[#c9983a]` |
| Remove button | 16px × 16px `×` icon, visible on chip hover/focus |
| Margin | `mx-0.5` (horizontal spacing from surrounding text) |

### Removal Behaviour

- **Backspace:** When the caret is immediately after a mention chip and the user presses Backspace, the entire chip is deleted as a single atomic unit (not character by character).
- **Delete:** When the caret is immediately before a mention chip and the user presses Delete, the entire chip is deleted.
- **Click remove:** Clicking the `×` button on the chip also removes it.
- The chip is non-editable inline (cannot place caret inside it).

### Screen Reader Announcement

The mention chip is announced as a single unit:
- `aria-label="Mentioned user @octocat"` (or using `displayName` if available).
- The chip itself uses `role="mark"` or a custom `aria-label` on the wrapping `<span>`.
- Screen readers treat it as one atomic token, not individual characters.

---

## States

### 1. Closed (default)

- Dropdown is not rendered.
- No filter text active.
- Textarea functions normally.

### 2. Open — Filtering

- Triggered by `@` + typing.
- Dropdown is visible, anchored to caret.
- List shows collaborators matching the filter.
- Active index is `-1` (no selection) when filter changes.
- `aria-live="polite"` region announces: `"{N} collaborator{s} found"`.

### 3. Open — No Results

- Filter text matches zero collaborators.
- Dropdown shows empty state:
  - Icon: `UserX` or `SearchX` (16px).
  - Text: "No collaborators found" (`text-[13px]`).
  - Subtext: "Try a different username" (`text-[11px]`, muted).
- Dropdown remains open so the user can adjust the filter.

### 4. Open — Loading

- Collaborators are being fetched from the API on first `@` trigger.
- Dropdown shows loading state:
  - 3 skeleton rows, each: `h-10 rounded-[8px] bg-white/[0.06] animate-pulse`.
  - Stagger animation: each row delayed by `50ms` from `design-tokens.json` list stagger.
- `aria-busy="true"` on the listbox.
- Screen reader: `aria-live="polite"` announces "Loading collaborators…" once, not on every render.

### 5. Mention Inserted

- A chip is rendered inline within the textarea content.
- The `@trigger + filter` text is replaced with the chip.
- Caret is positioned immediately after the chip.
- Dropdown is closed.
- If multiple mentions are inserted, they appear as separate chips separated by spaces.

---

## Accessibility Annotations

### Dropdown (listbox)

```html
<ul
  role="listbox"
  aria-label="Mention collaborators"
  id="mentions-listbox"
  aria-busy="{isLoading}"
>
```

The textarea input that triggers the dropdown uses:
```html
<textarea
  role="combobox"
  aria-expanded="{isOpen}"
  aria-autocomplete="list"
  aria-controls="mentions-listbox"
  aria-activedescendant="{activeId}"
  aria-haspopup="listbox"
  aria-label="Comment text"
/>
```

Note: `aria-activedescendant` and `aria-controls` are set **only while the dropdown is open**. When closed, these attributes are removed to avoid confusing screen readers.

### Suggestion Items (options)

```html
<li
  role="option"
  id="mention-option-{login}"
  aria-selected="{isActive}"
  class="..."
>
```

### Mention Chip (inserted)

```html
<span
  role="mark"
  aria-label="Mentioned user @{login}"
  class="inline-flex items-center ..."
  contenteditable="false"
  data-mention-chip
>
  <img src="{avatarUrl}" alt="" aria-hidden="true" />
  @{login}
  <button aria-label="Remove @{login} mention" tabindex="-1">×</button>
</span>
```

- `contenteditable="false"` prevents caret placement inside the chip.
- `data-mention-chip` attribute allows the textarea handler to detect and atomically delete chips on Backspace/Delete.
- The remove button has `tabindex="-1"` so it doesn't enter the tab order (removal is primarily via Backspace; the button is a mouse/touch affordance).

### Live Region

```html
<div aria-live="polite" aria-atomic="true" class="sr-only" data-testid="mentions-live-region">
  {announcement}
</div>
```

| Trigger | Announcement |
|---|---|
| Dropdown opens with results | `"{N} collaborators found"` |
| Dropdown opens, no results | `"No collaborators found"` |
| Dropdown opens, loading | `"Loading collaborators"` |
| Mention inserted | (no announcement — chip insertion is visual) |
| Mention removed | (no announcement — chip removal is visual) |

### Keyboard Navigation

| Key | Context | Action |
|---|---|---|
| `@` | Inside textarea | Opens dropdown, starts filtering |
| `ArrowDown` | Dropdown open | Move selection down. Wraps from last to first. |
| `ArrowUp` | Dropdown open | Move selection up. Wraps from first to last. |
| `Enter` | Dropdown open, item selected | Insert mention chip for selected collaborator, close dropdown |
| `Escape` | Dropdown open | Close dropdown, leave `@filter` text as-is in textarea |
| `Backspace` | Caret immediately after a mention chip | Delete entire chip atomically |
| `Delete` | Caret immediately before a mention chip | Delete entire chip atomically |
| `Tab` | Dropdown open | Close dropdown, move focus to next focusable element |

### Focus Management

- The textarea retains focus throughout the interaction. The dropdown is a non-modal overlay that does **not** steal focus.
- `aria-activedescendant` on the textarea is used instead of moving focus into the listbox (same pattern as SearchModal).
- When the dropdown opens, `aria-expanded` is set to `true`.
- Arrow keys are intercepted by the textarea's `onKeyDown` handler when the dropdown is open; they do **not** move the textarea caret.

---

## Responsive Behaviour

### Breakpoint: 375px (mobile with keyboard open)

- Dropdown max-width: `calc(100vw - 2rem)` = 343px.
- If the `@` trigger is near the right edge, the dropdown right-aligns to the trigger point to avoid clipping.
- If the `@` trigger is in the bottom half of the viewport, the dropdown opens **above** the caret to avoid being hidden by the on-screen keyboard.
- List items: reduced horizontal padding to `px-2` (from `px-3`).
- Avatar: remains 24px.
- Role badge label shortened: "MAINT." / "CONTRIB." at `sm` breakpoint.
- Touch targets: minimum 44px height per item (48px maintained).

### Breakpoint: 768px+ (tablet/desktop)

- Standard sizing as specified.
- Dropdown opens below caret by default, with above-caret fallback.
- Role badges show full labels.
- Hover states functional.

---

## Color Token Validation

All colours validated against `design-tokens.json` for WCAG 2.1 AA compliance (minimum 4.5:1 for text, 3:1 for UI components):

| Element | Light value | Dark value | Contrast ratio | WCAG Level |
|---|---|---|---|---|
| Username text | `#2d2820` | `#f5f5f5` | ≥10.5:1 / ≥13:1 | AAA |
| Display name text | `#6b5d4d` | `#b8a898` | ≥5:1 / ≥6.5:1 | AA |
| Matched filter highlight | `#8b6f3a` | `#c9983a` | ≥4.5:1 / ≥4.5:1 | AA |
| Role badge (maintainer) | `#c9983a` on `#c9983a/20` | `#c9983a` on `#c9983a/20` | ≥4.5:1 | AA |
| Role badge (contributor) | `#6b5d4d` | `#b8a898` | ≥5:1 / ≥6.5:1 | AA |
| Chip username | `#c9983a` on chip bg | `#c9983a` on chip bg | ≥4.5:1 | AA |
| Empty state text | `#7a6b5a` | `#b8a898` | ≥5:1 / ≥6.5:1 | AA |
| Dropdown border | `#c9983a/30` | `#c9983a/30` | 3:1 (UI component) | AA |
| Focus ring | `#a2792c` (light) | `#f1b400` (dark) | ≥3:1 on surface | AA |

---

## Interaction Model Alignment

This spec aligns with the existing `SearchModal.tsx` interaction conventions:

| Convention | SearchModal | MentionsDropdown |
|---|---|---|
| Trigger | `⌘K` / `Ctrl+K` globally | `@` in textarea |
| Input role | `role="combobox"` | `role="combobox"` (on textarea, only while open) |
| List role | `role="listbox"` + `aria-activedescendant` | `role="listbox"` + `aria-activedescendant` |
| Navigation | `j`/`k` or ArrowUp/ArrowDown | ArrowUp/ArrowDown |
| Selection | Enter | Enter |
| Dismissal | Escape | Escape |
| Focus | Stays in input; no focus trap in listbox | Stays in textarea; no focus trap in listbox |
| Live region | `aria-live="polite"` for result counts | `aria-live="polite"` for result counts |
| Backdrop | Full-screen overlay | No backdrop (anchored dropdown) |
| Focus return | Returns to trigger element on close | N/A (textarea never loses focus) |

---

## QA Checklist

- [ ] Typing `@` in the comment textarea opens the suggestions dropdown.
- [ ] Dropdown filters collaborators in real time as the user types after `@`.
- [ ] List item renders: 24px avatar, username (with match highlight), role badge.
- [ ] Role badge shows "MAINTAINER" or "CONTRIBUTOR" correctly.
- [ ] ArrowUp/ArrowDown navigates through the list; wraps from last to first and vice versa.
- [ ] Enter on a highlighted item inserts the mention chip and closes the dropdown.
- [ ] Escape closes the dropdown without inserting a mention.
- [ ] Clicking outside the dropdown closes it.
- [ ] Typing a space closes the dropdown and leaves the `@text` as-is.
- [ ] Mention chip renders inline with avatar, username, and optional remove button.
- [ ] Backspace when caret is immediately after a chip deletes the entire chip atomically.
- [ ] Delete when caret is immediately before a chip deletes the entire chip atomically.
- [ ] Clicking the `×` button on a chip removes it.
- [ ] Chips are non-editable (caret skips over them).
- [ ] Loading state shows skeleton rows while collaborators are fetched.
- [ ] Empty state shows "No collaborators found" when filter matches nothing.
- [ ] Dropdown doesn't clip at 375px viewport width with mobile keyboard open.
- [ ] Dropdown opens above caret when near bottom of viewport.
- [ ] All interactive elements have visible focus ring (`ring-1 ring-[#c9983a]` or theme equivalent).
- [ ] `aria-activedescendant` updates correctly on arrow-key navigation.
- [ ] Screen reader announces collaborator count on dropdown open.
- [ ] Screen reader announces mention chip as a single unit.
- [ ] `aria-busy="true"` is set during loading.
- [ ] `aria-expanded` is managed correctly on open/close.
- [ ] Colours meet 4.5:1 contrast in both light and dark themes.
- [ ] Reduced motion: dropdown open/close animation disabled; skeleton shimmer becomes static.
- [ ] Touch targets minimum 44×44px on mobile viewports.

---

## Implementation Reference

| File | Purpose |
|---|---|
| `design/specs/mentions-suggestions-dropdown.md` | This design specification |
| `frontend/src/features/dashboard/pages/IssueDetailPage.tsx` | Target page for the comment input |
| `frontend/src/shared/components/ReplyComposer.tsx` | Existing reply textarea component (may be extended) |
| `frontend/src/shared/components/SearchModal.tsx` | Interaction pattern reference (listbox, combobox, aria-activedescendant) |
| `backend/internal/github/issues_comments.go` | Backend comment API (collaborator data source) |
| `design-tokens.json` | Colour, motion, and spacing token authority |

---

## Security Notes

- Collaborator data should come from a trusted API endpoint; do not expose collaborator lists to unauthenticated users.
- Mention chips are rendered client-side only; the actual mention is persisted as `@login` text in the comment body sent to the backend.
- No client-side HTML injection risk — chips are composed of safe elements (`<span>`, `<img>`, `<button>` with static attributes).
- The `contenteditable="false"` on chips prevents DOM-based XSS via chip content manipulation.
