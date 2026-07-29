import { useTheme, isDarkVariant } from '../contexts/ThemeContext';

/**
 * SkeletonLoader
 *
 * Renders a placeholder loading block with an optional shimmer sweep.
 *
 * ## Theme behaviour
 *
 * | Theme           | Background         | Shimmer                         |
 * |-----------------|--------------------|---------------------------------|
 * | light           | white/12% opacity  | white/25% sweep (1.5 s linear)  |
 * | dark            | white/8% opacity   | white/15% sweep (1.5 s linear)  |
 * | high-contrast   | #333333 solid      | **none** — static block         |
 * | reduced-motion  | white/8% opacity   | **none** — static block         |
 *
 * High-contrast and reduced-motion variants disable the shimmer entirely:
 * - high-contrast: the `.high-contrast` root class suppresses animation via CSS;
 *   we also omit the shimmer child element so no invisible layer is rendered.
 * - reduced-motion: the `.reduced-motion` root class suppresses animation via CSS;
 *   we omit the shimmer child element for the same reason.
 *
 * The component respects both the explicit theme selection AND the OS-level
 * `prefers-reduced-motion` media query (handled in theme.css).
 *
 * @see design/skeleton-motion.md
 * @see design/specs/accessibility-theme-variants.md §Reduced-Motion §High-Contrast
 */

interface SkeletonLoaderProps {
  className?: string;
  variant?: 'default' | 'circle' | 'text';
  width?: string;
  height?: string;
}

export function SkeletonLoader({
  className,
  variant = 'default',
  width,
  height,
}: SkeletonLoaderProps) {
  const { theme } = useTheme();

  const isHighContrast = theme === 'high-contrast';
  const isReducedMotion = theme === 'reduced-motion';
  /** Shimmer is suppressed for both accessibility variants. */
  const showShimmer = !isHighContrast && !isReducedMotion;

  // Shape classes
  const shapeClass =
    variant === 'circle'
      ? 'rounded-full'
      : variant === 'text'
      ? 'rounded-[100px]'
      : 'rounded-[12px]';

  // Background
  let bgClass: string;
  if (isHighContrast) {
    // Solid opaque — class `skeleton-surface` is also targeted by high-contrast CSS
    bgClass = 'bg-[#333333] skeleton-surface';
  } else if (isDarkVariant(theme)) {
    bgClass = 'bg-white/[0.08]';
  } else {
    bgClass = 'bg-white/[0.12]';
  }

  // Shimmer gradient — only used when showShimmer is true
  const shimmerGradient = isDarkVariant(theme)
    ? 'from-transparent via-white/[0.15] to-transparent'
    : 'from-transparent via-white/[0.25] to-transparent';

  const style: React.CSSProperties = {};
  if (width) style.width = width;
  if (height) style.height = height;

  return (
    <div
      className={`relative overflow-hidden ${shapeClass} ${bgClass} ${className ?? ''}`}
      style={style}
      // Announced as presentation; parent should provide context via aria-label or
      // aria-busy on the containing region.
      role="presentation"
      aria-hidden="true"
    >
      {showShimmer && (
        <div
          className={`absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r ${shimmerGradient}`}
        />
      )}
    </div>
  );
}
