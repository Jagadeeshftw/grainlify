# Keyboard Shortcut Cheatsheet Modal Specification

## Overview
A standalone keyboard shortcuts cheatsheet modal to provide a discoverable reference for Grainlify's keyboard-driven surfaces. The modal adapts its list to the current page, ensuring users see the most relevant shortcuts first.

## Trigger Affordance
- **Key**: `?` (Shift + /)
- **Behavior**: Opens the modal from anywhere in the application.
- **Exception**: The modal should not open if focus is currently inside an input, textarea, or contenteditable element.

## Modal Layout (built on Modal.tsx)
- **Width**: Use the `width="lg"` prop (`w-[95vw] sm:w-[550px]`) from `Modal.tsx`.
- **Title**: "Keyboard Shortcuts"
- **Structure**:
  - Optional Searchable filter input at the top (`ModalInput` can be used).
  - Scrollable content area containing categories.
  - Category headers (e.g., Navigation, Search, Editing).
  - Two-column layout for key/description rows (collapses to single column on mobile).

### Component: KeyRender
Platform-aware rendering of keys.
- **Mac**: Displays `⌘` for Command, `⌥` for Option, `⇧` for Shift.
- **Windows/Linux**: Displays `Ctrl` for Control, `Alt` for Alt, `Shift` for Shift.

### Two-Column Layout
- Desktop (`sm` breakpoint and above): Keys aligned left, descriptions aligned right or tabular format.
- Mobile (`<375px`): Single-column layout where the description is stacked above or next to the key, or wraps naturally.

## Categories and Per-Page Content Mapping
Shortcuts are grouped by category. Empty categories should be **hidden**.

### Current Page Section
- The modal dynamically surfaces shortcuts relevant to the active route at the **top** of the list under a "Current Page" section.
- For example, if on the Search page, "Search" shortcuts are hoisted to the top.

### Defined Categories
1. **Navigation**
   - `g` then `h`: Go to Home
   - `g` then `s`: Go to Search
2. **Search** (Example Page-Scoped)
   - `⌘K` or `Ctrl+K`: Open Command Palette (`SearchModal.tsx`)
3. **Editing**
   - `⌘S` or `Ctrl+S`: Save changes
   - `Escape`: Cancel/Close active modal

## States
1. **Default**: All categories displayed, with the current page's category hoisted to the top.
2. **Page-Scoped-Section-Highlighted**: The current page section is visually distinct (e.g., slightly highlighted background or bolded header).
3. **Empty-Category**: If a category has no applicable shortcuts based on context/permissions, it is hidden.
4. **Searchable-Filter (Optional)**: A type-ahead search input filtering the list of shortcuts dynamically.

## Accessibility Annotations
- **Focus Management**: Focus must be trapped inside the modal when open. `Modal.tsx` handles this automatically. Focus returns to the trigger origin (if applicable) when closed.
- **Labeling**: The modal container uses `aria-labelledby` pointing to the title's ID (supported by `Modal.tsx`).
- **Semantics**: Key combinations must be marked up using the `<kbd>` HTML element (e.g., `<kbd>⌘</kbd> + <kbd>K</kbd>`) for proper screen reader semantics.
- **Interaction**: The modal can be closed via the `Escape` key, handled natively by `Modal.tsx`.

## Design tokens & Contrast Verification
Ensuring text meets the 4.5:1 WCAG 2.1 AA contrast ratio against the modal surface in both themes.
- **Light Theme**:
  - Modal Surface: `bg-[#fafaf9]/95` (from `Modal.tsx`)
  - Text Primary: `text-[#2d2820]` (contrast 13.9:1)
  - Key Background (`<kbd>`): `bg-white/[0.15]` or `bg-neutral-200`
- **Dark Theme**:
  - Modal Surface: `bg-[#1c1917]/95` (from `Modal.tsx`)
  - Text Primary: `text-[#e8dfd0]` or `text-[#f5f5f5]` (contrast >10:1)
  - Key Background (`<kbd>`): `bg-white/[0.08]` or `bg-neutral-800`

## Responsive Behavior
- At viewport widths `> 375px`: The key/description rows use a standard two-column layout (`flex justify-between` or `grid grid-cols-2`).
- At viewport widths `< 375px`: The layout collapses to a single-column layout, wrapping text appropriately and adjusting padding.
