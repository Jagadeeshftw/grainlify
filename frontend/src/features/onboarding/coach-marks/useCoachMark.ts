import { useCallback, useEffect, useRef, useState } from 'react';
import { useCoachMarkContext, type CoachMarkConfig } from './CoachMarkContext';

/**
 * Hook for surfaces to register a coach mark for a specific feature.
 *
 * Usage:
 * ```tsx
 * useCoachMark({
 *   featureId: 'browse-advanced-filters',
 *   title: 'Advanced Filters',
 *   body: 'Filter by language, ecosystem and more to narrow your search.',
 *   targetSelector: '[data-coach="filter-fab"]',
 * });
 * ```
 */
export function useCoachMark(config: CoachMarkConfig): void {
  const { register, unregister } = useCoachMarkContext();
  const registeredRef = useRef(false);

  useEffect(() => {
    if (!registeredRef.current) {
      register(config);
      registeredRef.current = true;
    }

    return () => {
      unregister(config.featureId);
      registeredRef.current = false;
    };
  }, [config.featureId]); // eslint-disable-line react-hooks/exhaustive-deps
}
