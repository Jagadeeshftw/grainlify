/**
 * @file Theme-aware Joyride configuration for the onboarding tour.
 *
 * Produces the `options`, `styles`, and `locale` passed to <Joyride>. The
 * glassmorphism surface itself lives in {@link ./TourTooltip} (a custom
 * `tooltipComponent`, the only reliable way to get `backdrop-filter` blur);
 * this module configures the overlay, spotlight, beacon, and behaviour flags.
 *
 * @remarks Reduced-motion variant (WCAG 2.3.3 / 2.2.2): when `reducedMotion`
 * is true the dark spotlight cutout is swapped for a static gold **border
 * highlight** (`spotlight.stroke`), the pulsing beacon is skipped, the overlay
 * is lightened, and scroll animation is disabled. The remaining CSS
 * enter/pulse transitions are killed in `onboarding-tour.css`.
 */

import type { Locale, Options, PartialDeep, Styles } from 'react-joyride';

/** Grainlify brand gold — primary accent for the tour. */
const BRAND_GOLD = '#c9983a';

export interface TourTheme {
  options: Partial<Options>;
  styles: PartialDeep<Styles>;
  locale: Locale;
}

/**
 * Build the Joyride theme for the current color scheme and motion preference.
 *
 * @param theme - `'light'` or `'dark'`, from the app ThemeContext.
 * @param reducedMotion - Result of `useReducedMotion()`.
 */
export function getTourTheme(theme: 'light' | 'dark', reducedMotion: boolean): TourTheme {
  const isDark = theme === 'dark';

  // Lighten the backdrop in reduced-motion mode so the border highlight reads
  // as a highlight rather than a heavy dimming mask.
  const overlayColor = reducedMotion
    ? isDark
      ? 'rgba(10,8,6,0.35)'
      : 'rgba(20,16,12,0.22)'
    : isDark
      ? 'rgba(10,8,6,0.66)'
      : 'rgba(20,16,12,0.5)';

  const options: Partial<Options> = {
    primaryColor: BRAND_GOLD,
    overlayColor,
    zIndex: 10000,
    width: 384,
    spotlightPadding: 8,
    spotlightRadius: 16,
    // The custom tooltip renders its own step counter + dots.
    showProgress: false,
    buttons: ['back', 'close', 'skip', 'primary'],
    // The ✕ dismisses (ends) the tour rather than silently advancing.
    closeButtonAction: 'skip',
    // Clicking the backdrop must never dismiss by accident.
    overlayClickAction: false,
    // ESC closes the active step; the provider treats that as a dismissal.
    dismissKeyAction: 'close',
    // Keep focus trapped inside the tooltip while a step is open (a11y).
    disableFocusTrap: false,
    // Open each step directly on the tooltip — no pulsing beacon to chase.
    skipBeacon: true,
    scrollDuration: reducedMotion ? 0 : 300,
    // Wait briefly for an anchor (e.g. just after a page switch) before failing.
    targetWaitTimeout: 1500,
    arrowColor: isDark ? 'rgba(45,40,32,0.92)' : 'rgba(255,255,255,0.92)',
  };

  const styles: PartialDeep<Styles> = {
    overlay: { mixBlendMode: 'normal' },
    // Reduced motion → outline the target instead of cutting a dark hole.
    spotlight: reducedMotion ? { stroke: BRAND_GOLD, strokeWidth: 3 } : {},
  };

  const locale: Locale = {
    back: 'Back',
    close: 'Dismiss',
    last: 'Finish',
    next: 'Next',
    nextWithProgress: 'Next',
    skip: 'Skip tour',
    open: 'Open onboarding step',
  };

  return { options, styles, locale };
}
