/**
 * Motion Design Examples Component
 * 
 * This component demonstrates all the motion design patterns
 * defined in /design/motion-spec.md
 * 
 * Use this as a reference when implementing motion in your components.
 * 
 * @see /design/motion-spec.md
 * @see /design/IMPLEMENTATION_GUIDE.md
 */

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'motion';
import { useReducedMotion } from '@/shared/hooks/useReducedMotion';
import {
  pageTransitionVariants,
  cardVariants,
  buttonVariants,
  listStaggerVariants,
  modalVariants,
  dropdownVariants,
  dropdownItemVariants,
  toastVariants,
  spinnerVariants,
} from '@/shared/animations/variants';

/**
 * Example: Page Transition
 * Used when navigating between routes
 */
export const PageTransitionExample = () => {
  const prefersReduced = useReducedMotion();

  return (
    <motion.div
      variants={pageTransitionVariants('up', prefersReduced)}
      initial="initial"
      animate="animate"
      exit="exit"
      className="p-8 bg-white rounded-lg"
    >
      <h2 className="text-2xl font-bold mb-4">Page Transition Example</h2>
      <p className="text-gray-600">
        This entire section fades in and slides up when the page loads.
        It respects prefers-reduced-motion preferences.
      </p>
    </motion.div>
  );
};

/**
 * Example: Card with Hover Effect
 * Used for dashboard cards, project cards, etc.
 */
export const CardHoverExample = () => {
  const prefersReduced = useReducedMotion();

  return (
    <motion.div
      variants={cardVariants(prefersReduced)}
      initial="initial"
      whileHover="whileHover"
      whileTap="whileTap"
      className="p-6 bg-white rounded-lg shadow-sm cursor-pointer border border-gray-200"
    >
      <h3 className="text-lg font-semibold mb-2">Card Hover Example</h3>
      <p className="text-gray-600">
        This card scales up 2% and elevates shadow on hover.
        Duration: 150ms with ease-out timing.
      </p>
      <div className="mt-4 flex gap-2">
        <span className="inline-block px-3 py-1 bg-primary/10 text-primary rounded-full text-sm">
          Hover me
        </span>
      </div>
    </motion.div>
  );
};

/**
 * Example: Button Interaction
 * Used for primary and secondary buttons
 */
export const ButtonInteractionExample = () => {
  const prefersReduced = useReducedMotion();

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold">Button Interaction Example</h3>

      <motion.button
        variants={buttonVariants(prefersReduced)}
        initial="initial"
        whileHover="whileHover"
        whileTap="whileTap"
        className="px-6 py-2 bg-primary text-white rounded-lg font-medium"
      >
        Primary Button
      </motion.button>

      <p className="text-sm text-gray-600">
        Click and hold to see tap animation. Scales on hover (102%) and on tap (98%).
      </p>
    </div>
  );
};

/**
 * Example: List with Stagger Animation
 * Used for lists, issue lists, PR lists, etc.
 */
export const ListStaggerExample = () => {
  const prefersReduced = useReducedMotion();
  const variants = listStaggerVariants(prefersReduced);

  const items = [
    { id: 1, title: 'Item 1', desc: 'First item with stagger animation' },
    { id: 2, title: 'Item 2', desc: 'Second item appears after 50ms' },
    { id: 3, title: 'Item 3', desc: 'Third item appears after 100ms' },
    { id: 4, title: 'Item 4', desc: 'Fourth item - staggered entry' },
    { id: 5, title: 'Item 5', desc: 'Items beyond 12 appear together' },
  ];

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">List Stagger Example</h3>

      <motion.div
        variants={variants.container}
        initial="hidden"
        animate="visible"
        className="space-y-3"
      >
        {items.map((item) => (
          <motion.div
            key={item.id}
            variants={variants.item}
            className="p-4 bg-white rounded-lg border border-gray-200 hover:shadow-md transition-shadow"
          >
            <h4 className="font-medium">{item.title}</h4>
            <p className="text-sm text-gray-600">{item.desc}</p>
          </motion.div>
        ))}
      </motion.div>

      <p className="mt-4 text-sm text-gray-600">
        Each item appears with 50ms stagger delay. If list had 12+ items,
        remaining items would appear together to avoid excessive delay.
      </p>
    </div>
  );
};

/**
 * Example: Modal Dialog
 * Used for confirmations, forms, alerts
 */
export const ModalExample = () => {
  const [isOpen, setIsOpen] = useState(false);
  const prefersReduced = useReducedMotion();
  const variants = modalVariants(prefersReduced);

  return (
    <div>
      <button
        onClick={() => setIsOpen(true)}
        className="px-6 py-2 bg-primary text-white rounded-lg font-medium"
      >
        Open Modal
      </button>

      <AnimatePresence>
        {isOpen && (
          <>
            {/* Backdrop */}
            <motion.div
              variants={variants.backdrop}
              initial="initial"
              animate="animate"
              exit="exit"
              onClick={() => setIsOpen(false)}
              className="fixed inset-0 bg-black/50"
            />

            {/* Modal */}
            <motion.div
              variants={variants.content}
              initial="initial"
              animate="animate"
              exit="exit"
              className="fixed inset-0 flex items-center justify-center"
            >
              <div className="bg-white rounded-lg p-6 max-w-sm shadow-xl">
                <h2 className="text-xl font-bold mb-4">Modal Example</h2>
                <p className="text-gray-600 mb-6">
                  This modal scales from 0.9 to 1.0 with fade effect.
                  Click backdrop to close.
                </p>
                <div className="flex gap-3 justify-end">
                  <button
                    onClick={() => setIsOpen(false)}
                    className="px-4 py-2 border rounded-lg hover:bg-gray-50"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={() => setIsOpen(false)}
                    className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90"
                  >
                    Confirm
                  </button>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
};

/**
 * Example: Dropdown Menu
 * Used for select, filter menus, action menus
 */
export const DropdownExample = () => {
  const [isOpen, setIsOpen] = useState(false);
  const prefersReduced = useReducedMotion();

  const items = [
    { id: 1, label: 'Option 1' },
    { id: 2, label: 'Option 2' },
    { id: 3, label: 'Option 3' },
    { id: 4, label: 'Option 4' },
  ];

  return (
    <div className="relative inline-block">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="px-4 py-2 border rounded-lg bg-white hover:bg-gray-50"
      >
        Select Option ▼
      </button>

      <AnimatePresence>
        {isOpen && (
          <motion.div
            variants={dropdownVariants(prefersReduced)}
            initial="initial"
            animate="animate"
            exit="exit"
            className="absolute top-full left-0 mt-2 bg-white rounded-lg shadow-lg border border-gray-200 min-w-48"
          >
            {items.map((item) => (
              <DropdownItemComponent
                key={item.id}
                item={item}
                onSelect={() => {
                  console.log('Selected:', item.label);
                  setIsOpen(false);
                }}
                prefersReduced={prefersReduced}
              />
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

const DropdownItemComponent = ({ item, onSelect, prefersReduced }) => {
  const variants = dropdownItemVariants(prefersReduced);

  return (
    <motion.button
      variants={variants}
      initial="initial"
      whileHover="whileHover"
      onClick={onSelect}
      className="w-full text-left px-4 py-2"
    >
      {item.label}
    </motion.button>
  );
};

/**
 * Example: Toast Notification
 * Used for success messages, errors, confirmations
 */
export const ToastExample = () => {
  const [toasts, setToasts] = useState<Array<{ id: number; message: string }>>([]);
  const prefersReduced = useReducedMotion();

  const addToast = () => {
    const id = Date.now();
    setToasts([...toasts, { id, message: 'Action completed successfully!' }]);

    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3000);
  };

  return (
    <>
      <button
        onClick={addToast}
        className="px-6 py-2 bg-primary text-white rounded-lg font-medium"
      >
        Show Toast
      </button>

      <div className="fixed bottom-4 right-4 space-y-2 pointer-events-none">
        <AnimatePresence>
          {toasts.map((toast) => (
            <motion.div
              key={toast.id}
              variants={toastVariants(prefersReduced)}
              initial="initial"
              animate="animate"
              exit="exit"
              className="bg-white rounded-lg shadow-lg p-4 pointer-events-auto border border-green-200"
            >
              <p className="font-medium text-gray-900">{toast.message}</p>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </>
  );
};

/**
 * Example: Loading Spinner
 * Used for async operations
 */
export const SpinnerExample = () => {
  const prefersReduced = useReducedMotion();
  const vars = spinnerVariants(prefersReduced);

  return (
    <div className="flex flex-col items-center gap-4">
      <motion.div
        animate={vars.animate}
        transition={vars.transition}
        className="w-8 h-8 border-3 border-gray-300 border-t-primary rounded-full"
      />
      <p className="text-sm text-gray-600">
        Loading... {prefersReduced && '(reduced motion: static spinner)'}
      </p>
    </div>
  );
};

/**
 * Main Examples Component
 * Demonstrates all motion patterns together
 */
export const MotionDesignExamples = () => {
  const prefersReduced = useReducedMotion();

  return (
    <div className="space-y-12 max-w-2xl mx-auto p-8">
      <div>
        <h1 className="text-4xl font-bold mb-2">Motion Design Examples</h1>
        <p className="text-gray-600 mb-4">
          {prefersReduced
            ? '✓ Reduced motion is enabled. All animations use instant transitions.'
            : 'Full motion design is enabled.'}
        </p>
        <p className="text-sm text-gray-500">
          To test reduced motion on your OS:
        </p>
        <ul className="text-sm text-gray-500 list-disc list-inside space-y-1">
          <li>macOS: System Preferences {'>'} Accessibility {'>'} Display {'>'} Reduce motion</li>
          <li>Windows: Settings {'>'} Ease of Access {'>'} Display {'>'} Show animations</li>
          <li>iOS: Settings {'>'} Accessibility {'>'} Motion {'>'} Reduce Motion</li>
        </ul>
      </div>

      <div className="border-t pt-8">
        <PageTransitionExample />
      </div>

      <div className="border-t pt-8">
        <h2 className="text-2xl font-bold mb-4">Components</h2>
        <div className="grid gap-8">
          <CardHoverExample />
          <ButtonInteractionExample />
          <ListStaggerExample />
          <ModalExample />
          <DropdownExample />
          <ToastExample />
          <SpinnerExample />
        </div>
      </div>

      <div className="border-t pt-8 pb-8">
        <h2 className="text-2xl font-bold mb-4">Resources</h2>
        <ul className="space-y-2 text-sm">
          <li>
            <a href="/design/motion-spec.md" className="text-primary hover:underline">
              📄 Full Motion Specification
            </a>
          </li>
          <li>
            <a href="/design/IMPLEMENTATION_GUIDE.md" className="text-primary hover:underline">
              📚 Implementation Guide
            </a>
          </li>
          <li>
            <a href="/design-tokens.json" className="text-primary hover:underline">
              🎨 Design Tokens (JSON)
            </a>
          </li>
        </ul>
      </div>
    </div>
  );
};
