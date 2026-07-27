import { isAllowedRedirectURI, encodeStateWithRedirect, decodeStateWithRedirect, validateCSRFToken } from './csrf';

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

    it('rejects malicious or disallowed origins (Open Redirect protection)', () => {
      expect(isAllowedRedirectURI('http://localhost.attacker.com')).toBe(false);
      expect(isAllowedRedirectURI('https://vercel.app.evil.com')).toBe(false);
      expect(isAllowedRedirectURI('javascript:alert(1)')).toBe(false);
      expect(isAllowedRedirectURI('data:text/html,<script>alert(1)</script>')).toBe(false);
      expect(isAllowedRedirectURI('https://untrusted-site.com')).toBe(false);
      expect(isAllowedRedirectURI('')).toBe(false);
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

    it('handles malformed base64 state gracefully', () => {
      const malformed = '!!!not_valid_base64!!!';
      const decoded = decodeStateWithRedirect(malformed);
      expect(decoded.csrfToken).toBe(malformed);
      expect(decoded.redirectURI).toBe('');
    });

    it('handles empty or null state', () => {
      expect(decodeStateWithRedirect('')).toEqual({ csrfToken: '', redirectURI: '' });
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
