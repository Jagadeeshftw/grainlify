/**
 * CSRF and Browser-Facing Request Security Middleware / Utilities
 * 
 * Provides state parameter encoding/decoding, origin validation for open redirect prevention,
 * CSRF token verification, and Express middleware for browser-facing API requests.
 */

export interface SecurityConfig {
  corsOrigins?: string;
  frontendBaseUrl?: string;
}

export interface DecodedState {
  csrfToken: string;
  redirectURI: string;
}

export interface RequestWithHeaders {
  method?: string;
  headers: Record<string, string | string[] | undefined>;
}

export interface ResponseWithStatus {
  status(code: number): ResponseWithStatus;
  json(body: any): any;
}

export type NextFunction = (err?: any) => void;

/**
 * Normalizes an origin or URL string by trimming trailing slashes and extracting protocol+host.
 */
function normalizeAllowedOrigin(allowedStr: string): string {
  const trimmed = allowedStr.trim();
  if (!trimmed) return '';
  try {
    const url = new URL(trimmed);
    return url.origin;
  } catch {
    // Strip trailing slashes if URL parsing fails
    return trimmed.replace(/\/+$/, '');
  }
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
 * - Userinfo credentials embedded in URL (e.g. http://user:pass@localhost)
 * - Malformed URLs
 */
export function isAllowedRedirectURI(redirectURI: string, config: SecurityConfig = {}): boolean {
  if (!redirectURI || typeof redirectURI !== 'string') {
    return false;
  }

  const trimmedURI = redirectURI.trim();
  if (!trimmedURI) {
    return false;
  }

  let parsed: URL;
  try {
    parsed = new URL(trimmedURI);
  } catch {
    return false;
  }

  // Reject non-http/https schemes
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    return false;
  }

  // Reject URLs with embedded credentials to prevent credential exposure & open redirect confusion
  if (parsed.username || parsed.password) {
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
      const normalizedAllowed = normalizeAllowedOrigin(allowed);
      if (origin === normalizedAllowed || origin === allowed || origin.startsWith(normalizedAllowed + '/')) {
        return true;
      }
    }
  }

  // Check FrontendBaseURL
  if (config.frontendBaseUrl && config.frontendBaseUrl.trim() !== '') {
    const baseUrl = config.frontendBaseUrl.trim();
    const normalizedBase = normalizeAllowedOrigin(baseUrl);
    if (origin === normalizedBase || origin === baseUrl || origin.startsWith(normalizedBase + '/')) {
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
  if (!csrfToken || typeof csrfToken !== 'string') {
    return '';
  }
  const trimmedToken = csrfToken.trim();
  const trimmedURI = redirectURI ? redirectURI.trim() : '';

  if (!trimmedURI) {
    return trimmedToken;
  }
  const payload = `${trimmedToken}|${trimmedURI}`;
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

  const trimmedState = encodedState.trim();
  if (!trimmedState) {
    return { csrfToken: '', redirectURI: '' };
  }

  try {
    const decoded = base64UrlDecode(trimmedState);
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
  return { csrfToken: trimmedState, redirectURI: '' };
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
 * Type-safe, rejects non-strings and empty values.
 */
export function validateCSRFToken(tokenFromRequest: string, tokenFromStorage: string, expiresAt?: Date): boolean {
  if (typeof tokenFromRequest !== 'string' || typeof tokenFromStorage !== 'string') {
    return false;
  }
  const reqToken = tokenFromRequest.trim();
  const storeToken = tokenFromStorage.trim();

  if (!reqToken || !storeToken) {
    return false;
  }
  if (reqToken !== storeToken) {
    return false;
  }
  if (expiresAt !== undefined) {
    if (!(expiresAt instanceof Date) || isNaN(expiresAt.getTime())) {
      return false;
    }
    if (expiresAt.getTime() < Date.now()) {
      return false;
    }
  }
  return true;
}

/**
 * Express-compatible CSRF middleware factory for browser-facing request security.
 * Inspects x-csrf-token, Origin, and Referer headers for state-changing HTTP requests (POST, PUT, DELETE, PATCH).
 * Non-browser API clients (without Origin or Referer) are allowed through.
 */
export function createCsrfMiddleware(config: SecurityConfig = {}) {
  return function csrfMiddleware(req: RequestWithHeaders, res: ResponseWithStatus, next: NextFunction) {
    const method = (req.method || 'GET').toUpperCase();

    // Safe methods (GET, HEAD, OPTIONS) do not alter server state
    if (['GET', 'HEAD', 'OPTIONS'].includes(method)) {
      return next();
    }

    const headers = req.headers || {};
    const originHeader = Array.isArray(headers['origin']) ? headers['origin'][0] : headers['origin'];
    const refererHeader = Array.isArray(headers['referer']) ? headers['referer'][0] : headers['referer'];
    const csrfTokenHeader = Array.isArray(headers['x-csrf-token']) ? headers['x-csrf-token'][0] : headers['x-csrf-token'];

    const isBrowserRequest = Boolean(originHeader || refererHeader);

    if (!isBrowserRequest) {
      // Non-browser API clients (curl, automated scripts) are allowed through
      return next();
    }

    // Browser request requires valid CSRF token header
    if (!csrfTokenHeader || typeof csrfTokenHeader !== 'string' || csrfTokenHeader.trim() === '') {
      return res.status(403).json({ error: 'missing_csrf_token', message: 'CSRF token is required for browser-facing requests' });
    }

    // Determine request origin from Origin or Referer header
    let requestOrigin: string | null = null;
    if (originHeader) {
      requestOrigin = originHeader;
    } else if (refererHeader) {
      try {
        requestOrigin = new URL(refererHeader).origin;
      } catch {
        return res.status(403).json({ error: 'disallowed_origin', message: 'Malformed Referer header' });
      }
    }

    if (requestOrigin && !isAllowedRedirectURI(requestOrigin, config)) {
      return res.status(403).json({ error: 'disallowed_origin', message: 'Origin or Referer not allowed' });
    }

    return next();
  };
}

