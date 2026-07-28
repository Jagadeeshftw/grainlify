# Design System Spec - Elevation, Shadows, and Glassmorphism

This specification documents and standardizes the elevation levels, box-shadow tokens, and glassmorphism (backdrop-blur + translucent fill) visual rules across the Grainlify dashboard and website interfaces. Consistent use of these elements provides legible, intentional depth cues while maintaining strict accessibility compliance.

---

## 1. Five-Level Elevation Model

Grainlify uses a 5-level elevation system (0–4) to separate layers, suggest interaction states, and organize complex pages. 

| Level | Token Name | Shadow Style (Light Theme) | Shadow Style (Dark Theme) | Core Use Cases |
| :--- | :--- | :--- | :--- | :--- |
| **0** | `elevation-0` | `none` | `none` | Ground layer, main page backgrounds, layout boundaries, disabled inputs. |
| **1** | `elevation-1` | `0 1px 3px 0 rgba(0, 0, 0, 0.05), 0 1px 2px -1px rgba(0, 0, 0, 0.05)` | `0 1px 3px 0 rgba(0, 0, 0, 0.2), 0 1px 2px -1px rgba(0, 0, 0, 0.2)` | Standard resting cards (Project, Blog, Stats), static list rows, secondary buttons. |
| **2** | `elevation-2` | `0 4px 6px -1px rgba(0, 0, 0, 0.08), 0 2px 4px -2px rgba(0, 0, 0, 0.08)` | `0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 2px 4px -2px rgba(0, 0, 0, 0.3)` | Hovered/active card states, primary dropdown triggers, filter chips, active navigation items. |
| **3** | `elevation-3` | `0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1)` | `0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.4)` | Floating elements, dropdown menus (`GlassDropdown`), user profile selectors, notifications. |
| **4** | `elevation-4` | `0 20px 25px -5px rgba(0, 0, 0, 0.15), 0 8px 10px -6px rgba(0, 0, 0, 0.15)` | `0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.5)` | Overlay modals (`Modal`), toast notifications, global alert overlays. |

---

## 2. Glassmorphism Design Tokens

Glassmorphism provides a premium, layered aesthetic that visually blends backgrounds. We standardize these values across light and dark modes to maintain high aesthetic standards and legible content.

### A. Specifications

*   **Light Theme Specs**:
    *   **Background Fill**: `rgba(255, 255, 255, 0.15)` (15% opacity white)
    *   **Backdrop Blur**: `blur(25px)`
    *   **Border Stroke**: `1px solid rgba(255, 255, 255, 0.25)` (25% opacity white)
    *   **Depth Cue**: Elevated with `shadow-elevation-1` at rest.
*   **Dark Theme Specs**:
    *   **Background Fill**: `rgba(255, 255, 255, 0.08)` (8% opacity white)
    *   **Backdrop Blur**: `blur(25px)`
    *   **Border Stroke**: `1px solid rgba(255, 255, 255, 0.15)` (15% opacity white)
    *   **Depth Cue**: Elevated with adjusted dark `shadow-elevation-1` at rest.

### B. High-Contrast Overlay Fallback (Accessible Overlays)

For popover elements (such as `GlassDropdown` menus and global `Modals`), background translucency poses contrast risks when sweeping over content-heavy background grids.
*   **Accessible Glass Fallback**: Floating select dropdown menus and modal panes must increase fill opacity to `90%` (light theme: `bg-[#fafaf9]/90`, dark theme: `bg-[#1c1917]/90`) combined with standard backdrop-blur. This ensures text remains perfectly legible (≥4.5:1 ratio) regardless of scrolling background content.

---

## 3. Usage Strategy Matrix

To prevent visual clutter, components must not double-up on separation techniques. Use this decision matrix to determine the primary separation method:

| Scenario / Layer | Best Separation Method | Rationale |
| :--- | :--- | :--- |
| **Inline content sections** | **Border Only** (`border-white/20`) | Subtle structural lines keep the page layout readable without causing "shadow fatigue." |
| **Resting cards in lists/grids** | **Glass + Shadow-Elevation-1** | Blends nicely with the canvas while giving distinct cards a premium floating quality. |
| **Interactive card state changes** | **Elevated Transition** (`scale-102 shadow-elevation-2`) | Clear tactile/depth indicator when a card becomes hoverable or active. |
| **Floating Popovers / Menus** | **Accessible Glass + Shadow-Elevation-3** | Higher-opacity glass + strong drop shadow separates context-menus clearly from the page beneath. |
| **Full Page Modals** | **Solid Overlay Glass + Shadow-Elevation-4** | High opacity prevents page bleed, and maximal shadow directs 100% of the user focus. |

---

## 4. Accessibility Compliance (WCAG 2.1 AA)

All translucent surfaces must enforce these baseline design criteria:
1.  **Text Contrast**: Standard body text overlaying translucent panels must maintain a contrast ratio of **≥4.5:1**.
    *   *Light Mode Text*: Charcoal (`#2d2820`) on white glass.
    *   *Dark Mode Text*: Warm White (`#f5f5f5` or `#e8dfd0`) on charcoal/black glass.
2.  **Focus States**: All interactive elements (dropdown triggers, card buttons, inputs) must draw a clear outline on keyboard focus. Focus rings must achieve **≥3:1** UI component contrast against background layers.
3.  **Reduced Motion Options**: If users specify `prefers-reduced-motion: reduce`, visual scaling animations (`scale-102` / `scale-105`) must transition instantly without transition durations, and scale effects are disabled.

---

## 5. Responsive Specification

Visual depth parameters adapt automatically across critical device breakpoints:
*   **Mobile (<640px - sm)**: Keep shadow spreads minimal to prevent visual bleeding on small screens. Modal containers fill `95vw` width, and backdrop-blur is limited to `20px` to maintain high rendering performance.
*   **Tablet (640px to 1024px - md, lg)**: Card-borders decrease to standard `1px`. Shadows spread normally (`shadow-elevation-1` / `shadow-elevation-2`).
*   **Desktop (>1024px - xl)**: Full spec animations, hover scales (`scale-[1.02]`), and max blur (`blur(25px)`) active for premium desktop viewports.
