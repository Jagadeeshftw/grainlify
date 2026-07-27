# Middleware

## CSRF Middleware
The `csrfMiddleware` provides security edge case handling for browser-facing requests.

### Behavior
- Validates the `x-csrf-token` header for all requests originating from a browser.
- Checks `Origin` and `Referer` headers to prevent Cross-Site Request Forgery (CSRF).
- Non-browser API clients (without Origin or Referer) are allowed through without CSRF validation.

### Edge Cases Documented
- Missing CSRF tokens are explicitly blocked for browser requests.
- Requests with spoofed or malformed origins are blocked.
- Fallback validation for `Referer` is performed if `Origin` is absent.
