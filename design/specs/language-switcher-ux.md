# Language Switcher UX — Design Spec

**Version:** 1.0
**Status:** Design specification
**Target placement:** `frontend/src/shared/components/UserProfileDropdown.tsx` (trigger + dropdown) or `frontend/src/features/settings/pages/SettingsPage.tsx` (dedicated Language & Region tab)
**Dependencies:** `GlassDropdown.tsx` interaction conventions, `FilterDropdown.tsx` search pattern, `design-tokens.json`, RTL layout prep spec
**Related issues:** i18n rollout epic, RTL layout prep

---

## Table of Contents

1. [Overview](#1-overview)
2. [Language Data Model](#2-language-data-model)
3. [Control Anatomy — Trigger](#3-control-anatomy--trigger)
4. [Control Anatomy — Dropdown List](#4-control-anatomy--dropdown-list)
5. [Search / Filter Behavior](#5-search--filter-behavior)
6. [List-Item Anatomy](#6-list-item-anatomy)
7. [RTL-Readiness & Beta Badge](#7-rtl-readiness--beta-badge)
8. [States](#8-states)
9. [Placement Strategy](#9-placement-strategy)
10. [Responsive Behavior](#10-responsive-behavior)
11. [Accessibility Contract](#11-accessibility-contract)
12. [Motion & Reduced Motion](#12-motion--reduced-motion)
13. [Design Tokens Used](#13-design-tokens-used)
14. [QA Checklist](#14-qa-checklist)
15. [Future Extensions](#15-future-extensions)

---

## 1. Overview

The language switcher enables Grainlify users to select their preferred UI language from a list of 10+ supported languages. The design supports:

- A **compact trigger** showing the current language's flag icon + ISO code (e.g., `🇺🇸 EN`)
- A **searchable dropdown list** for navigating a long language list
- **RTL readiness** with visual "beta" badges on RTL-language entries until full RTL layout support ships
- Reuse of existing `GlassDropdown.tsx` toggle/backdrop/dismiss conventions and `FilterDropdown.tsx` search-input pattern
- Placement in both the **User Profile Dropdown** (quick-access) and **Settings → Language & Region tab** (full configuration)

### Goals

- Enable frictionless language switching without page reload (i18n persistence via user preference API / localStorage)
- Keep the trigger compact enough to fit in the existing profile dropdown header area
- Provide a discoverable search/filter for the 10+ language list
- Surface RTL-language limitations transparently with a beta badge
- Meet WCAG 2.1 AA with full keyboard operability
- Follow Grainlify's glassmorphism design system

---

## 2. Language Data Model

```typescript
interface Language {
  /** ISO 639-1 two-letter code, uppercase — e.g. "EN", "FR", "AR" */
  code: string;
  /** Language name in the language's own script — e.g. "English", "Français", "العربية" */
  nativeName: string;
  /** Language name in English — e.g. "English", "French", "Arabic" */
  englishName: string;
  /** Emoji flag — e.g. "🇺🇸", "🇫🇷", "🇸🇦" */
  flag: string;
  /** Direction — "ltr" | "rtl" */
  direction: "ltr" | "rtl";
  /** Is this the browser-default / first-time visitor fallback? */
  isDefault: boolean;
}

type LanguageDirection = "ltr" | "rtl";
```

### Initial language list (illustrative — 12 entries)

| Code | Native Name | English Name | Flag | Direction | Notes |
|------|------------|-------------|------|-----------|-------|
| EN | English | English | 🇺🇸 | ltr | Default |
| FR | Français | French | 🇫🇷 | ltr | |
| ES | Español | Spanish | 🇪🇸 | ltr | |
| DE | Deutsch | German | 🇩🇪 | ltr | |
| PT | Português | Portuguese | 🇧🇷 | ltr | |
| IT | Italiano | Italian | 🇮🇹 | ltr | |
| NL | Nederlands | Dutch | 🇳🇱 | ltr | |
| PL | Polski | Polish | 🇵🇱 | ltr | |
| RU | Русский | Russian | 🇷🇺 | ltr | |
| JA | 日本語 | Japanese | 🇯🇵 | ltr | |
| ZH | 中文 | Chinese (Simplified) | 🇨🇳 | ltr | |
| AR | العربية | Arabic | 🇸🇦 | rtl | beta |
| HE | עברית | Hebrew | 🇮🇱 | rtl | beta |
| FA | فارسی | Persian (Farsi) | 🇮🇷 | rtl | beta |

> **Note:** The production list will be driven by the platform's actual translation coverage. Entries beyond 10+ should be loadable via the search input.

---

## 3. Control Anatomy — Trigger

### Placement context

The trigger is rendered **inside the UserProfileDropdown** header area, below the user avatar/name block, and/or as the first control in a **Settings → Language & Region tab**.

### Trigger layout (closed state)

```
┌──────────────────────────────────┐
│  🇺🇸 EN                    [▼]   │
└──────────────────────────────────┘
```

- **Height:** 40px (`h-10`)
- **Min-width:** 96px (accommodates longest code flag + code, e.g. `🇨🇳 ZH`)
- **Padding:** `px-3 py-2`
- **Border radius:** `rounded-[12px]` (matching `FilterDropdown` button shape)
- **Background:** glassmorphism, matching the parent surface:
  - Dark: `bg-white/[0.08] border-white/15`
  - Light: `bg-white/[0.15] border-white/25`
- **Hover:** same as `GlassDropdown.tsx` hover pattern:
  - Dark: `hover:bg-white/[0.12] hover:border-[#e8c571]/40`
  - Light: `hover:bg-white/[0.2] hover:border-[#c9983a]/30`

### Trigger content (left to right)

1. **Flag emoji** — `text-[16px]` (ensures legible emoji rendering)
2. **Language code** — `<span>` with:
   - `text-[13px] font-semibold`
   - Color: `text-[#e8dfd0]` (dark) / `text-[#2d2820]` (light)
3. **Chevron icon** — `ChevronDown` from `lucide-react`, `w-3.5 h-3.5`
   - Rotates `180deg` when dropdown is open
   - Color: `text-[#b8a898]` (dark) / `text-[#7a6b5a]` (light)

### ARIA attributes on trigger

```html
<button
  aria-haspopup="listbox"
  aria-expanded="true|false"
  aria-controls="language-switcher-listbox"
  aria-label="Current language: English. Change language"
  role="combobox"
>
```

### States

| State | Visual treatment |
|-------|-----------------|
| Default | Glass background, standard border |
| Hover | Elevated glass opacity + gold border tint |
| Focus | `outline-2 outline-offset-2 outline-[#f1b400]` |
| Open | Chevron rotated, dropdown visible (no style change to trigger itself) |
| Disabled | `opacity-50 cursor-not-allowed` (used only during language-switching loading state) |

---

## 4. Control Anatomy — Dropdown List

### Position & Sizing

- **Position:** `absolute top-full right-0 mt-2` (mirrors `GlassDropdown.tsx` positioning)
- **Width:** `w-72` (288px) — wider than standard GlassDropdown to accommodate flag + native name + English name + checkmark
- **Max height:** `max-h-[360px]` with `overflow-y-auto custom-scrollbar`
- **Border radius:** `rounded-[16px]` (matching `GlassDropdown.tsx`)
- **Elevation:** Elevation level 3 (High) — per `design-tokens.json`:
  - Light: `0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1)`
  - Dark: `0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.4)`
- **Backdrop:** `fixed inset-0 z-40` (mirrors `GlassDropdown.tsx` backdrop pattern)
- **Z-index:** `z-50` for the dropdown panel

### Surfaces

- **Dark:**
  - Background: `bg-[#3a3228]` (same as `GlassDropdown.tsx` menu background)
  - Border: `border-white/20`
- **Light:**
  - Background: `bg-[#d4c5b0]` (same as `GlassDropdown.tsx` menu background)
  - Border: `border-white/40`

### Structure (top to bottom)

```
┌──────────────────────────────────┐
│ Search input              [✕]   │  ← only if query is active
├──────────────────────────────────┤
│ ┌──────────────────────────────┐ │
│ │ 🔍 Search languages…         │ │  ← Search input
│ └──────────────────────────────┘ │
├──────────────────────────────────┤
│ 🇺🇸  English        English   ✓  │  ← selected item
│ 🇫🇷  Français       French       │
│ 🇩🇪  Deutsch        German       │
│ 🇸🇦  العربية        Arabic   ⚠β  │  ← RTL beta badge
│ …                                │
│ (empty state if no results)      │
└──────────────────────────────────┘
```

### Backdrop

The backdrop follows the `GlassDropdown.tsx` pattern — a `fixed inset-0 z-40` transparent layer that closes the dropdown on click. This handles both click-outside-dismiss and escape-key scenarios.

---

## 5. Search / Filter Behavior

### Search Input

Rendered inside the dropdown panel, mirroring `FilterDropdown.tsx` conventions:

```
┌──────────────────────────────────┐
│ 🔍  Search languages…       [✕] │
└──────────────────────────────────┘
```

- **Container:** `p-3 border-b border-white/10` (separates search from list)
- **Input wrapper:** `flex items-center gap-2 px-3 py-2 rounded-[10px] backdrop-blur-[20px] border`
  - Dark: `bg-white/[0.06] border-white/10`
  - Light: `bg-white/[0.08] border-white/15`
- **Search icon:** `Search` from `lucide-react`, `w-4 h-4 text-[#b8a898]`
- **Input field:**
  - `flex-1 bg-transparent border-none outline-none text-[13px]`
  - Text: `text-[#e8dfd0]` (dark) / `text-[#2d2820]` (light)
  - Placeholder: `text-[#b8a898]/50` — `"Search languages…"`
  - `autoComplete="off"`, `spellCheck="false"`
- **Clear button:** appears when query is non-empty:
  - `X` icon from `lucide-react`, `w-3 h-3 text-[#b8a898]`
  - `p-0.5 hover:bg-white/10 rounded transition-colors`

### Filtering Logic

- Filter applies to **both** `nativeName` and `englishName` fields
- Case-insensitive substring match (`.toLowerCase().includes(query.toLowerCase())`)
- Sorting: matched languages preserve original order; languages matching on `nativeName` float above `englishName`-only matches

```typescript
const filteredLanguages = languages
  .filter(lang =>
    lang.nativeName.toLowerCase().includes(query.toLowerCase()) ||
    lang.englishName.toLowerCase().includes(query.toLowerCase())
  )
  .sort((a, b) => {
    const aNative = a.nativeName.toLowerCase().includes(query.toLowerCase());
    const bNative = b.nativeName.toLowerCase().includes(query.toLowerCase());
    if (aNative && !bNative) return -1;
    if (!aNative && bNative) return 1;
    return 0;
  });
```

### No-results state

```
┌──────────────────────────────────┐
│                                  │
│        🔍                        │
│   No languages found             │
│   Try a different search term    │
│                                  │
└──────────────────────────────────┘
```

- Icon: `Search` in `text-[#b8a898]`, `w-8 h-8` centered above text
- Primary text: `"No languages found"` — `text-[13px] font-semibold`, centered
- Secondary text: `"Try a different search term"` — `text-[12px] text-[#b8a898]`, centered
- Padding: `py-8`

### Keyboard interaction within search

| Key | Action |
|-----|--------|
| Any printable character | Types into search input; filters list |
| `ArrowDown` | Moves focus to first list item; subsequent presses move down |
| `ArrowUp` | Returns focus to search input if first item focused; otherwise moves up |
| `Escape` | Clears search query if non-empty; if empty, closes dropdown |
| `Enter` | If a list item is focused, selects it; applies filter otherwise |

---

## 6. List-Item Anatomy

### Layout (per row)

```
┌──────────────────────────────────────┐
│ 🇺🇸  English        English      ✓   │
│ <--flag-><--native name--><--en--><-->│
└──────────────────────────────────────┘
```

```
┌──────────────────────────────────────┐
│ 🇸🇦  العربية        Arabic    ⚠β ✓   │  ← RTL entry with beta badge + selected
└──────────────────────────────────────┘
```

### Grid layout (CSS)

```css
display: grid;
grid-template-columns: 24px 1fr auto 20px;  /* flag | native name | english name | checkmark */
gap: 10px;
align-items: center;
```

### Detailed anatomy

| Column | Content | Typography | Color (dark) | Color (light) |
|--------|---------|-----------|-------------|--------------|
| 1 | Flag emoji | `text-[16px]` | — | — |
| 2 | `nativeName` | `text-[13px] font-semibold` | `text-[#e8dfd0]` | `text-[#2d2820]` |
| 3 | `englishName` (lowercase) | `text-[12px] font-normal` | `text-[#b8a898]` | `text-[#7a6b5a]` |
| 4 | Checkmark (when selected) | `Check` icon `w-4 h-4` | `text-[#c9983a]` | `text-[#c9983a]` |

### RTL item handling

For RTL-direction languages, the native name container gets `dir="auto"` so Arabic/Hebrew/Persian text aligns correctly regardless of the overall dropdown direction:

```html
<div class="language-item" dir="ltr">
  <span class="flag">🇸🇦</span>
  <span dir="auto" class="native-name">العربية</span>
  <span class="english-name">arabic</span>
  <span class="checkmark">✓</span>
</div>
```

### Interactive states

| State | Background | Notes |
|-------|-----------|-------|
| Default | Transparent | — |
| Hover | `bg-[#4a3e30]` (dark) / `bg-[#c9b8a0]` (light) | Same as `GlassDropdown.tsx` hover |
| Selected | Left border accent: `border-l-2 border-[#c9983a]` + `bg-[#c9983a]/10` | Checkmark icon visible |
| Selected + Hover | `bg-[#c9983a]/15` | — |
| Focus | `outline-2 outline-offset-2 outline-[#f1b400]` | Visible keyboard focus ring |
| Disabled | `opacity-40 cursor-not-allowed` | Used only during language-switching loading |

### Touch target

Each list item has **min-height: 44px** (`py-2.5` with 13px text) to meet WCAG 2.5.5 touch target requirements on mobile.

---

## 7. RTL-Readiness & Beta Badge

### Beta badge

For languages with `direction === "rtl"`, a small "beta" badge appears **before** the checkmark column:

```
Flag | العربية | arabic | ⚠β | ✓
```

### Beta badge specification

| Property | Value |
|----------|-------|
| Container | `inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[4px]` |
| Background | `bg-[#f59e0b]/20` (warning tone) |
| Text | `text-[10px] font-semibold uppercase tracking-wider` |
| Color | `text-[#f59e0b]` |
| Icon | `AlertTriangle` from `lucide-react`, `w-3 h-3` |
| Tooltip | `title="RTL layout support is in beta. Some UI may appear left-to-right."` |
| `aria-label` | `"Arabic — right-to-left language, beta support"` |

### Tooltip content

When the user hovers or focuses the beta badge, show a tooltip:

> **RTL support is in beta.**  
> Some UI elements may still display left-to-right. Full RTL layout is coming in a future update.

The tooltip follows Grainlify's existing tooltip pattern (not specified here; may use native `title` attribute initially).

### Groundwork for full RTL support

The spec prescribes these CSS hooks for the eventual full RTL implementation:

1. **CSS logical properties** in the dropdown list styles — use `padding-inline-start`/`padding-inline-end` instead of `padding-left`/`padding-right`
2. **`dir="auto"`** on native-name spans for RTL entries
3. **No hardcoded `left`/`right`** positions — use `inset-inline-end: 0` instead of `right: 0`
4. **Grid template:** `grid-template-columns: 24px 1fr auto auto 20px` (not `auto 1fr auto`) — allows swapping column order via `direction: rtl`

---

## 8. States

### State matrix

| # | State | Trigger | Dropdown | Search | List | Notes |
|---|-------|---------|----------|--------|------|-------|
| 1 | **Closed** | Shows current flag+code | Hidden | — | — | Default state |
| 2 | **Open (full list)** | Chevron rotated 180° | Visible | Empty, placeholder shown | All languages, current selected | Opened via click/Enter/Space |
| 3 | **Open (filtered)** | Chevron rotated 180° | Visible | Has query | Filtered subset | User typing |
| 4 | **No results** | Chevron rotated 180° | Visible | Has query with no match | Empty state | "No languages found" |
| 5 | **Language switching** | `opacity-60 cursor-wait`, spinner replaces flag | Closing | — | — | Brief loading state (≤500ms) |
| 6 | **Selected (new language)** | Updated flag+code | Closed | Reset | — | Language applied |

### State 5 — Language switching (loading)

```
┌──────────────────────────────────┐
│  ⟳  EN                    [▼]   │  ← spinner replaces flag
└──────────────────────────────────┘
```

- Trigger background remains the same
- Flag emoji replaced by `Loader2` from `lucide-react` with `className="animate-spin w-4 h-4"`
- `opacity-60` on trigger + `cursor-wait`
- `aria-busy="true"` on trigger
- Dropdown automatically closes
- Duration: brief (≤500ms); language preference should be stored client-side (localStorage) and synced to backend asynchronously for instant UI feedback
- On complete: trigger updates to new language's flag+code, spinner removed
- On error: brief error state (1.5s) then revert to previous language

### State 5 — error recovery

If the language switch fails (network error, backend rejection):

```
┌──────────────────────────────────┐
│  ⚠  EN                    [▼]   │  ← warning icon, previous flag
└──────────────────────────────────┘
```

- Flag replaced with `AlertTriangle` in `text-[#f59e0b]`
- Show a toast notification: `"Could not switch language. Please try again."`
- After 1.5s, revert trigger to original state
- Dropdown remains closed

---

## 9. Placement Strategy

### Option A — User Profile Dropdown (quick access)

**File:** `frontend/src/shared/components/UserProfileDropdown.tsx`

Location: Inside the `DropdownMenuContent`, **above** the menu items or **below the user info header**.

```
┌──────────────────────────────────┐
│  User info section               │
├──────────────────────────────────┤
│  🇺🇸 English ← Language Switcher  │  ← New section
├──────────────────────────────────┤
│  👤 Public Profile               │
│  ⚙️ Settings                     │
├──────────────────────────────────┤
│  🚪 Logout                       │
└──────────────────────────────────┘
```

- The language switcher is a separate **controlled section** within the dropdown
- It is not part of the menu items list (separator above and below)
- Compact: single row with flag + native name + chevron
- Clicking opens its own dropdown (portal-based, same as `FilterDropdown.tsx`)
- The language switcher's dropdown renders **outside** the profile dropdown's portal to avoid clipping

### Option B — Settings Page (Language & Region tab)

**File:** `frontend/src/features/settings/pages/SettingsPage.tsx`

Add a new tab `language` to the `tabs` array:

```typescript
const tabs: { id: SettingsTabType; label: string }[] = [
  { id: 'profile', label: 'Profile' },
  { id: 'notifications', label: 'Notifications' },
  { id: 'language', label: 'Language & Region' },  // New
  { id: 'payout', label: 'Payout Preferences' },
  // ...
];
```

**LanguageTab component** (`frontend/src/features/settings/components/language/LanguageTab.tsx`):

- Full-width language list (not a dropdown) with radio-button-style selection
- Search/filter input at the top
- Each row shows: flag, native name, English name, radio indicator, beta badge (for RTL)
- At the bottom: a **Region & Formatting** sub-section with:
  - Date format preview (e.g., "July 27, 2026" vs "27 July 2026" vs "2026-07-27")
  - Time format (12h / 24h)
  - Number format (1,000.00 vs 1.000,00)
  - Timezone selector
- Save button (or auto-save on selection)

### Recommendation

**Implement both** — Option A for quick access in the profile dropdown, Option B for full configuration in Settings.

---

## 10. Responsive Behavior

### Breakpoint: ≥ 640px (tablet/desktop)

- Dropdown appears as a positioned panel anchored to the trigger (as described in §4)
- `position: fixed` via `createPortal` (matching `FilterDropdown.tsx` pattern)
- Width: `w-72` (288px)

### Breakpoint: < 640px (mobile, 375px)

The dropdown transforms into a **full-screen bottom sheet** to avoid clipping and provide comfortable touch targets:

```
┌──────────────────────────────────┐
│  ✕  Choose language              │
├──────────────────────────────────┤
│  🔍 Search languages…            │
├──────────────────────────────────┤
│  🇺🇸 English        English   ✓  │
│  🇫🇷 Français       French       │
│  🇩🇪 Deutsch        German       │
│  🇸🇦 العربية        Arabic   ⚠β  │
│  ...                              │
├──────────────────────────────────┤
│     [ Cancel ]                   │
└──────────────────────────────────┘
```

- **Full screen:** `fixed inset-0 z-[200]`
- **Background:** same surface as dropdown, but full-height
  - Dark: `bg-[#1a1714]` (surfacePrimary)
  - Light: `bg-[#fafaf9]` (neutral-50)
- **Header:** Close button (X) + "Choose language" title
- **Search:** full-width at top of scrollable list
- **List:** `flex-1 overflow-y-auto`, items have larger touch targets (`py-3 min-h-[48px]`)
- **Bottom:** Cancel button (or the system back gesture dismisses)
- **Animation:** slides up from bottom (translateY), `300ms ease-out`
- **Body scroll:** `overflow: hidden` while sheet is open

### Media query trigger

```css
@media (max-width: 639px) {
  .language-dropdown { /* full-screen sheet styles */ }
}
```

---

## 11. Accessibility Contract

### WAI-ARIA pattern: Combobox (with listbox popup)

The language switcher follows the [WAI-ARIA combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/).

| Element | Role | ARIA Attributes |
|---------|------|-----------------|
| Trigger button | `combobox` | `aria-expanded`, `aria-controls`, `aria-haspopup="listbox"`, `aria-activedescendant` |
| Search input | (part of combobox) | Inherits from combobox trigger |
| Dropdown list | `listbox` | `aria-label="Available languages"` |
| Each item | `option` | `aria-selected`, `id` for `aria-activedescendant` reference |

### Keyboard navigation

| Key | Action |
|-----|--------|
| `Tab` | Focus the language switcher trigger |
| `Enter` / `Space` | Open dropdown, focus search input |
| `Escape` | Close dropdown (if open); return focus to trigger |
| `ArrowDown` | Move focus to next list item (wrap not allowed — clamped at last) |
| `ArrowUp` | Move focus to previous list item (clamped to search input) |
| `Home` | Select first item in filtered list |
| `End` | Select last item in filtered list |
| `Enter` (on item) | Select focused language, close dropdown |
| Type-ahead | When search input is focused, characters filter the list. When list item is focused, type-ahead jumps to next item starting with typed character |

### Focus management

- **Open:** Focus moves to search input
- **Close:** Focus returns to the trigger button
- **Tab within dropdown:** Only the search input and the close button (if mobile sheet) are tabbable; list items are navigated via arrow keys (roving tabindex)
- **Focus trap:** On desktop dropdown, focus does not leave the dropdown while open (Tab cycles between search input and the dropdown panel's close mechanisms)
- **Mobile sheet:** Full focus trap within the sheet; close via X button, Cancel button, or back gesture

### ARIA live region

When a language is selected:

```html
<div aria-live="polite" aria-atomic="true" class="sr-only">
  Language changed to English
</div>
```

### Screen reader announcements

| Action | Announcement |
|--------|-------------|
| Dropdown opens | "Choose language. Listbox. 12 items." |
| Search filters | "5 items filtered." (via `aria-live="polite"`) |
| No results | "No results found." |
| Item focused | "Arabic, right-to-left language, beta support. 4 of 12." |
| Item selected | "Arabic selected." |
| Language switching | "Loading…" + "English selected." |
| Error | "Could not change language. English remains selected." |

### Reduced motion

- `prefers-reduced-motion: reduce` disables the dropdown slide animation
- Mobile sheet uses opacity-only animation instead of translateY
- Spinner animation is replaced with a static icon
- All duration overrides use `0ms` or `150ms` opacity-only as specified in `design-tokens.json` → `reducedMotion`

### Contrast requirements

- Trigger text: ≥ 4.5:1 against glassmorphism background (validated across both themes)
- Dropdown list text: ≥ 4.5:1 against `#3a3228` (dark) or `#d4c5b0` (light)
- Beta badge text: ≥ 4.5:1 against `bg-[#f59e0b]/20`
- Checkmark icon: ≥ 3:1 against item background
- Focus ring: ≥ 3:1 contrast against adjacent background
- Selected item accent border: ≥ 3:1 (non-text contrast)

---

## 12. Motion & Reduced Motion

### Default motion

| Interaction | Duration | Easing | Effect |
|------------|----------|--------|--------|
| Dropdown open | 150ms | `easeOut` (0, 0, 0.2, 1) | `slide-fade` — translateY(-4px) + opacity |
| Dropdown close | 100ms | `easeIn` (0.4, 0, 1, 1) | `fade` — opacity only |
| Mobile sheet open | 300ms | `easeOut` | TranslateY(full) + opacity |
| Mobile sheet close | 200ms | `easeIn` | TranslateY(full) + opacity |
| Selected item highlight | 150ms | `easeOut` | Background color transition |
| Search input focus | 200ms | `easeOut` | Border glow transition |

### Reduced motion overrides

| Interaction | Override |
|------------|----------|
| Dropdown open/close | `opacity` only, 150ms max |
| Mobile sheet | `opacity` only, 150ms max |
| Spinner | Static icon (no animation) |

### Motion tokens reference (from `design-tokens.json`)

```json
{
  "motion.durations.fast": "150ms",
  "motion.durations.normal": "300ms",
  "motion.easing.easeOut": [0, 0, 0.2, 1],
  "motion.easing.easeIn": [0.4, 0, 1, 1],
  "motion.pageTransition.dropdown": {
    "duration": "150ms",
    "easing": "easeOut",
    "effect": "slide-fade"
  }
}
```

---

## 13. Design Tokens Used

### Color tokens

| Token | Usage |
|-------|-------|
| `color.primary.600` (`#c9983a`) | Checkmark icon, accent border, selected highlight |
| `color.primary.700` (`#a67c2e`) | Active state, dark variant |
| `color.neutral.800` (`#292524`) | Dropdown item text (dark theme) |
| `color.semantic.warning.500` (`#f59e0b`) | Beta badge text and icon |
| `darkMode.background.surfaceSecondary` (`#2d2820`) | Dropdown surface (dark) |
| `darkMode.background.surfaceTertiary` (`#3a3428`) | Dropdown surface alternative (dark) |
| `darkMode.text.primary` (`#f5f5f5`) | Item native name (dark) |
| `darkMode.text.tertiary` (`#b8a898`) | Item English name (dark) |
| `darkMode.interactive.hover` (`rgba(255,255,255,0.10)`) | Item hover (dark) |
| `darkMode.interactive.focusRing` (`#f1b400`) | Focus ring (dark) |
| `darkMode.accent.primary` (`#c9983a`) | Selected state accent |

### Elevation tokens

| Token | Usage |
|-------|-------|
| `elevation.levels.3.shadow.light` | Dropdown shadow (light theme) |
| `elevation.levels.3.shadow.dark` | Dropdown shadow (dark theme) |
| `elevation.levels.4.shadow.light` | Mobile sheet shadow (light) |
| `elevation.levels.4.shadow.dark` | Mobile sheet shadow (dark) |

### Glassmorphism tokens

| Token | Usage |
|-------|-------|
| `elevation.glassmorphism.light.backgroundOpacity` | Trigger background opacity (light) |
| `elevation.glassmorphism.dark.backgroundOpacity` | Trigger background opacity (dark) |
| `elevation.glassmorphism.light.borderColor` | Trigger border (light) |
| `elevation.glassmorphism.dark.borderColor` | Trigger border (dark) |
| `elevation.glassmorphism.compliance.contrastRatio` | ≥4.5:1 rule |

### Typography tokens

| Token | Usage |
|-------|-------|
| `typography.fontSize.sm` (`0.875rem`) | Trigger text, item native name |
| `typography.fontSize.xs` (`0.75rem`) | Item English name, beta badge |
| `typography.fontWeight.semibold` (`600`) | Trigger, item native name |
| `typography.fontWeight.medium` (`500`) | Search placeholder text |

### Spacing tokens

| Token | Usage |
|-------|-------|
| `spacing.2` (`0.5rem`) | Gap between items |
| `spacing.3` (`0.75rem`) | Padding in search container |
| `spacing.4` (`1rem`) | Padding in dropdown items |

### Accessibility tokens

| Token | Usage |
|-------|-------|
| `accessibility.focus.outlineStyle.width` (`2px`) | Focus ring width |
| `accessibility.focus.outlineStyle.offset` (`2px`) | Focus ring offset |
| `accessibility.focus.minTouchTarget` (`44x44px`) | Mobile item touch targets |
| `accessibility.motion.reducedMotion` | Reduced motion overrides |

---

## 14. QA Checklist

### Visual / layout

- [ ] Trigger correctly shows current language flag + code
- [ ] Trigger glassmorphism matches parent surface (both themes)
- [ ] Dropdown list width (288px) accommodates longest entries without overflow
- [ ] List item grid columns align correctly for all entries
- [ ] Selected item shows checkmark icon and left border accent
- [ ] Beta badge appears for all `direction === "rtl"` entries
- [ ] No-results empty state renders with icon and text
- [ ] Mobile full-screen sheet renders correctly at 375px
- [ ] Language-switching loading state shows spinner in trigger
- [ ] Error state shows warning icon and reverts

### Interaction

- [ ] Clicking trigger opens dropdown
- [ ] Backdrop click closes dropdown
- [ ] Clicking a list item selects it and closes dropdown
- [ ] Search input filters list by both native name and English name
- [ ] Clear button (X) resets search query
- [ ] Search with no results shows empty state
- [ ] Language switching shows brief loading state
- [ ] Error during language switch shows error state and reverts
- [ ] Mobile sheet opens full-screen on < 640px

### Accessibility

- [ ] Trigger has `role="combobox"`, `aria-expanded`, `aria-controls`
- [ ] List has `role="listbox"` with `aria-label`
- [ ] Items have `role="option"` and `aria-selected`
- [ ] Keyboard: Tab → trigger → Enter/Space opens → ArrowDown/ArrowUp navigates → Enter selects → Escape closes
- [ ] Type-ahead works: typing characters filters the list
- [ ] Focus returns to trigger on close
- [ ] Focus is trapped within dropdown while open
- [ ] Mobile sheet has full focus trap
- [ ] `aria-live="polite"` announces language changes
- [ ] Beta badge has `aria-label` explaining RTL beta status
- [ ] RTL native names use `dir="auto"`
- [ ] Touch targets ≥ 44px on mobile
- [ ] Color contrast passes WCAG 2.1 AA (4.5:1 text, 3:1 UI)
- [ ] Focus ring is visible (`2px solid #f1b400`)
- [ ] Reduced motion respected: opacity-only, no translate, no spinner animation

### Edge cases

- [ ] 20+ languages all render correctly (scrollable list)
- [ ] Language name with special characters / non-Latin script renders correctly
- [ ] Search term with diacritics matches appropriately (e.g. "franc" matches "Français")
- [ ] Very long language names don't break layout (ellipsis overflow)
- [ ] Quickly opening/closing dropdown multiple times doesn't break state
- [ ] Language switch during ongoing API request is debounced/queued
- [ ] Screen orientation change on mobile doesn't break full-screen sheet
- [ ] Browser back button on mobile closes the sheet

### Responsive

- [ ] Desktop (≥ 1024px): dropdown anchors to trigger
- [ ] Tablet (640-1023px): dropdown anchors to trigger, wider than mobile
- [ ] Mobile (< 640px / 375px): full-screen bottom sheet
- [ ] Text zoom to 200%: no horizontal overflow, no clipped content
- [ ] Landscape mobile: sheet still fills viewport, scrollable

---

## 15. Future Extensions

### Post-RTL-full-support

- Remove beta badge from RTL language entries
- Add `dir="rtl"` to the entire dropdown when an RTL language is selected
- Mirror grid column order for RTL layout
- Add language-specific font stacks (e.g., Noto Naskh Arabic for Arabic)

### Additional features

- **Language suggestions:** Show "Most used" or "Recently used" languages at top of list
- **Language search by code:** Match ISO code (e.g., typing "ar" matches Arabic)
- **Auto-detect:** Detect browser language on first visit and suggest switching
- **Translation coverage indicator:** Show a progress bar or percentage per language
- **Keyboard shortcut:** `Ctrl+Shift+L` to cycle through recently used languages
- **Preview mode:** Temporarily switch language without saving (with "Keep changes?" prompt)

### Integration with RTL layout prep spec

- CSS logical properties should be used throughout the dropdown styles
- The `beta` boolean in the language model should be controlled by a feature flag (`flagship.isRTLBetaEnabled`)
- When full RTL support ships, the beta badge is removed and `direction` property drives layout direction

---

## Appendix A — Example Implementation Files

| File | Purpose |
|------|---------|
| `frontend/src/shared/components/LanguageSwitcher.tsx` | Language switcher trigger + dropdown component |
| `frontend/src/shared/components/LanguageSwitcherList.tsx` | Shared list component (used by both dropdown and settings tab) |
| `frontend/src/shared/components/LanguageSwitcherSheet.tsx` | Mobile full-screen sheet variant |
| `frontend/src/shared/contexts/LanguageContext.tsx` | React context for current language state + switcher function |
| `frontend/src/shared/types/i18n.ts` | `Language`, `LanguageDirection` types |
| `frontend/src/features/settings/components/language/LanguageTab.tsx` | Settings page Language & Region tab |
| `frontend/src/features/settings/pages/SettingsPage.tsx` | Add `language` tab |
| `design/specs/language-switcher-ux.md` | This spec |

## Appendix B — Redlines Summary

| Element | Key dimensions |
|---------|---------------|
| Trigger height | 40px |
| Trigger padding | `px-3 py-2` |
| Trigger border radius | `12px` |
| Dropdown width | 288px |
| Dropdown border radius | `16px` |
| Dropdown max height | 360px |
| List item height | 44px min (`py-2.5`) |
| Search input height | 36px |
| Flag emoji size | `16px` |
| Native name font | `13px semibold` |
| English name font | `12px normal` |
| Beta badge | `10px semibold uppercase` |
| Checkmark icon | `16px` |
| Grid columns | `24px 1fr auto auto 20px` |
| Mobile sheet breakpoint | `< 640px` |
| Touch target (mobile) | `44px` min height |
| Focus ring | `2px solid #f1b400`, `2px offset` |