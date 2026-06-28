/**
 * @file Public surface of the contributor onboarding feature.
 */

export { OnboardingTourProvider } from './tour/OnboardingTourProvider';
export type { OnboardingTourProviderProps } from './tour/OnboardingTourProvider';
export { useOnboardingTour } from './tour/useOnboardingTour';
export { RestartTutorialButton } from './components/RestartTutorialButton';
export { TOUR_TARGET, tourSelector } from './tour/targets';
export type { TourTargetId } from './tour/targets';
export { tourSteps } from './tour/steps';
export type { TourPage, TourStep, TourStepData } from './tour/steps';
