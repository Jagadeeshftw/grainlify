import { createContext, useContext } from 'react';

export interface CoachMarkConfig {
  /** Unique feature identifier (e.g. 'browse-advanced-filters'). */
  featureId: string;
  /** Short title displayed in the bubble. */
  title: string;
  /** Descriptive body text. */
  body: string;
  /** CSS selector for the target element to highlight. */
  targetSelector: string;
  /** Placement hint relative to target. @default 'auto' */
  placement?: 'top' | 'right' | 'bottom' | 'left' | 'auto';
}

export interface CoachMarkContextValue {
  /** Register a coach mark. Returns immediately if already dismissed. */
  register: (config: CoachMarkConfig) => void;
  /** Unregister a coach mark (e.g. on component unmount). */
  unregister: (featureId: string) => void;
  /** Dismiss a coach mark permanently and show the next queued one. */
  dismiss: (featureId: string) => void;
}

export const CoachMarkContext = createContext<CoachMarkContextValue>({
  register: () => {},
  unregister: () => {},
  dismiss: () => {},
});

export function useCoachMarkContext(): CoachMarkContextValue {
  return useContext(CoachMarkContext);
}
