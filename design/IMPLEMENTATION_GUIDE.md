# Motion Design Implementation Guide

## Quick Start

This guide provides step-by-step instructions for implementing the motion design specification in Grainlify components.

### 1. Import Motion Library

```tsx
import { motion } from 'motion';
```

### 2. Import Motion Config and Hooks

```tsx
import { motionConfig } from '@/shared/config/motionConfig';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';
import {
  pageTransitionVariants,
  cardVariants,
  listStaggerVariants,
  buttonVariants,
} from '@/shared/animations/variants';
```

### 3. Use in Components

---

## Common Patterns

### Pattern 1: Page Transitions

**Use for:** Route changes between dashboard, profile, settings, etc.

```tsx
import { motion } from 'motion';
import { pageTransitionVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const DashboardPage = () => {
  const prefersReduced = useReducedMotion();

  return (
    <motion.div
      variants={pageTransitionVariants('up', prefersReduced)}
      initial="initial"
      animate="animate"
      exit="exit"
    >
      {/* Page content */}
    </motion.div>
  );
};
```

**Direction options:** 'up' | 'down' | 'left' | 'right' | 'none'

---

### Pattern 2: Card Hover Effects

**Use for:** Dashboard cards, project cards, issue cards, list items

```tsx
import { motion } from 'motion';
import { cardVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const DashboardCard = ({ title, icon, children }) => {
  const prefersReduced = useReducedMotion();

  return (
    <motion.div
      variants={cardVariants(prefersReduced)}
      initial="initial"
      whileHover="whileHover"
      whileTap="whileTap"
      className="rounded-lg bg-white p-6 shadow-sm cursor-pointer"
    >
      <div className="flex items-center gap-2 mb-4">
        {icon}
        <h3 className="font-semibold">{title}</h3>
      </div>
      {children}
    </motion.div>
  );
};
```

---

### Pattern 3: Button Interactions

**Use for:** Primary buttons, secondary buttons, action buttons

```tsx
import { motion } from 'motion';
import { buttonVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const Button = ({ children, onClick }) => {
  const prefersReduced = useReducedMotion();

  return (
    <motion.button
      variants={buttonVariants(prefersReduced)}
      initial="initial"
      whileHover="whileHover"
      whileTap="whileTap"
      onClick={onClick}
      className="px-4 py-2 bg-primary text-white rounded-lg"
    >
      {children}
    </motion.button>
  );
};
```

---

### Pattern 4: List Animations with Stagger

**Use for:** Issue lists, PR lists, activity logs, any repeating content

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
      className="space-y-2"
    >
      {issues.slice(0, 12).map((issue) => (
        <motion.div
          key={issue.id}
          variants={variants.item}
          className="p-4 border rounded-lg hover:shadow-md"
        >
          <h4 className="font-medium">{issue.title}</h4>
          <p className="text-sm text-gray-600">{issue.description}</p>
        </motion.div>
      ))}

      {/* Handle items beyond max stagger */}
      {issues.length > 12 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.65 }}
        >
          {issues.slice(12).map((issue) => (
            <div key={issue.id} className="p-4 border rounded-lg">
              <h4 className="font-medium">{issue.title}</h4>
            </div>
          ))}
        </motion.div>
      )}
    </motion.div>
  );
};
```

---

### Pattern 5: Modal Dialogs

**Use for:** Confirmation dialogs, forms, alerts, popovers

```tsx
import { motion, AnimatePresence } from 'motion';
import { modalVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const ConfirmDialog = ({ isOpen, onConfirm, onCancel }) => {
  const prefersReduced = useReducedMotion();
  const variants = modalVariants(prefersReduced);

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            variants={variants.backdrop}
            initial="initial"
            animate="animate"
            exit="exit"
            onClick={onCancel}
            className="fixed inset-0 bg-black/50"
          />

          {/* Modal Content */}
          <motion.div
            variants={variants.content}
            initial="initial"
            animate="animate"
            exit="exit"
            className="fixed inset-0 flex items-center justify-center"
          >
            <div className="bg-white rounded-lg p-6 max-w-sm">
              <h2 className="text-lg font-bold mb-4">Confirm Action</h2>
              <p className="mb-6 text-gray-600">Are you sure?</p>
              <div className="flex gap-3 justify-end">
                <button
                  onClick={onCancel}
                  className="px-4 py-2 border rounded-lg"
                >
                  Cancel
                </button>
                <button
                  onClick={onConfirm}
                  className="px-4 py-2 bg-primary text-white rounded-lg"
                >
                  Confirm
                </button>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
};
```

---

### Pattern 6: Dropdown Menus

**Use for:** Select dropdowns, filter menus, action menus

```tsx
import { motion, AnimatePresence } from 'motion';
import { dropdownVariants, dropdownItemVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const Dropdown = ({ isOpen, items, onSelect }) => {
  const prefersReduced = useReducedMotion();

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          variants={dropdownVariants(prefersReduced)}
          initial="initial"
          animate="animate"
          exit="exit"
          className="absolute top-full left-0 mt-2 bg-white rounded-lg shadow-lg border"
        >
          {items.map((item) => (
            <DropdownItem
              key={item.id}
              item={item}
              onSelect={onSelect}
              prefersReduced={prefersReduced}
            />
          ))}
        </motion.div>
      )}
    </AnimatePresence>
  );
};

const DropdownItem = ({ item, onSelect, prefersReduced }) => {
  const variants = dropdownItemVariants(prefersReduced);

  return (
    <motion.button
      variants={variants}
      initial="initial"
      whileHover="whileHover"
      onClick={() => onSelect(item)}
      className="w-full text-left px-4 py-2"
    >
      {item.label}
    </motion.button>
  );
};
```

---

### Pattern 7: Toast Notifications

**Use for:** Success messages, error alerts, confirmations

```tsx
import { motion, AnimatePresence } from 'motion';
import { toastVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const ToastContainer = ({ toasts, onRemove }) => {
  const prefersReduced = useReducedMotion();

  return (
    <div className="fixed bottom-4 right-4 space-y-2 pointer-events-none">
      <AnimatePresence>
        {toasts.map((toast) => (
          <motion.div
            key={toast.id}
            variants={toastVariants(prefersReduced)}
            initial="initial"
            animate="animate"
            exit="exit"
            className="bg-white rounded-lg shadow-lg p-4 pointer-events-auto"
          >
            <p className="font-medium">{toast.message}</p>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
};
```

---

### Pattern 8: Loading Spinners

**Use for:** Loading states, async operations

```tsx
import { motion } from 'motion';
import { spinnerVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const Spinner = () => {
  const prefersReduced = useReducedMotion();

  return (
    <motion.div
      variants={spinnerVariants(prefersReduced)}
      animate={spinnerVariants(prefersReduced).animate}
      transition={spinnerVariants(prefersReduced).transition}
      className="w-6 h-6 border-3 border-gray-300 border-t-primary rounded-full"
    />
  );
};
```

---

### Pattern 9: Skeleton Loaders

**Use for:** Placeholder content while loading

```tsx
import { motion } from 'motion';
import { skeletonShimmerVariants } from '@/shared/animations/variants';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';

export const SkeletonCard = () => {
  const prefersReduced = useReducedMotion();
  const variants = skeletonShimmerVariants(prefersReduced);

  if (prefersReduced) {
    return (
      <div className="rounded-lg bg-gray-200 h-48" />
    );
  }

  return (
    <motion.div
      variants={variants}
      animate="animate"
      transition={variants.transition}
      className="relative overflow-hidden rounded-lg bg-gray-200 h-48"
      style={{
        backgroundImage: 'linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent)',
        backgroundSize: '100% 100%',
      }}
    />
  );
};
```

---

## Using Motion Config Directly

For custom animations not covered by variants, use `motionConfig`:

```tsx
import { motion } from 'motion';
import { motionConfig } from '@/shared/config/motionConfig';

export const CustomAnimation = () => (
  <motion.div
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    transition={{
      duration: motionConfig.durations.normal,
      ease: motionConfig.easing.easeOut,
    }}
  />
);
```

---

## Accessibility Checklist

Before shipping any component with animations:

- [ ] Tested with `prefers-reduced-motion: reduce` enabled
- [ ] Animations complete in under 5 seconds or have pause control
- [ ] No rapid flashing (>3Hz)
- [ ] Focus states are visible
- [ ] Keyboard navigation works smoothly
- [ ] Screen reader announcements not disrupted
- [ ] Tested on low-end devices (Moto G4 simulation)

---

## Performance Tips

1. **Use GPU-accelerated properties only:**
   - ✅ `transform` (translate, scale, rotate)
   - ✅ `opacity`
   - ❌ `width`, `height`, `top`, `left`

2. **Limit concurrent animations:**
   - Max 3-4 simultaneous animations per view
   - Use CSS for simple state-less animations

3. **Lazy load animations:**
   ```tsx
   const [shouldAnimate, setShouldAnimate] = useState(false);

   useEffect(() => {
     const timer = setTimeout(() => setShouldAnimate(true), 100);
     return () => clearTimeout(timer);
   }, []);

   <motion.div animate={shouldAnimate ? "visible" : "hidden"} />
   ```

4. **Use `will-change` for optimized rendering:**
   ```tsx
   <motion.div className="will-change-transform" animate={{ scale: 1.02 }} />
   ```

---

## Common Issues

### Issue: Animation jumps on fast connection
**Solution:** Add `initial={false}` if you don't want entry animation, or use `AnimatePresence` for exit animations.

### Issue: Animation stutters on mobile
**Solution:** Reduce duration and simplify animations. Use `useResponsiveBreakpoint()` hook:

```tsx
const breakpoint = useResponsiveBreakpoint();
const duration = breakpoint.isMobile ? 150 : 300;
```

### Issue: Stagger delay is too long
**Solution:** Reduce `maxStaggerItems` or use `listStaggerVariants(false, 8)` to override max items.

### Issue: Animation doesn't respect reduced motion
**Solution:** Always pass `prefersReduced` to variant functions:

```tsx
const prefersReduced = useReducedMotion();
<motion.div variants={cardVariants(prefersReduced)} />
```

---

## Files Reference

- **Config:** `frontend/src/shared/config/motionConfig.ts`
- **Variants:** `frontend/src/shared/animations/variants.ts`
- **Hooks:** `frontend/src/shared/hooks/useReducedMotion.ts`
- **Spec:** `/design/motion-spec.md`
- **Design Tokens:** `/design-tokens.json`

---

## Testing

### Manual Testing
```bash
# Open DevTools > Rendering > Show Rendering Paint Flashes
# Verify animations render smoothly (60 FPS)

# Enable prefers-reduced-motion
# System Preferences > Accessibility > Display > Reduce motion
# Verify all animations are instant or faded only
```

### Automated Testing
```tsx
it('respects prefers-reduced-motion', () => {
  // Setup media query mock
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: jest.fn().mockImplementation(query => ({
      matches: query === '(prefers-reduced-motion: reduce)',
      media: query,
      onchange: null,
      addListener: jest.fn(),
      removeListener: jest.fn(),
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      dispatchEvent: jest.fn(),
    })),
  });

  render(<YourComponent />);
  // Assert animations have 0 duration
});
```

---

## Questions?

Refer to the main specification at `/design/motion-spec.md` or reach out to the design team.
