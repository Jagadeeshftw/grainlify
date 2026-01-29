# Screenshot Guide for Activity Item Alignment PR

## Overview

This guide helps capture before/after screenshots to demonstrate the alignment fixes. All screenshots should be taken from the maintainers dashboard activity feed.

---

## Screenshots to Capture

### 1. Issue Activity Item (Desktop View)
**File**: `issue-activity-desktop.png`

**What to show**:
- Single issue activity item in full width
- Icon, badge, title, and label all aligned horizontally on first row
- Time info on second row
- Review button on the right

**How to capture**:
```
1. Navigate to /dashboard (login as maintainer)
2. Go to Maintainers tab
3. Expand a project with activity
4. Capture one issue activity item
5. Focus on: Icon → Badge → Title alignment
```

**Alignment Points to Highlight**:
- ✓ Golden circle icon (8x8) is vertically centered
- ✓ Badge #123 is on same line as icon
- ✓ Title text "Fix button alignment" starts at same vertical center
- ✓ Review button is right-aligned

---

### 2. PR Activity Item (Desktop View)
**File**: `pr-activity-desktop.png`

**What to show**:
- Single PR activity item
- PR icon, badge, title all aligned
- No margin offset on PR icon
- Consistent height with issue items above

**How to capture**:
```
1. From same activity list, capture one PR item
2. Compare vertical alignment with issue items
3. Ensure no offset or spacing difference
```

**Alignment Points to Highlight**:
- ✓ PR icon (5x5) is vertically centered
- ✓ Badge #456 is on same line as icon
- ✓ Title starts at same baseline as issue item
- ✓ No mt-0.5 margin creating offset

---

### 3. Mixed Activity Feed (Multiple Items)
**File**: `activity-feed-mixed.png`

**What to show**:
- 3-4 activity items: mix of issues and PRs
- Consistent alignment across all items
- Icon, badge, and title perfectly lined up
- Proper spacing between items

**How to capture**:
```
1. Expand activity section to show multiple items
2. Include both issue and PR types
3. Capture full width to show Review button alignment
4. Ensure good contrast for clarity
```

**Alignment Points to Highlight**:
- ✓ All icons aligned vertically with each other
- ✓ All badges aligned on same line
- ✓ All titles start at same baseline
- ✓ No visual jitter between item types

---

### 4. Responsive Mobile View
**File**: `activity-item-mobile.png`

**What to show**:
- Activity item on mobile width (375px)
- Icon and badge on same horizontal line
- Title text wrapping properly
- Alignment maintained on small screen

**How to capture**:
```
1. Open DevTools (F12)
2. Toggle device toolbar (mobile view)
3. Set width to 375px (iPhone SE)
4. Navigate to activity feed
5. Capture one complete activity item
```

**Alignment Points to Highlight**:
- ✓ Icon and badge centered vertically
- ✓ Title wraps without misaligning
- ✓ Badge doesn't push icon down
- ✓ Review button stacks properly

---

### 5. Responsive Tablet View
**File**: `activity-item-tablet.png`

**What to show**:
- Activity item on tablet width (768px)
- All elements aligned with proper spacing
- Text wrapping with alignment preserved

**How to capture**:
```
1. In DevTools, set width to 768px
2. Capture activity item with wrapped text
3. Show proper alignment maintained
```

**Alignment Points to Highlight**:
- ✓ Icon and badge remain aligned
- ✓ Title wraps while maintaining baseline
- ✓ Responsive behavior working correctly

---

### 6. Dark Theme View
**File**: `activity-item-dark-theme.png`

**What to show**:
- Activity item in dark theme
- Text contrast and alignment clear
- Icon colors properly visible

**How to capture**:
```
1. Toggle to dark theme in app
2. Capture 1-2 activity items
3. Ensure readability
```

**Alignment Points to Highlight**:
- ✓ All alignment maintained in dark theme
- ✓ No theme-specific misalignment

---

### 7. Light Theme View
**File**: `activity-item-light-theme.png`

**What to show**:
- Activity item in light theme
- Text contrast and alignment clear
- Icon colors properly visible

**How to capture**:
```
1. Toggle to light theme
2. Capture same items as dark theme
3. Ensure contrast is good
```

**Alignment Points to Highlight**:
- ✓ All alignment consistent with dark theme

---

## Before/After Comparison

### Create a Side-by-Side Comparison

**File**: `alignment-before-after.md` (or composite image)

Include:
```
BEFORE (with issues):
[Icon ↓offset] [Badge] [Title ↑offset]
│           │          │
└─ mt-0.5   └─ baseline└─ pt-0.5
           (misaligned)

AFTER (fixed):
[Icon] [Badge] [Title]
│      │       │
└──────┴───────┴─ items-center (aligned)
    (perfectly aligned)
```

---

## Recommended Image Specifications

### Screenshot Settings
- **Resolution**: Capture at 100% zoom
- **Format**: PNG (lossless)
- **Dimensions**: Full browser width shown
- **Quality**: High contrast, readable text
- **Lighting**: Standard screen capture (no manual adjustments)

### File Organization
```
pr-screenshots/
├── issue-activity-desktop.png
├── pr-activity-desktop.png
├── activity-feed-mixed.png
├── activity-item-mobile.png
├── activity-item-tablet.png
├── activity-item-dark-theme.png
├── activity-item-light-theme.png
└── alignment-comparison.md
```

---

## How to Include in PR

### In PR Description

Add section like:
```markdown
## Visual Evidence

### Desktop View - Issue Item
![Issue Activity Item](./pr-screenshots/issue-activity-desktop.png)
*Icon, badge, and title perfectly aligned*

### Desktop View - PR Item  
![PR Activity Item](./pr-screenshots/pr-activity-desktop.png)
*Consistent alignment with issue items*

### Mixed Feed
![Activity Feed](./pr-screenshots/activity-feed-mixed.png)
*All items aligned consistently*

### Mobile View
![Mobile Responsive](./pr-screenshots/activity-item-mobile.png)
*Alignment maintained on small screens*

### Theme Compatibility
- Dark theme: ![Dark](./pr-screenshots/activity-item-dark-theme.png)
- Light theme: ![Light](./pr-screenshots/activity-item-light-theme.png)
```

---

## Testing Checklist Before Screenshots

- [ ] Dev server running at localhost:5174
- [ ] Logged in as maintainer user
- [ ] Have activity items with both issues and PRs
- [ ] Dark and light theme switching works
- [ ] Mobile view responsive (DevTools working)
- [ ] No console errors visible
- [ ] All components rendering correctly

---

## Common Issues & Solutions

### Issue: Icons don't look aligned
**Solution**: Zoom to 100%, ensure DevTools not affecting layout

### Issue: Badge seems misaligned
**Solution**: Check that Tailwind CSS is fully loaded, refresh page

### Issue: Mobile view shows different alignment
**Solution**: Ensure `items-center` class is working on main container

### Issue: Hard to see alignment in screenshot
**Solution**: Add subtle crosshair or highlight to show alignment line

---

## Annotation Tips

Consider adding subtle annotations to screenshots:
- Horizontal line showing icon/badge alignment
- Arrow pointing to the removed `mt-0.5` location
- Label highlighting the centered elements

Tools for annotations:
- MacOS: Preview (Mark up feature)
- Windows: Snipping Tool or Paint
- Cross-platform: GIMP, Photoshop, or online tools

---

## Expected Results

All screenshots should clearly show:

✅ Icons and badges on the same horizontal line
✅ Title text at same baseline as icon/badge row
✅ No visual offset or misalignment
✅ Consistent appearance across issue and PR types
✅ Responsive layout maintained at all screen sizes
✅ Both dark and light themes displaying correctly

---

## Questions While Capturing?

If alignment still looks off:
1. Hard refresh (Ctrl+Shift+R)
2. Clear browser cache
3. Check that CSS changes are in place
4. Verify Tailwind purge/build is correct
5. Review git diff for any missed changes

---

**Ready to capture!** Use this guide to create compelling visual evidence for your PR.
