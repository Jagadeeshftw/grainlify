# Motion Design Specification for Grainlify

## Overview
This specification defines all motion design patterns, transitions, and micro-interactions used throughout the Grainlify application. All animations respect WCAG 2.1 AA accessibility standards and include full `prefers-reduced-motion` alternatives.

**Last Updated**: May 31, 2026  
**Framework**: Motion (Framer Motion v12)  
**Status**: Active Design Specification

---

## Table of Contents
1. [Motion Tokens](#motion-tokens)
2. [Page Transitions](#page-transitions)
3. [Component Micro-Interactions](#component-micro-interactions)
4. [List Animations](#list-animations)
5. [Reduced Motion Alternatives](#reduced-motion-alternatives)
6. [Accessibility & Performance](#accessibility--performance)
7. [Implementation Guide](#implementation-guide)
8. [Responsive Behavior](#responsive-behavior)

---

## Motion Tokens

### Duration
Standard durations (milliseconds) to maintain consistency across interactions:

```json
{
  "duration": {
    "instant": 0,
    "fast": 150,
    "normal": 300,
    "slow": 500,
    "slower": 800,
    "slowest": 1200
  }
}
```

| Token | Duration | Use Case |
|-------|----------|----------|
| `instant` | 0ms | Reduced-motion fallback, immediate state changes |
| `fast` | 150ms | Quick micro-interactions (hover, focus) |
| `normal` | 300ms | Standard transitions (page navigation, modal open) |
| `slow` | 500ms | Emphasis transitions (important state changes) |
| `slower` | 800ms | Complex multi-step animations |
| `slowest` | 1200ms | Skeleton shimmer effects |

### Easing Functions
Standard easing curves for predictable, polished motion:

```tsx
// Easing values for Motion library
const easing = {
  easeIn: [0.4, 0, 1, 1],           // accelerating from zero velocity
  easeOut: [0, 0, 0.2, 1],          // decelerating to zero velocity
  easeInOut: [0.4, 0, 0.2, 1],      // acceleration until halfway, then deceleration
  spring: { type: "spring", stiffness: 100, damping: 10 }, // natural, bouncy
  smooth: { type: "spring", stiffness: 200, damping: 20 }  // tighter, faster
};
```

| Easing | Formula | Use Case |
|--------|---------|----------|
| `easeOut` | [0, 0, 0.2, 1] | Entrance animations (things appearing) |
| `easeIn` | [0.4, 0, 1, 1] | Exit animations (things disappearing) |
| `easeInOut` | [0.4, 0, 0.2, 1] | Interactive, continuous transitions |
| `spring` | spring config | Playful micro-interactions, bouncy feel |
| `smooth` | smooth config | Smooth continuous motion |

---

## Page Transitions

### Routing Structure
| From | To | Transition | Duration | Easing | Direction |
|------|----|-----------|---------|---------|----|
| Landing → Dashboard | Fade + Slide | 300ms | easeOut | Up (↑) |
| Dashboard → Profile Page | Fade + Slide | 300ms | easeOut | Right (→) |
| Dashboard → Leaderboard | Fade + Slide | 300ms | easeOut | Left (←) |
| Dashboard → Settings | Fade + Slide | 300ms | easeOut | Right (→) |
| Settings → Dashboard | Fade + Slide | 300ms | easeIn | Left (←) |
| Blog → Blog Detail | Fade + Scale | 300ms | easeOut | Center (◆) |
| Maintainers → Admin | Fade + Slide | 300ms | easeOut | Down (↓) |
| Any → Error Page | Fade | 200ms | easeOut | None |
| Any → Auth Pages | Fade | 250ms | easeOut | None |

### Page Transition Animation Pattern

#### 1. **Standard Page Transition (Fade + Slide)**

```tsx
// Exit animation (current page)
exit: {
  opacity: 0,
  y: direction === 'up' ? -40 : direction === 'down' ? 40 : 0,
  x: direction === 'left' ? -40 : direction === 'right' ? 40 : 0,
  transition: { duration: 0.3, ease: [0.4, 0, 1, 1] }
}

// Enter animation (new page)
initial: {
  opacity: 0,
  y: direction === 'up' ? 40 : direction === 'down' ? -40 : 0,
  x: direction === 'left' ? 40 : direction === 'right' ? -40 : 0
}

animate: {
  opacity: 1,
  y: 0,
  x: 0,
  transition: { duration: 0.3, ease: [0, 0, 0.2, 1] }
}
```

**Specs:**
- **Duration**: 300ms
- **Easing**: easeOut for enter, easeIn for exit
- **Distance**: 40px slide offset
- **Opacity**: 0 → 1

#### 2. **Scale + Fade (Detail View)**

Used for expanding views (blog detail, project detail):

```tsx
initial: {
  opacity: 0,
  scale: 0.95
}

animate: {
  opacity: 1,
  scale: 1,
  transition: { duration: 0.3, ease: [0, 0, 0.2, 1] }
}

exit: {
  opacity: 0,
  scale: 0.95,
  transition: { duration: 0.2, ease: [0.4, 0, 1, 1] }
}
```

#### 3. **Dialog/Modal Transitions**

```tsx
// Backdrop
initial: { opacity: 0 }
animate: { opacity: 1, transition: { duration: 0.2 } }
exit: { opacity: 0, transition: { duration: 0.15 } }

// Content
initial: { opacity: 0, scale: 0.9, y: 20 }
animate: { opacity: 1, scale: 1, y: 0, transition: { duration: 0.3 } }
exit: { opacity: 0, scale: 0.9, y: 20, transition: { duration: 0.2 } }
```

---

## Component Micro-Interactions

### 1. **Card Hover Effects**

Used on dashboard cards, project cards, issue cards, etc.

**Default State:**
```tsx
initial: {
  scale: 1,
  boxShadow: "0 1px 3px rgba(0, 0, 0, 0.1)"
}
```

**Hover State:**
```tsx
whileHover: {
  scale: 1.02,
  boxShadow: "0 20px 25px -5px rgba(0, 0, 0, 0.1)",
  transition: { duration: 0.15, ease: [0, 0, 0.2, 1] }
}

whileTap: {
  scale: 0.98,
  transition: { duration: 0.1 }
}
```

| Property | Default | Hover | Transition |
|----------|---------|-------|-----------|
| Scale | 1 | 1.02 | 150ms easeOut |
| Shadow | 1px soft | 20px elevation | 150ms easeOut |
| Opacity | 1 | 1 | None |
| Cursor | pointer | pointer | None |

**Implementation:**
```tsx
<motion.div
  initial={{ scale: 1, boxShadow: "0 1px 3px rgba(0, 0, 0, 0.1)" }}
  whileHover={{ 
    scale: 1.02, 
    boxShadow: "0 20px 25px -5px rgba(0, 0, 0, 0.1)",
    transition: { duration: 0.15, ease: [0, 0, 0.2, 1] }
  }}
  whileTap={{ scale: 0.98 }}
  className="rounded-lg bg-white p-4 cursor-pointer"
>
  {/* Card content */}
</motion.div>
```

### 2. **Button Interactions**

#### Primary Button
```tsx
whileHover: { scale: 1.02, transition: { duration: 0.15 } }
whileTap: { scale: 0.95, transition: { duration: 0.1 } }
```

#### Icon Button
```tsx
whileHover: { scale: 1.1, transition: { duration: 0.15 } }
whileTap: { scale: 0.9, transition: { duration: 0.1 } }
```

### 3. **Input Focus Effects**

```tsx
whileFocus: {
  boxShadow: "0 0 0 3px rgba(241, 180, 0, 0.1)",
  transition: { duration: 0.2 }
}

whileInvalid: {
  boxShadow: "0 0 0 3px rgba(239, 68, 68, 0.1)",
  transition: { duration: 0.2 }
}
```

### 4. **Dropdown/Select Animations**

#### Open Animation
```tsx
initial: { opacity: 0, y: -10 }
animate: { opacity: 1, y: 0, transition: { duration: 0.15 } }
exit: { opacity: 0, y: -10, transition: { duration: 0.1 } }
```

#### Item Hover
```tsx
whileHover: { 
  backgroundColor: "rgba(241, 180, 0, 0.1)",
  paddingLeft: "1.5rem",
  transition: { duration: 0.15 }
}
```

### 5. **Badge/Tag Animations**

```tsx
initial: { scale: 0, opacity: 0 }
animate: { scale: 1, opacity: 1, transition: { duration: 0.2 } }
exit: { scale: 0, opacity: 0, transition: { duration: 0.15 } }
```

### 6. **Toast/Notification Animations**

```tsx
// Entrance
initial: { x: 400, opacity: 0 }
animate: { x: 0, opacity: 1, transition: { duration: 0.3 } }

// Exit
exit: { x: 400, opacity: 0, transition: { duration: 0.2 } }
```

### 7. **Loading Spinner Animation**

```tsx
animate: { rotate: 360 }
transition: { 
  duration: 1, 
  repeat: Infinity, 
  ease: "linear"
}
```

---

## List Animations

### Stagger Configuration

Lists should use staggered animations to reveal items sequentially for visual feedback and performance.

```tsx
const containerVariants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: {
      staggerChildren: 0.05,           // 50ms delay between items
      delayChildren: 0.1,              // 100ms before first item
      duration: 0.3
    }
  }
};

const itemVariants = {
  hidden: { opacity: 0, y: 20 },
  visible: { 
    opacity: 1, 
    y: 0,
    transition: { duration: 0.3, ease: [0, 0, 0.2, 1] }
  }
};
```

| Parameter | Value | Description |
|-----------|-------|-------------|
| Stagger Delay | 50ms | Time between each item animation |
| Initial Delay | 100ms | Time before first item starts |
| Item Duration | 300ms | Time for each item to animate in |
| Max Stagger Items | 12 | Maximum items to stagger (after 12, show all at once) |

### Stagger Rules

| Scenario | Stagger | Reason |
|----------|---------|--------|
| List with <5 items | Yes, 50ms | Provides visual feedback |
| List with 5-12 items | Yes, 50ms | Guides eye through content |
| List with 12+ items | No (show all) | Avoids excessive delay (>600ms) |
| Virtual/Infinite scroll | No | Initial set only, rest loads on scroll |
| Search results | Yes, stagger new | Only animate newly matched items |
| Filter actions | Yes, stagger updated | Show updated items with stagger |

### Implementation Example

```tsx
<motion.div
  initial="hidden"
  animate="visible"
  variants={containerVariants}
  className="space-y-2"
>
  {items.slice(0, 12).map((item, i) => (
    <motion.div
      key={item.id}
      variants={itemVariants}
      className="p-4 rounded-lg bg-white hover:shadow-lg"
    >
      {item.content}
    </motion.div>
  ))}
  
  {items.length > 12 && (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ delay: 0.65 }}
    >
      {/* Show remaining items without stagger */}
      {items.slice(12).map(item => (
        <div key={item.id} className="p-4">
          {item.content}
        </div>
      ))}
    </motion.div>
  )}
</motion.div>
```

---

## Reduced Motion Alternatives

### WCAG 2.1 Compliance
Per **WCAG 2.1 AA (Animation from Interactions)**, all animations must respect `prefers-reduced-motion: reduce`.

### Media Query Detection

```tsx
// Hook for reduced motion preference
export const useReducedMotion = () => {
  const [prefersReduced, setPrefersReduced] = React.useState(false);

  React.useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    setPrefersReduced(mediaQuery.matches);

    const listener = (e) => setPrefersReduced(e.matches);
    mediaQuery.addEventListener("change", listener);
    return () => mediaQuery.removeEventListener("change", listener);
  }, []);

  return prefersReduced;
};
```

### Reduced Motion Patterns

#### Pattern 1: Instant Transitions
When motion must be removed, use instant state changes:

```tsx
const prefersReduced = useReducedMotion();

<motion.div
  initial={{ opacity: 0, scale: 0.95 }}
  animate={{ opacity: 1, scale: 1 }}
  transition={{
    duration: prefersReduced ? 0 : 0.3,
    ease: prefersReduced ? undefined : [0, 0, 0.2, 1]
  }}
/>
```

#### Pattern 2: Opacity Only
Replace complex animations with simple fade:

```tsx
animate={{
  opacity: prefersReduced ? 1 : targetOpacity,
  scale: prefersReduced ? 1 : targetScale,
  y: prefersReduced ? 0 : targetY
}}
transition={{ duration: prefersReduced ? 0 : 0.3 }}
```

#### Pattern 3: No Animation
Remove all animations completely:

```tsx
const getTransition = (prefersReduced) => 
  prefersReduced ? { duration: 0 } : { duration: 0.3 };
```

### Reduced Motion Specifications

| Component | Normal | Reduced Motion |
|-----------|--------|-----------------|
| Page transitions | Fade + Slide (300ms) | Instant opacity (0ms) |
| Card hover | Scale 1.02 + Shadow (150ms) | Instant opacity change |
| List stagger | 50ms per item | All items appear instantly |
| Modal open | Scale + Fade (300ms) | Instant appearance |
| Dropdown | Slide + Fade (150ms) | Instant appearance |
| Loading shimmer | 1.2s animation | Static placeholder |
| Spinner | Continuous rotate | Static icon or pulse |
| Toast | Slide in (300ms) | Instant appearance |

### Reduced Motion CSS Class Approach

Alternative: Use a CSS class for reduced motion styling:

```css
.prefers-reduced-motion {
  --transition-duration: 0s !important;
  --animation-duration: 0s !important;
}

.prefers-reduced-motion * {
  animation-duration: var(--animation-duration) !important;
  transition-duration: var(--transition-duration) !important;
}
```

---

## Accessibility & Performance

### Accessibility Requirements

#### 1. **No Auto-Playing Motion >5 seconds**
- Skeleton shimmer: 1.2s ✓
- Loading spinner: Disabled after 5s, show static state
- Toast notifications: No looping animations

#### 2. **Pause Mechanism for Long Animations**
```tsx
const [isPaused, setIsPaused] = React.useState(false);

<motion.div
  animate={isPaused ? { rotate: 0 } : { rotate: 360 }}
  transition={{ 
    duration: 1,
    repeat: Infinity,
    repeatType: "loop"
  }}
  onClick={() => setIsPaused(!isPaused)}
/>
```

#### 3. **Focus States Visible**
All interactive elements must have visible focus indicators:
```tsx
whileFocus={{ boxShadow: "0 0 0 3px rgba(241, 180, 0, 0.2)" }}
```

#### 4. **Keyboard Navigation**
All animations must work with keyboard navigation (tab, enter, space).

### Performance Requirements

#### 1. **GPU Acceleration**
Use `transform` and `opacity` only (GPU-composited):

```tsx
// ✓ Good
animate={{ 
  scale: 1.02,      // uses transform
  opacity: 0.8,     // uses opacity
  boxShadow: "..."  // filters
}}

// ✗ Bad
animate={{ 
  width: 200,       // causes layout shift
  height: 100,      // causes layout shift
  top: 50,          // causes reflow
}}
```

#### 2. **Optimize with `will-change`**
```tsx
<motion.div
  className="will-change-transform"
  animate={{ scale: 1.02 }}
/>
```

#### 3. **Contain Animations**
```tsx
<div className="overflow-hidden">
  <motion.div
    animate={{ x: 100 }}
    // Animation contained within parent
  />
</div>
```

#### 4. **Lazy Load Complex Animations**
```tsx
const [shouldAnimate, setShouldAnimate] = React.useState(false);

React.useEffect(() => {
  const timer = setTimeout(() => setShouldAnimate(true), 100);
  return () => clearTimeout(timer);
}, []);

<motion.div animate={shouldAnimate ? "visible" : "hidden"} />
```

#### 5. **Limit Concurrent Animations**
- Maximum 3-4 simultaneous animations per view
- Stagger items instead of all at once
- Use CSS animations for simple state-less animations

---

## Implementation Guide

### Step 1: Install Motion
```bash
npm install motion
# Already in dependencies: "motion": "12.23.24"
```

### Step 2: Create Motion Config File
Create `frontend/src/shared/config/motionConfig.ts`:

```tsx
export const motionConfig = {
  durations: {
    instant: 0,
    fast: 150,
    normal: 300,
    slow: 500,
    slower: 800,
    slowest: 1200
  },
  easing: {
    easeOut: [0, 0, 0.2, 1],
    easeIn: [0.4, 0, 1, 1],
    easeInOut: [0.4, 0, 0.2, 1],
    spring: { type: "spring", stiffness: 100, damping: 10 }
  }
};
```

### Step 3: Create Reusable Variants
Create `frontend/src/shared/animations/variants.ts`:

```tsx
import { motionConfig } from '../config/motionConfig';

export const pageTransitionVariants = {
  fadeSlide: (direction = 'up') => ({
    initial: {
      opacity: 0,
      y: direction === 'up' ? 40 : direction === 'down' ? -40 : 0,
      x: direction === 'left' ? 40 : direction === 'right' ? -40 : 0
    },
    animate: {
      opacity: 1,
      y: 0,
      x: 0,
      transition: {
        duration: motionConfig.durations.normal,
        ease: motionConfig.easing.easeOut
      }
    },
    exit: {
      opacity: 0,
      y: direction === 'up' ? -40 : direction === 'down' ? 40 : 0,
      x: direction === 'left' ? -40 : direction === 'right' ? 40 : 0,
      transition: {
        duration: motionConfig.durations.normal,
        ease: motionConfig.easing.easeIn
      }
    }
  })
};

export const cardHoverVariants = {
  initial: { scale: 1 },
  whileHover: {
    scale: 1.02,
    transition: { duration: motionConfig.durations.fast }
  },
  whileTap: {
    scale: 0.98,
    transition: { duration: 0.1 }
  }
};

export const listStaggerVariants = (prefersReduced = false) => ({
  container: {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        staggerChildren: prefersReduced ? 0 : 0.05,
        delayChildren: prefersReduced ? 0 : 0.1
      }
    }
  },
  item: {
    hidden: { opacity: 0, y: prefersReduced ? 0 : 20 },
    visible: {
      opacity: 1,
      y: 0,
      transition: { duration: prefersReduced ? 0 : motionConfig.durations.normal }
    }
  }
});
```

### Step 4: Create Reduced Motion Hook
Create `frontend/src/shared/hooks/useReducedMotion.ts`:

```tsx
import React from 'react';

export const useReducedMotion = () => {
  const [prefersReduced, setPrefersReduced] = React.useState(false);

  React.useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    setPrefersReduced(mediaQuery.matches);

    const listener = (e) => setPrefersReduced(e.matches);
    mediaQuery.addEventListener('change', listener);
    return () => mediaQuery.removeEventListener('change', listener);
  }, []);

  return prefersReduced;
};
```

### Step 5: Apply to Components

#### Page Layout with Transitions
```tsx
import { motion } from 'motion';
import { pageTransitionVariants } from '@/shared/animations/variants';

export const PageLayout = ({ children }) => (
  <motion.div
    variants={pageTransitionVariants('up')}
    initial="initial"
    animate="animate"
    exit="exit"
  >
    {children}
  </motion.div>
);
```

#### Card with Hover
```tsx
import { motion } from 'motion';
import { cardHoverVariants } from '@/shared/animations/variants';

export const DashboardCard = ({ title, children }) => (
  <motion.div
    variants={cardHoverVariants}
    initial="initial"
    whileHover="whileHover"
    whileTap="whileTap"
    className="rounded-lg bg-white p-6 shadow-sm"
  >
    <h3>{title}</h3>
    {children}
  </motion.div>
);
```

#### List with Stagger
```tsx
import { motion } from 'motion';
import { listStaggerVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const IssueList = ({ issues }) => {
  const prefersReduced = useReducedMotion();
  const variants = listStaggerVariants(prefersReduced);

  return (
    <motion.div
      variants={variants.container}
      initial="hidden"
      animate="visible"
    >
      {issues.map(issue => (
        <motion.div
          key={issue.id}
          variants={variants.item}
          className="p-4 border rounded"
        >
          {issue.title}
        </motion.div>
      ))}
    </motion.div>
  );
};
```

---

## Responsive Behavior

### Breakpoints (Tailwind Convention)
| Breakpoint | Width | Context |
|------------|-------|---------|
| `sm` | 640px | Small phones |
| `md` | 768px | Tablets |
| `lg` | 1024px | Desktops |
| `xl` | 1280px | Large desktops |

### Motion Adjustments by Screen Size

| Breakpoint | Duration Adjustment | Stagger Adjustment | Notes |
|------------|--------------------|--------------------|-------|
| `sm` | -25% (225ms → 300ms) | 30ms (down from 50ms) | Mobile: faster, tighter stagger |
| `md` | -10% (270ms → 300ms) | 40ms | Tablet: slightly optimized |
| `lg` | Default (300ms) | 50ms | Desktop: full spec |
| `xl` | Default (300ms) | 50ms | Large desktop: full spec |

### Implementation

```tsx
const getMotionConfig = (isMobile: boolean) => ({
  duration: isMobile ? 225 : 300,
  staggerDelay: isMobile ? 0.03 : 0.05
});

// Usage in components
const isMobile = window.innerWidth < 768;
const config = getMotionConfig(isMobile);
```

---

## Design Tokens Integration

These motion specifications are reflected in the global `design-tokens.json`:

```json
{
  "motion": {
    "duration": {
      "instant": "0ms",
      "fast": "150ms",
      "normal": "300ms",
      "slow": "500ms",
      "slower": "800ms",
      "slowest": "1200ms"
    },
    "easing": {
      "easeOut": "cubic-bezier(0, 0, 0.2, 1)",
      "easeIn": "cubic-bezier(0.4, 0, 1, 1)",
      "easeInOut": "cubic-bezier(0.4, 0, 0.2, 1)"
    }
  }
}
```

---

## Quality Assurance Checklist

### Accessibility Testing
- [ ] Tested with `prefers-reduced-motion: reduce` enabled
- [ ] All animations respect reduced-motion preference
- [ ] No animations trigger vestibular issues (no rapid flashing >3Hz)
- [ ] Focus states visible and animated smoothly
- [ ] Keyboard navigation works with all animations
- [ ] Screen readers not disrupted by animations

### Performance Testing
- [ ] Chrome DevTools: all animations 60 FPS
- [ ] No layout shifts (CLS < 0.1)
- [ ] GPU acceleration verified (transform/opacity only)
- [ ] Performance on low-end devices (Moto G4 simulation)
- [ ] Bundle size: Motion library < 20KB gzipped

### Visual Testing
- [ ] All page transitions smooth (no jank)
- [ ] Card hover effects responsive and consistent
- [ ] List stagger timing correct (50ms gaps)
- [ ] Reduced-motion fallbacks visible and functional
- [ ] Dark mode: animations respect theme
- [ ] Mobile: animations optimized for viewport

### Cross-Browser Testing
- [ ] Chrome/Edge: All animations smooth
- [ ] Firefox: GPU acceleration working
- [ ] Safari: Spring animations smooth
- [ ] Mobile browsers: Touch interactions responsive

### Responsive Testing (All breakpoints)
- [ ] `sm` (320px): Mobile stagger optimized
- [ ] `md` (768px): Tablet transitions smooth
- [ ] `lg` (1024px): Desktop full spec
- [ ] `xl` (1280px): Large displays

---

## Common Implementation Patterns

### Pattern 1: Simple Fade Transition
```tsx
<motion.div
  initial={{ opacity: 0 }}
  animate={{ opacity: 1 }}
  exit={{ opacity: 0 }}
  transition={{ duration: 0.3 }}
/>
```

### Pattern 2: Hover State Elevation
```tsx
<motion.div
  initial={{ y: 0 }}
  whileHover={{ y: -4 }}
  whileTap={{ y: -1 }}
  transition={{ type: "spring", stiffness: 300 }}
/>
```

### Pattern 3: Loading Skeleton Shimmer
```tsx
<motion.div
  className="relative overflow-hidden rounded bg-gray-200"
  animate={{ backgroundPosition: ["0% 0%", "100% 0%"] }}
  transition={{ duration: 1.2, repeat: Infinity, ease: "linear" }}
/>
```

### Pattern 4: Staggered List
```tsx
<motion.ul variants={containerVariants} initial="hidden" animate="visible">
  {items.map((item) => (
    <motion.li key={item.id} variants={itemVariants}>
      {item.name}
    </motion.li>
  ))}
</motion.ul>
```

---

## References

- [Motion Library Documentation](https://motion.dev/)
- [WCAG 2.1 Animation from Interactions](https://www.w3.org/WAI/WCAG21/Understanding/animation-from-interactions)
- [Reduced Motion Media Query](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion)
- [Web Animations Performance](https://web.dev/animations-guide/)
- [Accessibility Guidelines for Motion](https://www.a11y-101.com/design/animations)

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | May 31, 2026 | Initial motion specification with page transitions, micro-interactions, and reduced-motion alternatives |

---

**Document Status**: Ready for Implementation  
**Last Reviewed**: May 31, 2026  
**Next Review**: Q3 2026
