# Video / GIF Embed — Design Specification

**Version:** 1.0  
**Status:** Ready for implementation  
**Component:** `MediaEmbed`  
**Target:** `frontend/src/features/dashboard/pages/ProjectDetailPage.tsx`  
**Author:** Design system  
**Date:** 2026-07-26

---

## 1. Overview

ProjectDetailPage renders project READMEs via `ReactMarkdown`. README content can contain links or references to demo videos (MP4/WebM) and animated GIFs. Without explicit embed treatment, videos render as bare anchors and GIFs render as inline images with no playback controls, creating an inconsistent, inaccessible experience.

`MediaEmbed` introduces a first-class embed surface with:

- Fixed aspect-ratio container that prevents layout shift
- Lazy-loading via IntersectionObserver (poster frame shown until in-viewport)
- Explicit autoplay policy: GIFs may autoplay muted-looping; videos never autoplay with sound
- Full state machine: `poster-placeholder → loaded-paused → playing` (videos) or `gif-autoplay-with-pause-control → gif-paused` (GIFs)
- Error/unavailable state
- WCAG 2.1 AA accessible controls, keyboard operable, screen-reader announced

---

## 2. Scope and Usage Context

`MediaEmbed` is used in two locations on ProjectDetailPage:

1. **README renderer** — replaces `<video>` elements and `.gif` `<img>` tags inside `OverviewMarkdown`. The custom `img` renderer detects `.gif` URLs; a new custom `video` component handles `<video>` tags.
2. **Media gallery** (future) — a dedicated "Demo / Media" panel above the Issues section can render an ordered list of `MediaEmbed` items sourced from project metadata.

---

## 3. Design Tokens in Use

All values map directly to `/design-tokens.json`.

| Purpose | Token path | Resolved value |
|---|---|---|
| Container background (dark) | `darkMode.background.glassMedium` | `rgba(255,255,255,0.08)` |
| Container background (light) | `elevation.glassmorphism.light.backgroundFill` | `rgba(255,255,255,0.15)` |
| Container border (dark) | `darkMode.border.subtle` | `rgba(255,255,255,0.08)` |
| Container border (light) | `elevation.glassmorphism.light.borderColor` | `rgba(255,255,255,0.25)` |
| Border radius | `borderRadius.3xl` | `1.5rem` / `rounded-[24px]` |
| Poster placeholder bg (dark) | `darkMode.background.surfaceSecondary` | `#2d2820` |
| Poster placeholder bg (light) | `color.neutral.200` | `#e7e5e4` |
| Control icon (active) | `darkMode.accent.primary` | `#c9983a` |
| Control icon (rest) | `darkMode.text.primary` | `#f5f5f5` |
| Focus ring | `darkMode.interactive.focusRing` | `#f1b400` |
| Error icon | `color.semantic.error.500` | `#ef4444` |
| Motion duration | `motion.durations.normal` | `300ms` |
| Motion easing | `motion.easing.easeOut` | `cubic-bezier(0,0,0.2,1)` |

---

## 4. Aspect-Ratio Container

### 4.1 Default ratio

All embeds use **16:9** as the default container aspect ratio. This matches the most common format for screencasts, demos, and GitHub-hosted assets.

```
┌────────────────────────────────────────────┐
│  16 : 9 container (100% width, auto height)│
│  rounded-[24px]  overflow-hidden            │
│                                            │
│  ┌──────────────────────────────────────┐  │
│  │         media or poster              │  │
│  └──────────────────────────────────────┘  │
└────────────────────────────────────────────┘
```

Implemented using `@radix-ui/react-aspect-ratio` with `ratio={16/9}`.

### 4.2 Letterboxing for mismatched source ratios

When the source media's intrinsic ratio differs from the 16:9 container:

- The media element uses `object-fit: contain` inside the fixed container
- A blurred, dimmed version of the first frame fills the letterbox bars via a `position: absolute` background layer (`object-fit: cover`, `filter: blur(20px)`, `opacity: 0.25`)
- This avoids black bars while preserving the source's natural framing

```
┌──────────────────────────────────────────────────────┐
│  16:9 container                                      │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ ← blurred bg
│  ░░░░░░░┌──────────────────────────────┐░░░░░░░░░░░░ │
│  ░░░░░░░│  4:3 or 9:16 media content  │░░░░░░░░░░░░ │
│  ░░░░░░░│                              │░░░░░░░░░░░░ │
│  ░░░░░░░└──────────────────────────────┘░░░░░░░░░░░░ │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
└──────────────────────────────────────────────────────┘
```

### 4.3 Responsive behaviour

At all breakpoints the container fills its parent's width (100%). The aspect-ratio constraint ensures no layout shift. At 375px (mobile), the container collapses to full width with correct height preserved by the aspect-ratio primitive, so no CLS occurs.

---

## 5. Lazy-Load Strategy

### 5.1 IntersectionObserver trigger

- A sentinel `<div>` (1px × 1px, invisible) is placed at the bottom of the poster-placeholder state
- An `IntersectionObserver` with `rootMargin: '200px'` fires when the sentinel enters the viewport extended by 200px
- On intersection, the `src`/`srcSet` is set on the underlying media element and the component transitions to the loading phase
- The observer is disconnected after the first intersection (`threshold: 0`)

```
┌─────────────────────────────────────────┐
│  Viewport                               │
│                                         │
│  ┌───────────────────────────────────┐  │
│  │  poster-placeholder               │  │ ← not yet loading
│  └───────────────────────────────────┘  │
│              ...scroll...               │
│  - - - - - - - - - - - (200px margin)   │ ← observer fires here
│  ┌───────────────────────────────────┐  │
│  │  [sentinel]                       │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### 5.2 Poster frame

For videos, a `poster` attribute can be supplied. When absent, the first frame is used after load. The poster image is displayed as a full-bleed cover image inside the container until the video reports `canplay`.

For GIFs, the poster-placeholder shows a static JPEG/WebP preview when available. If not, a gradient placeholder fills the container.

### 5.3 Placeholder appearance

The poster-placeholder uses the `SkeletonLoader` shimmer pattern on the background while the asset is loading (state `loading`). Once the asset fires `canplay` (video) or `load` (GIF treated as `<img>`), the shimmer fades out with a `300ms easeOut` transition.

---

## 6. Autoplay Policy

### 6.1 Videos — never autoplay with sound

| Condition | Behaviour |
|---|---|
| Component enters viewport | No autoplay. Shows poster-placeholder then loaded-paused state |
| User taps play | Plays from beginning, unmuted (user-initiated gesture) |
| User leaves viewport while playing | Pauses automatically (IntersectionObserver disconnect) |
| Video ends | Returns to loaded-paused state with replay affordance |
| `muted` attribute supplied | Muted indicator shown; toggling unmute is user-initiated |

Videos never call `.play()` without a direct user gesture. This satisfies Chrome/Safari's autoplay policy and WCAG 1.4.2 (audio control).

### 6.2 GIFs — autoplay muted-looping with pause control

GIFs rendered as `<img>` natively autoplay. `MediaEmbed` wraps GIFs in an `<img>` for normal loop behaviour, but overlays a visible pause/play toggle button so users can stop the animation at will. This satisfies WCAG 2.2.2 (pause, stop, hide).

| State | Behaviour |
|---|---|
| Component enters viewport | GIF begins animating immediately (browser-native) |
| User presses pause control | GIF is frozen by replacing `<img>` src with a data-URI snapshot of the current frame (canvas capture), or by swapping to a static poster image |
| User presses play control | GIF resumes animation by restoring the original src |
| `prefers-reduced-motion: reduce` | GIF is displayed statically (pause applied on mount) and play control is shown |

### 6.3 `prefers-reduced-motion` handling

- Detected via `window.matchMedia('(prefers-reduced-motion: reduce)')` on mount
- When active: GIFs start paused, videos do not autoplay (same as default), all enter/exit transitions for the control overlay are instant (0ms)

---

## 7. State Machine

### 7.1 Video states

```
                         ┌─────────────────────┐
            mount        │   poster-placeholder │
         ──────────────► │   (not in viewport)  │
                         └─────────┬───────────┘
                                   │  viewport intersection
                                   ▼
                         ┌─────────────────────┐
                         │      loading         │
                         │   (shimmer + poster) │
                         └──────┬──────┬───────┘
                           canplay     │ error
                                │      ▼
                                │   ┌─────────────────────┐
                                │   │  error-unavailable   │
                                │   └─────────────────────┘
                                ▼
                         ┌─────────────────────┐
                   ┌────►│    loaded-paused     │◄────┐
                   │     │   (play button)      │     │
                   │     └─────────┬───────────┘     │
                   │               │ user tap play    │
                   │               ▼                  │
                   │     ┌─────────────────────┐      │
                   │     │       playing        │      │
                   │     │  (pause button)      │      │
                   │     └──────┬───────┬──────┘      │
                   │      pause │       │ ended        │
                   └────────────┘       └─────────────┘
```

### 7.2 GIF states

```
                         ┌─────────────────────┐
            mount        │   poster-placeholder │
         ──────────────► │   (not in viewport)  │
                         └─────────┬───────────┘
                                   │  viewport intersection
                                   ▼
                         ┌─────────────────────┐
                         │    gif-loading       │
                         └──────┬──────┬───────┘
                              load     │ error
                                │      ▼
                                │   ┌─────────────────────┐
                                │   │  error-unavailable   │
                                │   └─────────────────────┘
                                ▼
              ┌────────────────────────────────────────────┐
              │  gif-autoplay-with-pause-control            │
              │  (GIF animating, pause button visible)      │
              └─────────────────┬──────────────────────────┘
                                │  user tap pause  │  reduced-motion mount
                                ▼                  │
              ┌────────────────────────────────────────────┐
              │  gif-paused                                  │
              │  (static frame, play button visible)         │
              └─────────────────┬──────────────────────────┘
                                │  user tap play
                                └──► gif-autoplay-with-pause-control
```

### 7.3 State visual inventory

| State | Visual | ARIA |
|---|---|---|
| `poster-placeholder` | Gradient/SkeletonLoader fill, no controls | `aria-label="Loading media"` on container |
| `loading` | Shimmer over poster, spinner badge | `aria-busy="true"` on container |
| `loaded-paused` | Poster frame, centred play button | `aria-label="Play video"` on button |
| `playing` | Video playing, bottom-bar pause button | `aria-label="Pause video"` on button |
| `gif-autoplay-with-pause-control` | GIF animating, pause badge top-right | `aria-label="Pause animation"` on button |
| `gif-paused` | Static frame, play badge top-right | `aria-label="Play animation"` on button |
| `error-unavailable` | Error icon + message, retry button | `role="alert"`, `aria-label="Media unavailable"` |

---

## 8. Accessibility Annotations

### 8.1 Video controls

- The centred play button is the **only** way to start video playback. It must be keyboard-reachable and mouse-clickable. Do not rely solely on the native `controls` attribute, which is hidden in favour of custom controls.
- The bottom control bar (pause/play, time, mute) appears on hover and on focus-within. It must also appear on keyboard focus of the container.
- All icon buttons have an `aria-label` (see §7.3). Icons are `aria-hidden="true"`.
- The video element has `aria-hidden="true"` and its parent container carries `role="region"` + `aria-label` describing the video title when available.

### 8.2 GIF pause control

- The pause/play control for GIFs is a `<button>` element positioned top-right.
- `aria-label` switches between `"Pause animation"` and `"Play animation"` on state change.
- `aria-pressed` reflects the paused state: `aria-pressed="true"` when paused.
- The button is always visible (not only on hover) so that keyboard-only users can find it without hovering.

### 8.3 Captions / transcript placeholder

Video elements should carry a `<track kind="captions">` element. When no captions file is available, the component renders a visible "Captions unavailable" note in the control bar. This is a placeholder for future content management to supply `.vtt` files.

### 8.4 Focus ring

The play/pause button focus ring uses `outline: 2px solid #f1b400` (token `darkMode.interactive.focusRing`) with `outline-offset: 2px`. On light surfaces, the ring uses `#c9983a` (token `color.primary.600`) to maintain 3:1 contrast against the light container background.

### 8.5 Screen reader announcements

When the video transitions to `playing`, a live region with `aria-live="polite"` announces `"Video playing"`. When it transitions to `loaded-paused`, it announces `"Video paused"`. These announcements are throttled to avoid repetition on rapid play/pause.

---

## 9. Visual Anatomy — Redlines

### 9.1 Video embed (loaded-paused state)

```
┌────────────────────────────────────────────────────────────┐
│  rounded-[24px]  overflow-hidden                           │
│  bg-white/[0.08]  border border-white/[0.08]               │
│  aspect-ratio 16/9  width 100%                             │
│                                                            │
│                                                            │
│           ┌─────────────────────────────┐                  │
│           │                             │                  │
│           │        ▶  Play              │  ← 48×48px btn   │
│           │   [poster frame fill]       │    bg #0008      │
│           │                             │    icon #f5f5f5  │
│           └─────────────────────────────┘                  │
│                                                            │
│  ─────────────────────── control bar ─────────────────────  │
│  │ ▐▐  00:00 ─────────────────────── 01:23  🔊  [⬛] CC │  │
│  │ ↑                                                    │  │
│  │ 40px height  bg #0006  text #f5f5f5                  │  │
└────────────────────────────────────────────────────────────┘
```

Dimensions:
- Container: `width: 100%`, height auto-computed from 16:9 ratio
- Play button: 48×48px, `rounded-full`, `bg-black/50`, icon size 24px
- Control bar: `height: 40px`, `px-3`, `gap-2`, `bg-black/40`, shows on `hover` + `focus-within` + keyboard focus

### 9.2 GIF embed (gif-autoplay-with-pause-control state)

```
┌────────────────────────────────────────────────────────────┐
│  rounded-[24px]  overflow-hidden                           │
│  aspect-ratio 16/9                                         │
│                                                            │
│  ┌──────────────────────────────────────┐  ┌────────────┐ │
│  │                                      │  │ ⏸ Pause   │ │
│  │           [GIF animation]            │  │ 32×32px    │ │
│  │                                      │  │ top-right  │ │
│  └──────────────────────────────────────┘  └────────────┘ │
│                                            ↑              │
│                             margin: 8px from corner       │
└────────────────────────────────────────────────────────────┘
```

Pause control: `32×32px`, `rounded-[8px]`, `bg-black/60`, always visible, `position: absolute`, `top-2 right-2`.

### 9.3 Error state

```
┌────────────────────────────────────────────────────────────┐
│  aspect-ratio 16/9                                         │
│  bg surfaceSecondary  rounded-[24px]                       │
│                                                            │
│              ⚠  [error icon 24px, #ef4444]                 │
│              Media unavailable                             │
│              [14px, text.secondary]                        │
│                                                            │
│              ┌────────────────────┐                        │
│              │     Retry          │  ← ghost button        │
│              └────────────────────┘                        │
└────────────────────────────────────────────────────────────┘
```

---

## 10. Props Interface

```ts
export type MediaEmbedKind = 'video' | 'gif';

export type MediaEmbedState =
  | 'poster-placeholder'
  | 'loading'
  | 'loaded-paused'
  | 'playing'
  | 'gif-autoplay-with-pause-control'
  | 'gif-paused'
  | 'error-unavailable';

export interface MediaEmbedProps {
  /** The URL of the video (MP4/WebM) or animated GIF */
  src: string;
  /** Detected or supplied media kind */
  kind: MediaEmbedKind;
  /** Optional poster image URL for videos */
  poster?: string;
  /** Optional static preview image URL for GIFs */
  gifPoster?: string;
  /** Human-readable title for aria-label and screen reader announcement */
  title?: string;
  /** URL to a WebVTT captions file */
  captionsSrc?: string;
  /**
   * Aspect ratio as a number: width / height.
   * Defaults to 16/9.
   */
  aspectRatio?: number;
  /** Optional className for the outer container */
  className?: string;
  /** Called when the component enters the error state */
  onError?: (error: Error) => void;
}
```

---

## 11. Integration with OverviewMarkdown

The `img` custom renderer in `OverviewMarkdown` is extended to detect GIF URLs:

```tsx
img: ({ src, alt, ...props }) => {
  if (src && /\.gif(\?.*)?$/i.test(src)) {
    return (
      <MediaEmbed
        src={src}
        kind="gif"
        title={alt || undefined}
        className="my-4"
      />
    );
  }
  return <img className="rounded-[12px] max-w-full h-auto my-4" alt={alt || ''} src={src} {...props} />;
},
```

A new `video` custom renderer handles `<video>` tags in README markdown:

```tsx
video: ({ src, poster, ...props }) => (
  <MediaEmbed
    src={src || ''}
    kind="video"
    poster={poster}
    className="my-4"
  />
),
```

---

## 12. File Locations

| File | Purpose |
|---|---|
| `frontend/src/shared/components/MediaEmbed.tsx` | Component implementation |
| `frontend/src/shared/components/MediaEmbed.test.tsx` | Test suite |
| `frontend/src/features/dashboard/pages/ProjectDetailPage.tsx` | Integration site |
| `design/specs/video-gif-embed-spec.md` | This document |

---

## 13. Open Items / Future Work

- Supply `.vtt` captions via project metadata API (tracked separately)
- Media gallery panel in ProjectDetailPage for curated demo assets
- Analytics event: `media_play`, `media_pause`, `media_error`
- Progressive enhancement: if `IntersectionObserver` is unavailable (very old browsers), fall through to eager loading
