/**
 * Reusable Motion Variants
 * 
 * Pre-configured animation variants for common patterns used throughout Grainlify.
 * These variants respect the motion spec and include reduced-motion alternatives.
 * 
 * @see /design/motion-spec.md for the complete motion specification
 */

import { motionConfig } from '../config/motionConfig';

type Direction = 'up' | 'down' | 'left' | 'right' | 'none';

/**
 * Page transition variants with direction support
 * 
 * @param direction - Direction of slide transition
 * @param prefersReduced - Whether to respect prefers-reduced-motion
 */
export const pageTransitionVariants = (
  direction: Direction = 'up',
  prefersReduced: boolean = false
) => {
  if (prefersReduced) {
    return {
      initial: { opacity: 0 },
      animate: { opacity: 1, transition: { duration: 0 } },
      exit: { opacity: 0, transition: { duration: 0 } },
    };
  }

  const getSlideOffset = (dir: Direction) => {
    const distance = motionConfig.pageTransition.fadeSlide.distance;
    return {
      up: { y: distance },
      down: { y: -distance },
      left: { x: distance },
      right: { x: -distance },
      none: {},
    }[dir];
  };

  const initialOffset = getSlideOffset(direction);
  const exitOffset = {
    up: { y: -40 },
    down: { y: 40 },
    left: { x: -40 },
    right: { x: 40 },
    none: {},
  };

  return {
    initial: {
      opacity: 0,
      ...initialOffset,
    },
    animate: {
      opacity: 1,
      y: 0,
      x: 0,
      transition: {
        duration: motionConfig.pageTransition.fadeSlide.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    exit: {
      opacity: 0,
      ...exitOffset[direction],
      transition: {
        duration: motionConfig.pageTransition.fadeSlide.duration,
        ease: motionConfig.easing.easeIn,
      },
    },
  };
};

/**
 * Modal/Dialog transition variants
 * Scale up from center with fade
 */
export const modalVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      backdrop: {
        initial: { opacity: 0 },
        animate: { opacity: 1, transition: { duration: 0 } },
        exit: { opacity: 0, transition: { duration: 0 } },
      },
      content: {
        initial: { opacity: 0, scale: 1 },
        animate: { opacity: 1, scale: 1, transition: { duration: 0 } },
        exit: { opacity: 0, scale: 1, transition: { duration: 0 } },
      },
    };
  }

  return {
    backdrop: {
      initial: { opacity: 0 },
      animate: {
        opacity: 1,
        transition: { duration: motionConfig.durations.fast },
      },
      exit: {
        opacity: 0,
        transition: { duration: motionConfig.durations.fast * 0.75 },
      },
    },
    content: {
      initial: {
        opacity: 0,
        scale: 0.9,
        y: 20,
      },
      animate: {
        opacity: 1,
        scale: 1,
        y: 0,
        transition: {
          duration: motionConfig.pageTransition.scaleFade.duration,
          ease: motionConfig.easing.easeOut,
        },
      },
      exit: {
        opacity: 0,
        scale: 0.9,
        y: 20,
        transition: {
          duration: motionConfig.durations.fast,
          ease: motionConfig.easing.easeIn,
        },
      },
    },
  };
};

/**
 * Card hover micro-interaction
 * Slightly scale up with shadow elevation
 */
export const cardVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { scale: 1 },
      whileHover: { transition: { duration: 0 } },
      whileTap: { transition: { duration: 0 } },
    };
  }

  return {
    initial: {
      scale: 1,
    },
    whileHover: {
      scale: motionConfig.interactions.cardHover.scale,
      transition: {
        duration: motionConfig.interactions.cardHover.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    whileTap: {
      scale: motionConfig.interactions.cardTap.scale,
      transition: { duration: motionConfig.interactions.cardTap.duration },
    },
  };
};

/**
 * Button hover and tap interactions
 */
export const buttonVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { scale: 1 },
      whileHover: { transition: { duration: 0 } },
      whileTap: { transition: { duration: 0 } },
    };
  }

  return {
    initial: { scale: 1 },
    whileHover: {
      scale: motionConfig.interactions.buttonHover.scale,
      transition: {
        duration: motionConfig.interactions.buttonHover.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    whileTap: {
      scale: motionConfig.interactions.buttonTap.scale,
      transition: { duration: motionConfig.interactions.buttonTap.duration },
    },
  };
};

/**
 * Icon button interactions (larger scale on hover)
 */
export const iconButtonVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { scale: 1 },
      whileHover: { transition: { duration: 0 } },
      whileTap: { transition: { duration: 0 } },
    };
  }

  return {
    initial: { scale: 1 },
    whileHover: {
      scale: motionConfig.interactions.iconHover.scale,
      transition: {
        duration: motionConfig.interactions.iconHover.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    whileTap: {
      scale: motionConfig.interactions.iconTap.scale,
      transition: { duration: motionConfig.interactions.iconTap.duration },
    },
  };
};

/**
 * Input focus indicator animation
 */
export const inputFocusVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { boxShadow: '0 0 0 0px rgba(241, 180, 0, 0)' },
      whileFocus: { boxShadow: '0 0 0 0px rgba(241, 180, 0, 0)' },
      whileInvalid: { boxShadow: '0 0 0 0px rgba(239, 68, 68, 0)' },
    };
  }

  return {
    initial: {
      boxShadow: '0 0 0 0px rgba(241, 180, 0, 0)',
    },
    whileFocus: {
      boxShadow: '0 0 0 3px rgba(241, 180, 0, 0.1)',
      transition: {
        duration: motionConfig.interactions.inputFocus.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    whileInvalid: {
      boxShadow: '0 0 0 3px rgba(239, 68, 68, 0.1)',
      transition: {
        duration: motionConfig.interactions.inputFocus.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
  };
};

/**
 * Dropdown/Select animation
 * Slides up with fade
 */
export const dropdownVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { opacity: 0 },
      animate: { opacity: 1, transition: { duration: 0 } },
      exit: { opacity: 0, transition: { duration: 0 } },
    };
  }

  return {
    initial: {
      opacity: 0,
      y: -motionConfig.dropdown.open.distance,
    },
    animate: {
      opacity: 1,
      y: 0,
      transition: {
        duration: motionConfig.dropdown.open.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    exit: {
      opacity: 0,
      y: -motionConfig.dropdown.close.distance,
      transition: {
        duration: motionConfig.dropdown.close.duration,
        ease: motionConfig.easing.easeIn,
      },
    },
  };
};

/**
 * Dropdown item hover effect
 */
export const dropdownItemVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: {},
      whileHover: { transition: { duration: 0 } },
    };
  }

  return {
    initial: {
      backgroundColor: 'rgba(0, 0, 0, 0)',
      paddingLeft: '1rem',
    },
    whileHover: {
      backgroundColor: 'rgba(241, 180, 0, 0.1)',
      paddingLeft: '1.5rem',
      transition: {
        duration: motionConfig.interactions.cardHover.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
  };
};

/**
 * Badge/Tag entrance animation
 */
export const badgeVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { scale: 1, opacity: 1 },
      animate: { scale: 1, opacity: 1, transition: { duration: 0 } },
      exit: { scale: 1, opacity: 0, transition: { duration: 0 } },
    };
  }

  return {
    initial: {
      scale: 0,
      opacity: 0,
    },
    animate: {
      scale: 1,
      opacity: 1,
      transition: {
        duration: motionConfig.durations.fast,
        ease: motionConfig.easing.easeOut,
      },
    },
    exit: {
      scale: 0,
      opacity: 0,
      transition: {
        duration: motionConfig.durations.fast * 0.75,
        ease: motionConfig.easing.easeIn,
      },
    },
  };
};

/**
 * Toast notification animation
 * Slides in from right
 */
export const toastVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      initial: { x: 0, opacity: 1 },
      animate: { x: 0, opacity: 1, transition: { duration: 0 } },
      exit: { x: 0, opacity: 0, transition: { duration: 0 } },
    };
  }

  return {
    initial: {
      x: 400,
      opacity: 0,
    },
    animate: {
      x: 0,
      opacity: 1,
      transition: {
        duration: motionConfig.pageTransition.fadeSlide.duration,
        ease: motionConfig.easing.easeOut,
      },
    },
    exit: {
      x: 400,
      opacity: 0,
      transition: {
        duration: motionConfig.durations.fast,
        ease: motionConfig.easing.easeIn,
      },
    },
  };
};

/**
 * List container and item stagger animation
 * 
 * @param prefersReduced - Whether to respect prefers-reduced-motion
 * @param maxItems - Override max stagger items
 */
export const listStaggerVariants = (
  prefersReduced: boolean = false,
  maxItems?: number
) => {
  const maxStaggerItems = maxItems || motionConfig.list.maxStaggerItems;

  if (prefersReduced) {
    return {
      container: {
        hidden: { opacity: 0 },
        visible: {
          opacity: 1,
          transition: { duration: 0, staggerChildren: 0 },
        },
      },
      item: {
        hidden: { opacity: 0 },
        visible: {
          opacity: 1,
          transition: { duration: 0 },
        },
      },
    };
  }

  return {
    container: {
      hidden: { opacity: 0 },
      visible: {
        opacity: 1,
        transition: {
          staggerChildren: motionConfig.list.staggerDelay / 1000,
          delayChildren: motionConfig.list.initialDelay / 1000,
          duration: motionConfig.list.itemDuration / 1000,
        },
      },
    },
    item: {
      hidden: {
        opacity: 0,
        y: 20,
      },
      visible: {
        opacity: 1,
        y: 0,
        transition: {
          duration: motionConfig.list.itemDuration,
          ease: motionConfig.easing.easeOut,
        },
      },
    },
  };
};

/**
 * Loading spinner animation
 * Continuous rotation
 */
export const spinnerVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      animate: { rotate: 0 },
      transition: { duration: 0 },
    };
  }

  return {
    animate: {
      rotate: 360,
    },
    transition: {
      duration: 1000,
      repeat: Infinity,
      ease: 'linear' as const,
    },
  };
};

/**
 * Skeleton shimmer animation
 * Continuous gradient movement
 */
export const skeletonShimmerVariants = (prefersReduced: boolean = false) => {
  if (prefersReduced) {
    return {
      animate: { backgroundPosition: '0% 0%' },
      transition: { duration: 0 },
    };
  }

  return {
    animate: {
      backgroundPosition: ['0% 0%', '100% 0%'],
    },
    transition: {
      duration: motionConfig.skeleton.duration,
      repeat: Infinity,
      ease: 'linear' as const,
    },
  };
};
