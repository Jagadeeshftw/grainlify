# Security Edge Case Review & Regression Surface Specification

## Executive Summary

This document specifies the current behavior, security-sensitive edge cases, and expected regression surface for browser-facing request security in the backend service, specifically focusing on CSRF protection, browser-facing middleware, state parameter validation, open redirect prevention, and authentication middleware.

---

## 1. Current Behavior Overview

### 1.1 CSRF State Parameter Validation Flow
1. **Initiation (`LoginStart` / `Start`):**
   - Generates a 32-byte secure random state token (`csrfToken`).
   - Stores the state token in the `oauth_states` table with a 10-minute TTL (`expires_at = now() + 10m`).
   - Encodes the CSRF token and optional destination URI in the `state` parameter:
     - Format: `base64url(csrf_token + "|" + redirect_uri)`.
     - Legacy fallback: If no `redirect_uri` is supplied, returns the raw `csrf_token`.

2. **Callback Handling (`CallbackUnified`):**
   - Receives `state` and `code` query parameters.
   - Decodes the state via `decodeStateWithRedirect`:
     - If base64 decoding fails or contains no `|` separator, treats the input as a raw `csrfToken` (maintaining backward compatibility).
   - Queries `oauth_states` for a matching state record where `expires_at > now()`.
   - Immediately deletes the state from database (`DELETE FROM oauth_states WHERE state = $1`) upon retrieval to prevent state replay attacks.

3. **Open Redirect Validation (`isAllowedRedirectURI`):**
   - Parses destination origin from state or request parameter.
   - Validates that origin matches an allowed scheme and host whitelist:
     - Development: `http://localhost:*`, `http://127.0.0.1:*`, `https://localhost:*`, `https://127.0.0.1:*`.
     - Vercel Preview Deployments: `*.vercel.app` or `vercel.app`.
     - Configured Origins: `CORSOrigins` and `FrontendBaseURL` (normalized for trailing slashes).
   - If allowed, redirects user to `{redirect_uri}/auth/callback?token={jwt}&github={login}`.
   - If disallowed or missing, rejects request or falls back to production configured base URL.

4. **Browser-Facing Request Security Middleware (`createCsrfMiddleware`):**
   - Inspects state-changing HTTP requests (`POST`, `PUT`, `DELETE`, `PATCH`).
   - Validates `x-csrf-token` header for browser-originating requests (requests with `Origin` or `Referer`).
   - Validates `Origin` and `Referer` against `isAllowedRedirectURI`.
   - Non-browser API clients (requests without `Origin` or `Referer`) bypass browser CSRF checks.

---

## 2. Security-Sensitive Edge Cases Reviewed

| Category | Edge Case Scenario | Expected System Behavior | Security Purpose |
|----------|-------------------|--------------------------|------------------|
| **Origin Checking** | Malicious host suffix (e.g. `http://localhost.attacker.com`) | Rejected (`false`) | Prevents domain spoofing open redirects |
| **Origin Checking** | Subdomain takeover target (e.g. `https://vercel.app.attacker.com`) | Rejected (`false`) | Prevents sub-domain string matching bypasses |
| **Origin Checking** | URL containing userinfo credentials (`http://user:pass@localhost:3000`) | Rejected (`false`) | Prevents credential exposure & phishing via URL auth |
| **Origin Checking** | Configured origin with trailing slash (`https://grainlify.io/`) | Allowed (`true`) | Prevents config format mismatch false negatives |
| **Origin Checking** | Non-HTTP schemes (`javascript:...`, `data:...`, `file:...`) | Rejected (`false`) | Prevents XSS / payload injection via URL schemes |
| **Origin Checking** | Localhost dev origins (`http://localhost:3000`, `http://127.0.0.1:5173`) | Allowed (`true`) | Preserves developer workflow without weakening production |
| **Origin Checking** | Vercel preview domains (`https://pr-42.vercel.app`) | Allowed (`true`) | Supports dynamic preview deployments securely |
| **State Parameter** | Unencoded / legacy CSRF token without `\|` separator | Fallback to raw token | Maintains backward compatibility with ongoing OAuth flows |
| **State Parameter** | Base64 state containing multiple `\|` delimiters | Split at first `\|` only | Handles complex redirect query strings without corrupting token |
| **State Parameter** | Malformed / invalid base64 string in `state` | Fallback to raw string | Prevents panics or 500 errors on invalid inputs |
| **State Parameter** | Replay of previously consumed `state` parameter | Rejected (`invalid_or_expired_state`) | Ensures single-use state tokens |
| **State Parameter** | State lookup after 10-minute expiration | Rejected (`invalid_or_expired_state`) | Enforces time-bound state validity |
| **CSRF Validation** | Non-string / null token arguments passed | Rejected (`false`) | Prevents type confusion & runtime crashes |
| **CSRF Validation** | Invalid `Date` instance passed as `expiresAt` | Rejected (`false`) | Prevents `NaN` comparison bypasses |
| **Browser Middleware** | State-changing request with `Origin`/`Referer` missing `x-csrf-token` | HTTP 403 (`missing_csrf_token`) | Blocks browser CSRF attempts |
| **Browser Middleware** | Browser request with disallowed `Origin` or `Referer` | HTTP 403 (`disallowed_origin`) | Blocks cross-origin browser requests from untrusted origins |
| **Browser Middleware** | Non-browser request (no `Origin` or `Referer`) | Allowed (`next()`) | Allows direct server-to-server & CLI API calls |
| **Auth Middleware** | Request missing `Authorization` header | HTTP 401 (`missing_bearer_token`) | Prevents unauthenticated access to protected routes |
| **Auth Middleware** | `Authorization` header with non-Bearer scheme (`Basic ...`) | HTTP 401 (`missing_bearer_token`) | Enforces Bearer JWT standard |
| **Auth Middleware** | `Authorization` header with `Bearer ` but empty payload | HTTP 401 (`missing_bearer_token`) | Rejects empty tokens |
| **Auth Middleware** | `Authorization` header that is whitespace-only | HTTP 401 (`missing_bearer_token`) | Rejects blank headers after trimming |
| **Auth Middleware** | Corrupted or expired JWT payload | HTTP 401 (`invalid_token`) | Prevents access with forged/expired tokens |
| **Auth Middleware** | JWT token with trailing whitespace after token | HTTP 200 (allowed) | Whitespace is trimmed before parsing — token remains valid |
| **Auth Middleware** | Expired JWT token (past `exp` claim) | HTTP 401 (`invalid_token`) | Prevents reuse of stale tokens |
| **Auth Middleware** | `Authorization` header with `bearer ` (lowercase) | HTTP 200 (allowed) | Case-insensitive scheme matching |
| **RequireRole** | Empty `roles` argument slice (`RequireRole()`) | HTTP 403 (`insufficient_role`) | No roles configured → no access granted |
| **RequireRole** | Role not present in Fiber locals | HTTP 403 (`missing_role`) | Missing `role` local treated as denial |
| **RequireRole** | Role does not match any allowed role | HTTP 403 (`insufficient_role`) | Non-matching roles are rejected |
| **RequireScopedAdmin** | Pool is nil | HTTP 503 (`db_not_configured`) | Prevents nil-pointer panic on DB query |
| **RequireScopedAdmin** | Authenticated user has global `admin` role | HTTP 200 (allowed) bypass | Global admins skip scoped lookup |
| **RequireScopedAdmin** | Missing `user_id` in Fiber locals | HTTP 401 (`invalid_user`) | Prevents access without identity |
| **RequireScopedAdmin** | Route param `scopeID` is empty | HTTP 400 (`missing_scope_id`) | Ensures a valid scope is targeted |
| **RequireScopedAdmin** | User is not admin and no scoped admin record exists | HTTP 403 (`insufficient_role`) | Non-admin without scoped grant is denied |
| **JWT** | `JWT_SECRET` is empty during `IssueJWT` | Error returned (token not issued) | Prevents unsigned/weak token issuance |
| **JWT** | `JWT_SECRET` is empty during `ParseJWT` | Error returned (parse failure) | Prevents verification with empty secret |
| **JWT** | Token signed with a different algorithm (e.g. ES256 instead of HS256) | Error returned (unexpected signing method) | Enforces HS256-only policy |
| **JWT** | Malformed token string (non-JWT format) | Error returned (parse failure) | Prevents garbage input from passing |

---

## 3. Expected Regression Surface

To ensure future changes do not introduce security regressions or break existing workflows, the following invariants must be maintained:

1. **Backward Compatibility Invariant:**
   - Existing OAuth state strings generated prior to redirect encoding must continue to resolve successfully without throwing server errors.

2. **Open Redirect Invariant:**
   - Any modification to `isAllowedRedirectURI` MUST strictly parse URLs using `url.Parse` / `new URL()` and validate scheme + exact origin hostname. String prefix matching without host boundary checks is strictly prohibited.

3. **Replay Prevention Invariant:**
   - The database deletion `DELETE FROM oauth_states WHERE state = $1` MUST execute before or during callback processing. Re-using a state token MUST fail on second attempt.

4. **Response Contract Invariant:**
   - Error responses for authentication failures MUST preserve exact JSON error keys (`missing_bearer_token`, `invalid_token`, `invalid_or_expired_state`, `redirect_uri_not_allowed`, `missing_csrf_token`, `disallowed_origin`).

5. **Determinism & Retry Stability Invariant:**
   - State parameter encoding, decoding, and origin validation helper functions MUST be pure and strictly deterministic across retries, re-renders, and concurrent evaluations. Calling `encodeStateWithRedirect`, `decodeStateWithRedirect`, or `isAllowedRedirectURI` 100 times sequentially with identical input parameters MUST produce identical output results without state leakage or side effects.

6. **JWT Signing Invariant:**
   - All JWTs MUST be signed with HS256. Any other signing method (ES256, RS256, etc.) MUST be rejected by `ParseJWT`.

7. **Role Hierarchy Invariant:**
   - The global `admin` role MUST bypass all scoped RBAC checks (`RequireScopedAdmin`). No database query is executed for admin users.

8. **Empty Roles Invariant:**
   - Calling `RequireRole()` with no arguments MUST deny all requests (returns `insufficient_role`). An empty allowed set implies no role is permitted.

9. **Nil Pool Invariant:**
   - `RequireScopedAdmin` MUST check for a nil `*pgxpool.Pool` and return HTTP 503 (`db_not_configured`) before attempting any database operation, preventing nil-pointer panics.

---

## 4. Test Coverage Reference

The regression surface is explicitly covered and pinned down by unit test suites:
- **Go Handlers & OAuth Edge Cases:** `backend/internal/handlers/github_oauth_test.go`
- **Go Auth Middleware Edge Cases:** `backend/internal/auth/middleware_test.go`
- **Go JWT Edge Cases:** `backend/internal/auth/jwt_test.go`
- **Go Scoped Admin Handler Edge Cases:** `backend/internal/handlers/scoped_admin_test.go`
- **TypeScript CSRF & Security Edge Cases:** `backend/src/middleware/csrf.test.ts`

