/**
 * @file Provider that owns the onboarding tour state and renders <Joyride>.
 *
 * The tour runs in Joyride v3 *controlled* mode: this component owns `run` and
 * `stepIndex` and reacts to `onEvent`. Because every step targets persistent
 * chrome (sidebar / top bar), the spotlight anchor always exists, and the
 * provider additionally navigates the dashboard to each step's `data.page` so
 * the walkthrough mirrors the real product flow.
 *
 * Robustness:
 * - `EVENTS.TARGET_NOT_FOUND` advances past a missing anchor instead of stalling.
 * - `overlayClickAction: false` prevents accidental dismissal on backdrop click.
 * - All step-index math is clamped to a valid range.
 */

import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { Joyride, ACTIONS, EVENTS, STATUS, type EventHandler } from 'react-joyride';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { useReducedMotion } from '../../../shared/hooks/useReducedMotion';
import {
  OnboardingTourContext,
  type OnboardingTourContextValue,
} from './OnboardingTourContext';
import { tourSteps, type TourPage } from './steps';
import { getTourTheme } from './tourStyles';
import { TourTooltip } from './TourTooltip';
import { clearTourStatus, hasSeenTour, setTourStatus } from './storage';
import './onboarding-tour.css';

const LAST_INDEX = tourSteps.length - 1;
const clampIndex = (i: number): number => Math.max(0, Math.min(LAST_INDEX, i));

export interface OnboardingTourProviderProps {
  children: ReactNode;
  /** Navigate the dashboard to a page (wired to `Dashboard.tsx`'s `setCurrentPage`). */
  onNavigate?: (page: TourPage) => void;
  /** Auto-start the tour for first-time users. @default true */
  autoStart?: boolean;
  /** Delay (ms) before first-run auto-start, letting the layout settle. @default 900 */
  autoStartDelay?: number;
}

export function OnboardingTourProvider({
  children,
  onNavigate,
  autoStart = true,
  autoStartDelay = 900,
}: OnboardingTourProviderProps) {
  const { theme } = useTheme();
  const reducedMotion = useReducedMotion();

  const [run, setRun] = useState(false);
  const [stepIndex, setStepIndex] = useState(0);
  const [hasSeen, setHasSeen] = useState<boolean>(() => hasSeenTour());

  const { options, styles, locale } = useMemo(
    () => getTourTheme(theme, reducedMotion),
    [theme, reducedMotion],
  );

  const stopTour = useCallback((completed = false) => {
    setRun(false);
    setStepIndex(0);
    setTourStatus(completed ? 'completed' : 'dismissed');
    setHasSeen(true);
  }, []);

  const startTour = useCallback(() => {
    onNavigate?.(tourSteps[0].data.page);
    setStepIndex(0);
    setRun(true);
  }, [onNavigate]);

  const restartTour = useCallback(() => {
    clearTourStatus();
    setHasSeen(false);
    onNavigate?.(tourSteps[0].data.page);
    setStepIndex(0);
    setRun(true);
  }, [onNavigate]);

  // First-run auto-start — only when the tour has never been seen.
  useEffect(() => {
    if (!autoStart || hasSeenTour()) return;
    const timer = window.setTimeout(() => {
      onNavigate?.(tourSteps[0].data.page);
      setStepIndex(0);
      setRun(true);
    }, autoStartDelay);
    return () => window.clearTimeout(timer);
  }, [autoStart, autoStartDelay, onNavigate]);

  // Keep the visible dashboard page in sync with the active step.
  useEffect(() => {
    if (!run) return;
    const page = tourSteps[stepIndex]?.data.page;
    if (page) onNavigate?.(page);
  }, [run, stepIndex, onNavigate]);

  const handleEvent: EventHandler = useCallback(
    (data) => {
      const { action, index, status, type } = data;

      // Terminal statuses (covers paths where Joyride emits them directly).
      if (status === STATUS.FINISHED) {
        stopTour(true);
        return;
      }
      if (status === STATUS.SKIPPED) {
        stopTour(false);
        return;
      }

      const isLastStep = index >= LAST_INDEX;

      if (type === EVENTS.STEP_AFTER) {
        if (action === ACTIONS.CLOSE) {
          // ✕ / ESC → dismiss the tour (records "dismissed").
          stopTour(false);
          return;
        }
        if (action === ACTIONS.PREV) {
          setStepIndex(clampIndex(index - 1));
          return;
        }
        // Forward (Next / Finish). In controlled mode Joyride does NOT emit a
        // FINISHED status on its own, so completing the final step must be
        // detected here — otherwise the overlay stays open on the last step.
        if (isLastStep) {
          stopTour(true);
          return;
        }
        setStepIndex(clampIndex(index + 1));
        return;
      }

      if (type === EVENTS.TARGET_NOT_FOUND) {
        // Never stall on a missing anchor — advance, or finish if it was last.
        if (isLastStep) {
          stopTour(true);
          return;
        }
        setStepIndex(clampIndex(index + 1));
      }
    },
    [stopTour],
  );

  const value = useMemo<OnboardingTourContextValue>(
    () => ({ isActive: run, hasSeen, startTour, restartTour, stopTour }),
    [run, hasSeen, startTour, restartTour, stopTour],
  );

  return (
    <OnboardingTourContext.Provider value={value}>
      {children}
      <Joyride
        steps={tourSteps}
        run={run}
        stepIndex={stepIndex}
        continuous
        scrollToFirstStep
        onEvent={handleEvent}
        tooltipComponent={TourTooltip}
        options={options}
        styles={styles}
        locale={locale}
      />
    </OnboardingTourContext.Provider>
  );
}
