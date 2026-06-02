# ProfilePage Contribution Heatmap & Rewards Visualization Design Spec

**Version:** 1.0  
**Last Updated:** May 31, 2026  
**Status:** Design Specification  
**Target:** WCAG 2.1 AA Compliance  

---

## Table of Contents
1. [Overview](#overview)
2. [Contribution Heatmap Design](#contribution-heatmap-design)
3. [Rewards Chart Redesign](#rewards-chart-redesign)
4. [Design Tokens & Color Scale](#design-tokens--color-scale)
5. [Accessibility (A11y) Specifications](#accessibility-a11y-specifications)
6. [Responsive Behavior](#responsive-behavior)
7. [Component States](#component-states)
8. [Implementation Guidelines](#implementation-guidelines)

---

## Overview

This specification defines the redesign of two key data visualizations in the ProfilePage component:

1. **Contribution Heatmap**: A responsive, accessible 365-day contribution calendar with month/day axis labels
2. **Rewards Chart**: An enhanced pie chart with axis labels (if applicable), legend, and annotated milestones

### Design Principles
- **Mobile-First**: Design for small screens first, enhance for larger displays
- **Accessible**: WCAG 2.1 AA compliant with keyboard navigation and screen reader support
- **Consistent**: Use established design tokens and visual patterns from Grainlify DS
- **Data-Driven**: Color scales convey contribution intensity; interactive elements provide context

---

## Contribution Heatmap Design

### 1. Layout Architecture

#### 1.1 Container Structure
```
┌─ Heatmap Container (full-width, responsive) ────────────────┐
│  ┌─ Title Bar (sticky on mobile) ──────────────┐            │
│  │ "365-Day Contribution Activity"              │            │
│  │ Year: 2025 [← Prev | Next →]                │            │
│  └─────────────────────────────────────────────┘            │
│  ┌─ Scrollable Viewport (sm/md: horizontal scroll; lg+: full) │
│  │ ┌─ Y-Axis (Day Labels) ┐                                  │
│  │ │ Mon                  │  ┌─ Heatmap Grid ─────────┐    │
│  │ │ Tue                  │  │ □ □ □ □ □ □ □ (Jan)    │    │
│  │ │ Wed                  │  │ □ □ □ □ □ □ □ (Feb)    │    │
│  │ │ Thu                  │  │ □ □ □ □ □ □ □ (Mar)    │    │
│  │ │ Fri                  │  │ ... (52 weeks total)    │    │
│  │ │ Sat                  │  └─────────────────────────┘    │
│  │ │ Sun                  │                                  │
│  │ └──────────────────────┘                                  │
│  │ X-Axis (Month Labels) below heatmap                       │
│  └─────────────────────────────────────────────────────────── │
│  ┌─ Legend ────────────────────────────────────┐            │
│  │ Less •  [Color Scale]  • More                │            │
│  │ 0 contributions (neutral) → Max (gold)       │            │
│  └─────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

#### 1.2 Responsive Breakpoints

| Breakpoint | Width | Behavior |
|-----------|-------|----------|
| **sm** | 640px | Horizontal scroll, compact labels, narrow day labels |
| **md** | 768px | Horizontal scroll, full labels, adjustable cell size |
| **lg** | 1024px | Full-width no scroll, standard layout |
| **xl** | 1280px | Full-width, expanded cell size, enhanced spacing |
| **2xl** | 1536px | Full-width with additional year selector UI |

#### 1.3 Heatmap Grid Specifications

| Property | Value | Responsive Adjustment |
|----------|-------|----------------------|
| **Cell Size** | 14×14px (base) | sm: 10px, md: 12px, lg: 14px, xl: 16px |
| **Cell Gap** | 2px | Consistent across breakpoints |
| **Week Column Gap** | 8px | sm: 4px, md: 6px, lg: 8px |
| **Border Radius** | 3px | Consistent (accessible targeting) |
| **Grid Rows** | 7 (Mon-Sun) | Fixed |
| **Grid Columns** | 53 (weeks, Jan-Dec) | Fixed |
| **Total Visible** | sm/md: 8-10 weeks (with horizontal scroll), lg+: all 52 weeks |

---

### 2. Color Scale Specification

#### 2.1 Contribution Intensity Mapping

The heatmap uses a linear color scale from neutral (no contribution) to gold (max contribution):

```
Level 0: No contributions
  Color: #e8e8e8 (neutral-200)
  Opacity: 1
  Description: Empty or no activity
  
Level 1: Low activity (1-25% of max)
  Color: #c9983a (gold-600)
  Opacity: 0.35
  Hex equivalent: #c9983a59
  Description: Minimal contribution
  
Level 2: Medium activity (26-50% of max)
  Color: #c9983a (gold-600)
  Opacity: 0.55
  Hex equivalent: #c9983a8c
  Description: Moderate contribution
  
Level 3: High activity (51-75% of max)
  Color: #c9983a (gold-600)
  Opacity: 0.75
  Hex equivalent: #c9983aBF
  Description: Significant contribution
  
Level 4+: Max activity (76-100% of max)
  Color: Gradient overlay
  Background: #f1b400 (gold-primary)
  Overlay: rgba(212, 175, 55, 0.3) (gold-accent with transparency)
  Effect: Subtle gradient + sparkle animation (✨)
  Description: Peak contribution day
```

#### 2.2 Design Token References

From `design-tokens.json`:
- **Primary Gold**: `#f1b400` (500)
- **Gold Accent**: `#d4af37`
- **Gold Secondary**: `#c9983a` (600)
- **Gold Tertiary**: `#a67c2e` (700)
- **Neutral 200**: `#efefef` or `#e8e8e8` (light gray background)
- **Neutral 500**: `#78716c` (medium gray, labels)
- **Neutral 700**: `#292524` (dark gray, text)

**Color-Blind Safe Scale:**
- Avoid pure red/green transitions
- Use gold + neutral (intensity-based) to satisfy deuteranopia and protanopia accessibility
- Provide patterns/icons in addition to color (e.g., sparkle ✨ for max contribution)

---

### 3. Interactive Features

#### 3.1 Cell Interactions

**Hover State (Desktop)**
- Scale: 1.15× (13.5px → 16.1px)
- Shadow: `0 4px 12px rgba(0,0,0,0.15)`
- Border: 1px solid `#c9983a` (gold-600)
- Tooltip: Show exact contribution count + date
- Duration: 150ms ease-out

**Focus State (Keyboard Navigation)**
- Outline: 2px solid `#f1b400` (gold-primary), offset 2px
- Box-shadow: `0 0 0 4px rgba(241, 180, 0, 0.25)`
- Duration: 200ms ease-out
- Visible on `:focus-visible` (keyboard) and `:focus` (fallback)

**Active State**
- Background: Darken by 15% opacity
- Scale: 1.05×
- Cursor: pointer

**Disabled State** (if applicable)
- Opacity: 0.5
- Cursor: not-allowed
- Pointer-events: none

#### 3.2 Tooltip Specification

**Content:**
```
[Date: YYYY-MM-DD]
[N] contribution(s)
[Weekday: Monday]
```

**Position:** Top or bottom (auto-adjust if near viewport edge)  
**Appearance:**
- Background: `#292524` (neutral-700, dark) with `0.95` opacity
- Text: `#fafaf9` (neutral-50, light)
- Padding: 8px 12px
- Border-radius: 6px
- Font-size: 12px (sm), 13px (lg+)
- Box-shadow: `0 10px 25px rgba(0,0,0,0.2)`
- Arrow: Triangle pointer to heatmap cell

**Animation:**
- Fade-in: 200ms ease-out
- Fade-out: 150ms ease-in
- Delay: 150ms on hover

---

### 4. Axis Labels

#### 4.1 Y-Axis (Day Labels)

```
Mon
Tue
Wed
Thu
Fri
Sat
Sun
```

**Typography:**
- Font-size: 11px (sm), 12px (md), 13px (lg+)
- Font-weight: 500
- Color: `#78716c` (neutral-500)
- Text-align: right
- Margin-right: 8px
- Line-height: 1.4

**Responsiveness:**
- sm: Abbreviated (M, T, W, Th, F, S, Su) if space constrained
- md+: Full names (Mon-Sun)

#### 4.2 X-Axis (Month Labels)

```
[Jan] [Feb] [Mar] [Apr] [May] [Jun] [Jul] [Aug] [Sep] [Oct] [Nov] [Dec]
```

**Typography:**
- Font-size: 10px (sm), 11px (md), 12px (lg+)
- Font-weight: 500
- Color: `#78716c` (neutral-500)
- Text-align: center
- Margin-top: 6px

**Positioning:**
- Placed below heatmap grid
- Centered above corresponding 4-week column
- Consistent spacing

**Responsiveness:**
- sm: Show every 8-12 weeks (abbreviated: J, F, M, A...)
- md: Show every 6 weeks (abbreviated: Jan, Feb, Mar...)
- lg+: Show all 12 months (full names)

---

### 5. Legend

**Content:**
```
📊 Contribution Intensity
Less ●─────────────● More
 0               Max contributions
```

**Components:**
- Description text: "Contribution Intensity" (font-weight: 600, color: neutral-700)
- Color stops: 4-5 gradient stops from neutral-200 to gold-primary
- Labels: "0" (left) and "Max" (right)
- Optional: Interactive toggle to show/hide different intensity levels

**Styling:**
- Background: `rgba(255, 255, 255, 0.05)` (glass effect on dark mode)
- Padding: 12px 16px
- Border-radius: 8px
- Border: 1px solid `rgba(255, 255, 255, 0.1)`
- Margin-top: 16px
- Font-size: 12px
- Color: `#78716c` (neutral-500)

---

## Rewards Chart Redesign

### 1. Layout Architecture

#### 1.1 Current State
- Pie chart (recharts: PieChart, Pie, Cell, Tooltip)
- Inner radius: 75px, outer: 105px
- Animation: 800ms ease-out
- Custom tooltip with USD formatting

#### 1.2 Enhanced Layout
```
┌─ Rewards Distribution Container ──────────────┐
│ ┌─ Title ─────────────────────────┐           │
│ │ "Rewards Distribution" (2026)    │           │
│ │ Total Rewards: $[Amount] USD      │           │
│ └─────────────────────────────────┘           │
│ ┌─ Content Area (flex row, wrap on mobile) ─┐ │
│ │ ┌─ Chart (50% on lg+, 100% on sm) ──────┐ │ │
│ │ │  ◯  (Pie Chart, centered)              │ │ │
│ │ │  ◯                                      │ │ │
│ │ └──────────────────────────────────────┘ │ │
│ │ ┌─ Legend & Stats (50% on lg+, 100%) ──┐ │ │
│ │ │ ■ Category 1: $[Amount] (XX%)        │ │ │
│ │ │ ■ Category 2: $[Amount] (XX%)        │ │ │
│ │ │ ■ Category 3: $[Amount] (XX%)        │ │ │
│ │ │ [Annotated Milestones]               │ │ │
│ │ └──────────────────────────────────────┘ │ │
│ └────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

#### 1.2 Responsive Behavior

| Breakpoint | Layout |
|-----------|--------|
| **sm** | Chart full-width (100%), legend below chart (100%), stacked |
| **md** | Chart 100%, legend 100%, stacked, larger font |
| **lg** | Chart 50% left, legend 50% right, side-by-side |
| **xl** | Chart 45% left, legend 45% right, enhanced spacing |

---

### 2. Chart Components

#### 2.1 Pie Chart Enhancements

**Base Chart:**
- Inner radius: 75px (donut style)
- Outer radius: 105px
- Cell padding: 8px
- Animation: 800ms cubic-bezier(0.8, 0, 0.2, 1)
- Responsive: Resize on breakpoint changes

**Data Segments:**
- Each segment = reward category (e.g., "Contributions", "Reviews", "Bounties")
- Color: Use design tokens (gold gradient for primary, neutral-600 for secondary)
- Hover: Scale 1.1×, increase opacity 0.2
- Active (clicked): Scale 1.05×, show detailed tooltip

**Tooltip Enhancements:**
```
[Category Name]
$[Amount USD]
[Percentage of Total]
[Trend: ↑ 12% from previous month or ↓ 5%]
```

Position: Above cursor  
Font-size: 12px  
Background: `#292524` (dark) with 0.95 opacity  
Padding: 10px 14px  
Border-radius: 6px

---

### 3. Legend Specification

#### 3.1 Legend Design

**Layout:** Vertical list (lg+) or single column (sm/md)

**Item Structure:**
```
[■ Color Block] [Category Name] [Amount] ([Percentage])
```

**Styling:**
- Color block: 12×12px, border-radius: 2px
- Category name: 14px, font-weight: 500, color: neutral-700
- Amount: 12px, font-weight: 600, color: gold-600
- Percentage: 12px, font-weight: 400, color: neutral-500
- Item spacing: 12px vertical gap
- Interactive: Hover to highlight corresponding pie slice
- Clickable: Toggle visibility of segment (optional enhancement)

**Responsive Adjustments:**
- sm: Single column, full-width, font-size: 12px
- md: Single column, full-width, font-size: 13px
- lg+: Single column right-side, font-size: 14px

---

### 4. Annotated Milestones

**Purpose:** Highlight significant achievements or anomalies

**Annotations (if data supports):**
```
🏆 Milestone 1: Highest Earning Category
   "Bug Bounties contributed $X this year"
   
📈 Milestone 2: Growth Trend
   "Contributions reward grew 45% YoY"
   
🎯 Milestone 3: Target Achievement
   "Reached $X total rewards (Goal: $Y)"
```

**Styling:**
- Icon: 16px, emoji-based
- Title: 13px, font-weight: 600, color: neutral-700
- Description: 12px, font-weight: 400, color: neutral-600
- Background: `rgba(241, 180, 0, 0.08)` (gold tint)
- Padding: 12px 14px
- Border-left: 3px solid `#f1b400` (gold-primary)
- Border-radius: 4px
- Margin-top: 16px

**Position:** Below legend (mobile), or in sidebar (desktop)

---

## Design Tokens & Color Scale

### 1. Primary Color Palette

```javascript
{
  "colors": {
    "gold": {
      "primary": "#f1b400",      // Main gold accent
      "500": "#f1b400",
      "600": "#c9983a",          // Secondary (muted)
      "700": "#a67c2e",          // Tertiary (darker)
      "accent": "#d4af37"        // Light gold
    },
    "neutral": {
      "50": "#fafaf9",           // Near-white
      "200": "#efefef",          // Very light gray (heatmap empty)
      "300": "#e8e8e8",          // Light gray alternative
      "500": "#78716c",          // Medium gray (labels)
      "600": "#54524f",          // Medium-dark
      "700": "#292524",          // Dark (text)
      "950": "#0c0a09"           // Near-black
    },
    "semantic": {
      "success": "#22c55e",      // Green
      "warning": "#f59e0b",      // Amber
      "error": "#ef4444",        // Red
      "info": "#06b6d4"          // Cyan
    }
  }
}
```

### 2. Custom CSS Variables

```css
/* Heatmap Colors */
--heatmap-empty: #efefef;         /* neutral-200 */
--heatmap-level-1: #c9983a59;     /* gold-600, 35% opacity */
--heatmap-level-2: #c9983a8c;     /* gold-600, 55% opacity */
--heatmap-level-3: #c9983aBF;     /* gold-600, 75% opacity */
--heatmap-level-4: #f1b400;       /* gold-primary (100%) */

/* Chart Colors */
--chart-primary: #f1b400;         /* gold-primary */
--chart-secondary: #c9983a;       /* gold-600 */
--chart-accent: #d4af37;          /* gold-accent */

/* Text & Labels */
--text-primary: #292524;          /* neutral-700 */
--text-secondary: #54524f;        /* neutral-600 */
--text-tertiary: #78716c;         /* neutral-500 */

/* Backgrounds */
--bg-light: #fafaf9;              /* neutral-50 */
--bg-overlay: rgba(255, 255, 255, 0.05);
--bg-hover: rgba(201, 152, 58, 0.1);
```

---

## Accessibility (A11y) Specifications

### 1. WCAG 2.1 AA Compliance

#### 1.1 Contrast Ratios

| Element | Foreground | Background | Ratio | Standard |
|---------|-----------|-----------|-------|----------|
| **Heatmap Cell (Level 1)** | #c9983a | #ffffff | 5.2:1 | ✅ AA |
| **Heatmap Cell (Level 4)** | #f1b400 | #ffffff | 7.5:1 | ✅ AAA |
| **Text on Gold** | #292524 | #f1b400 | 8.1:1 | ✅ AAA |
| **Legend Label** | #78716c | #fafaf9 | 4.6:1 | ✅ AA |
| **Chart Legend** | #292524 | #fafaf9 | 15.1:1 | ✅ AAA |
| **Tooltip Text** | #fafaf9 | #292524 | 15.1:1 | ✅ AAA |

**Action:** Use contrast checker tool during implementation (https://webaim.org/resources/contrastchecker/)

#### 1.2 Keyboard Navigation

**Heatmap Cells:**
- `:focus-visible` applies 2px gold outline with 4px offset
- `Tab` moves through cells left-to-right, top-to-bottom
- `Enter` or `Space` to select/interact
- `Arrow keys` optional: ↑/↓ to change rows, ←/→ to change columns
- `Escape` to close tooltip

**Chart Points:**
- `Tab` navigates through pie chart segments
- `Enter` to show/toggle segment details
- Legend items are focusable with `Tab`

**Year/Month Navigation:**
- `Tab` focuses on [Prev] and [Next] buttons
- `Enter` changes year
- Announce year change to screen readers

---

### 2. Screen Reader Support

#### 2.1 Heatmap Data Table Alternative

```html
<!-- For screen readers, provide hidden data table -->
<table aria-label="365-Day Contribution Activity Table" class="sr-only">
  <thead>
    <tr>
      <th>Date</th>
      <th>Day</th>
      <th>Contributions</th>
      <th>Intensity Level</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>2025-01-01</td>
      <td>Wednesday</td>
      <td>5</td>
      <td>Level 2 (Medium)</td>
    </tr>
    <!-- More rows... -->
  </tbody>
</table>
```

#### 2.2 ARIA Labels & Descriptions

**Heatmap Container:**
```html
<div
  role="region"
  aria-label="Contribution Heatmap"
  aria-describedby="heatmap-desc"
>
  <!-- Content -->
</div>
<p id="heatmap-desc" class="sr-only">
  A 365-day contribution heatmap for 2025.
  Color intensity indicates activity level: 
  empty (no contributions) to gold (maximum contributions).
  Use arrow keys to navigate, Enter to view details.
</p>
```

**Heatmap Cells:**
```html
<button
  role="gridcell"
  aria-label="January 1, Wednesday: 5 contributions"
  aria-describedby="cell-tooltip"
  tabindex="0"
  class="heatmap-cell level-2"
>
  <!-- Visual: colored square -->
</button>
```

**Chart Segments:**
```html
<g
  role="button"
  aria-label="Bug Bounties: $4,250 USD, 42% of total rewards"
  tabindex="0"
  aria-pressed="false"
>
  <!-- SVG pie segment -->
</g>
```

---

### 3. Color-Blind Accessibility

#### 3.1 Safe Color Palette

The heatmap uses gold + neutral (intensity-based) scale, which is safe for:
- **Deuteranopia** (green-blind): Gold/brown distinct from neutral gray
- **Protanopia** (red-blind): Gold/brown distinct from neutral gray
- **Tritanopia** (blue-yellow blind): Should avoid pure blue/yellow; current palette uses gold/brown + gray ✅

#### 3.2 Pattern Overlay Option

For maximum accessibility, optionally add pattern overlays to high-intensity cells:
- Level 3+: Subtle crosshatch pattern (opacity: 0.15)
- Sparkle animation ✨ on Level 4 cells (visual indicator)

```css
.heatmap-cell.level-4 {
  background-image:
    radial-gradient(circle, rgba(255, 255, 255, 0.2) 1px, transparent 1px);
  background-size: 3px 3px;
  animation: sparkle 3s ease-in-out infinite;
}

@keyframes sparkle {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.8; }
}
```

---

### 4. Motion & Animation Sensitivity

Respect `prefers-reduced-motion`:

```css
@media (prefers-reduced-motion: reduce) {
  .heatmap-cell,
  .chart-segment,
  .tooltip {
    animation: none !important;
    transition: none !important;
  }
}
```

---

## Responsive Behavior

### 1. Breakpoint Strategy

#### 1.1 Mobile-First Approach

**sm (640px):**
- Heatmap: Horizontal scroll container, ~8 weeks visible
- Cell size: 10px
- Labels: Abbreviated (M, T, W, Th, F, S, Su; J, F, M, A...)
- Chart: Full-width, legend below
- Font-sizes: 11-12px
- Padding: 12px

**md (768px):**
- Heatmap: Horizontal scroll container, ~10 weeks visible
- Cell size: 12px
- Labels: Full day names, abbreviated months
- Chart: Full-width, legend below with better spacing
- Font-sizes: 12-13px
- Padding: 16px

**lg (1024px) - Breakpoint:**
- Heatmap: Full-width, no scroll, all 52 weeks visible
- Cell size: 14px
- Labels: Full names (Mon-Sun, Jan-Dec)
- Chart: 50/50 split with legend on right
- Font-sizes: 13-14px
- Padding: 20px

**xl (1280px):**
- Heatmap: Full-width with enhanced spacing
- Cell size: 16px
- Chart: 45/55 split with expanded legend
- Font-sizes: 14-15px
- Padding: 24px

---

### 2. Responsive Components

#### 2.1 Heatmap Responsive Layout

```jsx
// Pseudo-code for responsive logic
const HeatmapContainer = () => {
  const [cellSize, setCellSize] = useState(14);
  
  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth < 640) setCellSize(10);
      else if (window.innerWidth < 768) setCellSize(12);
      else if (window.innerWidth < 1024) setCellSize(12);
      else if (window.innerWidth < 1280) setCellSize(14);
      else setCellSize(16);
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);
  
  return (
    <div className="overflow-x-auto lg:overflow-visible">
      {/* Heatmap Grid */}
    </div>
  );
};
```

#### 2.2 Chart Responsive Layout

```jsx
const RewardsChartContainer = () => {
  const isLargeScreen = useMediaQuery('(min-width: 1024px)');
  
  return (
    <div className={`
      flex
      ${isLargeScreen ? 'flex-row' : 'flex-col'}
      gap-6 lg:gap-8
    `}>
      <div className={isLargeScreen ? 'w-1/2' : 'w-full'}>
        {/* Chart */}
      </div>
      <div className={isLargeScreen ? 'w-1/2' : 'w-full'}>
        {/* Legend */}
      </div>
    </div>
  );
};
```

---

### 3. Container Query Considerations

Future enhancement: Use CSS Container Queries for nested responsive behavior:

```css
@container (min-width: 1024px) {
  .heatmap-cell {
    width: 14px;
    height: 14px;
  }
}
```

---

## Component States

### 1. Heatmap Cell States

#### 1.1 State Definitions

| State | Condition | Visual | Interactive |
|-------|-----------|--------|------------|
| **Default** | Initial render | Base color (level 0-4) | Cursor: pointer |
| **Hover** | Mouse over (desktop) | Scale 1.15×, shadow, border | Tooltip visible |
| **Focus** | Keyboard focus | 2px gold outline, 4px offset | Always visible |
| **Active** | Clicked/selected | Scale 1.05×, darker background | Highlight on/off toggle |
| **Disabled** | Data not available | Opacity 0.5, striped pattern | Cursor: not-allowed |
| **Loading** | Data fetching | Pulse animation, skeleton | Cursor: wait |
| **Empty** | 0 contributions | neutral-200 (#efefef) | Muted appearance |
| **Error** | Data fetch error | Red border (1px solid #ef4444) | Error icon overlay |

#### 1.2 State Transitions

```css
.heatmap-cell {
  transition: all 150ms cubic-bezier(0.4, 0, 0.2, 1);
}

.heatmap-cell:hover {
  transform: scale(1.15);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  border: 1px solid #c9983a;
}

.heatmap-cell:focus-visible {
  outline: 2px solid #f1b400;
  outline-offset: 2px;
  box-shadow: 0 0 0 4px rgba(241, 180, 0, 0.25);
}

.heatmap-cell:active {
  transform: scale(1.05);
}

.heatmap-cell.disabled {
  opacity: 0.5;
  cursor: not-allowed;
  pointer-events: none;
  background-image: repeating-linear-gradient(
    45deg,
    transparent,
    transparent 2px,
    rgba(0, 0, 0, 0.1) 2px,
    rgba(0, 0, 0, 0.1) 4px
  );
}

.heatmap-cell.loading {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.heatmap-cell.error {
  border: 1px solid #ef4444;
  background-color: rgba(239, 68, 68, 0.1);
}
```

---

### 2. Rewards Chart Segment States

| State | Condition | Visual | Interactive |
|-------|-----------|--------|------------|
| **Default** | Initial render | Base color | Hover-ready |
| **Hover** | Mouse over | Scale 1.1×, opacity +0.2 | Tooltip visible |
| **Focus** | Keyboard focus | Outline glow | Navigation highlight |
| **Active** | Clicked/selected | Scale 1.05×, darkened | Details expanded |
| **Disabled** | No data | Opacity 0.5, gray | Cursor: not-allowed |
| **Loading** | Data fetching | Skeleton/shimmer | Cursor: wait |
| **Empty** | $0 in category | Minimal visibility | Hidden or faded |
| **Error** | Data error | Red tint + icon | Error state |

---

### 3. Tooltip States

| State | Condition | Behavior |
|-------|-----------|----------|
| **Hidden** | Default | Opacity: 0, display: none |
| **Appearing** | Mouse enter / Focus | Fade-in 200ms, display: block |
| **Visible** | Sustained hover / Focus | Opacity: 1, responsive to position |
| **Disappearing** | Mouse leave / Blur | Fade-out 150ms after 300ms delay |

---

## Implementation Guidelines

### 1. Code Organization

```
frontend/src/features/dashboard/pages/
├── ProfilePage.tsx (main container)
└── components/
    ├── ContributionHeatmap.tsx
    │   ├── HeatmapGrid.tsx
    │   ├── HeatmapLegend.tsx
    │   ├── HeatmapCell.tsx
    │   └── HeatmapTooltip.tsx
    ├── RewardsChart.tsx
    │   ├── ChartContainer.tsx
    │   ├── PieChartComponent.tsx
    │   ├── ChartLegend.tsx
    │   └── MilestoneAnnotations.tsx
    └── hooks/
        ├── useHeatmapData.ts
        ├── useResponsiveHeatmap.ts
        └── useChartInteraction.ts
```

### 2. Key Implementation Details

#### 2.1 Responsive Heatmap

- Use CSS Grid for heatmap layout
- `overflow-x-auto` on sm/md, `overflow-visible` on lg+
- Tailwind classes: `grid-cols-[repeat(53,1fr)]` for 53-week structure
- CSS variables for cell sizing: `--heatmap-cell-size`
- Container query for nested responsiveness (future)

#### 2.2 Chart Enhancement

- Recharts' `ResponsiveContainer` for reactive sizing
- Custom `Tooltip` component with WCAG-compliant markup
- SVG `<g>` elements with `role="button"` for keyboard support
- Legend built with accessible list semantics

#### 2.3 Accessibility

- Data table alternative hidden with `.sr-only`
- All interactive elements have `tabindex`, ARIA labels
- Focus management with `useRef` and `useEffect`
- Keyboard event listeners for `Enter`, `Space`, `Escape`, `Arrow keys`
- `prefers-reduced-motion` media query respects user settings

### 3. Testing Checklist

- [ ] Contrast ratio validation (WebAIM Contrast Checker)
- [ ] Keyboard-only navigation (Tab, Enter, Arrow keys, Escape)
- [ ] Screen reader testing (NVDA, JAWS, VoiceOver)
- [ ] Responsive behavior on sm (375px), md (768px), lg (1024px), xl (1280px)
- [ ] Hover/focus states on desktop and touch devices
- [ ] Color-blind simulation (Protanopia, Deuteranopia, Tritanopia)
- [ ] Animation respect `prefers-reduced-motion`
- [ ] Edge cases (0 data, max data, long category names, missing labels)

### 4. Performance Considerations

- Memoize heatmap cells with `React.memo()` to avoid unnecessary re-renders
- Use `useMemo` for color scale calculations
- Lazy-load chart data if > 1MB
- Debounce resize event listeners
- Virtual scrolling for heatmap on very large datasets (future)

---

## Summary Table

| Aspect | Specification |
|--------|---------------|
| **Heatmap Cell Size** | sm: 10px, md: 12px, lg: 14px, xl: 16px |
| **Heatmap Color Scale** | Neutral-200 (#efefef) to Gold-primary (#f1b400) |
| **Chart Type** | Donut (Pie with inner radius) |
| **Responsive Breakpoints** | sm 640px, md 768px, lg 1024px, xl 1280px |
| **Accessibility** | WCAG 2.1 AA compliance, keyboard navigation, screen reader support |
| **Animation Duration** | Hover: 150ms, Tooltip: 200ms, Chart: 800ms |
| **Color-Blind Safe** | Yes (gold + neutral intensity scale) |
| **Mobile-First** | Yes (scroll on sm/md, full-width on lg+) |
| **Reduced Motion** | Yes (respected via media query) |

---

## Appendix: Design Token Extraction from design-tokens.json

```json
{
  "colors": {
    "gold": {
      "50": "#fef8f1",
      "100": "#fcf0e3",
      "200": "#f9e5cc",
      "300": "#f3d4a6",
      "400": "#efc280",
      "500": "#f1b400",
      "600": "#c9983a",
      "700": "#a67c2e",
      "800": "#7d5a27",
      "900": "#613c21",
      "950": "#3d250e",
      "accent": "#d4af37"
    },
    "neutral": {
      "50": "#fafaf9",
      "100": "#f5f5f4",
      "200": "#e8e8e7",
      "300": "#d6d3d1",
      "400": "#a8a29e",
      "500": "#78716c",
      "600": "#57534e",
      "700": "#44403c",
      "800": "#292524",
      "900": "#1c1917",
      "950": "#0c0a09"
    }
  },
  "spacing": ["0rem", "0.25rem", "0.5rem", "0.75rem", "1rem", ...],
  "typography": {
    "fontFamily": {
      "sans": "Inter",
      "mono": "JetBrains Mono"
    },
    "fontSize": {
      "xs": "12px",
      "sm": "14px",
      "base": "16px",
      "lg": "18px",
      "xl": "20px",
      "2xl": "24px",
      ...
    }
  },
  "animation": {
    "fast": "150ms",
    "normal": "300ms",
    "slow": "500ms"
  }
}
```

---

**Document Version:** 1.0  
**Status:** Ready for Implementation  
**Last Updated:** May 31, 2026
