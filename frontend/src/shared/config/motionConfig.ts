/**
 * Motion Design Configuration
 * 
 * Centralized motion design tokens for consistent animations across Grainlify.
 * All durations and easing functions are defined here to ensure consistency.
 * 
 * @see /design/motion-spec.md for comprehensive motion specification
 */

export const motionConfig = {
  /**
   * Standard durations for animations
   * Use these tokens across all components for consistency
   */
  durations: {
    instant: 0,
    fast: 150,
    normal: 300,
    slow: 500,
    slower: 800,
    slowest: 1200,
  } as const,

  /**
   * Easing functions for Motion library (cubic-bezier arrays)
   * For CSS strings, use the corresponding "String" versions
   */
  easing: {
    easeOut: [0, 0, 0.2, 1] as [number, number, number, number],
    easeIn: [0.4, 0, 1, 1] as [number, number, number, number],
    easeInOut: [0.4, 0, 0.2, 1] as [number, number, number, number],
  } as const,

  /**
   * Easing strings for CSS animations
   */
  easingString: {
    easeOut: 'cubic-bezier(0, 0, 0.2, 1)',
    easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
    easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)',
  } as const,

  /**
   * Spring configurations for bouncy, natural-feeling animations
   */
  spring: {
    default: { type: 'spring' as const, stiffness: 100, damping: 10 },
    smooth: { type: 'spring' as const, stiffness: 200, damping: 20 },
    bouncy: { type: 'spring' as const, stiffness: 300, damping: 10 },
  } as const,

  /**
   * Page transition specifications
   */
  pageTransition: {
    fadeSlide: {
      duration: 300,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
      distance: 40,
    },
    scaleFade: {
      duration: 300,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
      scale: 0.95,
    },
    simpleFade: {
      duration: 250,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
    },
  } as const,

  /**
   * Component micro-interaction configurations
   */
  interactions: {
    cardHover: {
      duration: 150,
      scale: 1.02,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
    },
    cardTap: {
      duration: 100,
      scale: 0.98,
    },
    buttonHover: {
      duration: 150,
      scale: 1.02,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
    },
    buttonTap: {
      duration: 100,
      scale: 0.95,
    },
    iconHover: {
      duration: 150,
      scale: 1.1,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
    },
    iconTap: {
      duration: 100,
      scale: 0.9,
    },
    inputFocus: {
      duration: 200,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
    },
  } as const,

  /**
   * List animation configurations
   */
  list: {
    staggerDelay: 50, // ms between each item
    itemDuration: 300,
    initialDelay: 100, // delay before first item
    maxStaggerItems: 12, // beyond this, show all at once
    skipStaggerThreshold: 12, // threshold for skipping stagger
  } as const,

  /**
   * Dropdown/popover animation configurations
   */
  dropdown: {
    open: {
      duration: 150,
      easing: [0, 0, 0.2, 1] as [number, number, number, number],
      distance: 10,
    },
    close: {
      duration: 100,
      easing: [0.4, 0, 1, 1] as [number, number, number, number],
      distance: 10,
    },
  } as const,

  /**
   * Loading skeleton shimmer configuration
   */
  skeleton: {
    duration: 1200,
    timing: 'linear' as const,
    direction: 'ltr' as const,
  } as const,

  /**
   * Responsive breakpoint adjustments
   */
  responsive: {
    sm: {
      // Mobile: 320px - 639px
      durationMultiplier: 0.75, // 25% faster
      staggerDelay: 30,
    },
    md: {
      // Tablet: 640px - 1023px
      durationMultiplier: 0.9, // 10% faster
      staggerDelay: 40,
    },
    lg: {
      // Desktop: 1024px+
      durationMultiplier: 1, // default
      staggerDelay: 50,
    },
  } as const,
} as const;

/**
 * Type exports for TypeScript usage
 */
export type MotionConfig = typeof motionConfig;
export type Duration = keyof typeof motionConfig.durations;
export type Easing = keyof typeof motionConfig.easing;
