import { describe, it, expect } from 'vitest';
import {
  parseTimestamp,
  formatRelativeTime,
  formatLocalDate,
  formatFullLocalDateTime,
  formatUTCTimestamp,
  getUpdateInterval,
  getFormattedTimestamp,
  RECENCY_THRESHOLD_MS,
} from '../timestamp';

describe('timestamp utils', () => {
  const mockNow = new Date('2026-07-26T20:30:00.000Z');

  describe('parseTimestamp', () => {
    it('returns Date for valid ISO string', () => {
      const parsed = parseTimestamp('2026-07-26T12:00:00Z');
      expect(parsed).toBeInstanceOf(Date);
      expect(parsed?.toISOString()).toBe('2026-07-26T12:00:00.000Z');
    });

    it('returns Date for numeric epoch ms', () => {
      const epoch = mockNow.getTime();
      const parsed = parseTimestamp(epoch);
      expect(parsed?.getTime()).toBe(epoch);
    });

    it('returns Date for numeric string', () => {
      const epoch = mockNow.getTime();
      const parsed = parseTimestamp(String(epoch));
      expect(parsed?.getTime()).toBe(epoch);
    });

    it('returns null for empty or invalid input', () => {
      expect(parseTimestamp(null)).toBeNull();
      expect(parseTimestamp(undefined)).toBeNull();
      expect(parseTimestamp('')).toBeNull();
      expect(parseTimestamp('invalid-date')).toBeNull();
    });
  });

  describe('formatRelativeTime', () => {
    it('returns "Just now" for events less than 45 seconds old', () => {
      const date = new Date(mockNow.getTime() - 20 * 1000);
      expect(formatRelativeTime(date, mockNow)).toBe('Just now');
    });

    it('returns minutes ago for events less than 1 hour old', () => {
      const date = new Date(mockNow.getTime() - 15 * 60 * 1000);
      expect(formatRelativeTime(date, mockNow)).toBe('15m ago');
    });

    it('returns hours ago for events less than 24 hours old', () => {
      const date = new Date(mockNow.getTime() - 4 * 60 * 60 * 1000);
      expect(formatRelativeTime(date, mockNow)).toBe('4h ago');
    });

    it('returns "Yesterday" for events 1 day old', () => {
      const date = new Date(mockNow.getTime() - 24 * 60 * 60 * 1000);
      expect(formatRelativeTime(date, mockNow)).toBe('Yesterday');
    });

    it('returns days ago for events under 7 days old', () => {
      const date = new Date(mockNow.getTime() - 5 * 24 * 60 * 60 * 1000);
      expect(formatRelativeTime(date, mockNow)).toBe('5d ago');
    });

    it('falls back to local date for events 7 days old or older', () => {
      const date = new Date(mockNow.getTime() - 8 * 24 * 60 * 60 * 1000);
      const expected = formatLocalDate(date, mockNow);
      expect(formatRelativeTime(date, mockNow)).toBe(expected);
    });
  });

  describe('formatUTCTimestamp', () => {
    it('formats exact UTC string', () => {
      const date = new Date('2026-07-26T20:31:25.000Z');
      expect(formatUTCTimestamp(date)).toBe('2026-07-26 20:31:25 UTC');
    });
  });

  describe('formatFullLocalDateTime', () => {
    it('includes full local date, time, and timezone offset', () => {
      const date = new Date('2026-07-26T20:31:25.000Z');
      const formatted = formatFullLocalDateTime(date);
      expect(formatted).toContain('GMT');
      expect(formatted).toContain('2026');
    });
  });

  describe('getUpdateInterval', () => {
    it('returns 10s for events < 1 minute old', () => {
      const date = new Date(mockNow.getTime() - 30 * 1000);
      expect(getUpdateInterval(date, mockNow)).toBe(10 * 1000);
    });

    it('returns 1m for events < 1 hour old', () => {
      const date = new Date(mockNow.getTime() - 10 * 60 * 1000);
      expect(getUpdateInterval(date, mockNow)).toBe(60 * 1000);
    });

    it('returns 5m for events < 24 hours old', () => {
      const date = new Date(mockNow.getTime() - 5 * 60 * 60 * 1000);
      expect(getUpdateInterval(date, mockNow)).toBe(5 * 60 * 1000);
    });

    it('returns 1h for events < 7 days old', () => {
      const date = new Date(mockNow.getTime() - 3 * 24 * 60 * 60 * 1000);
      expect(getUpdateInterval(date, mockNow)).toBe(60 * 60 * 1000);
    });

    it('returns null for events >= 7 days old', () => {
      const date = new Date(mockNow.getTime() - 8 * 24 * 60 * 60 * 1000);
      expect(getUpdateInterval(date, mockNow)).toBeNull();
    });
  });

  describe('getFormattedTimestamp', () => {
    it('returns fallback formatting when timestamp is unparseable', () => {
      const formatted = getFormattedTimestamp('invalid-date', '3h ago', mockNow);
      expect(formatted.display).toBe('3h ago');
      expect(formatted.isRelative).toBe(true);
      expect(formatted.updateIntervalMs).toBeNull();
    });

    it('formats recent event relative', () => {
      const dateStr = new Date(mockNow.getTime() - 3 * 60 * 60 * 1000).toISOString();
      const formatted = getFormattedTimestamp(dateStr, 'Recently', mockNow);
      expect(formatted.display).toBe('3h ago');
      expect(formatted.isRelative).toBe(true);
      expect(formatted.utcFull).toContain('UTC');
    });

    it('formats older event as absolute local date', () => {
      const dateStr = new Date(mockNow.getTime() - 10 * 24 * 60 * 60 * 1000).toISOString();
      const formatted = getFormattedTimestamp(dateStr, 'Recently', mockNow);
      expect(formatted.isRelative).toBe(false);
      expect(formatted.updateIntervalMs).toBeNull();
    });
  });
});
