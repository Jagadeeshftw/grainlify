# Skill Endorsement UI Design Spec for ProfilePage

**Version:** 1.0
**Status:** Design specification and QA checklist
**Target:** `frontend/src/features/dashboard/pages/ProfilePage.tsx`

---

## Overview

A contributor skill endorsement system adds peer validation to ProfilePage. The design includes:
- A compact top-skills pill list with endorsement counts.
- A primary endorse button for each skill.
- A full skill modal with search and paginated endorser listing.
- An add-skill flow for profile owners.
- Clear states for top skill, viewer-endorsed skill, and owner-edit mode.
- Motion and reduced-motion variants.

This specification covers desktop (`1440px`) and mobile (`375px`) layouts, accessibility requirements, interaction states, security assumptions, and test coverage.

---

## Goals

- Make technical skills visible and social proof believable.
- Enable peer endorsements in a lightweight, secure UI.
- Keep the component visually aligned with the redesigned ProfilePage glassmorphism system.
- Surface the top 5 skills by endorsement count while preserving access to all skills via modal.
- Ensure keyboard operability, ARIA updates, and reduced-motion compatibility.

---

## Data Contract

### Skill endorsement model

```ts
interface ProfileSkill {
  id: string;
  name: string; // e.g. "Rust", "Soroban", "React"
  endorsement_count: number;
  is_top_skill: boolean;
  is_endorsed_by_viewer: boolean;
  is_owner_skill: boolean;
  owners_approved?: boolean;
}

interface SkillEndorser {
  id: string;
  login: string;
  avatar_url?: string;
  endorsement_date: string; // ISO 8601
}

interface SkillEndorsementResponse {
  skills: ProfileSkill[];
  endorsers: Record<string, SkillEndorser[]>;
}
```

### Required API behavior

- `GET /profile/:userId/skills` should return a sorted skill list with endorsement counts and viewer endorsement flags.
- `POST /profile/:userId/skills/:skillId/endorse` should create a new endorsement for the viewer.
- `DELETE /profile/:userId/skills/:skillId/endorse` should remove the viewer endorsement if supported.
- `POST /profile/:userId/skills` should allow owners to add a skill.
- The backend must reject self-endorsement requests and enforce authenticated identity.

---

## UI Structure

### Desktop layout (1440px)

- Skill section appears inside the profile summary area below the bio / social row or within the right-hand stats card.
- A section header: `Skills & endorsements` plus a subtle info tooltip or caption.
- Top 5 skill pills arranged horizontally in a responsive row.
- Each pill includes:
  - Skill name
  - Endorser count badge
  - Endorse button (secondary action on desktop or inline icon button)
- A `See all skills` button opens the full skills modal.
- Owners see an `Add skill` pill/button at the end of the row.

### Mobile layout (375px)

- Stack header and row vertically.
- Display up to 3 top skills; use a `+ more` pill when more than 5 skills exist.
- Endorse button appears as an icon button inside each pill or directly below the skill list.
- The full modal uses a bottom-sheet style or centered dialog with full-width inputs.

---

## Component States

### Skill pill states

| State | Visual treatment | Behavior |
|---|---|---|
| Default | soft glass background, standard border, `text-[#2d2820]` / `text-[#f5f5f5]` | clickable skill target, open modal on desktop if not endorsed? |
| Highlighted (top skill) | bright gold border, soft glow, `bg-white/[0.2]` or `bg-[#c9983a]/10` | visually elevated, denoted as top skill |
| Endorsed by viewer | solid gold accent border, filled count pill, check icon or `Endorsed` label | button disabled / toggled, count badge updates on endorsement |
| Owner edit mode | dashed border, `+ Add skill` button or inline input token | owner can add skill, remove suggested skills, open add-skill flow |

### Button states

- Endorse button default: `bg-[#ffffff]/[0.16] border-white/20 text-[#c9983a]`
- Hover: `bg-[#c9983a]/10 border-[#c9983a]/40`
- Active: `scale-95`, `shadow-[0_0_0_4px_rgba(201,152,58,0.12)]`
- Disabled (already endorsed): `bg-[#d4af37]/20 text-[#f5f5f5] opacity-80 cursor-not-allowed`
- Focus: `outline-2 outline-offset-2 outline-[#f1b400]`

---

## Full Skills Modal

### Modal contents

- Header: `All skills` + close button.
- Search input with placeholder: `Search skills...`.
- Skill table or grid with:
  - Skill name
  - endorsement count
  - `Endorsed` state label or button
- Paginated endorser list per selected skill below the skill grid.
- Summary row for selected skill:
  - `Top endorsed by 34 contributors`
  - small avatars or initials of endorsers
- Owner-only callout: `Add a skill to this profile`.

### Modal behavior

- Focus trap inside modal.
- Close on `Esc`.
- Open at first skill if route includes `#skill=...` or from the pill action.
- Keyboard navigation in search and pagination: Tab between search, skill items, endorser list.
- Clear state when closed.

### Pagination

- Use page size 10 endorsers.
- `Previous` / `Next` controls.
- `Page 1 of N` label.
- If endorsements exceed 100, show `Showing 10 of 120 endorsers`.

---

## Add Skill Flow

### Owner experience

- Render an `Add skill` pill with icon and `+ Add skill` label.
- Open a modal or inline control with:
  - Text input
  - `Add` button
  - Suggested common skills list
- Validation:
  - Non-empty
  - max 30 characters
  - only letters, numbers, spaces, dashes
- After add:
  - show success toast: `Skill added to profile`
  - insert skill into top skill list if endorsement count is high enough.

### Edge cases

- Duplicate skill name: show `This skill already exists on your profile.`
- Empty submission: show inline validation error.
- New skill appears with `0 endorsements` and owner label.

---

## Interaction Motion

### Endorse confirmation micro-animation

- Trigger when the viewer clicks endorse.
- Animate:
  1. Pill pulse: `scale(1.08)` for `130ms`
  2. Count badge expands from `scale(0.9)` to `scale(1)` and increments.
  3. A small gold `+1` fade-out from the count badge.
- Timeline:
  - 0ms: button press feedback.
  - 120ms: pulse peak.
  - 220ms: badge count update.
  - 260ms: fade-out complete.

### Reduced motion variant

- Respect `prefers-reduced-motion: reduce`.
- Use opacity and color transitions only; disable scale transforms.
- Keep `Endorsed` state change instantaneous and skip pulsing.

---

## Accessibility Requirements

- All interactive pills and buttons must support `Enter` and `Space`.
- Use `aria-pressed` on endorse toggle buttons.
- Use `aria-live="polite"` for endorsement count updates.
- Provide labels:
  - `aria-label="Endorse Rust skill"`
  - `aria-label="Open all skills modal"`
- Ensure contrast for pill text and counts on glassmorphism backgrounds.
- Modal should use `role="dialog"`, `aria-modal="true"`, and `aria-labelledby`.
- Endorser avatars should include `alt="Avatar of <login>"` or `alt=""` if decorative.
- Keyboard users must be able to close modal with `Esc` and tab through all controls.

---

## Security Assumptions

- Self-endorsement is blocked server-side.
- Viewer endorsement state is determined by authenticated session, never trusted from the client.
- The client only renders sanitized skill names.
- The add-skill flow is available only to profile owners and requires the same authenticated user ID as the profile.
- Endorsement count updates are authoritative from the backend.
- UI may optimistically update but must reconcile with server response.

---

## QA Checklist

### Visual / layout
- [ ] Top 5 skills render correctly on `1440px` and `375px`.
- [ ] Top skill is visually highlighted with a gold accent.
- [ ] Owner edit mode pill appears only for profile owners.
- [ ] Glassmorphism pill contrast passes on both light and dark backgrounds.
- [ ] `See all skills` / `+ more` behavior is responsive.

### Interaction
- [ ] Endorse button is keyboard operable (Tab, Enter, Space).
- [ ] Already endorsed button is disabled and announced.
- [ ] Modal can open and close with mouse and keyboard.
- [ ] Search input filters skills as expected.
- [ ] Pagination controls can move between endorser pages.
- [ ] Add-skill input validates duplicate and invalid names.

### Accessibility
- [ ] Endorsement count updates announce text via `aria-live="polite"`.
- [ ] Modal focus is trapped and restored after close.
- [ ] `Esc` closes modal.
- [ ] All buttons have descriptive `aria-label`s.
- [ ] Skill pills have visible focus rings.
- [ ] Reduced motion is honored for animation.

### Edge cases
- [ ] No skills: shows fallback empty state.
- [ ] More than 5 skills: `+ more` pill or `See all skills` appears.
- [ ] Skill name > 30 chars is rejected in add-skill flow.
- [ ] Owner views own profile with edit controls only.
- [ ] Non-owner views another profile with endorse actions only.

---

## Notes for Implementation

- Use `@radix-ui/react-dialog` or existing modal pattern in `frontend/src/shared/components`.
- Prefer a lightweight component with local state for endorsement updates and server reconciliation.
- Keep the skill section self-contained for later test coverage.
- Render the top skill pill with a subtle glowing border and badge so it stands out.
- Use consistent typography with ProfilePage headings and text.

---

## Expected Artifacts

- Annotated mockups for:
  1. In-profile skill section view (desktop + mobile)
  2. Endorse interaction state and confirmation animation
  3. Full-skills modal with search + paginated endorser list
- NatSpec-style component documentation in `frontend/src/features/dashboard/components/SkillEndorsementSection.tsx`.
- A design test checklist and security notes in this document.

---

## Future extension

- Add badge filtering for `Blockchain`, `Frontend`, `Backend`, `UI` categories.
- Support `unendorse` if policy allows.
- Add a `view full endorsers` modal from the skill count badge.
- Surface aggregated skill score and endorsement heatmap later.
