/**
 * useReducedMotion Hook
 * 
 * Detects user's prefers-reduced-motion preference and provides it to components.
 * This hook respects user accessibility preferences for motion-sensitive users.
 * 
 * @see https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion
 * @see /design/motion-spec.md for accessibility guidelines
 */

import { useEffect, useState } from 'react';

/**
 * Hook to detect if user prefers reduced motion
 * 
 * Returns true if the user has set `prefers-reduced-motion: reduce` in their OS settings.
 * This preference is typically found in accessibility settings across major operating systems:
 * - macOS: System Preferences > Accessibility > Display > Reduce motion
 * - Windows: Settings > Ease of Access > Display > Show animations
 * - iOS: Settings > Accessibility > Motion > Reduce Motion
 * - Android: Settings > Accessibility > Remove animations
 * 
 * @returns {boolean} Whether user prefers reduced motion
 * 
 * @example
 * const prefersReduced = useReducedMotion();
 * 
 * <motion.div
 *   animate={{ opacity: prefersReduced ? 1 : targetOpacity }}
 *   transition={{ duration: prefersReduced ? 0 : 300 }}
 * />
 */
export const useReducedMotion = (): boolean => {
  const [prefersReduced, setPrefersReduced] = useState<boolean>(false);

  useEffect(() => {
    // Create media query for prefers-reduced-motion
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

    // Set initial value
    setPrefersReduced(mediaQuery.matches);

    // Create listener function
    const handleChange = (e: MediaQueryListEvent) => {
      setPrefersReduced(e.matches);
    };

    // Add listener (using addEventListener for better browser support)
    mediaQuery.addEventListener('change', handleChange);

    // Cleanup listener
    return () => {
      mediaQuery.removeEventListener('change', handleChange);
    };
  }, []);

  return prefersReduced;
};

/**
 * Hook to detect if user prefers dark mode
 * Can be useful for adjusting animation intensity in dark mode
 * 
 * @returns {boolean} Whether user prefers dark mode
 */
export const usePrefersDarkMode = (): boolean => {
  const [prefersDark, setPrefersDark] = useState<boolean>(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    setPrefersDark(mediaQuery.matches);

    const handleChange = (e: MediaQueryListEvent) => {
      setPrefersDark(e.matches);
    };

    mediaQuery.addEventListener('change', handleChange);

    return () => {
      mediaQuery.removeEventListener('change', handleChange);
    };
  }, []);

  return prefersDark;
};

/**
 * Hook to detect viewport size (mobile/tablet/desktop)
 * Useful for adjusting animation parameters based on screen size
 * 
 * @returns {Object} Object with boolean properties for each breakpoint
 */
export const useResponsiveBreakpoint = () => {
  const [breakpoint, setBreakpoint] = useState({
    isMobile: false, // sm: < 640px
    isTablet: false, // md: 640px - 1023px
    isDesktop: false, // lg: >= 1024px
  });

  useEffect(() => {
    const updateBreakpoint = () => {
      const width = window.innerWidth;
      setBreakpoint({
        isMobile: width < 768,
        isTablet: width >= 768 && width < 1024,
        isDesktop: width >= 1024,
      });
    };

    // Set initial value
    updateBreakpoint();

    // Listen for resize
    window.addEventListener('resize', updateBreakpoint);

    return () => {
      window.removeEventListener('resize', updateBreakpoint);
    };
  }, []);

  return breakpoint;
};
