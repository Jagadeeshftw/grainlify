/**
 * @file Hook for reading and controlling the onboarding tour.
 *
 * @example
 * const { restartTour, hasSeen } = useOnboardingTour();
 * <button onClick={restartTour}>{hasSeen ? 'Restart' : 'Start'} tutorial</button>
 */

import { useContext } from 'react';
import { OnboardingTourContext, type OnboardingTourContextValue } from './OnboardingTourContext';

export function useOnboardingTour(): OnboardingTourContextValue {
  return useContext(OnboardingTourContext);
}
