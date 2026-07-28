/**
 * @file Stable DOM anchors for the contributor onboarding tour.
 *
 * Every value is rendered as a `data-tour="…"` attribute on a *persistent*
 * piece of dashboard chrome — a sidebar nav item or a top-bar control that is
 * mounted on every page. Anchoring the spotlight to chrome (never to async
 * content cards) guarantees the target always exists, so the tour can never
 * crash, stall, or point at an empty region while data is still loading.
 *
 * @security The selectors are static, build-time constants. No user-controlled
 * value is ever interpolated into a `data-tour` attribute or a CSS selector,
 * which keeps the selector path free of injection surface.
 */

/** Canonical `data-tour` attribute values used across the dashboard. */
export const TOUR_TARGET = {
  navDiscover: 'nav-discover',
  navBrowse: 'nav-browse',
  navSettings: 'nav-settings',
  navLeaderboard: 'nav-leaderboard',
  topbarProfile: 'topbar-profile',
} as const;

export type TourTargetId = (typeof TOUR_TARGET)[keyof typeof TOUR_TARGET];

/**
 * Build the attribute selector Joyride uses to locate an anchor element.
 *
 * @param id - One of the {@link TOUR_TARGET} values.
 * @returns A `[data-tour="…"]` CSS selector string.
 */
export const tourSelector = (id: TourTargetId): string => `[data-tour="${id}"]`;
