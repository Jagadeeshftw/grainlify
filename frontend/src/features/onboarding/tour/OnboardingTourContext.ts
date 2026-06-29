/**
 * @file React context for controlling the onboarding tour from anywhere in the
 * dashboard tree (e.g. the "Restart tutorial" button in Settings).
 *
 * The default value is a safe no-op so a consumer rendered outside the provider
 * never throws — it simply can't start a tour. Within the dashboard shell
 * (`Dashboard.tsx`) the real provider is always present.
 */

import { createContext } from 'react';

export interface OnboardingTourContextValue {
  /** Whether a tour is currently running. */
  isActive: boolean;
  /** Whether the user has completed or dismissed the tour before. */
  hasSeen: boolean;
  /** Start the tour from the first step (used for first-run auto-start). */
  startTour: () => void;
  /** Clear persisted state and replay the tour from the first step. */
  restartTour: () => void;
  /** Stop the tour. `completed` records whether the user reached the end. */
  stopTour: (completed?: boolean) => void;
}

const noop = (): void => {};

export const defaultOnboardingTourValue: OnboardingTourContextValue = {
  isActive: false,
  hasSeen: false,
  startTour: noop,
  restartTour: noop,
  stopTour: noop,
};

export const OnboardingTourContext = createContext<OnboardingTourContextValue>(
  defaultOnboardingTourValue,
);
