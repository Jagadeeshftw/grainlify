# Dark-Mode Coverage Implementation Checklist

**Status:** Phase 1 Complete, Phase 2 Ready  
**Last Updated:** May 31, 2026  
**Estimated Effort (Phase 2):** 8-12 hours

---

## Quick Start for Developers

### Apply Semantic Tokens to a Component

**Pattern:**

```typescript
import { useTheme } from '../contexts/ThemeContext';
import { DARK_MODE_TOKENS, FOCUS_RING_SPEC } from '../contexts/ThemeContext';

export function MyComponent() {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div className={`
      ${isDark ? DARK_MODE_TOKENS.background.surfaceSecondary : 'bg-white'}
      ${isDark ? DARK_MODE_TOKENS.text.primary : 'text-[#2d2820]'}
      ${FOCUS_RING_SPEC.tailwind(isDark)}
      transition-colors duration-300
    `}>
      {/* Content */}
    </div>
  );
}
```

**Semantic Colors:**

- **Background (dark):** `DARK_MODE_TOKENS.background.surfaceSecondary` → `#2d2820`
- **Text Primary (dark):** `DARK_MODE_TOKENS.text.primary` → `#f5f5f5`
- **Text Secondary (dark):** `DARK_MODE_TOKENS.text.secondary` → `#d4d4d4`
- **Accent (dark):** `DARK_MODE_TOKENS.semantic.accentPrimary` → `#c9983a`
- **Focus Ring (dark):** `FOCUS_RING_SPEC.tailwind(isDark)` → outline `#f1b400`

---

## Phase 2 Task Breakdown

### Task 1: Implement Focus Rings on Shared Components (2-3 hours)

**Components to Update (18 total):**

#### 🟢 Already Compliant (Global CSS):

- Button (inherited from global `:focus-visible`)
- Input/DatePicker (inherited)
- Dropdown (inherited)
- Modal (inherited)
- Select/Textarea (inherited)

#### 🟡 Need Focus Ring Integration (Manual):

1. **GlassDropdown** – Add to button & menu items
   - File: `frontend/src/shared/components/GlassDropdown.tsx`
   - Lines 31-36 (button), 57-62 (menu items)
   - Apply: `${FOCUS_RING_SPEC.tailwind(isDark)}`
   - ⏱️ 10 min

2. **IssueCard** – Add to clickable container
   - File: `frontend/src/shared/components/ui/IssueCard.tsx`
   - Line: Root container
   - Apply: Ensure `role="button"` or tabindex + focus handler
   - ⏱️ 15 min

3. **SearchModal** – Add to suggestion pills & close button
   - File: `frontend/src/shared/components/SearchModal.tsx`
   - Lines 51-52 (close button), 113-150 (suggestion pills)
   - Apply: `:focus-visible` outline on buttons
   - ⏱️ 10 min

4. **NotificationsDropdown** – Add to menu items
   - File: `frontend/src/shared/components/NotificationsDropdown.tsx`
   - Menu item buttons: Add `focus:outline-...`
   - ⏱️ 15 min

5. **UserProfileDropdown** – Add to menu items
   - File: `frontend/src/shared/components/UserProfileDropdown.tsx`
   - Menu item buttons: Add `focus:outline-...`
   - ⏱️ 15 min

6. **RoleSwitcher** – Add to role buttons
   - File: `frontend/src/shared/components/RoleSwitcher.tsx`
   - Button group: Add focus ring class
   - ⏱️ 10 min

7. **FilterDropdown** – Add to filter buttons & checkboxes
   - File: `frontend/src/shared/components/FilterDropdown.tsx`
   - Checkbox + button labels: Add focus handlers
   - ⏱️ 15 min

8. **Toast** – Add to close button (if present)
   - File: `frontend/src/shared/components/Toast.tsx`
   - Close button: Add focus ring
   - ⏱️ 5 min

9-13. **Skeleton Loaders** (ActivityItemSkeleton, ChartSkeleton, IssueCardSkeleton, PRRowSkeleton, SkeletonLoader)

- Files: `frontend/src/shared/components/*Skeleton.tsx`
- Task: Add `aria-busy="true"` role="status" (no focus, these are non-interactive)
- ⏱️ 20 min total

14-18. **Other Components** (LanguageIcon, index.ts, Blank slots):

- Status: No focus rings needed (SVG icon, exports, reserved)
- ✅ Skip

**Total Time:** 2-3 hours

---

### Task 2: Audit & Fix Remaining Pages (4-6 hours)

#### **Auth Pages (3 pages, 1 hour)**

_Files:_

- `frontend/src/features/auth/pages/SignInPage.tsx`
- `frontend/src/features/auth/pages/SignUpPage.tsx`
- `frontend/src/features/auth/pages/AuthCallbackPage.tsx`

_Checklist:_

- [ ] Form background: Light `bg-white/30` → Dark `${DARK_MODE_TOKENS.background.surfaceSecondary}`
- [ ] Input labels: Light `text-[#7a6b5a]` → Dark `${DARK_MODE_TOKENS.text.secondary}`
- [ ] Button: Inherit from global focus ring
- [ ] Error messages: Light `text-red-600` → Dark `text-red-400`
- [ ] Links: Light `text-[#a2792c]` → Dark `text-[#e8c77f]`

_Template:_

```typescript
const isDark = theme === "dark";
const bgClass = isDark
  ? DARK_MODE_TOKENS.background.surfaceSecondary
  : "bg-white/30";
const textClass = isDark ? DARK_MODE_TOKENS.text.secondary : "text-[#7a6b5a]";
```

---

#### **Blog Pages (1 page, 1 hour)**

_File:_

- `frontend/src/features/blog/pages/BlogPage.tsx`

_Checklist:_

- [ ] Article body text: `text-[#d4d4d4]` (dark)
- [ ] Code blocks: Background `bg-[#1a1714]`, text `text-[#f5f5f5]`
- [ ] Blockquotes: Border `border-l-[#c9983a]`, text `text-[#d4d4d4]`
- [ ] Links: `text-[#e8c77f]` with underline
- [ ] Headings: `text-[#f5f5f5]`

---

#### **Leaderboard Pages (1 page, 1 hour)**

_File:_

- `frontend/src/features/leaderboard/pages/LeaderboardPage.tsx`

_Checklist:_

- [ ] Table header: Dark `bg-[#3a3428]`, light `bg-[#f5f5f5]`
- [ ] Table rows: Dark `bg-[#2d2820]/50`, light `bg-white/20`
- [ ] Striped rows: Alternate opacity for readability
- [ ] Podium: Gold gradient in both themes (adjust brightness)
- [ ] Text: Use `DARK_MODE_TOKENS.text.secondary` (dark)
- [ ] Rank number: Accent color `#c9983a` (dark)

---

#### **Maintainers Pages (1 page, 1 hour)**

_File:_

- `frontend/src/features/maintainers/pages/MaintainersPage.tsx`

_Checklist:_

- [ ] Dashboard tabs: Dark background `bg-[#2d2820]`, borders `border-white/10`
- [ ] Active tab: Underline `border-b-[#c9983a]`
- [ ] Cards: Dark `bg-white/[0.08]`, light `bg-white/[0.15]`
- [ ] Text: Use standard semantic tokens
- [ ] Buttons: Inherit focus ring

---

#### **Settings Pages (1 page, 1 hour)**

_File:_

- `frontend/src/features/settings/pages/SettingsPage.tsx`

_Checklist:_

- [ ] Form inputs: Dark `bg-white/10 border-white/15`, light `bg-white/40 border-white/30`
- [ ] Input labels: `DARK_MODE_TOKENS.text.secondary`
- [ ] Toggle switch: Background dark `bg-white/10` (off), `bg-[#c9983a]` (on)
- [ ] Radio buttons: Checked state uses gold accent
- [ ] Checkboxes: Checked state uses gold accent
- [ ] Save/Cancel buttons: Inherit focus ring + semantic colors

---

#### **Sub-Pages (13 pages, 2-3 hours)**

_Files:_

- `frontend/src/features/dashboard/pages/SearchPage.tsx`
- `frontend/src/features/dashboard/pages/BrowsePage.tsx`
- `frontend/src/features/dashboard/pages/EcosystemsPage.tsx`
- `frontend/src/features/dashboard/pages/EcosystemDetailPage.tsx`
- `frontend/src/features/dashboard/pages/ContributorsPage.tsx`
- `frontend/src/features/dashboard/pages/IssueDetailPage.tsx`
- `frontend/src/features/dashboard/pages/OpenSourceWeekPage.tsx`
- `frontend/src/features/dashboard/pages/OpenSourceWeekDetailPage.tsx`
- `frontend/src/features/landing/pages/LandingPage.tsx` (check Navbar/Hero)
- - 3 admin/complex pages (variations already audited)

_Unified Checklist for List/Grid Pages:_

- [ ] Container: `bg-gradient-to-br from-[#1a1714]` (dark)
- [ ] Card backgrounds: `bg-white/[0.08]` (dark) or `bg-[#2d2820]`
- [ ] Card text: `text-[#f5f5f5]` (headings), `text-[#d4d4d4]` (body)
- [ ] Empty state: Illustration visible, text `text-[#8b7a6a]`
- [ ] Loading state: Skeleton uses `bg-white/10`, pulse animation
- [ ] Hover state: Cards use `hover:bg-white/15` (dark)
- [ ] Buttons: Inherit focus ring
- [ ] Links: Gold accent `text-[#e8c77f]` (dark)
- [ ] Badges/Tags: Dark background `bg-white/10`, light text `text-[#f5f5f5]`

---

### Task 3: Edge Cases & Responsive Testing (2-3 hours)

#### **Empty States (30 min)**

_Files to search:_

```bash
grep -r "empty\|no data\|no results" frontend/src --include="*.tsx"
```

_Fixes:_

- [ ] Illustration/icon color: Use `DARK_MODE_TOKENS.text.tertiary` for outlines
- [ ] Heading: `DARK_MODE_TOKENS.text.primary`
- [ ] Description: `DARK_MODE_TOKENS.text.secondary`
- [ ] CTA button: Inherit focus ring + semantic accent

---

#### **Charts & Data Visualizations (45 min)**

_Files:_

- `frontend/src/features/dashboard/pages/DataPage.tsx`
- Chart containers in other dashboard pages

_Fixes:_

- [ ] Chart background: `bg-[#2d2820]` (dark)
- [ ] Chart grid lines: `stroke="rgba(255, 255, 255, 0.05)"` (nearly invisible)
- [ ] Axis labels: `fill="#d4d4d4"` (dark)
- [ ] Legend text: `fill="#d4d4d4"` (dark)
- [ ] Series colors: Ensure 3:1 contrast against `#2d2820`
  - Use provided palette: Gold `#c9983a`, Green `#22c55e`, Red `#ef4444`, Blue `#3b82f6`
- [ ] Tooltip: Background `bg-[#1a1714]`, text `text-[#f5f5f5]`, border `border-[#c9983a]`

---

#### **Responsive Breakpoints (45 min)**

_Test Dark Mode on All Breakpoints:_

| Breakpoint  | Devices | Focus                             |
| ----------- | ------- | --------------------------------- |
| sm (640px)  | Mobile  | Dropdowns/modals fit, not cut off |
| md (768px)  | Tablet  | Two-column layouts readable       |
| lg (1024px) | Desktop | Glass effects visible, no flicker |
| xl (1280px) | Wide    | Spacing/typography balanced       |

_Test Matrix:_

- [ ] 375px (iPhone SE): Text readability, button size
- [ ] 640px (sm): Modal max-width, dropdown positioning
- [ ] 768px (iPad): Multi-column layout, sidebar
- [ ] 1024px (lg): Full desktop, hero sections
- [ ] 1440px (xl): Extra space, large dashboards

---

### Task 4: Accessibility Testing (1-2 hours)

#### **Keyboard Navigation (45 min)**

1. [ ] Disable mouse; use Tab key only
2. [ ] Test every page with dark mode enabled
3. [ ] Verify focus visible on all interactive elements
4. [ ] Verify tab order is logical (left-to-right, top-to-bottom)
5. [ ] Test Skip Link (if present) in dark mode
6. [ ] Test Escape key on modals/dropdowns
7. [ ] Document any issues in GitHub issue

---

#### **Screen Reader Testing (45 min)**

_Tools:_ VoiceOver (macOS), NVDA (Windows), or online scanner

1. [ ] Dark mode toggle: Announces "Toggle theme" + current state
2. [ ] Form labels: Read aloud with input value
3. [ ] Error messages: Announced immediately after input
4. [ ] Image alt text: Dark mode doesn't hide images
5. [ ] Skeleton loaders: `aria-busy="true"` announced (Phase 1 ✅)
6. [ ] Buttons: Label + purpose clear

---

#### **Contrast Checking (20 min)**

_Tools:_

- WebAIM Contrast Checker: https://webaim.org/resources/contrastchecker/
- WAVE Browser Extension: https://www.webaccessibility.org/articles/contrast-checker/
- Lighthouse DevTools: Built-in WCAG 2.1 AA audit

_Steps:_

1. [ ] Open production dark mode page
2. [ ] Use eyedropper to sample colors
3. [ ] Check text on backgrounds meet 4.5:1 ratio
4. [ ] Check UI elements meet 3:1 ratio
5. [ ] Document any failures

---

#### **Automated Testing (Optional)**

```bash
# Run accessibility audit (requires axe-core or similar)
npm run test:a11y --dark-mode

# Or use Cypress + axe-core
npx cypress run --config video=false --spec "**/*.cy.a11y.ts"
```

---

## Acceptance Criteria

### Phase 2 Complete When:

**Code Quality:**

- [ ] All 18 shared components have focus rings (automated or inherited)
- [ ] All 24 pages have semantic dark backgrounds
- [ ] All user-facing text uses `DARK_MODE_TOKENS` constants
- [ ] No hardcoded hex colors outside of token definitions
- [ ] All changes pass ESLint / TypeScript type check

**Accessibility:**

- [ ] 100% keyboard navigation (Tab, Shift+Tab, Escape, Enter)
- [ ] Focus visible on every interactive element (2px gold outline)
- [ ] 4.5:1+ contrast on all text (verified with tool)
- [ ] 3:1+ contrast on all UI elements
- [ ] No color-only indicators (icon + color for status)
- [ ] Screen reader test: Dark mode toggle announces state

**Visual/UX:**

- [ ] Dark mode looks intentional & polished (not washed out)
- [ ] No visual regressions on light mode
- [ ] Responsive on sm/md/lg/xl breakpoints
- [ ] Charts & data viz visible and readable in dark mode
- [ ] Empty states / loading states look professional

**Documentation:**

- [ ] README updated with dark mode usage guide
- [ ] DARK_MODE_TOKENS documented in code comments
- [ ] design/dark-mode-spec.md kept up-to-date
- [ ] Figma tokens exported (if applicable)

---

## Template: Copy-Paste Dark Mode Pattern

Use this template for any new component or page that needs dark mode support:

```typescript
import { useTheme } from '../contexts/ThemeContext';
import { DARK_MODE_TOKENS, FOCUS_RING_SPEC } from '../contexts/ThemeContext';

interface MyComponentProps {
  title: string;
  onClick?: () => void;
}

export function MyComponent({ title, onClick }: MyComponentProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div className={`
      rounded-lg p-4 transition-colors duration-300
      ${isDark
        ? `${DARK_MODE_TOKENS.background.surfaceSecondary} ${DARK_MODE_TOKENS.text.primary}`
        : 'bg-white text-[#2d2820]'
      }
      border ${isDark ? DARK_MODE_TOKENS.border.default : 'border-white/30'}
    `}>
      <h2 className={`text-lg font-semibold ${isDark ? DARK_MODE_TOKENS.text.primary : 'text-[#2d2820]'}`}>
        {title}
      </h2>

      <button
        onClick={onClick}
        className={`
          mt-4 px-4 py-2 rounded-lg font-medium transition-all
          ${isDark
            ? `bg-[${DARK_MODE_TOKENS.semantic.accentPrimary}] text-white hover:bg-[${DARK_MODE_TOKENS.semantic.accentHover}]`
            : 'bg-[#c9983a] text-white hover:bg-[#e8c571]'
          }
          ${FOCUS_RING_SPEC.tailwind(isDark)}
        `}
      >
        Click Me
      </button>
    </div>
  );
}
```

---

## Commit Message Template

```git
design: implement dark-mode coverage for [component/page]

- Update [component] to use DARK_MODE_TOKENS for background/text
- Add focus rings (2px gold outline, 2px offset)
- Verify 4.5:1 contrast on text, 3:1 on UI
- Test keyboard navigation, screen reader (if applicable)
- Ref: design/dark-mode-spec.md

Fixes: #[issue-number]
```

---

## Resources

- **Design Spec:** `design/dark-mode-spec.md` (480+ lines, full state matrix)
- **Token Definitions:** `design-tokens.json` (darkMode object)
- **Theme Constants:** `frontend/src/shared/contexts/ThemeContext.tsx` (DARK_MODE_TOKENS, FOCUS_RING_SPEC)
- **Global Focus Ring:** `frontend/src/styles/theme.css` (CSS enforcement)
- **Phase 1 Summary:** `DARK_MODE_AUDIT_PHASE1.md`

---

**Estimated Time Breakdown:**

- Task 1 (Focus Rings): 2-3 hours
- Task 2 (Page Audits): 4-6 hours
- Task 3 (Edge Cases): 2-3 hours
- Task 4 (Accessibility): 1-2 hours
- **Total Phase 2:** 9-14 hours

**Suggested Batch Size:** 2-3 tasks per PR to maintain review quality.

---

**Questions?** Refer to design/dark-mode-spec.md for detailed specifications, contrast ratios, and component state matrices.
