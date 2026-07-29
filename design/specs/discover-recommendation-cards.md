# DiscoverPage recommendation card layout spec

**Version:** 1.0
**Status:** UI hand-off ready
**Target:** frontend/src/features/dashboard/pages/DiscoverPage.tsx

## Overview

The DiscoverPage recommendation experience introduces a dedicated card system for personalized picks that distinguishes between project recommendations and contributor recommendations while staying aligned with the existing glassmorphism dashboard treatment.

## Goals

- Present project picks and contributor picks in a single, scannable grid.
- Reuse the warm gold accent and glass surfaces from the dashboard design system.
- Provide a clear “Why recommended” rationale chip with truncation and accessible labeling.
- Maintain keyboard focus clarity and WCAG 2.1 AA contrast across light and dark themes.

## Card anatomy

### Shared shell

- Rounded glass card with 22px radius, soft shadow, and gold-tinted border.
- One tab stop per card via a single button element.
- The rationale chip sits above the description block and uses a single-line truncation rule with an ellipsis when space is tight.
- The card uses a focus-visible ring and subtle hover elevation to reinforce interaction state.

### Project pick

- Eyebrow label: “Recommended project”.
- Primary visual treatment: stronger gold border glow and project-oriented icon.
- Supports metadata such as stars, forks, ecosystem, and relevant tags.

### Contributor pick

- Eyebrow label: “Recommended contributor”.
- Primary visual treatment: slightly lighter gold border accent and contributor-oriented icon.
- Supports contributor cues such as skill alignment, recent activity, or ecosystem network tags.

## States

| State | Behavior |
|---|---|
| Default | Glass panel, warm neutral text, rationale chip visible. |
| Hover | Elevation increases, border brightens, soft gold glow appears. |
| Focus-visible | Two-pixel gold ring and strong outline contrast. |
| Loading skeleton | Reuses the skeleton conventions from the dashboard card system with a compact header, body, and tag placeholders. |
| Empty | Friendly empty state with clear guidance to revisit later or explore other recommendations. |

## Accessibility

- Each card is exposed as a single focusable control with an `aria-label` describing the recommendation type and title.
- Each card uses `aria-describedby` to associate the rationale content with the control.
- Text and chip colors are selected to meet WCAG 2.1 AA contrast targets in light and dark themes.
- Logical reading order is project picks first, then contributor picks.

## Responsive layout

- Mobile: 1 column.
- Tablet (768px): 2 columns.
- Desktop (1280px+): 3 columns.

## Design tokens

- Gold accent follows the dashboard gold tokens in design-tokens.json.
- Elevation uses the existing medium and high elevation values for hover and focus emphasis.
- Glass surfaces align with the existing blur, border, and transparency treatment used throughout the dashboard.
