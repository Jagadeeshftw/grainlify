import { describe, it, expect } from 'vitest';
import { classifyOAuthError } from '../oauthErrors';

describe('classifyOAuthError', () => {
  // ── denied-scopes ──────────────────────────────────────────────
  describe('denied-scopes', () => {
    it('classifies "access_denied" as denied-scopes', () => {
      const result = classifyOAuthError('access_denied');
      expect(result.code).toBe('denied-scopes');
      expect(result.icon).toBe('ShieldOff');
      expect(result.primaryCta).toBeTruthy();
    });

    it('classifies "User denied scopes" as denied-scopes', () => {
      const result = classifyOAuthError('User denied scopes');
      expect(result.code).toBe('denied-scopes');
    });

    it('classifies "cancelled" as denied-scopes', () => {
      const result = classifyOAuthError('cancelled');
      expect(result.code).toBe('denied-scopes');
    });

    it('classifies "canceled" (US spelling) as denied-scopes', () => {
      const result = classifyOAuthError('canceled');
      expect(result.code).toBe('denied-scopes');
    });
  });

  // ── rate-limited ───────────────────────────────────────────────
  describe('rate-limited', () => {
    it('classifies "rate limit exceeded" as rate-limited', () => {
      const result = classifyOAuthError('rate limit exceeded');
      expect(result.code).toBe('rate-limited');
      expect(result.icon).toBe('Clock');
      expect(result.retryAfterSeconds).toBe(60); // default
    });

    it('classifies "429" as rate-limited', () => {
      const result = classifyOAuthError('429');
      expect(result.code).toBe('rate-limited');
    });

    it('classifies "too many requests" as rate-limited', () => {
      const result = classifyOAuthError('too many requests');
      expect(result.code).toBe('rate-limited');
    });

    it('uses retryAfterHeader when provided', () => {
      const result = classifyOAuthError('rate limit', 120);
      expect(result.code).toBe('rate-limited');
      expect(result.retryAfterSeconds).toBe(120);
    });
  });

  // ── network-failure ────────────────────────────────────────────
  describe('network-failure', () => {
    it('classifies "network error" as network-failure', () => {
      const result = classifyOAuthError('network error');
      expect(result.code).toBe('network-failure');
      expect(result.icon).toBe('WifiOff');
    });

    it('classifies "timeout" as network-failure', () => {
      const result = classifyOAuthError('timeout');
      expect(result.code).toBe('network-failure');
    });

    it('classifies "Failed to fetch" as network-failure', () => {
      const result = classifyOAuthError('Failed to fetch');
      expect(result.code).toBe('network-failure');
    });

    it('classifies Error objects with network message', () => {
      const err = new Error('NetworkError when attempting to fetch resource');
      const result = classifyOAuthError(err);
      expect(result.code).toBe('network-failure');
    });
  });

  // ── unknown-error ──────────────────────────────────────────────
  describe('unknown-error', () => {
    it('classifies unrecognised string as unknown-error', () => {
      const result = classifyOAuthError('something completely different');
      expect(result.code).toBe('unknown-error');
      expect(result.icon).toBe('AlertCircle');
    });

    it('classifies null as unknown-error', () => {
      const result = classifyOAuthError(null);
      expect(result.code).toBe('unknown-error');
    });

    it('classifies undefined as unknown-error', () => {
      const result = classifyOAuthError(undefined);
      expect(result.code).toBe('unknown-error');
    });

    it('classifies empty string as unknown-error', () => {
      const result = classifyOAuthError('');
      expect(result.code).toBe('unknown-error');
    });
  });

  // ── structural guarantees ──────────────────────────────────────
  describe('structural guarantees', () => {
    const codes = ['access_denied', 'rate limit', 'network error', 'xyz'] as const;

    it.each(codes)('always returns heading, description, primaryCta for "%s"', (input) => {
      const result = classifyOAuthError(input);
      expect(result.heading).toBeTruthy();
      expect(result.description).toBeTruthy();
      expect(result.primaryCta).toBeTruthy();
      expect(result.icon).toBeTruthy();
    });
  });
});
