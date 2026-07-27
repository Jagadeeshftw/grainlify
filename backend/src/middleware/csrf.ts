/**
 * CSRF and Browser-Facing Request Security Middleware / Utilities
 * 
 * Provides state parameter encoding/decoding, origin validation for open redirect prevention,
 * and CSRF token verification for browser-facing OAuth and API flows.
 */

export interface SecurityConfig {
  corsOrigins?: string;
  frontendBaseUrl?: string;
}

export interface DecodedState {
  csrfToken: string;
  redirectURI: string;
}

/**
 * Validates that a redirect URI originates from an allowed origin.
 * Prevents open redirect vulnerabilities by strictly matching:
 * - localhost origins (http://localhost:*, http://127.0.0.1:*, https://localhost:*, https://127.0.0.1:*)
 * - *.vercel.app preview deployment domains
 * - Explicit origins specified in CORS_ORIGINS config
 * - FrontendBaseURL config
 * 
 * Rejecting:
 * - Non http/https schemes (javascript:, data:, file:)
 * - Subdomain spoofing (e.g., http://localhost.attacker.com)
 * - Malformed URLs
 */
export function isAllowedRedirectURI(redirectURI: string, config: SecurityConfig = {}): boolean {
  if (!redirectURI || typeof redirectURI !== 'string') {
    return false;
  }

  let parsed: URL;
  try {
    parsed = new URL(redirectURI);
  } catch {
    return false;
  }

  // Reject non-http/https schemes
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    return false;
  }

  const origin = parsed.origin; // e.g. "https://example.com" or "http://localhost:3000"
  const host = parsed.hostname; // e.g. "localhost", "sub.vercel.app", "attacker.com"

  // Always allow localhost origins for local development
  if (
    host === 'localhost' ||
    host === '127.0.0.1' ||
    origin.startsWith('http://localhost:') ||
    origin.startsWith('http://127.0.0.1:') ||
    origin.startsWith('https://localhost:') ||
    origin.startsWith('https://127.0.0.1:')
  ) {
    return true;
  }

  // Allow all Vercel preview deployments (*.vercel.app)
  if (host.endsWith('.vercel.app') || host === 'vercel.app') {
    return true;
  }

  // Check explicit CORS origins
  if (config.corsOrigins && config.corsOrigins.trim() !== '') {
    const origins = config.corsOrigins.split(',').map(o => o.trim()).filter(Boolean);
    for (const allowed of origins) {
      if (origin === allowed || origin.startsWith(allowed + '/')) {
        return true;
      }
    }
  }

  // Check FrontendBaseURL
  if (config.frontendBaseUrl && config.frontendBaseUrl.trim() !== '') {
    const baseUrl = config.frontendBaseUrl.trim();
    if (origin === baseUrl || origin.startsWith(baseUrl + '/')) {
      return true;
    }
  }

  return false;
}

/**
 * Encodes both a CSRF token and redirect_uri in the state parameter.
 * Format: base64url(csrf_token + "|" + redirect_uri)
 * If no redirect_uri is provided, returns raw csrfToken for backward compatibility.
 */
export function encodeStateWithRedirect(csrfToken: string, redirectURI?: string): string {
  if (!redirectURI || redirectURI.trim() === '') {
    return csrfToken;
  }
  const payload = `${csrfToken}|${redirectURI}`;
  return base64UrlEncode(payload);
}

/**
 * Decodes the state parameter to extract CSRF token and redirect_uri.
 * Handles backward compatibility:
 * - Legacy format: unencoded state or single token -> returns { csrfToken: state, redirectURI: "" }
 * - New format: base64url(csrf_token|redirect_uri) -> returns { csrfToken, redirectURI }
 */
export function decodeStateWithRedirect(encodedState: string): DecodedState {
  if (!encodedState || typeof encodedState !== 'string') {
    return { csrfToken: '', redirectURI: '' };
  }

  try {
    const decoded = base64UrlDecode(encodedState);
    const firstPipeIndex = decoded.indexOf('|');
    if (firstPipeIndex !== -1) {
      const csrfToken = decoded.substring(0, firstPipeIndex);
      const redirectURI = decoded.substring(firstPipeIndex + 1);
      return { csrfToken, redirectURI };
    }
  } catch {
    // If base64 decoding fails, fall back to treating entire state as raw CSRF token
  }

  // Legacy fallback: unencoded CSRF token
  return { csrfToken: encodedState, redirectURI: '' };
}

/**
 * Helper: Base64URL encode string (URL-safe without padding)
 */
function base64UrlEncode(str: string): string {
  let base64: string;
  if (typeof Buffer !== 'undefined') {
    base64 = Buffer.from(str, 'utf-8').toString('base64');
  } else {
    base64 = btoa(encodeURIComponent(str).replace(/%([0-9A-F]{2})/g, (_, p1) => String.fromCharCode(parseInt(p1, 16))));
  }
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/**
 * Helper: Base64URL decode string
 */
function base64UrlDecode(str: string): string {
  let base64 = str.replace(/-/g, '+').replace(/_/g, '/');
  while (base64.length % 4 !== 0) {
    base64 += '=';
  }
  if (typeof Buffer !== 'undefined') {
    return Buffer.from(base64, 'base64').toString('utf-8');
  } else {
    return decodeURIComponent(atob(base64).split('').map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)).join(''));
  }
}

/**
 * Validates CSRF token against stored token and optional expiration timestamp.
 */
export function validateCSRFToken(tokenFromRequest: string, tokenFromStorage: string, expiresAt?: Date): boolean {
  if (!tokenFromRequest || !tokenFromStorage) {
    return false;
  }
  if (tokenFromRequest !== tokenFromStorage) {
    return false;
  }
  if (expiresAt && expiresAt.getTime() < Date.now()) {
    return false;
  }
  return true;
}
