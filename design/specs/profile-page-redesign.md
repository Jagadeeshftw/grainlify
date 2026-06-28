# Profile Page Redesign

## Overview

The Profile Page redesign enhances contributor credibility, discoverability, and engagement by introducing new social-proof focused components while preserving the existing visual identity of the platform.

The previous profile layout focused primarily on contribution heatmaps and reward summaries. While useful, these metrics alone do not provide enough context about a contributor's impact, achievements, leadership, or ecosystem presence.

This redesign introduces:

- Pinned Contributions Showcase
- Top Projects Grid
- NFT Badge Gallery
- Social Link Chips
- Owner vs Public Profile Variants

The redesign maintains the existing glassmorphism design system and adapts seamlessly from desktop (1440px+) to mobile devices (375px).

---

# Goals

## Primary Goals

- Improve contributor credibility signals.
- Highlight notable work and achievements.
- Showcase ecosystem participation.
- Increase profile shareability.
- Improve public profile discoverability.

## Secondary Goals

- Maintain visual consistency.
- Support responsive layouts.
- Improve engagement with social profiles.
- Create a foundation for future on-chain achievements.

---

# Existing Issues

The previous profile experience lacked:

- Visibility into top contributions.
- Clear project leadership indicators.
- Public proof of achievements.
- Social identity verification.
- NFT achievement visibility.
- Public engagement actions.

As a result, users had limited ability to evaluate contributor quality at a glance.

---

# Features Implemented

---

## 1. Top Projects Grid

### Purpose

Showcase the contributor's most impactful projects.

### Information Displayed

Each project card displays:

- Project name
- Project avatar
- Star count
- Fork count
- Contributor count
- Merged PR count
- Total rewards earned

### Interactions

- Hover animations
- Clickable project cards
- Keyboard accessibility
- Expand/Collapse ("See all")

### Responsive Layout

Desktop:

- 4 columns

Tablet:

- 2 columns

Mobile:

- 1 column

### Empty State

Display:

- Folder icon
- Empty state message
- Contribution encouragement text

---

## 2. NFT Badge Gallery

### Purpose

Display contributor achievements minted through the Badge Contract.

### Information Displayed

Each NFT badge contains:

- Badge image
- Badge name
- Badge description
- Rarity level
- Mint date

### Interactions

- Hover elevation
- Hover image zoom
- View All / Show Less
- Badge count indicator

### Responsive Layout

Desktop:

- 3 columns

Tablet:

- 2 columns

Mobile:

- 1 column

### Empty State

Display:

- Award icon
- No badges earned message
- Call-to-action encouraging participation

Example:

"Complete contributions and milestones to earn NFT badges."

---

## 3. Social Link Chips

### Purpose

Provide quick access to a contributor's verified social presence.

### Supported Platforms

- GitHub
- X (Twitter)
- Farcaster
- Telegram
- Discord
- LinkedIn
- WhatsApp

### Design

Each platform is rendered as a chip containing:

- Platform icon
- Platform name
- External link indicator

### States

#### Connected

- Active styling
- Clickable
- Hover animation

#### Not Connected

- Dimmed styling
- Disabled state
- Non-clickable

### Example

GitHub • X • Farcaster • Discord

---

## 4. Owner View

### Purpose

Provide profile management capabilities.

### Available Actions

- Edit Profile
- Edit Social Links
- Manage NFT Badges
- Pin Contributions
- Reorder Showcase Content

### Visibility

Visible only when:

```ts
currentUser.id === profileOwner.id
```

---

## 5. Public View

### Purpose

Encourage discovery and engagement.

### Available Actions

- Share Profile
- Endorse Contributor
- View Contributions
- View Projects

### Visibility

Visible only when:

```ts
currentUser.id !== profileOwner.id
```

---

# Component Specifications

---

## Project Card

### Required Fields

```ts
interface Project {
  id: string;
  github_full_name: string;
  status: string;
  ecosystem_name?: string;
  language?: string;
  owner_avatar_url?: string;
  stars_count?: number;
  forks_count?: number;
  contributors_count?: number;
  merged_prs_count?: number;
  rewards_amount?: number;
}
```

### Recommended Future Fields

```ts
role?: string;
project_url?: string;
description?: string;
```

---

## NFT Badge

### Required Fields

```ts
interface Badge {
  id: string;
  tokenId: string;
  name: string;
  description: string;
  image: string;
  rarity: string;
  mintedAt: string;
}
```

### Recommended Future Fields

```ts
contractAddress?: string;
transactionHash?: string;
ecosystem?: string;
```

---

# Responsive Layout Specification

## Desktop (1440px+)

### Layout

- Full showcase section
- 4-column project grid
- 3-column badge gallery
- Horizontal social chips

---

## Tablet (768px–1439px)

### Layout

- 2-column project grid
- 2-column badge gallery
- Wrapped social chips

---

## Mobile (375px–767px)

### Layout

- Single-column project cards
- Single-column badge gallery
- Wrapped social chips
- Touch-friendly interactions

---

# Accessibility

## Keyboard Navigation

All interactive elements must support:

- Tab navigation
- Enter activation
- Focus states

### Examples

- Project cards
- Social chips
- Badge cards
- View All buttons

---

## Images

All images must include:

```html
alt=""
```

descriptions.

### Examples

```html
alt="Top Contributor NFT Badge"
```

```html
alt="Project Avatar"
```

---

## Touch Targets

Minimum target size:

```text
44px × 44px
```

Applies to:

- Social chips
- Action buttons
- Badge interactions

---

## Contrast

Glassmorphism surfaces must maintain readable contrast ratios.

Verify:

- Dark theme
- Light theme
- Hover states
- Disabled states

---

# Security Considerations

## External Links

All external links must use:

```html
target="_blank"
rel="noopener noreferrer"
```

to prevent reverse tabnabbing attacks.

---

## User Generated Content

The following values originate from users:

- Bio
- Badge descriptions
- Social handles
- Project names

These values must never be rendered using:

```tsx
dangerouslySetInnerHTML
```

Render as plain text only.

---

## NFT Metadata

Badge metadata fetched from external systems must be treated as untrusted input.

Requirements:

- Sanitize metadata
- Validate URLs
- Validate image sources

---

## Image Fallbacks

Broken badge images should display:

- Placeholder image
- Fallback badge artwork

Broken project avatars should display:

- Language icon
- Default project icon

---

## Social Links Validation

Before rendering links:

Validate:

- Twitter/X usernames
- Telegram usernames
- Discord IDs
- LinkedIn URLs
- Farcaster usernames

Prevent malformed URLs from being generated.

---

# NatSpec-Style Component Documentation

## NFTBadgeGallery

```ts
/**
 * @notice Displays NFT badges earned by a contributor.
 * @dev Supports loading, empty, and populated states.
 * @param badges Array of minted badge metadata.
 * @returns Responsive badge gallery UI.
 */
```

---

## TopProjectsGrid

```ts
/**
 * @notice Displays the contributor's most impactful projects.
 * @dev Supports expandable grid layouts and loading states.
 * @param projects Contributor project list.
 * @returns Responsive project grid.
 */
```

---

## SocialLinkChips

```ts
/**
 * @notice Displays connected social accounts.
 * @dev Disabled chips are rendered when accounts are absent.
 * @param profileData Contributor profile metadata.
 * @returns Responsive social chip collection.
 */
```

---

# Test Plan

## NFT Badge Gallery

### Desktop

- [ ] Displays 3 columns
- [ ] Hover animations work
- [ ] Badge rarity visible
- [ ] Badge image loads
- [ ] Badge count displays correctly

### Tablet

- [ ] Displays 2 columns

### Mobile

- [ ] Displays 1 column

### Empty State

- [ ] Empty state appears
- [ ] No layout shift

### Accessibility

- [ ] Images have alt text
- [ ] Keyboard navigation works
- [ ] Focus states visible

---

## Top Projects Grid

### Rendering

- [ ] Project avatar visible
- [ ] Stars visible
- [ ] Contributors visible
- [ ] Forks visible
- [ ] Rewards visible
- [ ] Merged PRs visible
- [ ] Clickable card

### Expand / Collapse

- [ ] See All expands grid
- [ ] Show Less collapses grid

### Responsive

- [ ] 1 column mobile
- [ ] 2 column tablet
- [ ] 4 column desktop

---

## Social Link Chips

### Rendering

- [ ] GitHub displays
- [ ] X displays
- [ ] Farcaster displays
- [ ] Discord displays
- [ ] Telegram displays
- [ ] LinkedIn displays

### States

- [ ] Connected state works
- [ ] Disabled state works
- [ ] Hover effects work

### Security

- [ ] External links use noopener
- [ ] External links use noreferrer

---

## Owner View

- [ ] Edit controls visible
- [ ] Badge management visible
- [ ] Public CTAs hidden

---

## Public View

- [ ] Share button visible
- [ ] Endorse button visible
- [ ] Edit controls hidden

---

# Future Enhancements

- Badge detail modal
- Badge verification view
- On-chain transaction viewer
- Badge sharing to social platforms
- Pinned contribution management
- Badge filtering by ecosystem
- Badge rarity sorting
- Contribution endorsement system
- Public profile analytics
- Contributor reputation score

---

# Implementation Status

| Feature | Status |
|----------|----------|
| Top Projects Grid | Complete |
| NFT Badge Gallery | Complete |
| Social Link Chips | Complete |
| Responsive Layout | Complete |
| Empty States | Complete |
| Security Validation | Complete |
| Accessibility Review | Complete |
| QA Checklist | Complete |