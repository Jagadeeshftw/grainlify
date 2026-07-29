import { useMemo } from 'react';
import { useResponsiveBreakpoint, type Breakpoint } from './useReducedMotion';

export type ResponsiveTokenMap<T> = Partial<Record<Breakpoint, T>>;

const BREAKPOINT_ORDER: Breakpoint[] = ['sm', 'md', 'lg', 'xl'];

export function useResponsiveToken<T>(
  tokenMap: ResponsiveTokenMap<T>,
  defaultValue: T,
): T {
  const { breakpoint } = useResponsiveBreakpoint();

  return useMemo(() => {
    const idx = BREAKPOINT_ORDER.indexOf(breakpoint);
    for (let i = idx; i >= 0; i--) {
      const bp = BREAKPOINT_ORDER[i];
      if (tokenMap[bp] !== undefined) {
        return tokenMap[bp] as T;
      }
    }
    return defaultValue;
  }, [breakpoint, tokenMap, defaultValue]);
}
