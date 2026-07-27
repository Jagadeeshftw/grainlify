/**
 * @file Persistence for the onboarding tour's completion state.
 *
 * @security / @privacy
 * - A single short enum string (`'completed' | 'dismissed'`) is written under
 *   one namespaced key. No PII, auth tokens, or user-controlled input is ever
 *   stored, so the value cannot become an injection / XSS vector when read.
 * - Reads are validated against the known enum before use; any unexpected
 *   value is treated as "never seen" rather than trusted.
 * - All access is wrapped in try/catch so Safari Private Mode, disabled
 *   storage, and quota errors degrade gracefully (tour simply re-offers next
 *   session) instead of throwing and breaking the dashboard shell.
 */

/** Versioned key — bump the suffix to re-show the tour after a redesign. */
const STORAGE_KEY = 'grainlify.onboarding.tour.v1';

export type TourStatus = 'completed' | 'dismissed';

/**
 * Read the persisted tour status.
 * @returns `'completed'`, `'dismissed'`, or `null` when never seen / unreadable.
 */
export function getTourStatus(): TourStatus | null {
  try {
    const value = window.localStorage.getItem(STORAGE_KEY);
    return value === 'completed' || value === 'dismissed' ? value : null;
  } catch {
    return null;
  }
}

/** Persist the tour status. Failures are non-fatal and intentionally ignored. */
export function setTourStatus(status: TourStatus): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, status);
  } catch {
    /* storage unavailable — tour will simply re-offer next session */
  }
}

/** Remove the persisted status so the tour is treated as never seen. */
export function clearTourStatus(): void {
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* non-fatal */
  }
}

/** Convenience predicate: has the user completed or dismissed the tour before? */
export function hasSeenTour(): boolean {
  return getTourStatus() !== null;
}
