# DataPage World Map — Mobile Touch Gesture Spec

**Branch:** `design/data-page-map-gestures`  
**Extends:** [`design/chart-interaction-spec.md`](../chart-interaction-spec.md) §3 (World Map — Hover, Focus & Color Scale)  
**File:** `frontend/src/features/dashboard/pages/DataPage.tsx`  
**Surface:** Contributors map (`ComposableMap` + `ZoomableGroup`)  
**WCAG:** 2.1 AA  
**Status:** Spec (hand-off ready)  
**Last updated:** 2026-07-26  

---

## 1. Overview

[`chart-interaction-spec.md`](../chart-interaction-spec.md) defines desktop legend, hover tooltip, focus, color scale, and export for the DataPage world map. It does **not** specify mobile touch gestures.

This extension adds:

| Capability | Intent |
|---|---|
| Pinch-to-zoom | Two-finger scale within explicit min/max bounds |
| Single-finger pan | Translate the map while zoomed in |
| Double-tap-to-zoom | One-handed zoom without pinch |
| On-screen zoom / reset | Non-gesture fallback for motor / assistive users |
| Tap tooltip | Country detail on tap (replaces hover); dismiss on tap-outside |

Desktop hover/focus behaviour in the parent spec remains unchanged. Touch rules apply at viewports **≤ 768 px** and on any pointer classified as coarse (`pointer: coarse`), including larger tablets.

### Goals

- Keep gesture math aligned with existing `mapZoom` / `mapCenter` state and the current `+` / `−` controls in `DataPage.tsx`.
- Prevent pinch / pan / double-tap from fighting vertical page scroll on 375–428 px viewports.
- Meet WCAG 2.1 AA (keyboard, focus, live regions, contrast, 44×44 touch targets).
- Stay reviewable: states, tokens, redlines, and a Design QA checklist in one file.

---

## 2. Relationship to Parent Spec

| Parent section | This extension |
|---|---|
| §3 Country region states | Adds **Pressed / Tap-selected** for touch; hover unchanged on fine pointers |
| §3 Tooltip panel | Adds **tap-open**, **dismiss-on-tap-outside**, live-region copy |
| §3 SVG Accessibility | Country list + `sr-only` table remain the non-visual fallback; zoom chrome becomes keyboard-primary on touch |
| §6 Touch targets min 44×44 | Current zoom buttons are `32×32` (`w-8 h-8`) — this spec **requires** `44×44` |
| §7 Responsive | Adds gesture / scroll contract for 375–428 px |

Do not duplicate legend, export, or chart-container rules here; implementers follow the parent for those.

---

## 3. Zoom Model

Existing state in `DataPage.tsx`:

```ts
const [mapZoom, setMapZoom] = useState(1);           // scale factor
const [mapCenter, setMapCenter] = useState<[number, number]>([0, 0]);
```

| Constant | Value | Rationale |
|---|---|---|
| `ZOOM_MIN` | `1` | Fits full world; matches current zoom-out clamp |
| `ZOOM_MAX` | `8` | Matches current zoom-in clamp (`Math.min(z * 1.5, 8)`) |
| `ZOOM_BUTTON_FACTOR` | `1.5` | Matches existing `+` / `−` buttons |
| `ZOOM_DOUBLE_TAP_FACTOR` | `2` | Larger step for one-handed use; still clamped to `[1, 8]` |
| `ZOOM_DEFAULT` | `1` | Reset-view target |
| `CENTER_DEFAULT` | `[0, 0]` | Reset-view target |

### Clamp helper (implementation contract)

```ts
const clampZoom = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z));
```

All gesture and button paths **must** pass through this clamp. At `ZOOM_MIN`, further zoom-out / pinch-out is a no-op (no rubber-band). At `ZOOM_MAX`, further zoom-in / pinch-in is a no-op.

### Focal point

| Input | Focal point |
|---|---|
| Pinch | Midpoint between the two touch points |
| Double-tap | Tap coordinates (map → projection → `mapCenter` nudge so the tapped country stays under the finger) |
| `+` / `−` buttons | Current `mapCenter` (no recentre) |
| Reset | Force `mapZoom = 1`, `mapCenter = [0, 0]` |

---

## 4. Gesture Spec

### 4.1 Pinch-to-zoom

| Property | Spec |
|---|---|
| Fingers | Exactly 2 |
| Action | Continuous scale around pinch midpoint |
| Bounds | `ZOOM_MIN` … `ZOOM_MAX` |
| During gesture | State → `zooming` (see §6); suppress tooltip open/close |
| On end | Commit `mapZoom` + `mapCenter` via existing `ZoomableGroup.onMoveEnd` |
| Page scroll | Blocked for the duration of the gesture (`touch-action: none` on map surface) |

### 4.2 Single-finger pan

| Property | Spec |
|---|---|
| Fingers | Exactly 1 |
| Enabled when | `mapZoom > ZOOM_MIN` |
| Disabled when | `mapZoom === ZOOM_MIN` — single-finger movement does **not** pan; page scroll only from touches that begin **outside** the map canvas (see §8) |
| Action | Translate `mapCenter` following finger delta |
| Edge behaviour | Soft clamp so empty ocean never fills > ~70% of the viewport (implementation may use `ZoomableGroup` filterExtent / translateExtent) |
| During gesture | State → `panning`; keep tooltip closed (dismiss if open at pan start) |
| Conflict with tap | If movement > `8 px` before `touchend`, treat as pan — do **not** open tooltip |

### 4.3 Double-tap-to-zoom

| Property | Spec |
|---|---|
| Detection | Two `touchend` events within **300 ms**, second touch within **24 px** of the first |
| At `mapZoom < ZOOM_MAX` | Zoom by `ZOOM_DOUBLE_TAP_FACTOR` around tap point, then clamp |
| At `mapZoom === ZOOM_MAX` | Reset to `ZOOM_DEFAULT` + `CENTER_DEFAULT` (toggle-out), so users are never stuck max-zoomed without pinch or buttons |
| Timing vs tooltip | Double-tap **wins**: first tap starts a 300 ms deferral before opening tooltip; if second tap arrives, cancel tooltip and zoom instead |
| `prefers-reduced-motion` | Snap zoom with **no** animated transition (instant commit) |

### 4.4 Gesture priority (highest → lowest)

1. Two-finger pinch (always wins while 2 contacts are active)
2. Double-tap zoom (after second tap confirmed)
3. Single-finger pan (when `mapZoom > 1` and movement > 8 px)
4. Single tap → tooltip (when movement ≤ 8 px and no second tap)
5. Page scroll (only for touches that begin outside the map canvas — §8)

---

## 5. Non-Gesture Fallback (Accessibility)

On-screen controls are **required** on all viewports (not mobile-only). They are the accessible equivalent of pinch / double-tap for users who cannot perform multi-touch gestures (WCAG 2.5.1 Pointer Gestures — Level A).

### 5.1 Control cluster (redline)

```
Map container (relative)
┌──────────────────────────────────────────────┐
│                                              │
│                                    ┌───┐     │  ← top-right, 12px inset
│                                    │ + │     │     gap: 8px
│                                    ├───┤     │
│                                    │ − │     │
│                                    ├───┤     │
│                                    │ ↺ │     │  ← Reset view (NEW)
│                                    └───┘     │
│                                              │
└──────────────────────────────────────────────┘
```

### 5.2 Button anatomy

| Token / prop | Value |
|---|---|
| Size | **44×44 px** min (`min-w-[44px] min-h-[44px]`) — replaces current `w-8 h-8` |
| Gap | `8px` (`gap-2`) |
| Radius | `8px` |
| Background | `rgba(255, 255, 255, 0.2)` + `backdrop-blur: 25px` |
| Border | `1px solid rgba(255, 255, 255, 0.3)` |
| Icon / glyph color | `#ffffff` |
| Focus ring | `outline: 2px solid #c9983a`; `outline-offset: 2px` (`chart.container.focus-ring`) |
| Disabled (at bound) | `opacity: 0.4`; `aria-disabled="true"`; still focusable |

### 5.3 Behaviours

| Control | `aria-label` | Action |
|---|---|---|
| Zoom in | `"Zoom in on map"` | `setMapZoom(z => clampZoom(z * 1.5))` |
| Zoom out | `"Zoom out on map"` | `setMapZoom(z => clampZoom(z / 1.5))` |
| Reset view | `"Reset map view"` | `setMapZoom(1)`; `setMapCenter([0, 0])`; dismiss tooltip |

### 5.4 Keyboard

| Key | When focused on… | Action |
|---|---|---|
| `Enter` / `Space` | Any zoom control | Activate that control |
| `Tab` / `Shift+Tab` | Map chrome | Order: Zoom in → Zoom out → Reset → country list rows (see §7) |
| `+` / `=` | Map container (roving) optional | Same as Zoom in |
| `-` / `_` | Map container (roving) optional | Same as Zoom out |
| `0` | Map container (roving) optional | Reset view |

Keyboard path **must not** require touch. Country data remains available via the existing `sr-only` table and the visible country bar list below the map.

---

## 6. Interaction States

| State id | Visual | Behaviour |
|---|---|---|
| `default-zoom` | `mapZoom === 1`, center `[0,0]`; Reset disabled | Full world visible; single-finger vertical scroll may leave the map (§8) |
| `zoomed-in` | `mapZoom > 1`; Reset enabled | Pinch / pan / double-tap / buttons active |
| `panning` | Same as zoomed-in; cursor/finger grab | Tooltip forced closed; ignore tap-open until `touchend` |
| `zooming` | Continuous scale during pinch or animated double-tap | Tooltip forced closed; buttons remain visible but inactive until gesture ends |
| `tooltip-open` | Country highlighted (hover-equivalent fill/stroke from parent §3); tooltip panel visible | Live region announces country + value (§7) |
| `zoom-button-focus` | Focus ring `2px #c9983a` offset 2px on `+`, `−`, or Reset | Keyboard operable; does not open tooltip |

State diagram (logical):

```
default-zoom ──pinch/button/dbltap──► zoomed-in ◄──► panning
     ▲                                   │
     │              reset / dbltap@max   │
     └───────────────────────────────────┘
                    │
              tap country ──► tooltip-open
              tap outside / pan start / Esc ──► dismiss
```

---

## 7. Tap Tooltip (Replaces Hover on Touch)

Parent tooltip panel tokens still apply ([`chart-interaction-spec.md`](../chart-interaction-spec.md) §3 Tooltip panel; `design-tokens.json` → `chart.tooltip` / map tooltip layout).

### 7.1 Open

| Pointer | Open trigger |
|---|---|
| Fine (`hover: hover`) | Unchanged: `onMouseEnter` on geography / marker |
| Coarse / touch | Single tap on a **highlighted** country (data present), after 300 ms double-tap window clears |

Tap on a no-data geography: no tooltip; optional brief live-region `"No contributor data for {country}"` (polite, once).

### 7.2 Position on touch

- Anchor: **fixed top-right inside the map container** (parent already specifies this for the map tooltip), inset `12px`, so the finger does not obscure the panel.
- Width / typography / colors: unchanged from parent (`160px`, `#c9983a` value, etc.).

### 7.3 Dismiss

| Trigger | Result |
|---|---|
| Tap on empty map / ocean / no-data country | Close tooltip; clear highlight |
| Tap outside map container | Close tooltip |
| Start pan or pinch | Close tooltip immediately |
| Activate Reset | Close tooltip |
| `Escape` (keyboard) | Close tooltip |
| Tap a different country | Move tooltip to that country (no flicker: update in place) |

### 7.4 Live region

```html
<div
  role="status"
  aria-live="polite"
  aria-atomic="true"
  class="sr-only"
  data-testid="map-tooltip-live-region"
>
  <!-- e.g. "Germany: 720 contributors, 14 percent" -->
</div>
```

Visible tooltip retains `role="tooltip"` from the parent spec. The live region is the touch announcement path (hover does not need a second announcement if focus already moves).

Copy template: `"{country}: {value} contributors, {percentage} percent"`.

---

## 8. Scroll vs Gesture Contract (375–428 px)

| Condition | `touch-action` on map surface | Outcome |
|---|---|---|
| Always on map canvas | `none` | Browser does not scroll/zoom the **page** from touches that begin on the map; pinch/pan stay in-map |
| Touches that begin **outside** the map | default | Normal page scroll |
| `mapZoom === 1` + single finger vertical drag on map | Still `none` on canvas | Map does not pan (disabled at min zoom); **no** accidental page scroll from the map — user scrolls from page chrome / siblings |
| Nested scroll (country bar list below map) | default on the list | Vertical list scroll unaffected |

Rationale: the map is only `280px` tall; reserving the canvas for gestures avoids pinch/pan fighting body scroll. Document this in the PR description for QA.

**Do not** attach `preventDefault` on `touchmove` at `document` level — scope listeners to the map container only.

---

## 9. Accessibility Annotations

| Requirement | Annotation |
|---|---|
| Pointer Gestures (2.5.1) | Pinch and double-tap have equivalent `+` / `−` / Reset buttons |
| Target Size (2.5.5 best-effort / 2.5.8 AA 24px min) | Controls **44×44**; spacing ≥ 8px |
| Name, Role, Value (4.1.2) | Native `<button>` with explicit `aria-label` (table §5.3) |
| Focus Visible (2.4.7) | Gold focus ring token `chart.container.focus-ring` `#c9983a` |
| Status Messages (4.1.3) | Tap tooltip content mirrored to `aria-live="polite"` region |
| Keyboard (2.1.1) | Zoom / reset and country list fully operable without touch |
| Compatible (4.1.2) | Map keeps `role="img"` + `aria-label` from parent; countries with data keep `role="button"` + `tabIndex={0}` where already specified |
| Reduced motion (2.3.3) | Instant zoom commits; no bounce / elastic overscroll animation |

### Country list fallback (keyboard / no-touch)

The visible country bar list under the map is the primary non-gesture exploration path:

- Each row is a focusable control (`tabIndex={0}` or `<button>`).
- `Enter` / `Space` selects the country → same highlight + tooltip/live-region as a map tap.
- Does not require map focus or touch.

---

## 10. Token & Contrast Validation

Validated against `/design-tokens.json` (`chart.*`) and the map surface in `DataPage.tsx`  
(`bg-gradient-to-br from-[#2d2820]/80 via-[#1a1410]/70 to-[#2d2820]/80`).

Contrast formula: WCAG 2.x relative luminance. Map background reference: `#1a1410` (tooltip / deep map) and `#2d2820` (`color` / chart trend / surfaceSecondary).

| Element | Foreground | Background | Ratio | AA (≥4.5:1 text / ≥3:1 UI) |
|---|---|---|---|---|
| Zoom glyph `+` `−` `↺` | `#ffffff` | Zoom btn glass ≈ `#484340` (20% white over `#1a1410`) | **9.76:1** | ✅ text |
| Zoom glyph on hover | `#ffffff` | ≈ `#5f5b58` (30% white over `#1a1410`) | **6.72:1** | ✅ text |
| Tooltip country label | `#ffffff` | `chart.tooltip.background` `rgba(26,20,16,0.95)` ≈ `#1a1410` | **18.24:1** | ✅ |
| Tooltip value | `#c9983a` (`chart.series.new` / gold-600) | `#1a1410` | **6.98:1** | ✅ |
| Focus ring | `#c9983a` | `#1a1410` | **6.98:1** | ✅ UI (≥3:1) |
| Color-scale legend labels (current `text-white/40`) | blended ≈ `#767270` | `#1a1410` | **3.83:1** | ❌ **fail** — raise to `text-white/60` (≈ `#a3a19f` → **7.08:1**) |
| Density legend title (current `text-white/60`) | ≈ `#a3a19f` | `#1a1410` | **7.08:1** | ✅ |

### Required token notes for implementers

```json
"chart": {
  "map": {
    "zoom-min": 1,
    "zoom-max": 8,
    "zoom-button-factor": 1.5,
    "zoom-double-tap-factor": 2,
    "control-size": "44px",
    "control-gap": "8px",
    "control-bg": "rgba(255, 255, 255, 0.2)",
    "control-border": "rgba(255, 255, 255, 0.3)",
    "control-icon": "#ffffff",
    "legend-label": "rgba(255, 255, 255, 0.6)"
  }
}
```

Add these under the existing `chart.map` object in `design-tokens.json` at implementation time (this design PR documents them; token file update may land with the engineering ticket).

---

## 11. Annotated Redlines (Review Aid)

### 11.1 Touch viewport 390×844 (example)

```
┌─ DataPage / Contributors map card ─────────────────┐
│  Contributors map                                  │
│  ┌─ Density legend ──────────────────────────────┐ │
│  │ Low ═══════ Medium ═══════ High               │ │
│  └───────────────────────────────────────────────┘ │
│  ┌─ Map 280px ─ touch-action:none ───────────────┐ │
│  │                                    [+] 44²    │ │
│  │                                    [−] 44²    │ │
│  │                                    [↺] 44²    │ │
│  │         (pinch mid) ●────●                    │ │
│  │              ↑ ZOOM_MIN 1 … ZOOM_MAX 8        │ │
│  │  tooltip ──► ┌ Germany ─────────┐ (top-right) │ │
│  │              │ 720   14%        │             │ │
│  │              └──────────────────┘             │ │
│  └───────────────────────────────────────────────┘ │
│  Country bars (keyboard + tap fallback)            │
└────────────────────────────────────────────────────┘
         ↕ page scroll from outside map only
```

### 11.2 Spacing

| Measurement | Value |
|---|---|
| Map container height | `280px` (existing) |
| Control cluster inset | `12px` from top + right |
| Control hit area | `44×44` |
| Control visual gap | `8px` |
| Tooltip inset | `12px` from top + right (below or left of controls if collision — prefer **left of** controls by `8px`) |
| Double-tap slop | `24px` |
| Pan vs tap threshold | `8px` |

### 11.3 Tooltip vs controls collision

If the tooltip and the control cluster overlap, shift tooltip to `left: 12px` (top-left) while controls stay top-right. Never cover the 44×44 hit targets.

---

## 12. Design QA Checklist

### Contrast (WCAG 1.4.3 / 1.4.11)

- [x] Zoom control icons `#ffffff` on control glass ≥ 4.5:1 (measured **9.76:1**)
- [x] Tooltip text / gold value on `chart.tooltip.background` ≥ 4.5:1 (**18.24:1** / **6.98:1**)
- [ ] Legend Low/Medium/High labels bumped to `white/60` before ship (current `white/40` fails at **3.83:1**)

### Keyboard-only walkthrough

- [ ] Tab reaches Zoom in, Zoom out, Reset — each activates with Enter/Space
- [ ] Reset restores zoom 1 + center `[0,0]` and closes tooltip
- [ ] Country list rows open the same detail as a map tap (live region fires)
- [ ] `Escape` dismisses tooltip
- [ ] Focus ring visible on all three controls (`#c9983a`, 2px / 2px offset)

### Touch viewport (375 / 390 / 428 px)

- [ ] Pinch zooms within 1…8 and does not zoom the browser page
- [ ] Single-finger pan only when `mapZoom > 1`; no horizontal “drag lock” of the page
- [ ] Double-tap zooms ×2 (clamped); double-tap at max resets
- [ ] Tap opens tooltip; tap-outside / pan / pinch dismisses
- [ ] Page still scrolls when dragging on card header, legend, or country list

### Pointer gestures fallback (2.5.1)

- [ ] All zoom outcomes achievable with on-screen buttons alone
- [ ] Buttons remain visible at default and zoomed states

### Reduced motion

- [ ] `prefers-reduced-motion: reduce` → no animated zoom easing

---

## 13. Implementation Notes (Out of Scope for This Spec PR)

Engineering follow-up (separate ticket) should:

1. Raise zoom control hit areas to 44×44 and add Reset.
2. Wire `touch-action: none` on the map canvas wrapper.
3. Implement double-tap detector + tap-tooltip deferral (300 ms).
4. Mirror tooltip string into the live region.
5. Fix legend label opacity to `white/60`.
6. Optionally persist tokens from §10 into `design-tokens.json`.

This document is the design source of truth for that work.

---

## 14. PR Description Template (when opened)

```markdown
## Summary
- Extends chart-interaction-spec.md with DataPage world-map mobile gestures
- Spec: design/specs/data-page-map-mobile-gestures.md (pinch / pan / double-tap + button fallback + tap tooltip)

## Test plan
- [ ] Contrast check: zoom glyphs + tooltip on map background ≥ 4.5:1
- [ ] Keyboard: + / − / Reset + country list without touch
- [ ] Touch 375–428: pinch/pan/double-tap do not steal page scroll
- [ ] Tap tooltip opens/dismisses per §7

## Redlines
See ASCII redlines in §11 of the spec (annotated control cluster + scroll contract).
```
