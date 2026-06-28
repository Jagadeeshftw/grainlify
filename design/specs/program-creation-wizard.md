# Program Creation Wizard — Design Spec

**Feature:** Multi-step program creation wizard for MaintainersPage
**Issue:** #1386
**Branch:** `design/maintainer-program-creation-wizard`
**Status:** Specification + Implementation
**WCAG Target:** 2.1 AA
**Breakpoints:** mobile 375px · desktop 1440px

---

## Overview

Maintainers creating a new funding program currently face a single dense form. This spec defines a **4-step wizard** that breaks creation into focused decisions, reducing cognitive load and drop-off. The wizard is a modal overlay with glassmorphism styling consistent with the rest of the MaintainersPage.

---

## Trigger

A **"Create program"** button is added to the MaintainersPage top nav bar, right-aligned after the tab group. It uses the same gold-gradient treatment as the primary action buttons elsewhere in the app.

```
[ Select repositories ▾ ]  [ Dashboard ]  [ Issues ]  [ Pull Requests ]   [ + Create program ]
```

---

## Step Structure

| Step | Label    | Icon     | Concern                              |
| ---- | -------- | -------- | ------------------------------------ |
| 1    | Recipe   | BookOpen | Program type, name, ecosystem, desc  |
| 2    | Funding  | Coins    | Prize pool amount, token, per-bounty |
| 3    | Schedule | Calendar | Payout brackets, release schedule    |
| 4    | Review   | Eye      | Read-only summary + publish action   |

---

## Component: Progress Indicator

```
● Recipe ──── ○ Funding ──── ○ Schedule ──── ○ Review
```

- Each step renders as a pill: number badge + label (label hidden on mobile <sm)
- **Complete:** green pill, green connector line, checkmark badge
- **Active:** gold pill, gold connector, numbered badge filled gold
- **Upcoming:** muted pill, muted connector, numbered badge grey
- `role="tablist"` on container; each pill has `role="tab"` + `aria-selected`
- Connector lines are `aria-hidden`

### States (token mapping)

| State    | Dark bg           | Light bg          | Border                | Text          |
| -------- | ----------------- | ----------------- | --------------------- | ------------- |
| Active   | `bg-[#c9983a]/30` | `bg-[#c9983a]/20` | `border-[#c9983a]/60` | `#e8c77f`     |
| Complete | `bg-green-500/20` | `bg-green-100`    | `border-green-500/40` | green-400/700 |
| Upcoming | `bg-white/[0.06]` | `bg-white/20`     | `border-white/10`     | `#b8a898`     |

---

## Step 1 — Recipe & Details

### Recipe selector

4 cards in a 2×2 grid. Each card is a `role="radio"` button:

- **Hackathon 🏆** — Time-boxed event with ranked prizes
- **Bounty 🎯** — Discrete tasks with fixed rewards
- **Grant 🌱** — Milestone-based long-form funding
- **Ongoing ♾️** — Continuous contribution rewards

**Selected state:** gold border + glow shadow + "Selected" checkmark label
**Hover state:** border-white/20 → border-white/30, slight background lift
**Focus:** 2px gold ring (`focus:ring-[#c9983a]/40`)

### Detail fields

- **Program name** — required text input
- **Ecosystem** — required `<select>` (populated from `GET /ecosystems`)
- **Description** — optional textarea, 3 rows

### Validation (on Next)

| Field       | Rule            | Error message                        |
| ----------- | --------------- | ------------------------------------ |
| Recipe      | Must select one | "Select a program type to continue." |
| programName | Non-empty       | "Program name is required."          |
| ecosystem   | Must select one | "Ecosystem is required."             |

---

## Step 2 — Funding

### Prize pool

- **Total amount** — numeric input, required
- **Token** — select: XLM | USDC | EURC (28px width, right of amount)

Live preview card appears once amount > 0:

```
Prize pool preview
50,000 XLM
```

### Escrow callout

Gold-tinted info banner: "Funds are locked in the on-chain Soroban escrow before the program goes live. Contributors are paid directly — the platform never holds funds."

### Per-bounty limits (optional)

- **Min bounty** / **Max bounty** — side-by-side numeric inputs, labelled with token

### Validation (on Next)

| Field         | Rule                  | Error message                          |
| ------------- | --------------------- | -------------------------------------- |
| fundingAmount | > 0                   | "Enter a valid funding amount."        |
| min vs max    | min ≤ max if both set | "Min bounty cannot exceed max bounty." |

---

## Step 3 — Schedule / Brackets

Two independent sections, both optional.

### Payout brackets

A list of rows, each with:

- **Label input** — "1st Place", "Runner-up", etc.
- **Percentage input** — numeric, 0–100
- **Radix Slider** — visually synced with percentage input (`@radix-ui/react-slider`)
- **Remove button** — trash icon, hover red

Running total shown beneath the list:

- **Red + AlertCircle** if total ≠ 100%: "Brackets total N% — must equal 100%"
- **Green + Check** if total = 100%

Uses `role="status"` / `role="alert"` + `aria-live="polite"`.

### Release schedule (toggle gated)

A `@radix-ui/react-switch` toggles the section. When on, milestone rows appear, each with:

- **Milestone name** — text input
- **Release %** — numeric input (what fraction unlocks at this milestone)
- **Unlock after (days)** — numeric, days from program start

### Validation (on Next)

| Condition                      | Error                                                 |
| ------------------------------ | ----------------------------------------------------- |
| Brackets exist and total ≠ 100 | "Bracket percentages must total 100% (currently N%)." |

---

## Step 4 — Review & Publish

Read-only summary in three sections (Recipe & details, Funding, Brackets & schedule). Each section has an "Edit" button that jumps back to the relevant step without clearing state.

A green info banner at the bottom:

> "Publishing will create the program on-chain and lock your prize pool into escrow. Contributors won't be paid until you trigger payouts."

**Publish button** triggers the API call with loading → success states.

---

## Error Surface

All step-level errors appear as a red banner at the top of the scrollable body, with `role="alert"` and `aria-live="assertive"`. Field-level errors appear inline below the input.

| Error type    | Placement   | Role             | aria-live   |
| ------------- | ----------- | ---------------- | ----------- |
| Step-level    | Top of body | `alert`          | `assertive` |
| Field-level   | Below input | `alert`          | `polite`    |
| Bracket total | Below list  | `status`/`alert` | `polite`    |

---

## Modal Shell

| Property         | Value                                                                |
| ---------------- | -------------------------------------------------------------------- |
| Max width        | 560px                                                                |
| Max height       | 90vh (flex column, body scrolls)                                     |
| Border radius    | 24px                                                                 |
| Dark background  | `#3a3228` + `border-white/25`                                        |
| Light background | `#d4c5b0` + `border-white/40`                                        |
| Backdrop         | `bg-black/55 backdrop-blur-sm`                                       |
| z-index          | 10000                                                                |
| ARIA             | `role="dialog"` `aria-modal="true"` `aria-labelledby="wizard-title"` |
| Dismiss          | Escape key, backdrop click, Cancel                                   |
| Focus trap       | Tab / Shift+Tab cycle within modal                                   |
| Focus restore    | Returns to trigger on close                                          |

---

## Navigation

| Situation  | Left button | Right button         |
| ---------- | ----------- | -------------------- |
| Step 1     | Cancel      | Next →               |
| Steps 2–3  | ← Back      | Next →               |
| Step 4     | ← Back      | Publish program ✓    |
| Submitting | disabled    | disabled + spinner   |
| Success    | —           | "Published!" (brief) |

All buttons have `focus:ring-2 focus:ring-[#c9983a]/40` and `focus:outline-none`.

---

## Accessibility Checklist (WCAG 2.1 AA)

- [x] Dialog has `role="dialog"` + `aria-modal` + `aria-labelledby`
- [x] Focus enters modal on open, returns to trigger on close
- [x] Tab / Shift+Tab trapped within modal
- [x] Escape closes modal
- [x] All inputs have `<label htmlFor>` associations
- [x] Required fields marked with `aria-required`
- [x] Invalid fields use `aria-invalid` + `aria-describedby`
- [x] Error messages use `role="alert"` + `aria-live`
- [x] Recipe cards use `role="radiogroup"` / `role="radio"` + `aria-checked`
- [x] Bracket total status uses `aria-live="polite"`
- [x] Progress indicator uses `role="tablist"` / `role="tab"` + `aria-selected`
- [x] Slider uses Radix UI (`@radix-ui/react-slider`) — keyboard accessible out of the box
- [x] Switch uses Radix UI (`@radix-ui/react-switch`) — keyboard accessible out of the box
- [x] Icon-only buttons have `aria-label`
- [x] Color is never the sole status indicator (icons accompany all status colors)
- [x] Focus ring: 2px gold ring on `:focus-visible` (matches `theme.css` global spec)
- [x] Disabled states use `opacity-50 cursor-not-allowed` + HTML `disabled` attribute
- [x] `prefers-reduced-motion`: no mandatory animations in wizard (Loader2 spin only during submit — acceptable as it is user-triggered)
- [x] Touch targets: all buttons min-height 40px (inherits from `py-2.5` + text)
- [x] Mobile (375px): 2×2 recipe grid wraps cleanly, step labels hidden, body scrolls

---

## Responsive Behaviour

| Breakpoint   | Changes                                                      |
| ------------ | ------------------------------------------------------------ |
| < sm (375px) | Step labels hidden in progress indicator (number badge only) |
| sm+          | Full labels visible                                          |
| All          | Modal max-width 560px, max-height 90vh, padded p-4           |

---

## Glassmorphism Design Tokens (used in wizard)

| Token              | Dark                                                      | Light             |
| ------------------ | --------------------------------------------------------- | ----------------- |
| Modal bg           | `#3a3228`                                                 | `#d4c5b0`         |
| Modal border       | `border-white/25`                                         | `border-white/40` |
| Input bg           | `bg-white/[0.08]`                                         | `bg-white/40`     |
| Input border       | `border-white/15`                                         | `border-white/50` |
| Input focus        | `border-[#c9983a]/50`                                     | same              |
| Card bg (inactive) | `bg-white/[0.06]`                                         | `bg-white/20`     |
| Card bg (selected) | `bg-[#c9983a]/20`                                         | `bg-[#c9983a]/15` |
| Primary text       | `#e8dfd0`                                                 | `#2d2820`         |
| Secondary text     | `#b8a898`                                                 | `#7a6b5a`         |
| Gold accent        | `#c9983a`                                                 | same              |
| Gold text          | `#e8c77f`                                                 | `#8b6f3a`         |
| CTA button (dark)  | `from-[#c9983a]/40 to-[#d4af37]/30` border `[#c9983a]/60` |
| CTA button (light) | `from-[#c9983a]/30 to-[#d4af37]/25` border `[#c9983a]/50` |

---

## File Map

| File                                                                     | Action                                                                                  |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| `frontend/src/features/maintainers/components/ProgramCreationWizard.tsx` | **New** — wizard component (self-contained)                                             |
| `frontend/src/features/maintainers/pages/MaintainersPage.tsx`            | **Edit** — add `isWizardOpen` state, "Create program" button, `<ProgramCreationWizard>` |
| `frontend/src/features/maintainers/types/index.ts`                       | **Edit** — append `RecipeType`, `PayoutBracket`, `ScheduleEntry`, `WizardFormState`     |
| `design/specs/program-creation-wizard.md`                                | **New** — this file                                                                     |

---

## Design QA Checklist

- [ ] WCAG 2.1 AA: contrast on all text ≥ 4.5:1 (verified against dark-mode-spec.md tokens)
- [ ] Keyboard navigation: Tab enters modal, cycles through all fields, Escape closes
- [ ] Screen reader: dialog announced, step changes announced via title update
- [ ] Mobile 375px: recipe grid 2×2, inputs full-width, footer buttons stack cleanly
- [ ] Desktop 1440px: modal centered, max-width 560px, no overflow
- [ ] Bracket validation fires on Next (not on input change) to avoid distraction
- [ ] Ecosystem dropdown populated from API; loading skeleton shown while fetching
- [ ] Publish success: brief confirmation, then `onSuccess()` + `onClose()` called
- [ ] Edit links on Review step jump to correct step without clearing any form data
- [ ] `prefers-reduced-motion: reduce` respected (no continuous animations at rest)

---

## Example commit message

```
design: add multi-step program creation wizard spec for MaintainersPage

- Add ProgramCreationWizard.tsx: 4-step modal (Recipe, Funding, Schedule, Review)
- Use @radix-ui/react-slider for bracket sliders, @radix-ui/react-switch for schedule toggle
- Full WCAG 2.1 AA: focus trap, aria-modal, role=radiogroup, aria-live error regions
- Mobile-first: step labels hidden <sm, 2x2 recipe grid, scrollable body
- Add "Create program" CTA button to MaintainersPage tab bar
- Append wizard types to maintainers/types/index.ts
- Add design/specs/program-creation-wizard.md spec

Closes #1386
```
