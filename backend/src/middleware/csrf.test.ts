import { describe, it, expect } from 'vitest';
import { isAllowedRedirectURI, encodeStateWithRedirect, decodeStateWithRedirect, validateCSRFToken, createCsrfMiddleware, RequestWithHeaders, ResponseWithStatus } from './csrf';

describe('CSRF and Browser Security Edge Cases', () => {
  describe('isAllowedRedirectURI', () => {
    it('allows valid localhost origins', () => {
      expect(isAllowedRedirectURI('http://localhost:3000')).toBe(true);
      expect(isAllowedRedirectURI('http://localhost:8080/auth/callback')).toBe(true);
      expect(isAllowedRedirectURI('http://127.0.0.1:5173')).toBe(true);
      expect(isAllowedRedirectURI('https://localhost:443')).toBe(true);
    });

    it('allows vercel.app domains', () => {
      expect(isAllowedRedirectURI('https://my-app.vercel.app')).toBe(true);
      expect(isAllowedRedirectURI('https://preview-123.vercel.app/callback')).toBe(true);
    });

    it('allows origins configured in CORS_ORIGINS or frontendBaseUrl', () => {
      const config = { corsOrigins: 'https://grainlify.io, https://app.grainlify.io', frontendBaseUrl: 'https://dashboard.grainlify.io' };
      expect(isAllowedRedirectURI('https://grainlify.io/auth', config)).toBe(true);
      expect(isAllowedRedirectURI('https://app.grainlify.io', config)).toBe(true);
      expect(isAllowedRedirectURI('https://dashboard.grainlify.io', config)).toBe(true);
    });

    it('normalizes origins configured with trailing slashes', () => {
      const config = { corsOrigins: 'https://grainlify.io/', frontendBaseUrl: 'https://dashboard.grainlify.io/' };
      expect(isAllowedRedirectURI('https://grainlify.io/auth', config)).toBe(true);
      expect(isAllowedRedirectURI('https://dashboard.grainlify.io', config)).toBe(true);
    });

    it('trims whitespace from input redirect URIs', () => {
      expect(isAllowedRedirectURI('  https://my-app.vercel.app  ')).toBe(true);
    });

    it('rejects malicious or disallowed origins (Open Redirect protection)', () => {
      expect(isAllowedRedirectURI('http://localhost.attacker.com')).toBe(false);
      expect(isAllowedRedirectURI('https://vercel.app.evil.com')).toBe(false);
      expect(isAllowedRedirectURI('javascript:alert(1)')).toBe(false);
      expect(isAllowedRedirectURI('data:text/html,<script>alert(1)</script>')).toBe(false);
      expect(isAllowedRedirectURI('https://untrusted-site.com')).toBe(false);
      expect(isAllowedRedirectURI('')).toBe(false);
    });

    it('rejects redirect URIs containing embedded userinfo credentials', () => {
      expect(isAllowedRedirectURI('http://user:password@localhost:3000')).toBe(false);
      expect(isAllowedRedirectURI('https://admin:secret@my-app.vercel.app')).toBe(false);
    });

    it('handles non-string config values gracefully', () => {
      const configWithNonStrings = { corsOrigins: 123 as any, frontendBaseUrl: {} as any };
      // Should not throw, just reject non-configured origins
      expect(isAllowedRedirectURI('https://untrusted.com', configWithNonStrings)).toBe(false);
      // Localhost still allowed regardless of config
      expect(isAllowedRedirectURI('http://localhost:3000', configWithNonStrings)).toBe(true);
    });

    it('handles null/undefined config values gracefully', () => {
      const configWithNulls = { corsOrigins: null as any, frontendBaseUrl: undefined as any };
      expect(isAllowedRedirectURI('https://untrusted.com', configWithNulls)).toBe(false);
      expect(isAllowedRedirectURI('http://localhost:3000', configWithNulls)).toBe(true);
    });
  });

  describe('encodeStateWithRedirect and decodeStateWithRedirect', () => {
    it('encodes and decodes state with CSRF token and redirect URI', () => {
      const token = 'csrf_token_12345';
      const redirect = 'https://my-app.vercel.app';
      const encoded = encodeStateWithRedirect(token, redirect);
      expect(encoded).not.toBe(token);

      const decoded = decodeStateWithRedirect(encoded);
      expect(decoded.csrfToken).toBe(token);
      expect(decoded.redirectURI).toBe(redirect);
    });

    it('handles legacy state format without redirect URI for backward compatibility', () => {
      const token = 'legacy_csrf_token';
      const encoded = encodeStateWithRedirect(token, '');
      expect(encoded).toBe(token);

      const decoded = decodeStateWithRedirect(token);
      expect(decoded.csrfToken).toBe(token);
      expect(decoded.redirectURI).toBe('');
    });

    it('handles redirect URIs containing pipe characters in query params', () => {
      const token = 'token_with_pipes';
      const redirect = 'https://my-app.vercel.app/callback?filter=a|b';
      const encoded = encodeStateWithRedirect(token, redirect);
      const decoded = decodeStateWithRedirect(encoded);
      expect(decoded.csrfToken).toBe(token);
      expect(decoded.redirectURI).toBe(redirect);
    });

    it('handles malformed base64 state gracefully', () => {
      const malformed = '!!!not_valid_base64!!!';
      const decoded = decodeStateWithRedirect(malformed);
      expect(decoded.csrfToken).toBe(malformed);
      expect(decoded.redirectURI).toBe('');
    });

    it('handles empty or non-string state gracefully', () => {
      expect(decodeStateWithRedirect('')).toEqual({ csrfToken: '', redirectURI: '' });
      expect(decodeStateWithRedirect('   ')).toEqual({ csrfToken: '', redirectURI: '' });
      expect(decodeStateWithRedirect(null as any)).toEqual({ csrfToken: '', redirectURI: '' });
      expect(encodeStateWithRedirect(null as any)).toBe('');
    });

    it('handles state with empty CSRF token after pipe separator', () => {
      // Manually construct base64url encoded string with empty token
      const payload = '|https://my-app.vercel.app';
      const encoded = Buffer.from(payload, 'utf-8').toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
      const decoded = decodeStateWithRedirect(encoded);
      // Falls back to legacy format when CSRF token is empty
      expect(decoded.csrfToken).toBe(encoded);
      expect(decoded.redirectURI).toBe('');
    });

    it('handles state with only pipe separator', () => {
      // Manually construct base64url encoded string with only pipe
      const payload = '|';
      const encoded = Buffer.from(payload, 'utf-8').toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
      const decoded = decodeStateWithRedirect(encoded);
      // Falls back to legacy format when CSRF token is empty
      expect(decoded.csrfToken).toBe(encoded);
      expect(decoded.redirectURI).toBe('');
    });
  });

  describe('validateCSRFToken', () => {
    it('validates matching tokens', () => {
      expect(validateCSRFToken('token_1', 'token_1')).toBe(true);
    });

    it('rejects mismatched tokens', () => {
      expect(validateCSRFToken('token_1', 'token_2')).toBe(false);
    });

    it('rejects expired tokens', () => {
      const past = new Date(Date.now() - 10000);
      expect(validateCSRFToken('token_1', 'token_1', past)).toBe(false);
    });

    it('rejects invalid Date objects for expiration', () => {
      const invalidDate = new Date('invalid_date_string');
      expect(validateCSRFToken('token_1', 'token_1', invalidDate)).toBe(false);
    });

    it('rejects non-string or whitespace-only tokens', () => {
      expect(validateCSRFToken(null as any, 'token_1')).toBe(false);
      expect(validateCSRFToken('token_1', undefined as any)).toBe(false);
      expect(validateCSRFToken('   ', 'token_1')).toBe(false);
      expect(validateCSRFToken('token_1', '   ')).toBe(false);
    });
  });

  describe('createCsrfMiddleware', () => {
    const config = { corsOrigins: 'https://app.grainlify.io' };
    const csrfMiddleware = createCsrfMiddleware(config);

    function createMockRes() {
      const res: any = {
        statusCode: 200,
        body: null,
        status(code: number) {
          this.statusCode = code;
          return this;
        },
        json(payload: any) {
          this.body = payload;
          return this;
        }
      };
      return res as ResponseWithStatus & { statusCode: number; body: any };
    }

    it('allows read-only HTTP methods without CSRF token headers', () => {
      const req: RequestWithHeaders = { method: 'GET', headers: { origin: 'https://app.grainlify.io' } };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(true);
      expect(res.statusCode).toBe(200);
    });

    it('allows non-browser API clients (no Origin or Referer)', () => {
      const req: RequestWithHeaders = { method: 'POST', headers: {} };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(true);
    });

    it('rejects browser POST request missing x-csrf-token header', () => {
      const req: RequestWithHeaders = { method: 'POST', headers: { origin: 'https://app.grainlify.io' } };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(false);
      expect(res.statusCode).toBe(403);
      expect(res.body).toEqual({ error: 'missing_csrf_token', message: 'CSRF token is required for browser-facing requests' });
    });

    it('rejects browser POST request from disallowed origin', () => {
      const req: RequestWithHeaders = {
        method: 'POST',
        headers: {
          origin: 'https://evil-attacker.com',
          'x-csrf-token': 'valid_token'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(false);
      expect(res.statusCode).toBe(403);
      expect(res.body).toEqual({ error: 'disallowed_origin', message: 'Origin or Referer not allowed' });
    });

    it('accepts browser POST request from allowed origin with x-csrf-token header', () => {
      const req: RequestWithHeaders = {
        method: 'POST',
        headers: {
          origin: 'https://app.grainlify.io',
          'x-csrf-token': 'valid_token'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(true);
    });

    it('validates Referer header when Origin header is absent', () => {
      const reqValid: RequestWithHeaders = {
        method: 'POST',
        headers: {
          referer: 'https://app.grainlify.io/dashboard',
          'x-csrf-token': 'valid_token'
        }
      };
      const resValid = createMockRes();
      let nextValid = false;
      csrfMiddleware(reqValid, resValid, () => { nextValid = true; });
      expect(nextValid).toBe(true);

      const reqInvalid: RequestWithHeaders = {
        method: 'POST',
        headers: {
          referer: 'https://attacker.com/page',
          'x-csrf-token': 'valid_token'
        }
      };
      const resInvalid = createMockRes();
      let nextInvalid = false;
      csrfMiddleware(reqInvalid, resInvalid, () => { nextInvalid = true; });
      expect(nextInvalid).toBe(false);
      expect(resInvalid.statusCode).toBe(403);
    });

    it('rejects malformed Origin header', () => {
      const req: RequestWithHeaders = {
        method: 'POST',
        headers: {
          origin: 'not-a-valid-url',
          'x-csrf-token': 'valid_token'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(false);
      expect(res.statusCode).toBe(403);
      expect(res.body).toEqual({ error: 'disallowed_origin', message: 'Malformed Origin header' });
    });

    it('handles array headers safely (extracts first string element)', () => {
      const req: RequestWithHeaders = {
        method: 'POST',
        headers: {
          origin: ['https://app.grainlify.io', 'https://other.com'],
          'x-csrf-token': ['valid_token']
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      expect(nextCalled).toBe(true);
    });

    it('handles array headers with non-string elements (treats as missing)', () => {
      const req: RequestWithHeaders = {
        method: 'POST',
        headers: {
          origin: [123 as any],
          'x-csrf-token': 'valid_token'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      // Non-browser request (no valid origin) allowed through
      expect(nextCalled).toBe(true);
    });

    it('handles empty array headers (treats as missing)', () => {
      const req: RequestWithHeaders = {
        method: 'POST',
        headers: {
          origin: [],
          'x-csrf-token': 'valid_token'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      // Non-browser request (no origin) allowed through
      expect(nextCalled).toBe(true);
    });

    it('handles non-string method header (defaults to GET)', () => {
      const req: RequestWithHeaders = {
        method: 123 as any,
        headers: {
          origin: 'https://app.grainlify.io'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      // Defaults to GET (safe method), allowed through
      expect(nextCalled).toBe(true);
    });

    it('handles undefined method header (defaults to GET)', () => {
      const req: RequestWithHeaders = {
        method: undefined as any,
        headers: {
          origin: 'https://app.grainlify.io'
        }
      };
      const res = createMockRes();
      let nextCalled = false;
      csrfMiddleware(req, res, () => { nextCalled = true; });
      // Defaults to GET (safe method), allowed through
      expect(nextCalled).toBe(true);
    });
  });

  describe('Determinism and Retry Stability', () => {
    it('is strictly deterministic across 100 repeated retries', () => {
      const token = 'repeat_test_token_999';
      const redirect = 'https://my-app.vercel.app/callback';
      const initialEncoded = encodeStateWithRedirect(token, redirect);
      const initialDecoded = decodeStateWithRedirect(initialEncoded);
      const initialAllowed = isAllowedRedirectURI(redirect);

      for (let i = 0; i < 100; i++) {
        const encoded = encodeStateWithRedirect(token, redirect);
        const decoded = decodeStateWithRedirect(encoded);
        const allowed = isAllowedRedirectURI(redirect);

        expect(encoded).toBe(initialEncoded);
        expect(decoded).toEqual(initialDecoded);
        expect(allowed).toBe(initialAllowed);
      }
    });
  });
});

