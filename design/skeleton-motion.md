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

## Motion Specifications
- **Duration**: 1.5s (range: 1.2s - 1.6s).
- **Timing Function**: Linear (consistent movement).
- **Contrast**: Low-contrast gradients (subtle highlights).
- **Direction**: Left-to-right (`translateX`).

## Reduced Motion Behavior
- **Requirement**: Respect `prefers-reduced-motion: reduce`.
- **Fallback**: Motion is completely disabled. Skeletons remain as static colored blocks (`bg-white/[0.08]` or `bg-white/[0.12]`).

## Implementation Examples

### List Skeleton (Shimmer Recommended)
For lists, use shimmer on the entire row or individual items to indicate loading progress across multiple elements.

```tsx
// Example: ActivityItemSkeleton.tsx
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

### Detail Skeleton (Static Recommended)
For large blocks or detail views, use static placeholders to avoid excessive movement.

```tsx
// Example: SingleDetailSkeleton.tsx
<div className="space-y-6">
  {/* Large static header */}
  <SkeletonLoader variant="default" width="100%" height="200px" className="animate-none" />
  
  {/* Content blocks */}
  <div className="grid grid-cols-2 gap-4">
    <SkeletonLoader variant="default" height="100px" />
    <SkeletonLoader variant="default" height="100px" />
  </div>
</div>
```

## Performance Constraints
- **GPU Acceleration**: Use `transform: translateX()` instead of `left` or `background-position` to ensure composite-only animations.
- **Repaint Areas**: Shimmer is applied to a single absolute-positioned element within the skeleton container to minimize layout shifts.
- **Containment**: Use `overflow: hidden` on containers to prevent shimmer bleed.
- **Group Animation**: When possible, apply a single shimmer overlay to a container instead of individual animations on each child to reduce draw calls.
