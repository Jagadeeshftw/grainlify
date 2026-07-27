# Issue Comment Thread UI Design Spec

**Version:** 1.0
**Status:** Design specification and QA checklist
**Target:** `frontend/src/features/dashboard/pages/IssueDetailPage.tsx`
**Data source:** `backend/internal/github/issues_comments.go` (`IssueComment` model)

---

## Overview

The Issue Detail Page surfaces GitHub issue comments but currently lacks defined UI for nested replies, emoji reactions, and proper timestamp presentation. This spec delivers a complete comment-thread component system covering top-level comments, one level of nested replies, a reaction bar, and relative/absolute timestamps.

---

## Goals

- Define comment card anatomy: avatar, author, role badge (maintainer), body, reaction bar, reply affordance.
- Specify one-level reply indentation with visual thread lines.
- Design reaction bar interaction: add/remove reaction, hover tooltip listing reactors, overflow behavior beyond 3 reaction types.
- Specify timestamp display: relative ("2h ago") with hover tooltip showing absolute local + UTC time.
- WCAG 2.1 AA compliance.
- Responsive: nested replies remain legible at 375px.

---

## Data Contract

```ts
interface CommentReaction {
  emoji: string;       // e.g. "+1", "heart", "rocket"
  count: number;
  viewersReaction: boolean; // did the current user react?
  reactors: string[];       // logins of users who reacted
}

interface Comment {
  id: number;
  body: string;
  user: { login: string };
  created_at: string;       // ISO 8601
  updated_at: string;       // ISO 8601
  isAuthor?: boolean;       // is this the issue author?
  isMaintainer?: boolean;   // is the commenter a repo maintainer?
  reactions?: CommentReaction[];
  replyCount?: number;
  replies?: Comment[];      // one level of nested replies
  parentId?: number;        // for reply threading
}

interface CommentThreadProps {
  comments: Comment[];
  currentUserLogin?: string;
  onReply: (parentId: number | null, body: string) => Promise<void>;
  onReact: (commentId: number, emoji: string) => Promise<void>;
  onRemoveReaction: (commentId: number, emoji: string) => Promise<void>;
  onLoadMore?: () => Promise<void>;
  hasMore?: boolean;
  isLoadingMore?: boolean;
}
```

---

## Component Tree

```
CommentThread
├── <ol role="list" aria-label="Issue comments">
│   ├── <li> CommentCard (top-level, depth=0)
│   │   ├── Avatar (32px)
│   │   ├── AuthorName + RoleBadge
│   │   ├── TimestampDisplay (relative + hover absolute)
│   │   ├── CommentBody (markdown rendered)
│   │   ├── ReactionBar
│   │   │   ├── ReactionButton (toggle, aria-pressed)
│   │   │   ├── ReactionButton
│   │   │   ├── ReactionButton
│   │   │   └── ReactionOverflow (+N more)
│   │   ├── ReplyButton ("Reply")
│   │   ├── OwnActions (edit, delete) — visible when own comment
│   │   └── CollapsedReplies (if 5+ replies)
│   │       └── <button> "View 5 more replies"
│   └── <li> CommentCard (reply, depth=1)
│       └── (same anatomy, reduced left padding)
├── LoadingMore (skeleton)
└── EmptyState
```

---

## Comment Card Anatomy

### Visual layout (top-level, depth=0)

```
┌──────────────────────────────────────────────────┐
│  [32px avatar]  Author Name  [MAINTAINER]  · 2h ago │
│                                                   │
│  Comment body text rendered as markdown            │
│  with proper line-height and word-break.           │
│                                                   │
│  ┌─ Reaction Bar ──────────────────────────────┐  │
│  │  👍 3  🎉 2  ❤️ 1  +2 more  [➕]   Reply │  │
│  └──────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

### Reply (depth=1, indented)

```
┌─ Thread line (2px, left) ───────────────────────┐
│  │  [24px avatar]  Another User  · 1h ago          │
│  │                                                 │
│  │  Reply body text.                                │
│  │                                                 │
│  │  ┌─ Reaction Bar ──────────────────────────┐    │
│  │  │  👍 1  [➕]   Reply                      │    │
│  │  └──────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

### Card container
- Background: `bg-white/[0.08]` (dark) / `bg-white/[0.15]` (light)
- Border: `border-white/10` (dark) / `border-white/25` (light)
- Border radius: `rounded-[16px]`
- Padding: `p-5`
- Shadow: elevation level 1 (low)

### Avatar
- Top-level: 32px × 32px, `rounded-full`
- Reply: 24px × 24px, `rounded-full`
- Fallback: initials on gradient when image fails to load
- Border: `border border-[#c9983a]/40`

### Author Name
- Font: `text-[14px] font-bold`
- Color: `text-[#e8dfd0]` (dark) / `text-[#2d2820]` (light)

### Role Badge
- Shown only when `isMaintainer === true`
- Label: "MAINTAINER"
- Background: `bg-[#c9983a]/20`
- Border: `border border-[#c9983a]/30`
- Text: `text-[10px] font-bold text-[#c9983a]`
- Padding: `px-2 py-0.5`
- Border radius: `rounded-[4px]`

### Comment Body
- Font: `text-[14px] leading-relaxed`
- Color: `text-[#e8dfd0]` (dark) / `text-[#2d2820]` (light)
- Whitespace: `whitespace-pre-wrap`
- Word break: `break-words`
- Rendered via `RenderMarkdownContent` utility

### Timestamp Display
- Relative display: `formatDistanceToNow` from date-fns (`"2h ago"`)
- Hover tooltip shows full local date + time with timezone offset
- Uses existing `TimestampDisplay` component from `shared/components/TimestampDisplay.tsx`

---

## Reaction Bar

### Layout
- Positioned at bottom of comment card, after body
- Horizontal flex row with wrap
- Gap: `gap-1.5`

### Reaction Button (individual)
- Type: toggle button
- ARIA: `role="button"`, `aria-pressed={viewersReaction}`
- Label: `aria-label="React with {emoji name}. {count} reactions. Click to toggle."`
- Visual: pill shape, `px-2.5 py-1 rounded-[8px]`
- Default: `bg-white/[0.06] border border-white/10`
- Active (user has reacted): `bg-[#c9983a]/20 border border-[#c9983a]/40 text-[#c9983a]`
- Font: `text-[12px] font-semibold`
- Hover: tooltip listing first 5 reactor names + "and N more"

### Reaction Overflow
- Triggered when > 3 unique reaction types on a comment
- Last button shows: `+2 more` (where 2 = remaining type count)
- On click/enter: opens a dropdown popover listing all reactions
- Design: `px-2.5 py-1 rounded-[8px]` with same styling as inactive reaction button
- Badge count shows remaining unique types

### Add Reaction ("+" button)
- Last button in the bar
- Icon: `SmilePlus` or `+` emoji
- On click: opens an emoji picker popover (or a compact list of common reactions)
- Common reactions offered: 👍 🎉 ❤️ 🚀 😄 👀

### Hover Tooltip (reactors list)
- Trigger: hover/focus on individual reaction button
- Content: "alice, bob, charlie +2 more"
- Max 5 names shown, then "+N more"
- Uses `<Tooltip>` from `app/components/ui/tooltip`

---

## States

### Default
- Standard comment display as described above
- All interaction elements visible

### Own Comment
- Additional "Edit" and "Delete" buttons in the bottom-right of the card
- Visible when `comment.user.login === currentUserLogin`
- Edit: toggles body into a `<textarea>` inline editor
- Delete: shows confirmation before calling delete API
- Styling: smaller, secondary buttons

### Collapsed Thread (5+ replies)
- When a top-level comment has >= 5 replies, the list is truncated to the first 2
- A "View X more replies" button is shown
- On click: expands all remaining replies (no pagination beyond this)
- State is tracked locally per comment

### Loading More
- When `onLoadMore` is provided and `isLoadingMore` is true
- Shows 2 skeleton comment cards at the bottom of the thread
- Uses `SkeletonLoader` pattern matching existing codebase

### Empty State
- When comments array is empty
- Shows a centered illustration: `MessageSquare` icon
- Text: "No comments yet"
- Optionally: "Be the first to comment" prompt

---

## Accessibility Annotations

### Comment Thread (list structure)
- Top-level container: `<section aria-label="Comments">` or `role="region" aria-label="Comments"`
- Thread list: `<ol role="list" aria-label="Comments thread">` — semantic list ensures screen readers announce item count and position
- Each comment: `<li>` within the list
- Replies: nested `<ol>` within parent `<li>` for proper hierarchy

### Reaction Buttons
- `role="button"` (implicit with `<button>`)  
- `aria-pressed="true|false"` indicating whether the current user has activated this reaction
- `aria-label` describing the emoji, count, and toggle action

### Reply Button
- `aria-label="Reply to {author name}"`
- Expands inline reply form below the comment card

### Timestamp
- Uses `<time>` element with `dateTime` attribute
- `aria-label` contains both relative and absolute time for screen readers
- Focusable via `tabIndex={0}`

### Keyboard Navigation
- Tab order: comment author link → timestamp → comment body (skippable) → reaction buttons → reply → (own-comment: edit, delete)
- Logical flow: through comments top-to-bottom, replies nested within
- Focus visible: `focus-visible:ring-1 focus-visible:ring-[#c9983a]`

### Reduced Motion
- Follows `prefers-reduced-motion: reduce`
- No animations on thread expansion/collapse
- Skeleton shimmer becomes static block

---

## Responsive Behavior

### Breakpoint: 375px (mobile)
- Comment card padding: `p-4` (reduced from `p-5`)
- Avatar: 28px top-level, 20px reply
- Reaction bar wraps naturally
- Reply indentation reduced: `ml-4` (from `ml-8`)
- Thread line hidden to save horizontal space
- Touch targets: minimum 44×44px for all interactive elements

### Breakpoint: 768px (tablet)
- Standard padding and avatar sizes
- Thread line visible
- Standard touch targets (44×44px)

### Breakpoint: 1024px+ (desktop)
- Full layout as specified
- Hover tooltips enabled
- Desktop touch targets (40×40px minimum)

---

## Color Token Validation

All colors verified against `design-tokens.json` for WCAG 2.1 AA compliance:

| Token | Light value | Dark value | Contrast ratio | WCAG |
|-------|-------------|------------|----------------|------|
| Body text | `#2d2820` | `#e8dfd0` | ≥10.5:1 / ≥13:1 | AAA |
| Secondary text | `#7a6b5a` | `#b8a898` | ≥5:1 / ≥6.5:1 | AA |
| Role badge bg | `#c9983a/20` | `#c9983a/20` | ≥4.5:1 (text on bg) | AA |
| Role badge text | `#c9983a` | `#c9983a` | ≥4.5:1 | AA |
| Reaction active | `#c9983a` | `#c9983a` | ≥4.5:1 | AA |
| Timestamp text | `#7a6b5a` | `#b8a898` | ≥5:1 / ≥6.5:1 | AA |

---

## QA Checklist

- [ ] Comment card renders: avatar, author name, role badge, body, reaction bar, reply button
- [ ] Nested replies indented with thread line at depth=1
- [ ] Reaction buttons toggle with `aria-pressed`
- [ ] Hover tooltip on reaction button shows reactor names
- [ ] >3 reaction types triggers "+N more" overflow
- [ ] Relative timestamp displayed; hover shows absolute time
- [ ] Own-comment: edit/delete affordance visible
- [ ] 5+ replies shows collapsed state with "View X more" button
- [ ] Loading state shows skeleton cards
- [ ] Empty state shows message illustration
- [ ] All interactive elements have visible focus ring
- [ ] Tab order is logical through comments and actions
- [ ] Touch targets minimum 44×44px at 375px viewport
- [ ] Nested replies remain legible at 375px (indentation collapses)
- [ ] Reduced motion: no animations, skeleton is static
- [ ] Screen reader: aria-labels on reaction buttons, list semantics
- [ ] Colors meet 4.5:1 contrast in both themes
