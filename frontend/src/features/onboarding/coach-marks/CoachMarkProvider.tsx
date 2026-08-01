/**
 * @file Provider that manages the coach mark queue and renders the active coach mark.
 *
 * Only one coach mark is visible at a time. When dismissed, the next queued
 * coach mark appears after a 300ms delay. Escape key dismisses the active
 * coach mark.
 */

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { CoachMarkContext, type CoachMarkConfig } from './CoachMarkContext';
import { CoachMarkTooltip } from './CoachMarkTooltip';
import { dismissCoachMark, hasDismissedCoachMark } from './storage';
import './coach-marks.css';

const SHOW_DELAY_MS = 300;

export interface CoachMarkProviderProps {
  children: ReactNode;
}

interface QueuedCoachMark extends CoachMarkConfig {
  /** Position of the target element for absolute positioning. */
  targetRect?: DOMRect;
}

export function CoachMarkProvider({ children }: CoachMarkProviderProps) {
  const [queue, setQueue] = useState<QueuedCoachMark[]>([]);
  const [active, setActive] = useState<QueuedCoachMark | null>(null);
  const dismissTimeoutRef = useRef<ReturnType<typeof setTimeout>>();

  const computeTargetRect = useCallback((selector: string): DOMRect | undefined => {
    const el = document.querySelector(selector);
    return el?.getBoundingClientRect();
  }, []);

  const showNext = useCallback(
    (currentQueue: QueuedCoachMark[]) => {
      const next = currentQueue[0];
      if (!next) {
        setActive(null);
        return;
      }
      const rect = computeTargetRect(next.targetSelector);
      setActive({ ...next, targetRect: rect });
    },
    [computeTargetRect],
  );

  const register = useCallback(
    (config: CoachMarkConfig) => {
      if (hasDismissedCoachMark(config.featureId)) return;

      setQueue((prev) => {
        if (prev.some((q) => q.featureId === config.featureId)) return prev;
        const next = [...prev, config];
        // If nothing is active, schedule showing the first item
        if (!active) {
          clearTimeout(dismissTimeoutRef.current);
          dismissTimeoutRef.current = setTimeout(() => {
            showNext(next);
          }, SHOW_DELAY_MS);
        }
        return next;
      });
    },
    [active, showNext],
  );

  const unregister = useCallback((featureId: string) => {
    setQueue((prev) => prev.filter((q) => q.featureId !== featureId));
  }, []);

  const dismiss = useCallback(
    (featureId: string) => {
      dismissCoachMark(featureId);

      setQueue((prev) => {
        const next = prev.filter((q) => q.featureId !== featureId);
        // Show the next queued coach mark after a delay
        clearTimeout(dismissTimeoutRef.current);
        dismissTimeoutRef.current = setTimeout(() => {
          showNext(next);
        }, SHOW_DELAY_MS);
        return next;
      });

      setActive(null);
    },
    [showNext],
  );

  // Escape key handler
  useEffect(() => {
    if (!active) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        dismiss(active.featureId);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [active, dismiss]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => clearTimeout(dismissTimeoutRef.current);
  }, []);

  const contextValue = useMemo(
    () => ({ register, unregister, dismiss }),
    [register, unregister, dismiss],
  );

  // Compute tooltip position based on target rect
  const tooltipPosition = useMemo(() => {
    if (!active?.targetRect) return { top: '50%', left: '50%' };

    const rect = active.targetRect;
    const placement = active.placement || 'auto';

    // Auto placement: pick the direction with most space
    let finalPlacement = placement;
    if (placement === 'auto') {
      const spaceTop = rect.top;
      const spaceBottom = window.innerHeight - rect.bottom;
      const spaceLeft = rect.left;
      const spaceRight = window.innerWidth - rect.right;

      const maxSpace = Math.max(spaceTop, spaceBottom, spaceLeft, spaceRight);
      if (maxSpace === spaceTop) finalPlacement = 'top';
      else if (maxSpace === spaceBottom) finalPlacement = 'bottom';
      else if (maxSpace === spaceLeft) finalPlacement = 'left';
      else finalPlacement = 'right';
    }

    const OFFSET = 12;
    switch (finalPlacement) {
      case 'top':
        return {
          top: `${rect.top - OFFSET}px`,
          left: `${rect.left + rect.width / 2}px`,
          transform: 'translate(-50%, -100%)',
        };
      case 'bottom':
        return {
          top: `${rect.bottom + OFFSET}px`,
          left: `${rect.left + rect.width / 2}px`,
          transform: 'translate(-50%, 0)',
        };
      case 'left':
        return {
          top: `${rect.top + rect.height / 2}px`,
          left: `${rect.left - OFFSET}px`,
          transform: 'translate(-100%, -50%)',
        };
      case 'right':
        return {
          top: `${rect.top + rect.height / 2}px`,
          left: `${rect.right + OFFSET}px`,
          transform: 'translate(0, -50%)',
        };
      default:
        return {
          top: `${rect.bottom + OFFSET}px`,
          left: `${rect.left + rect.width / 2}px`,
          transform: 'translate(-50%, 0)',
        };
    }
  }, [active]);

  const tooltipPlacement = useMemo(() => {
    if (!active?.targetRect) return 'bottom';
    const rect = active.targetRect;
    const placement = active.placement || 'auto';
    if (placement !== 'auto') return placement;
    const spaceTop = rect.top;
    const spaceBottom = window.innerHeight - rect.bottom;
    const spaceLeft = rect.left;
    const spaceRight = window.innerWidth - rect.right;
    const maxSpace = Math.max(spaceTop, spaceBottom, spaceLeft, spaceRight);
    if (maxSpace === spaceTop) return 'top';
    if (maxSpace === spaceBottom) return 'bottom';
    if (maxSpace === spaceLeft) return 'left';
    return 'right';
  }, [active]);

  return (
    <CoachMarkContext.Provider value={contextValue}>
      {children}

      {/* Active coach mark overlay */}
      {active && active.targetRect && (
        <>
          {/* Semi-transparent backdrop behind the target */}
          <div
            className="fixed inset-0 z-40 bg-black/30 transition-opacity animate-coach-mark-backdrop-in"
            aria-hidden="true"
            onClick={() => dismiss(active.featureId)}
          />

          {/* Highlight ring around the target */}
          <div
            className="fixed z-50 pointer-events-none animate-coach-mark-ring-in"
            style={{
              top: `${active.targetRect.top - 4}px`,
              left: `${active.targetRect.left - 4}px`,
              width: `${active.targetRect.width + 8}px`,
              height: `${active.targetRect.height + 8}px`,
              borderRadius: '12px',
              border: '2px solid',
              borderColor: document.documentElement.classList.contains('dark')
                ? '#c9983a'
                : '#f1b400',
              boxShadow: '0 0 0 4px rgba(201,152,58,0.15)',
            }}
            aria-hidden="true"
          />

          {/* Tooltip bubble */}
          <div
            className="fixed z-50"
            style={{
              ...tooltipPosition,
              position: 'fixed',
            }}
          >
            <CoachMarkTooltip
              title={active.title}
              body={active.body}
              placement={tooltipPlacement}
              onDismiss={() => dismiss(active.featureId)}
            />
          </div>
        </>
      )}
    </CoachMarkContext.Provider>
  );
}
