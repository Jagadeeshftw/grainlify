import { useMediaQuery } from './useMediaQuery';

export const useReducedMotion = (): boolean => {
  return useMediaQuery('(prefers-reduced-motion: reduce)');
};

export const usePrefersDarkMode = (): boolean => {
  return useMediaQuery('(prefers-color-scheme: dark)');
};

export type Breakpoint = 'sm' | 'md' | 'lg' | 'xl';

export interface BreakpointState {
  isMobile: boolean;
  isTablet: boolean;
  isDesktop: boolean;
  breakpoint: Breakpoint;
}

export const useResponsiveBreakpoint = (): BreakpointState => {
  const isMobile = useMediaQuery('(max-width: 767px)');
  const isTablet = useMediaQuery('(min-width: 768px) and (max-width: 1023px)');
  const isDesktop = useMediaQuery('(min-width: 1024px)');

  let breakpoint: Breakpoint;
  if (isDesktop) {
    breakpoint = 'lg';
  } else if (isTablet) {
    breakpoint = 'md';
  } else {
    breakpoint = 'sm';
  }

  return { isMobile, isTablet, isDesktop, breakpoint };
};
