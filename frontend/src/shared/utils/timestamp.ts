/**
 * Timezone-aware timestamp utilities for Grainlify.
 * Supports relative display for recent events (< 7 days),
 * absolute local date for older events (>= 7 days),
 * full local + UTC offset formatting, and dynamic live update intervals.
 */

export const RECENCY_THRESHOLD_MS = 7 * 24 * 60 * 60 * 1000; // 7 days in ms

export interface FormattedTimestamp {
  /** Primary string for UI display (relative e.g. "3h ago" or short local date e.g. "Jun 14") */
  display: string;
  /** True if the primary display is a relative duration */
  isRelative: boolean;
  /** ISO string representation for standard html <time dateTime="..."> attribute */
  isoString: string;
  /** Full local date-time string including timezone offset e.g. "Jul 26, 2026, 8:31:25 PM GMT+1" */
  localFull: string;
  /** Absolute UTC timestamp string e.g. "2026-07-26 19:31:25 UTC" */
  utcFull: string;
  /** Suggested live re-render interval in milliseconds, or null if event is older than 7 days */
  updateIntervalMs: number | null;
}

/**
 * Safely parses string, Date, or number timestamp into a valid Date object or null.
 */
export function parseTimestamp(input?: string | Date | number | null): Date | null {
  if (input === null || input === undefined || input === '') {
    return null;
  }
  if (input instanceof Date) {
    return isNaN(input.getTime()) ? null : input;
  }
  if (typeof input === 'number') {
    const d = new Date(input);
    return isNaN(d.getTime()) ? null : d;
  }
  if (typeof input === 'string') {
    const trimmed = input.trim();
    // Check if numeric string timestamp
    if (/^\d+$/.test(trimmed)) {
      const num = parseInt(trimmed, 10);
      const d = new Date(num);
      if (!isNaN(d.getTime())) return d;
    }
    const d = new Date(trimmed);
    return isNaN(d.getTime()) ? null : d;
  }
  return null;
}

/**
 * Computes the GMT/UTC offset string for a date in the current environment's locale, e.g. "GMT+1" or "GMT-05:00".
 */
export function getTimezoneOffsetString(date: Date): string {
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes >= 0 ? '+' : '-';
  const absMins = Math.abs(offsetMinutes);
  const hours = Math.floor(absMins / 60);
  const mins = absMins % 60;

  if (mins === 0) {
    return `GMT${sign}${hours}`;
  }
  const paddedMins = mins < 10 ? `0${mins}` : `${mins}`;
  return `GMT${sign}${hours}:${paddedMins}`;
}

/**
 * Formats a Date object into a readable primary local date string.
 * e.g. "Jun 14" if same year, "Jun 14, 2024" if different year.
 */
export function formatLocalDate(date: Date, now: Date = new Date()): string {
  const isSameYear = date.getFullYear() === now.getFullYear();
  const options: Intl.DateTimeFormatOptions = {
    month: 'short',
    day: 'numeric',
    ...(isSameYear ? {} : { year: 'numeric' }),
  };
  return date.toLocaleDateString(undefined, options);
}

/**
 * Formats relative time for events within recency threshold (< 7 days).
 */
export function formatRelativeTime(date: Date, now: Date = new Date()): string {
  const diffMs = Math.max(0, now.getTime() - date.getTime());
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffSecs < 45 || diffMins < 1) {
    return 'Just now';
  }
  if (diffMins < 60) {
    return `${diffMins}m ago`;
  }
  if (diffHours < 24) {
    return `${diffHours}h ago`;
  }
  if (diffDays === 1) {
    return 'Yesterday';
  }
  if (diffDays < 7) {
    return `${diffDays}d ago`;
  }
  return formatLocalDate(date, now);
}

/**
 * Formats full local date-time with explicit timezone offset for disambiguation.
 * e.g. "Jul 26, 2026, 8:31:25 PM (GMT+1)"
 */
export function formatFullLocalDateTime(date: Date): string {
  const dateStr = date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
  const timeStr = date.toLocaleTimeString(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  });
  const tzOffset = getTimezoneOffsetString(date);
  return `${dateStr}, ${timeStr} (${tzOffset})`;
}

/**
 * Formats full UTC timestamp for precision.
 * e.g. "2026-07-26 19:31:25 UTC"
 */
export function formatUTCTimestamp(date: Date): string {
  const pad = (n: number) => (n < 10 ? `0${n}` : `${n}`);
  const year = date.getUTCFullYear();
  const month = pad(date.getUTCMonth() + 1);
  const day = pad(date.getUTCDate());
  const hours = pad(date.getUTCHours());
  const mins = pad(date.getUTCMinutes());
  const secs = pad(date.getUTCSeconds());

  return `${year}-${month}-${day} ${hours}:${mins}:${secs} UTC`;
}

/**
 * Determines the next refresh interval (in ms) for dynamic live updating:
 * - Age < 1 min: 10 seconds (10,000 ms)
 * - Age < 1 hour: 60 seconds (60,000 ms)
 * - Age < 24 hours: 5 minutes (300,000 ms)
 * - Age < 7 days: 1 hour (3,600,000 ms)
 * - Age >= 7 days: null (static date)
 */
export function getUpdateInterval(date: Date, now: Date = new Date()): number | null {
  const diffMs = Math.abs(now.getTime() - date.getTime());

  if (diffMs >= RECENCY_THRESHOLD_MS) {
    return null; // Event is static, no timer needed
  }
  if (diffMs < 60 * 1000) {
    return 10 * 1000; // 10s
  }
  if (diffMs < 60 * 60 * 1000) {
    return 60 * 1000; // 1m
  }
  if (diffMs < 24 * 60 * 60 * 1000) {
    return 5 * 60 * 1000; // 5m
  }
  return 60 * 60 * 1000; // 1h
}

/**
 * Formats a complete FormattedTimestamp object for UI rendering.
 */
export function getFormattedTimestamp(
  rawInput?: string | Date | number | null,
  fallbackText: string = 'Recently',
  now: Date = new Date()
): FormattedTimestamp {
  const parsedDate = parseTimestamp(rawInput);

  if (!parsedDate) {
    return {
      display: fallbackText,
      isRelative: true,
      isoString: '',
      localFull: fallbackText,
      utcFull: fallbackText,
      updateIntervalMs: null,
    };
  }

  const diffMs = Math.abs(now.getTime() - parsedDate.getTime());
  const isWithinThreshold = diffMs < RECENCY_THRESHOLD_MS;

  const display = isWithinThreshold
    ? formatRelativeTime(parsedDate, now)
    : formatLocalDate(parsedDate, now);

  return {
    display,
    isRelative: isWithinThreshold,
    isoString: parsedDate.toISOString(),
    localFull: formatFullLocalDateTime(parsedDate),
    utcFull: formatUTCTimestamp(parsedDate),
    updateIntervalMs: getUpdateInterval(parsedDate, now),
  };
}
