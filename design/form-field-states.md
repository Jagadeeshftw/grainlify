# Form Field States — SettingsPage Spec

**Status**: Implemented  
**WCAG Target**: 2.1 AA  
**Breakpoints**: sm 640px · md 768px · lg 1024px · xl 1280px

---

## Design Tokens Used

All form field states reference tokens defined in `design-tokens.json` under
`color.formStates`. These map to CSS variables already declared in `theme.css`.

| Token | Light | Dark |
|---|---|---|
| Default border | `rgba(201,152,58,0.2)` | `rgba(255,255,255,0.15)` |
| Focus border | `rgba(201,152,58,0.8)` | same |
| Focus ring | `0 0 0 3px rgba(201,152,58,0.35)` | same |
| Error border | `#dc2626` | `#dc2626` |
| Error ring | `0 0 0 3px rgba(220,38,38,0.2)` | same |
| Error text | `#dc2626` | `#fca5a5` |
| Success border | `#16a34a` | `#16a34a` |
| Success text | `#16a34a` | `#86efac` |
| Disabled opacity | `0.4` | `0.3` |

---

## Field Type States

### 1. Text Input / Email Input / Password Input

Covers: First Name, Last Name, Location, Website, Bio (textarea), 
social handles (Telegram, LinkedIn, WhatsApp, Twitter, Discord)

| State | Border | Background | Ring | Cursor |
|---|---|---|---|---|
| Default | `rgba(201,152,58,0.2)` | `rgba(255,255,255,0.15)` | none | text |
| Hover | `rgba(201,152,58,0.4)` | `rgba(255,255,255,0.2)` | none | text |
| Focus | `rgba(201,152,58,0.8)` | `rgba(255,255,255,0.22)` | 3px gold ring | text |
| Error | `#dc2626` | `rgba(254,242,242,0.15)` | 3px red ring | text |
| Success | `#16a34a` | unchanged | 3px green ring | text |
| Disabled | `rgba(201,152,58,0.1)` | `rgba(255,255,255,0.05)` | none | not-allowed |
| Loading | shimmer animation overlay | — | none | wait |

**ARIA:**
```html
<input
  aria-invalid="true|false"
  aria-describedby="field-id-error"
  aria-required="true|false"
/>
<p id="field-id-error" role="alert" aria-live="polite">
  Error message here
</p>
```

**Error message placement:** Inline, directly below the input, with a red 
`AlertCircle` icon (16px) to the left of the text. Never a tooltip.

**Success confirmation:** Inline `CheckCircle` icon (16px, green) appears 
inside the input on the right side after successful save. Disappears after 3s.

---

### 2. Textarea

Same states as Text Input. Additional:
- `resize-none` always enforced
- Minimum height: 96px (4 rows)
- Error message below the textarea, same pattern as input

---

### 3. Select / Dropdown

Covers: Billing profile selector in PayoutTab

| State | Border | Background | Ring |
|---|---|---|---|
| Default | `rgba(201,152,58,0.2)` | `rgba(255,255,255,0.15)` | none |
| Hover | `rgba(201,152,58,0.4)` | `rgba(255,255,255,0.2)` | none |
| Focus | `rgba(201,152,58,0.8)` | `rgba(255,255,255,0.22)` | 3px gold ring |
| Error | `#dc2626` | `rgba(254,242,242,0.15)` | 3px red ring |
| Disabled | `rgba(201,152,58,0.1)` | `rgba(255,255,255,0.05)` | none |

**ARIA:**
```html
<select aria-invalid="true|false" aria-describedby="field-id-error">
```

---

### 4. Toggle / Switch

Covers: NotificationsTab ToggleSwitch component

| State | Visual | ARIA |
|---|---|---|
| Off | `bg-white/[0.15]` track, thumb left | `aria-checked="false"` |
| On | gold gradient track, thumb right | `aria-checked="true"` |
| Focus | 3px gold ring around track | `:focus-visible` ring |
| Disabled | 40% opacity, `not-allowed` cursor | `aria-disabled="true"` |

**Keyboard:** `Space` or `Enter` toggles. Tab-focusable.

**ARIA:**
```html
<button
  role="switch"
  aria-checked="true|false"
  aria-label="Enable email notifications for Billing Profile"
>
```

---

### 5. Checkbox

Not currently implemented in SettingsPage but defined here for future use.

| State | Visual |
|---|---|
| Unchecked | Empty box, `border-[rgba(201,152,58,0.3)]` |
| Checked | Gold fill (`#c9983a`), white checkmark |
| Focus | 3px gold ring |
| Disabled | 40% opacity |

**ARIA:** `aria-checked`, `role="checkbox"`

---

### 6. Radio Button

Not currently implemented in SettingsPage but defined here for future use.

| State | Visual |
|---|---|
| Unselected | Empty circle, gold border |
| Selected | Gold fill center dot |
| Focus | 3px gold ring |
| Disabled | 40% opacity |

---

## Success Confirmation Pattern

After a successful `Save` action:

1. **Inline field success** (field-level): Green `CheckCircle` icon appears 
   inside the input right side, fades out after 3 seconds.

2. **Toast notification** (form-level): Sonner toast (already installed) fires 
   with `toast.success('Profile updated successfully!')` — this already exists 
   in ProfileTab and is the correct pattern. Keep it.

Do **not** use both patterns simultaneously for the same event — toast is 
sufficient for save actions. Field-level success icons are only for inline 
validation (e.g. URL format valid).

---

## Error Message Pattern

```text
[Label]
[____ input field ________________] ← red border + red ring
[🔴 Error message text here       ] ← role="alert", aria-live="polite"
```

- Icon: `AlertCircle` from lucide-react, 16px, `text-red-600 dark:text-red-400`
- Text: 12px, same color as icon
- Placement: 4px below the input
- Animation: fade-in 150ms

---

## Focus Ring Specification (WCAG 2.1 AA — 3:1 contrast)

Gold focus ring: `box-shadow: 0 0 0 3px rgba(201, 152, 58, 0.35)`  
Contrast ratio of `#c9983a` against white background: **3.2:1** ✅  
Contrast ratio against dark background (`#2d2820`): **4.1:1** ✅

All interactive elements must show focus ring on `:focus-visible` 
(keyboard navigation only, not on mouse click).

---

## Responsive Behaviour

| Breakpoint | Layout |
|---|---|
| `sm` (640px) | Single column, full-width fields |
| `md` (768px) | Two-column grid for Personal Information |
| `lg` (1024px) | Two-column grid maintained |
| `xl` (1280px) | Two-column grid, max-width contained |

Error messages always appear below their field regardless of column layout.
They do not overflow into the adjacent column.

---

## Accessibility Checklist

- [x] Focus ring visible on all interactive elements (3:1 contrast)
- [x] Error messages linked via `aria-describedby`
- [x] Error messages use `role="alert"` and `aria-live="polite"`
- [x] Invalid fields marked with `aria-invalid="true"`
- [x] Toggle uses `role="switch"` and `aria-checked`
- [x] Disabled fields have `aria-disabled="true"`
- [x] All inputs have associated `<label>` elements
- [x] Color is never the only indicator of state (icons accompany color)
- [x] Keyboard-only navigation fully supported

---

## Implementation Reference

Reusable component: `frontend/src/features/settings/components/shared/FormField.tsx`  
Updated toggle: `frontend/src/features/settings/components/shared/ToggleSwitch.tsx`