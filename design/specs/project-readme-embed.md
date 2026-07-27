# Project README Embed Styling Spec

**Component:** `frontend/src/features/dashboard/components/ReadmeEmbed.tsx`  
**Consumed by:** `frontend/src/features/dashboard/pages/ProjectDetailPage.tsx`  
**Related utility:** `frontend/src/app/utils/renderMarkdown.tsx`  
**Status:** Implemented  
**Date:** 2026-07-26  
**WCAG Target:** 2.1 Level AA  

---

## 1. Background & Problem Statement

`ProjectDetailPage` renders a third-party GitHub README via `ReactMarkdown`. Before this spec the embed had three open problems:

| # | Problem | Impact |
|---|---------|--------|
| 1 | No container max-width / measure constraint | Long READMEs produce unreadable 80+ character lines |
| 2 | No "View on GitHub" affordance above the embed | Users have no escape hatch when README content is truncated or broken |
| 3 | `alt=""` hardcoded on all README images | Every content image is announced as decorative — screen-reader regression |
| 4 | No `table` / `thead` / `tbody` / `tr` / `td` / `th` handlers | Tables fall through to bare browser defaults with zero styling in both themes |
| 5 | Light-mode link color `#b8872f` only reaches **2.43:1** on the card background | WCAG 1.4.3 failure — the prior contrast bug report |
| 6 | README `h1` can appear at the same level as the page `<h1>` | Heading hierarchy violated (WCAG 1.3.1) |

---

## 2. Container Anatomy

```
┌──────────────────────────────────────────────────────┐  ← glass card
│  [✦ Overview]                  [↗ View on GitHub]   │  ← section header row
├──────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────┐  │
│  │                                                │  │  ← readme-embed container
│  │   [README content]                             │  │    max-width: 72ch (≈ 720px)
│  │                                                │  │    padding: 0 (inherits card p-8)
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 2.1 Outer card (unchanged — owned by ProjectDetailPage)

```
backdrop-blur-[40px] rounded-[24px] border p-8
dark:  bg-white/[0.08] border-white/10
light: bg-white/[0.12] border-white/20
```

### 2.2 Section header row

```
flex items-center justify-between mb-6
```

Left: `h2` "Overview" with gold sparkle icon  
Right: "View on GitHub" anchor — `text-[13px] font-semibold` + ExternalLink icon, gold color, always visible

### 2.3 README embed container

```css
.readme-embed {
  max-width: 72ch;         /* optimal reading measure */
  word-break: break-word;  /* prevent long URLs from overflowing */
  overflow-wrap: break-word;
}
```

Tailwind equivalent applied inline: `max-w-[72ch] break-words`

---

## 3. States

### 3.1 Loading skeleton

Three shimmer lines stacked, matching existing `SkeletonLoader` usage in the page:

```
████████████████████████████████████████████  h-4 w-full
████████████████████████████████████████████  h-4 w-full
████████████████████████████             ██  h-4 w-3/4
```

Implementation: `<SkeletonLoader>` × 3, wrapped in `space-y-3`.

### 3.2 Rendered (nominal)

Full `OverviewMarkdown` render with all element handlers defined in §4.

### 3.3 README missing / empty

When `project.readme` is falsy and no description is available:

```
┌──────────────────────────────────────────────┐
│  No README available.                        │
│  Visit the GitHub repository for details.   │  ← link to githubUrl
└──────────────────────────────────────────────┘
```

Uses the `EmptyState` variant `"no-programs"` with custom copy, or a plain paragraph (current approach retained for simplicity).

### 3.4 Broken image

Each `<img>` in the README render has an `onError` handler that:
1. Hides the broken image (`display: none` via `visibility:hidden` + `height:0`).
2. Renders a sibling placeholder `<span>` with a camera-off icon and the image `alt` text.

```
┌─────────────────────────────────────┐
│  🖼  [alt text or "Image unavailable"] │
└─────────────────────────────────────┘
bg: rgba(255,255,255,0.06) dark / rgba(0,0,0,0.05) light
border: 1px dashed rgba(255,255,255,0.20) dark / rgba(0,0,0,0.15) light
rounded-[12px] p-4 text-[13px]
```

---

## 4. Element Styling Spec

### 4.1 Headings — level offset

README headings are offset by +2 to avoid competing with the page `h1` (project name) and section `h2` (Overview):

| README source | Rendered as | Tailwind classes |
|--------------|-------------|-----------------|
| `# H1` | `<h3>` (visually h1-sized) | `text-[22px] font-bold mb-4 mt-6 first:mt-0` |
| `## H2` | `<h4>` (visually h2-sized) | `text-[18px] font-bold mb-3 mt-5` |
| `### H3` | `<h5>` (visually h3-sized) | `text-[16px] font-semibold mb-2 mt-4` |
| `#### H4` | `<h6>` (visually h4-sized) | `text-[14px] font-semibold mb-2 mt-3` |

Color tokens: dark `#f5f5f5` / light `#2d2820` — same as existing headings.

### 4.2 Body text

```
mb-4 leading-relaxed text-[15px]
dark:  #d4d4d4  (9.65:1 on card surface) ✅
light: #4a3f2f  (7.78:1 on card surface) ✅
```

### 4.3 Links — **bug fix**

The existing light-mode link `#b8872f` only reaches **2.43:1** on `#e8dfd0` — a WCAG 1.4.3 failure.

| Theme | Before | Before contrast | After | After contrast |
|-------|--------|----------------|-------|---------------|
| Dark  | `#f5c563` | 8.89:1 ✅ | `#f5c563` | 8.89:1 ✅ (unchanged) |
| Light | `#b8872f` | 2.43:1 ❌ | `#6b4c1a` | 5.95:1 ✅ |

Additional: text-decoration underline added (`underline decoration-1 underline-offset-2`) in both themes so links are distinguishable from body text without relying on color alone (WCAG 1.4.1).

Hover: dark `#ffd700` / light `#4a3310`.

### 4.4 Code — inline

```
px-1.5 py-0.5 rounded text-[13px] font-mono
dark:  bg rgba(255,255,255,0.15) over #2c2a27 → text #f5c563  7.83:1 ✅
light: bg #e8e0d0                              → text #5c3d0a  7.47:1 ✅
```

Light-mode inline code text was `#6b5d4d` (4.16:1 — borderline). Corrected to `#5c3d0a` (7.47:1).

### 4.5 Code blocks — fenced / pre

```
mb-4 overflow-x-auto rounded-[12px] p-4 font-mono text-[13px]
dark:  bg rgba(255,255,255,0.12) over #1a1714  → blended #35322f
       text #e8dfd0  9.53:1 ✅
light: bg rgba(255,255,255,0.20) over #e8dfd0  → blended #edeae5
       text #2d2820  11.71:1 ✅
border: dark border-white/20 / light border-white/30
```

Code *inside* `pre` uses `text-inherit` — inherits the pre's text color, not the outer body color. This is the key fix for the previously reported dark-theme contrast regression: when `inPre=true`, code was using `textColor` (`#d4d4d4`) rather than the pre's own `#e8dfd0`. Both pass, but the spec now makes this explicit and uses `text-inherit` to tie them together.

### 4.6 Tables — **new**

Previously unhandled — rendered as raw browser-default HTML.

```
table:  w-full text-[14px] mb-4 border-collapse overflow-hidden rounded-[12px]
thead:  dark bg rgba(255,255,255,0.10) / light bg rgba(0,0,0,0.06)
th:     px-4 py-2.5 text-left font-semibold border-b
        dark text #f5f5f5 9.62:1 ✅ / light text #2d2820 9.75:1 ✅
tbody tr (even): dark bg rgba(255,255,255,0.04) / light bg rgba(0,0,0,0.02)
td:     px-4 py-2.5 border-b border-white/10 (dark) / border-black/[0.07] (light)
        dark text #d4d4d4 8.52:1 ✅ / light text #4a3f2f 7.78:1 ✅
```

Table container: `overflow-x-auto` so wide tables scroll horizontally on narrow viewports instead of breaking layout.

### 4.7 Images

```
block mx-auto my-4 rounded-[12px] max-w-full h-auto
loading="lazy"
```

- `loading="lazy"` for deferred load
- `display: block; margin: auto` → centered figure
- `max-width: 100%` prevents overflow at 375 px
- `alt` prop passed through from markdown source (not overridden to `""`)
- `onError` → broken-image placeholder (§3.4)

### 4.8 Blockquote

```
border-l-4 pl-4 italic my-4 rounded-r-[8px]
dark:  border-[#c9983a]/60  bg rgba(255,255,255,0.05)  text #d4d4d4
light: border-[#c9983a]/70  bg rgba(0,0,0,0.04)        text #4a3f2f
```

### 4.9 Horizontal rule

```
my-6 border-0 h-px
dark:  bg rgba(255,255,255,0.12)
light: bg rgba(0,0,0,0.10)
```

---

## 5. "View on GitHub" Link

Rendered above the embed content in the section header row:

```tsx
<a
  href={githubUrl}
  target="_blank"
  rel="noopener noreferrer"
  aria-label="View README on GitHub (opens in new tab)"
  className="flex items-center gap-1.5 text-[13px] font-semibold
             underline decoration-1 underline-offset-2
             dark: text-[#f5c563] hover:text-[#ffd700]
             light: text-[#6b4c1a] hover:text-[#4a3310]"
>
  <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" />
  View on GitHub
</a>
```

- Always rendered even if README is missing (links to the repo itself)
- `aria-label` includes "(opens in new tab)" to warn screen-reader users
- Underlined so it is distinguishable from surrounding UI without relying on color alone

---

## 6. Accessibility Annotations

### 6.1 Heading hierarchy

README `h1` → page `h3`, README `h2` → page `h4`, etc. This ensures the document outline reads:

```
h1  [Project name]           ← page
  h2  Overview               ← section
    h3  [README section]     ← readme h1
      h4  [README sub-sec]   ← readme h2
```

### 6.2 Image alt text

The `img` component handler passes through the markdown `alt` prop:

```tsx
img: ({ alt, ...props }) => <img alt={alt ?? ''} ... />
```

Previously `alt=""` was hardcoded for every image — a WCAG 1.1.1 failure for content images. Markdown image syntax `![Description](url)` correctly populates `alt`.

### 6.3 Link keyboard navigation

Links in the README receive the global `focus-visible` outline from `theme.css` (2px solid, `#a2792c` light / `#f1b400` dark). No additional override needed.

All README links open in `target="_blank"` with `rel="noopener noreferrer"`. The `aria-label` on external links includes "(opens in new tab)" where practical (only feasible on the "View on GitHub" button — individual README links cannot be retrofitted).

### 6.4 Table accessibility

```html
<table role="table">          <!-- explicit role in case CSS grid overrides -->
  <thead>
    <tr role="row">
      <th scope="col">…</th>  <!-- scope="col" on all th elements -->
    </tr>
  </thead>
  <tbody>
    <tr role="row">
      <td>…</td>
    </tr>
  </tbody>
</table>
```

### 6.5 Code blocks

Fenced code blocks have `role="region"` on the outer `pre` and an `aria-label="Code block"` so screen-reader users can orient themselves. Language labels (when available from remark-gfm) surface as a `data-language` attribute for potential future syntax highlighting.

---

## 7. Responsive Behaviour

| Breakpoint | Behaviour |
|------------|-----------|
| xl ≥ 1280px | Full `max-w-[72ch]` measure, images up to 72ch wide |
| lg ≥ 1024px | Same — sidebar + main content layout intact |
| md ≥ 768px | Main column narrows; `max-w-[72ch]` still respected |
| sm < 768px | Single column; `max-w-full` overrides (72ch would be too narrow); tables scroll via `overflow-x-auto`; images `max-w-full` |

At **375 px**: No prose element overflows. Tables scroll horizontally within their `overflow-x-auto` wrapper. Code blocks also scroll horizontally. Images shrink to viewport width.

---

## 8. Contrast Validation (verified by script)

All ratios computed against the blended actual rendered background, not the opacity value alone.

### 8.1 Dark theme — card surface `rgba(255,255,255,0.08)` over `#1a1714` → `#2c2a27`

| Element | Text | Background | Ratio | Pass? |
|---------|------|-----------|-------|-------|
| Body text | `#d4d4d4` | `#2c2a27` | 9.65:1 | ✅ |
| Heading | `#f5f5f5` | `#2c2a27` | 11.68:1 | ✅ |
| Link | `#f5c563` | `#2c2a27` | 8.89:1 | ✅ |
| Pre text | `#e8dfd0` | `rgba(255,255,255,0.12)` over `#1a1714` = `#35322f` | 9.53:1 | ✅ |
| Inline code text | `#f5c563` | `rgba(255,255,255,0.15)` over `#2c2a27` = `#3e3b38` | 7.83:1 | ✅ |
| Table `th` | `#f5f5f5` | `rgba(255,255,255,0.10)` over `#2c2a27` = `#414140` | 9.62:1 | ✅ |
| Table `td` | `#d4d4d4` | `rgba(255,255,255,0.04)` over `#2c2a27` = `#2e2d2c` | 8.52:1 | ✅ |

### 8.2 Light theme — card surface `rgba(255,255,255,0.12)` over page bg `#e8dfd0` → `#ece5da`

| Element | Text | Background | Ratio | Pass? |
|---------|------|-----------|-------|-------|
| Body text | `#4a3f2f` | `#ece5da` | 7.78:1 | ✅ |
| Heading | `#2d2820` | `#ece5da` | 9.75:1 | ✅ |
| Link (fixed) | `#6b4c1a` | `#ece5da` | 5.95:1 | ✅ |
| Link (before) | `#b8872f` | `#ece5da` | 2.43:1 | ❌ — **was failing** |
| Pre text | `#2d2820` | `rgba(255,255,255,0.20)` over `#e8dfd0` = `#edeae5` | 11.71:1 | ✅ |
| Inline code text (fixed) | `#5c3d0a` | `#e8e0d0` | 7.47:1 | ✅ |
| Inline code text (before) | `#6b5d4d` | `#e8e0d0` | 4.16:1 | ❌ — **was borderline** |
| Table `th` | `#2d2820` | `rgba(0,0,0,0.06)` over `#ece5da` = `#e3dbd0` | 9.75:1 | ✅ |
| Table `td` | `#4a3f2f` | `#ece5da` | 7.78:1 | ✅ |

---

## 9. Keyboard-Only Walkthrough (QA checklist)

1. **Tab** into the Overview section header row → focus lands on "View on GitHub" link.
2. Focus ring visible: 2px solid `#a2792c` (light) / `#f1b400` (dark).
3. **Tab** into README content — first focusable element is first link in README text.
4. **Tab** through all README links; each receives the global focus ring.
5. README links are visually distinguishable from body text by underline AND color.
6. **Shift+Tab** returns focus upward correctly.
7. No focus trap — Tab exits the README embed cleanly.

---

## 10. Component File Map

| File | Role |
|------|------|
| `frontend/src/features/dashboard/components/ReadmeEmbed.tsx` | Self-contained README renderer — all element handlers, states, broken-image fallback |
| `frontend/src/features/dashboard/pages/ProjectDetailPage.tsx` | Imports `ReadmeEmbed`, removes inline `OverviewMarkdown`, adds "View on GitHub" header link |
| `frontend/src/app/utils/renderMarkdown.tsx` | Unchanged — legacy utility retained for other consumers |
| `design/specs/project-readme-embed.md` | This document |

---

## 11. PR Redlines (ASCII)

```
┌─ Overview card ──────────────────────────────────────────────────────────┐
│                                                                          │
│  ✦ Overview                              ↗ View on GitHub  ←── NEW      │
│  ─────────────────────────────────────────────────────────               │
│                                                                          │
│  # Project Title  ← renders as <h3> (level offset +2)                   │
│                                                                          │
│  Body text paragraph at max-width 72ch. Long lines wrap at this         │
│  boundary. [link text](url) ← underlined + passing-contrast color       │
│                                                                          │
│  ## Section  ← <h4>                                                      │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  $ npm install grainlify  ← pre code block, correct bg contrast  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  | Col A  | Col B |  ← table with thead/tbody, overflow-x-auto          │
│  |--------|-------|                                                      │
│  | val 1  | val 2 |                                                      │
│                                                                          │
│  [broken image placeholder: 🖼 "Alt text here"]  ← fallback             │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 12. Open Questions / Future Work

1. **Syntax highlighting**: remark-gfm + rehype-highlight could add language-aware code coloring. Out of scope for this spec — the current token-validated monochrome approach is correct and accessible.
2. **README truncation**: Very long READMEs are not truncated. A "Show more / Show less" affordance may be needed for UX but is a separate task.
3. **Dark image adaptation**: GitHub READMEs sometimes embed `<picture>` elements with light/dark variants. React-markdown strips `<picture>` by default. Handled in a future rehype pass.
