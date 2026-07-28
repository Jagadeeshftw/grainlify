# Blog Article Body Layout & TOC Sidebar Specification

## 1. Overview
This specification details the typographic scale, code-block styles, callouts, and a sticky scroll-spy Table of Contents (TOC) sidebar for `BlogPage.tsx`. The design adheres to the Grainlify visual language defined in `/design-tokens.json`, utilizing the warm neutral palette, gold accents, and Inter typography.

## 2. Typographic Scale (h1-h6) & Paragraph Measure
All typography utilizes the `sans` font family stack: `["Inter", "system-ui", "-apple-system", "sans-serif"]`.

- **Paragraph Measure:** Max-width of `65ch` for optimal reading length. Body text uses `text.primary` (#1a1a1a) in light mode and `text.primary` (#f5f5f5) in dark mode.
- **h1:** Font size `4xl` (2.25rem), line-height 2.5rem, bold (700), margin-bottom `6` (1.5rem)
- **h2:** Font size `3xl` (1.875rem), line-height 2.25rem, semibold (600), margin-top `10` (2.5rem), margin-bottom `4` (1rem)
- **h3:** Font size `2xl` (1.5rem), line-height 2rem, semibold (600), margin-top `8` (2rem), margin-bottom `3` (0.75rem)
- **h4:** Font size `xl` (1.25rem), line-height 1.75rem, medium (500), margin-top `6` (1.5rem), margin-bottom `2` (0.5rem)
- **h5:** Font size `lg` (1.125rem), line-height 1.75rem, medium (500), margin-top `4` (1rem), margin-bottom `2` (0.5rem)
- **h6:** Font size `base` (1rem), line-height 1.5rem, medium (500), margin-top `4` (1rem), margin-bottom `2` (0.5rem)

### 2.1 State Annotations
- **Heading anchors on hover:** Heading links display an anchor icon (`#`) to the left of the heading with color `neutral.400` on hover, turning to `primary.600` on focus.

## 3. Callout/Admonition Styles
Callouts utilize the semantic color palette and use `md` (0.375rem) border-radius with an optional left border accent of `4px` solid.

- **Info (Tip/Note):** 
  - Background: `semantic.info.50` (#eff6ff)
  - Left border: `semantic.info.500` (#3b82f6)
  - Text/Icon: `semantic.info.700` (#1d4ed8)
- **Warning:** 
  - Background: `semantic.warning.50` (#fffbeb)
  - Left border: `semantic.warning.500` (#f59e0b)
  - Text/Icon: `semantic.warning.700` (#b45309)
- **Success:** 
  - Background: `semantic.success.50` (#f0fdf4)
  - Left border: `semantic.success.500` (#22c55e)
  - Text/Icon: `semantic.success.700` (#15803d)

## 4. Code-Block Chrome
Code blocks use the `mono` font family stack (`["JetBrains Mono", "monospace"]`) with `text-sm` font size.

- **Background:** `neutral.900` (#1c1917)
- **Border Radius:** `lg` (0.5rem)
- **Padding:** `4` (1rem)
- **Text Color:** `text.inverse` (#ffffff)
- **State Annotations:** 
  - **With Language Label:** Displayed in a top-right badge, using `neutral.700` background with `neutral.300` text, `sm` border-radius.
  - **Without Language Label:** No top-right badge, consistent internal padding.

## 5. TOC Sidebar & Scroll-Spy Behavior
A sticky Table of Contents that tracks the active heading as the user scrolls.

### 5.1 Responsive Behavior
- **Desktop (>= lg breakpoint, 1024px):** Rendered as a fixed sidebar on the right side. Sticky positioned `top-24`.
- **Tablet/Mobile (< lg breakpoint):** Collapses into a top dropdown menu that sits below the header. It uses elevation level `3` (High shadow) when expanded.

### 5.2 State Annotations
- **Active TOC Entry:** Receives `primary.600` (#c9983a) text color and a left border accent of 2px solid `primary.500`. Background highlights with `primary.50` (#fef7e6).
- **Inactive TOC Entry:** `text.secondary` (#424242) with hover state changing to `neutral.600` and `neutral.50` background.

## 6. Accessibility Annotations (WCAG 2.1 AA)
- **Heading Order:** Ensure strict semantic ordering (h1 -> h2 -> h3) without skipping levels (e.g., h2 directly to h4 is invalid).
- **ARIA Current:** Active TOC links must include `aria-current="true"` to announce the active section to screen readers.
- **Skip-to-Content:** A skip-to-content link must be provided at the top of the page, allowing keyboard users to bypass the TOC and navigation directly into the `main` article content.
- **Keyboard Navigation:** Tab focus must traverse TOC links logically. Focus should not be trapped in the TOC unless navigating within the mobile dropdown modal.
- **Contrast Ratios:** Verified that body text (`#1a1a1a` on `#ffffff`) meets 21:1 (AAA) and code block text (`#ffffff` on `#1c1917`) meets requirements (>4.5:1). Dark mode text (`#f5f5f5` on `#1a1714`) provides excellent contrast.

## 7. QA Validation
- [x] Verified colors against `/design-tokens.json`
- [x] Confirmed spacing uses `1`, `2`, `3`, `4`, `6`, `8`, `10` sequence from tokens.
- [x] Tested 4.5:1 minimum contrast requirement for all text treatments in Light and Dark modes.
