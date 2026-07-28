# Skeleton Shimmer Rules & Performance

## Shimmer Usage Guidelines

### When to use Shimmer
- **Lists and Tables**: Use shimmer for repeating rows of content (e.g., Pull Request lists, Activity logs).
- **Repeated Content Blocks**: Use for grids or card layouts where multiple identical structures are loading.
- **Initial Load**: Shimmer helps indicate progress during the first meaningful paint.

### When NOT to use Shimmer (Static Placeholders)
- **Single Detail Views**: Large static blocks or detail pages should use static placeholders to avoid "motion overload".
- **Small UI Elements**: Buttons, icons, or small tags should remain static to prevent "busy" UI.
- **Low-End Devices**: System-level constraints should minimize animation.
- **High-Contrast Theme**: The shimmer child element is not rendered at all — a solid `#333333` block is shown instead.
- **Reduced-Motion Theme**: The shimmer child element is not rendered — a dark `bg-white/[0.08]` static block is shown instead.

---

## Motion Specifications (light / dark themes)

| Property          | Value                     |
|-------------------|---------------------------|
| Duration          | 1.5 s (range: 1.2 s–1.6 s) |
| Timing Function   | linear                    |
| Contrast          | Low-contrast gradient (subtle highlight) |
| Direction         | Left-to-right (`translateX`) |
| GPU acceleration  | `transform: translateX()` (composite-only) |

---

## Reduced-Motion Override

**Applies when:**
1. The user has selected the `reduced-motion` theme variant, **or**
2. The OS/browser reports `prefers-reduced-motion: reduce`

**Behaviour:** All shimmer motion is completely suppressed. The `animate-shimmer` child
element is **not rendered** by `SkeletonLoader` for these variants (not just `animation: none` —
the element is omitted from the DOM to eliminate any invisible layer overhead).

**Static fallback appearance:**

| Theme          | Skeleton background          |
|----------------|------------------------------|
| reduced-motion | `rgba(255,255,255,0.08)` — same as dark |
| high-contrast  | `#333333` solid opaque       |

**WCAG reference:** WCAG 2.3.3 Animation from Interactions — users must be able to disable
motion that is not essential to the functionality of content.

```tsx
// SkeletonLoader correctly suppresses shimmer for a11y variants:
const showShimmer = theme !== 'high-contrast' && theme !== 'reduced-motion';

return (
  <div className={`relative overflow-hidden ${shapeClass} ${bgClass}`} ...>
    {showShimmer && (
      <div className={`absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r ${shimmerGradient}`} />
    )}
  </div>
);
```

---

## High-Contrast Override

**Applies when:** The user has selected the `high-contrast` theme variant.

**Behaviour:**
- Background: `#333333` solid — 3.5:1 contrast on `#000000` page canvas (meets WCAG 1.4.11)
- No shimmer animation (element not rendered)
- The CSS class `skeleton-surface` is applied for targeted overrides via `.high-contrast .skeleton-surface`
- No `backdrop-filter` or gradient effects

---

## Implementation Examples

### List Skeleton (Shimmer on light/dark, static on a11y variants)

```tsx
// SkeletonLoader handles all variants automatically via useTheme()
<div className="relative overflow-hidden">
  <div className="flex items-center gap-3">
    <SkeletonLoader variant="circle" width="32px" height="32px" />
    <div className="flex-1 space-y-2">
      <SkeletonLoader variant="text" width="80%" height="16px" />
      <SkeletonLoader variant="text" width="40%" height="12px" />
    </div>
  </div>
</div>
```

### Detail Skeleton (Static recommended for all themes)

```tsx
<div className="space-y-6">
  {/* Large static header — className="animate-none" opts out of shimmer */}
  <SkeletonLoader variant="default" width="100%" height="200px" className="animate-none" />

  <div className="grid grid-cols-2 gap-4">
    <SkeletonLoader variant="default" height="100px" />
    <SkeletonLoader variant="default" height="100px" />
  </div>
</div>
```

---

## Performance Constraints

- **GPU Acceleration**: Use `transform: translateX()` — composite-only, no layout recalculation.
- **Repaint Areas**: Shimmer is a single absolute child; it clips inside `overflow: hidden` container.
- **Containment**: Always use `overflow: hidden` on the skeleton wrapper.
- **Group Animation**: Apply one shimmer overlay per container where possible to reduce draw calls.
- **Reduced-motion cost**: Zero — the shimmer element is not added to the DOM, so no animation overhead.

---

## Theme Token Reference

See `design-tokens.json`:
- `reducedMotion.motion.skeletonShimmer` → `"static"`
- `highContrast.skeleton.background` → `"#333333"`
