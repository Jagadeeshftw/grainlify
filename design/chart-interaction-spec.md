# Chart & Data-Viz Interaction Spec — DataPage

**Branch:** `design/datapage-chart-interaction`  
**File:** `frontend/src/features/dashboard/pages/DataPage.tsx`  
**WCAG:** 2.1 AA  

---

## 1. Tooltip

### Layout
- Background: `rgba(26,20,16,0.95)` with `backdrop-blur`
- Border: `2px solid rgba(255,255,255,0.2)`
- Border radius: `12px`
- Padding: `16px 20px`
- Min width: `200px`

### Data row format
```
[color dot] Label         Value
[●] New                   12
[●] Reactivated            5
[●] Active                28
[●] Churned               -8
────────────────────────────
[●] Rewarded        15,420 USD
```

### Position logic
- Default: right of cursor + 12px
- Near right edge: flip to left
- Near top edge: anchor to bottom
- Show: `opacity` transition 120ms, 80ms delay
- Hide: immediate

### States
| State    | Behaviour                        |
|----------|----------------------------------|
| Default  | Hidden                           |
| Hover    | Visible, follows cursor          |
| Keyboard | Visible on focused data point    |
| Empty    | Shows "No data for this period"  |
| Error    | Shows "Failed to load · Retry"   |

### Accessibility
- `role="tooltip"`
- `aria-live="polite"`
- Keyboard: Arrow keys navigate data points, tooltip updates on focus

---

## 2. Legend — Click-to-Toggle Series

### Anatomy
- Layout: flex row, wrap, gap 8px
- Item padding: `5px 10px`
- Item border radius: `10px`
- Border: `1px solid rgba(255,255,255,0.25)`
- Color swatch: `12px × 12px`, `border-radius: 3px`
- Label: `13px`, `font-weight: 600`

### States
| State    | Visual                                    |
|----------|-------------------------------------------|
| Active   | Normal opacity, colored border            |
| Hover    | `bg-white/[0.2]`                         |
| Focus    | `outline: 2px solid #c9983a`, offset 2px |
| Inactive | `opacity: 0.4`, swatch `filter: grayscale(1)` |

### Interaction
- Click: toggles series visibility in chart
- Keyboard: `Tab` to focus, `Enter`/`Space` to toggle
- `aria-pressed="true/false"`
- `aria-label="Toggle New series"`

### Series colors (existing tokens)
| Series      | Color     |
|-------------|-----------|
| New         | `#c9983a` |
| Reactivated | `#d4af37` |
| Active      | `#c9983a` at 70% |
| Churned     | `#ff6b6b` |
| Trend line  | `#2d2820` |

---

## 3. World Map — Hover, Focus & Color Scale

### Country region states
| State   | Fill                        | Stroke              |
|---------|-----------------------------|---------------------|
| Default | `rgba(255,255,255,0.05)`   | `#c9983a` 0.5px     |
| Highlighted | `url(#mapGradient)`    | `#c9983a` 0.5px     |
| Hover   | `#d4af37` at 80% opacity   | `#ffffff` 1.5px     |
| Focus   | Same as hover              | `outline: 2px solid #c9983a` |
| No data | `rgba(255,255,255,0.03)`   | `rgba(255,255,255,0.1)` |

### Tooltip panel
- Width: `160px`
- Background: `rgba(26,20,16,0.95)`
- Border radius: `8px`
- Padding: `10px 13px`
- Country label: `13px / 600`
- Value: monospace `11px / 500` in `#c9983a`
- Position: fixed top-right of map container

### Color scale legend
- Height: `6px`, border radius `3px`
- Gradient: low `#2d2820` → mid `#c9983a` → high `#d4af37`
- Labels: `10px`, muted color, below bar

### SVG Accessibility
```tsx
<ComposableMap
  role="img"
  aria-label="World map showing contributor distribution by country"
>
```
- Each `<Geography>` that is highlighted: `role="button"`, `tabIndex={0}`, `aria-label="Germany: 720 contributors"`
- Visually hidden `<table>` below map for screen readers

---

## 4. Export UX

### Trigger button
- Position: top-right of each chart card header
- Label: "Export" with download icon
- Height: `34px`, padding `8px 14px`
- Style: matches existing interval dropdown style

### Format selection panel
- Width: `220px`, dropdown below trigger
- Options: CSV (raw data), PNG (chart snapshot)
- Selected state: `bg-[#c9983a]/10`, border `#c9983a/30`
- Role: `role="radiogroup"`, options `role="radio"`
- Close: click outside or `Escape`

### Download button
- Full width, `bg-[#c9983a]`, `color: white`
- Loading state: disabled + progress bar + `aria-busy="true"`

### States
| State   | Behaviour                            |
|---------|--------------------------------------|
| Default | Trigger button only                  |
| Open    | Panel visible, focus trapped         |
| Loading | Progress bar, button disabled        |
| Error   | "Export failed · Retry" message      |
| Success | Panel closes, file downloads         |

---

## 5. Chart Container States

| State   | Visual                                      | Behaviour              |
|---------|---------------------------------------------|------------------------|
| Default | Normal render                               | —                      |
| Hover   | Border brightens to `rgba(255,255,255,0.3)` | Cursor: pointer        |
| Focus   | `outline: 2px solid #c9983a`               | Keyboard accessible    |
| Loading | Shimmer animation (see skeleton-motion.md)  | Data fetching          |
| Empty   | Dashed border + "No data" message           | No data returned       |
| Error   | Amber border + "Failed to load · Retry"     | Fetch failed           |

---

## 6. Accessibility Checklist (WCAG 2.1 AA)

- [x] All chart containers: `role="img"` + `aria-label`
- [x] Tooltip: `role="tooltip"` + `aria-live="polite"`
- [x] Legend items: `role="button"` + `aria-pressed`
- [x] Keyboard: Tab → charts, Arrow → data points
- [x] Focus indicators: `2px solid #c9983a`, offset `2px`
- [x] Color-blind safe: shapes + patterns as secondary indicators
- [x] Screen reader fallback: visually-hidden data table
- [x] `prefers-reduced-motion`: disables shimmer (see skeleton-motion.md)
- [x] Text contrast: body `9.4:1`, muted `4.6:1` ✓
- [x] Touch targets: min `44×44px` on mobile

---

## 7. Responsive Breakpoints

| Breakpoint | Layout change                                      |
|------------|----------------------------------------------------|
| sm 640px   | Charts stack single column, legend wraps below     |
| md 768px   | 2-col grid, legend inline in header                |
| lg 1024px  | Full 2-col, export button in chart top-right       |
| xl 1280px  | Max-width hit, map shows country labels            |

---

## 8. Design Tokens Added

```json
"chart": {
  "series-1": "#c9983a",
  "series-2": "#d4af37",
  "series-3": "#c9983a-70",
  "series-churned": "#ff6b6b",
  "series-trend": "#2d2820",
  "tooltip-bg": "rgba(26,20,16,0.95)",
  "tooltip-border": "rgba(255,255,255,0.2)",
  "map-highlighted": "url(#mapGradient)",
  "map-hover": "#d4af37",
  "map-hover-stroke": "#ffffff",
  "focus-ring": "#c9983a"
}
```