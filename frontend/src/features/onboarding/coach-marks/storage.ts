/**
 * @file Per-feature persistence for coach mark dismissal state.
 *
 * Mirrors the conventions in tour/storage.ts:
 * - Versioned keys per feature (`grainlify.coach-mark.<featureId>.v1`)
 * - Only `'dismissed'` is stored; any other value is treated as unseen
 * - All access wrapped in try/catch for Safari Private Mode / quota errors
 */

const KEY_PREFIX = 'grainlify.coach-mark.';
const KEY_SUFFIX = '.v1';

export type CoachMarkStatus = 'dismissed';

function storageKey(featureId: string): string {
  return `${KEY_PREFIX}${featureId}${KEY_SUFFIX}`;
}

/**
 * Check if a coach mark has been dismissed for a given feature.
 * @returns `true` if the user has previously dismissed this coach mark.
 */
export function hasDismissedCoachMark(featureId: string): boolean {
  try {
    return window.localStorage.getItem(storageKey(featureId)) === 'dismissed';
  } catch {
    return false;
  }
}

/** Mark a coach mark as dismissed. Failures are non-fatal. */
export function dismissCoachMark(featureId: string): void {
  try {
    window.localStorage.setItem(storageKey(featureId), 'dismissed');
  } catch {
    /* storage unavailable — coach mark will simply re-offer next session */
  }
}

/** Remove persisted dismissal so the coach mark is shown again. */
export function clearCoachMark(featureId: string): void {
  try {
    window.localStorage.removeItem(storageKey(featureId));
  } catch {
    /* non-fatal */
  }
}
