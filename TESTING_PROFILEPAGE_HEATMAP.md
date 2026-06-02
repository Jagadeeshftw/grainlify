# ProfilePage Heatmap & Rewards Chart - Testing Guide

**Date:** May 31, 2026  
**Version:** 1.0  
**Status:** Pre-Commit Testing

---

## Testing Checklist

### 1. Responsive Behavior Testing

#### 1.1 Mobile View (sm: 375-640px)
- [ ] Heatmap: Horizontal scroll container visible (not full-width)
- [ ] Heatmap: Cell size approximately 10px
- [ ] Heatmap: Day labels abbreviated (M, T, W, Th, F, Sa, Su)
- [ ] Heatmap: Month labels abbreviated (J, F, M, A, M, J, J, A, S, O, N, D)
- [ ] Rewards chart: Legend displayed below chart (stacked layout)
- [ ] Rewards chart: Legend items single-column layout
- [ ] Font sizes responsive: sm (text-xs/text-sm)
- [ ] Touch friendly: tap targets at least 44x44px
- [ ] No horizontal overflow of main content

#### 1.2 Tablet View (md: 768px)
- [ ] Heatmap: Horizontal scroll container still visible
- [ ] Heatmap: Cell size approximately 12px
- [ ] Heatmap: Day labels full names (Mon-Sun)
- [ ] Heatmap: Month labels full abbreviated (Jan-Dec)
- [ ] Rewards chart: Legend below chart (stacked)
- [ ] Rewards chart: Better spacing between legend items
- [ ] Font sizes: text-sm (14px)
- [ ] Padding: 16px or 24px
- [ ] Readable on landscape orientation

#### 1.3 Desktop View (lg: 1024px)
- [ ] Heatmap: Full-width, NO horizontal scroll
- [ ] Heatmap: ALL 52 weeks visible
- [ ] Heatmap: Cell size approximately 14px
- [ ] Heatmap: Day labels full names (Mon-Sun)
- [ ] Heatmap: All month labels visible
- [ ] Rewards chart: Side-by-side layout (chart left, legend right, 50/50 split)
- [ ] Rewards chart: Legend items properly aligned
- [ ] Font sizes: text-base (16px)
- [ ] Padding: 24px
- [ ] Milestones visible

#### 1.4 Large Desktop (xl: 1280px)
- [ ] Heatmap: Full-width with enhanced spacing
- [ ] Heatmap: Cell size approximately 16px
- [ ] Rewards chart: Enhanced spacing and visual hierarchy
- [ ] All elements properly scaled
- [ ] No excessive whitespace

---

### 2. Component State Testing

#### 2.1 Heatmap Cell States
- [ ] **Empty state (level 0):** Light gray color, no sparkle
- [ ] **Low activity (level 1):** Gold 35% opacity
- [ ] **Medium activity (level 2):** Gold 55% opacity
- [ ] **High activity (level 3):** Gold 75% opacity
- [ ] **Maximum activity (level 4+):** Gold gradient + sparkle animation ✨
- [ ] **Hover state:** Scale 1.15x, shadow increases, border visible
- [ ] **Focus state (keyboard):** 2px gold outline with 4px offset
- [ ] **Focused cell ring:** Gold ring visible and persistent
- [ ] **Tooltip:** Appears on hover/focus with date and contribution count

#### 2.2 Rewards Chart States
- [ ] **Empty state:** Trophy icon with "No rewards yet" message
- [ ] **Pie chart:** Renders with animation (800ms ease-out)
- [ ] **Chart segment hover:** Opacity change, other segments fade to 0.6
- [ ] **Legend item hover:** Scale 1.05x, background highlight
- [ ] **Legend item click:** Toggle selected state (ring visible)
- [ ] **Tooltip on segment:** Shows category, amount, percentage
- [ ] **Milestones:** Display below legend with proper styling
- [ ] **Center total:** Shows "$XK USD Earned"

---

### 3. Accessibility Testing (WCAG 2.1 AA)

#### 3.1 Keyboard Navigation
- [ ] **Tab key:** Can navigate through all interactive elements (cells, legend items)
- [ ] **Shift+Tab:** Can reverse navigate
- [ ] **Enter/Space:** Activate selected cell/segment (toggle detail view)
- [ ] **Escape:** Close tooltip/detail view
- [ ] **Arrow keys (optional):** Move between grid cells
- [ ] **Focus management:** Focus visible at all times, not lost
- [ ] **Focus order:** Logical left-to-right, top-to-bottom order

#### 3.2 Screen Reader Support
- [ ] **Heatmap title:** Announced as "365 contributions last year" or similar
- [ ] **Heatmap description:** SR users hear description of how to navigate
- [ ] **Cell labels:** Each cell has aria-label with date and contribution count
- [ ] **Data table alternative:** `<table class="sr-only">` contains all heatmap data
- [ ] **Chart title:** "Rewards Distribution 2025"
- [ ] **Chart description:** Users understand chart purpose via description
- [ ] **Legend items:** Each item has aria-label with category, amount, percentage
- [ ] **Data table alternative:** `<table class="sr-only">` contains all chart data
- [ ] **Milestones:** Properly labeled with icon and text
- [ ] **NVDA/JAWS/VoiceOver:** Test with actual screen reader

#### 3.3 Contrast Ratios (4.5:1 minimum, 7:1 preferred)
- [ ] Text on light backgrounds: ≥ 4.5:1
- [ ] Text on dark backgrounds: ≥ 4.5:1
- [ ] Heatmap cell borders: ≥ 3:1 (UI element)
- [ ] Rewards legend items: ≥ 4.5:1
- [ ] Tooltip text: ≥ 4.5:1
- [ ] Use WebAIM Contrast Checker: https://webaim.org/resources/contrastchecker/

#### 3.4 Color-Blindness Testing
- [ ] **Deuteranopia (green-blind):** Heatmap scale visible (gold + neutral)
- [ ] **Protanopia (red-blind):** Heatmap scale visible (gold + neutral)
- [ ] **Tritanopia (blue-yellow blind):** Colors distinguishable
- [ ] **Sparkle animation:** Acts as visual supplement to color
- [ ] Use simulator: https://www.color-blindness.com/coblis-color-blindness-simulator/

#### 3.5 Motion & Animation
- [ ] **prefers-reduced-motion:** Animations disabled on system level
- [ ] **Sparkle animation:** Respects prefers-reduced-motion
- [ ] **Hover effects:** Smooth transitions (150-300ms)
- [ ] **Focus effects:** Visible without animation dependency
- [ ] No flickering or strobe effects

---

### 4. Visual Design Testing

#### 4.1 Color Accuracy
- [ ] Neutral-200: #efefef (light gray background)
- [ ] Gold-600: #c9983a (opacity levels: 35%, 55%, 75%)
- [ ] Gold-primary: #f1b400 (maximum intensity, accent)
- [ ] Gold-accent: #d4af37 (border/highlight)
- [ ] Neutral-500: #78716c (labels, text)
- [ ] Neutral-700: #292524 (primary text)

#### 4.2 Typography
- [ ] Title font: Bold/Black, responsive size (sm: 18px → lg: 24px+)
- [ ] Body text: Regular, 12-14px responsive
- [ ] Label text: Medium, 11-13px responsive
- [ ] Line height: Appropriate (1.4-1.6 for readability)
- [ ] Font weights: 400 (regular), 500 (medium), 600 (semibold), 700+ (bold/black)

#### 4.3 Spacing & Alignment
- [ ] Heatmap cell gap: 1-2px (consistent)
- [ ] Month gap: Proper spacing (no overlaps)
- [ ] Legend items: Consistent spacing (12-16px gaps)
- [ ] Milestone cards: Proper padding (12-16px)
- [ ] Responsive padding: Scale with breakpoints

#### 4.4 Shadows & Elevation
- [ ] Heatmap cells: Subtle shadows increase with intensity level
- [ ] Hover states: Shadows increase for elevation effect
- [ ] Tooltips: Strong shadow for visibility (shadow-lg)
- [ ] Legend items: Subtle background elevation

---

### 5. Edge Cases Testing

#### 5.1 Data Edge Cases
- [ ] **Zero contributions:** All cells empty/neutral, text shows "0"
- [ ] **All max contributions:** All cells gold, sparkles on all
- [ ] **Single category rewards:** Shows 100% in one segment
- [ ] **Missing data:** Graceful degradation (skeleton/placeholder)
- [ ] **Very long category names:** Text truncation/wrapping
- [ ] **Large numbers:** Formatting with commas ($1,234,567)

#### 5.2 Browser Compatibility
- [ ] Chrome/Edge: Latest version
- [ ] Firefox: Latest version
- [ ] Safari: Latest version (iOS + macOS)
- [ ] Mobile browsers: Chrome, Safari, Firefox mobile
- [ ] CSS Grid support: Fallbacks if needed
- [ ] Flexbox wrapping: Works as expected

#### 5.3 Performance
- [ ] No console errors
- [ ] No console warnings
- [ ] Page load time: < 2s
- [ ] Component render time: < 100ms
- [ ] Smooth scrolling in heatmap
- [ ] No lag on hover/focus

---

### 6. Dark Mode Testing

#### 6.1 Dark Mode Specific
- [ ] Text colors: Adjust for contrast (light text on dark background)
- [ ] Background opacity: Visible on dark backgrounds
- [ ] Heatmap cells: Properly colored in dark theme
- [ ] Rewards chart: Properly colored in dark theme
- [ ] Tooltips: Dark background with light text
- [ ] Legend items: Proper contrast in dark mode
- [ ] Transitions: Smooth between light and dark

#### 6.2 Light Mode Testing
- [ ] All text readable (≥4.5:1 contrast)
- [ ] Background colors: Not too bright
- [ ] Heatmap cells: Properly colored in light theme
- [ ] Tooltips: Light background with dark text
- [ ] No glare or eye strain

---

### 7. Interaction Testing

#### 7.1 Hover Interactions
- [ ] Heatmap cell hover: Scale, shadow, border changes
- [ ] Tooltip appears: Smooth fade-in (200ms)
- [ ] Tooltip positioning: Above cell, not cut off
- [ ] Legend item hover: Background highlight, scale
- [ ] Chart segment hover: Opacity change, fade adjacent segments
- [ ] Escape hover: No cursor change on non-interactive elements

#### 7.2 Click Interactions
- [ ] Heatmap cell click: Toggle detail view (if applicable)
- [ ] Legend item click: Toggle visibility or show details
- [ ] Chart segment click: Show details or highlight
- [ ] Milestones: Clickable for more info (if applicable)
- [ ] No double-click issues

#### 7.3 Focus Interactions
- [ ] Tab through heatmap: All cells focusable
- [ ] Focused cell: Outline visible, persistent
- [ ] Tab through legend: All items focusable
- [ ] Enter on legend item: Activate functionality
- [ ] Focus trap: Not trapped in modal/tooltip

---

### 8. Responsiveness Visual Regression

#### 8.1 Screenshot Comparison (Before/After)
- [ ] Mobile view (375px): Compare with design spec
- [ ] Tablet view (768px): Compare with design spec
- [ ] Desktop view (1024px): Compare with design spec
- [ ] Large desktop (1280px): Compare with design spec
- [ ] No layout shifts or CLS (Cumulative Layout Shift)

#### 8.2 Smooth Responsive Transitions
- [ ] No jump at breakpoint boundaries
- [ ] Smooth resizing from 375 → 640 → 768 → 1024 → 1280px
- [ ] Charts/heatmaps reflow properly
- [ ] Text/labels scale smoothly

---

### 9. Documentation & Handoff

#### 9.1 Code Documentation
- [ ] Component comments: Purpose, props, usage
- [ ] Accessibility annotations: ARIA labels documented
- [ ] Tailwind classes: Complex utilities explained
- [ ] Responsive logic: Breakpoint conditions clear

#### 9.2 Design Spec Compliance
- [ ] All design spec requirements met: ✓
- [ ] Color scale accurate: ✓
- [ ] Typography matches spec: ✓
- [ ] Spacing/padding matches spec: ✓
- [ ] Accessibility requirements met: ✓

#### 9.3 Handoff Package
- [ ] design/profilepage-visualizations.md: Complete and updated
- [ ] Component exports: Properly documented
- [ ] Figma mockups (if applicable): Ready for QA
- [ ] Change log: Documented improvements

---

### 10. Sign-Off

- [ ] **Designer Review:** Approved ✓
- [ ] **Accessibility Review:** WCAG 2.1 AA compliant ✓
- [ ] **Performance Review:** No performance issues ✓
- [ ] **Cross-browser Review:** Works on all major browsers ✓
- [ ] **Ready for Merge:** All tests passed ✓

---

## Test Execution Log

### Tester: [Your Name]
### Date: [Date]
### Results: PASS / FAIL

**Notes:**
[Add any findings, issues, or deviations from spec]

---

## Known Issues / Future Enhancements

1. **Virtual scrolling:** Consider for 365+ day heatmaps
2. **Gesture support:** Pinch-zoom on mobile (optional)
3. **Chart interactivity:** Click to drill down (future phase)
4. **Custom date range:** Year/month selector (future phase)
5. **Data export:** CSV/PDF export (future phase)

---

**Reference Documents:**
- Design Specification: [design/profilepage-visualizations.md](design/profilepage-visualizations.md)
- Components: [ContributionHeatmap.tsx](frontend/src/features/dashboard/components/ContributionHeatmap.tsx), [RewardsChart.tsx](frontend/src/features/dashboard/components/RewardsChart.tsx)
- ProfilePage: [ProfilePage.tsx](frontend/src/features/dashboard/pages/ProfilePage.tsx)
