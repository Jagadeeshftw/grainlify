# OAuth 2.0 Spec Compliance - State Parameter Implementation

## Overview

Our OAuth implementation follows the OAuth 2.0 specification recommendation for dynamic redirection using the `state` parameter, while maintaining security best practices.

## OAuth 2.0 Specification Requirements

### ✅ Static Redirect URI (Required)
- **Requirement:** Authorization callback URL must be static and pre-registered
- **Our Implementation:** 
  - Single static callback URL: `https://api.grainlify.0xo.in/auth/github/login/callback`
  - Registered in GitHub OAuth App settings
  - Never changes, works for all environments

### ✅ State Parameter for Dynamic Redirection (Recommended)
- **Requirement:** Use `state` parameter to encode dynamic destination URL
- **Our Implementation:**
  - Encodes both CSRF token and redirect_uri in state: `base64(csrf_token|redirect_uri)`
  - Follows OAuth 2.0 spec recommendation
  - Maintains CSRF protection

### ✅ CSRF Protection (Required)
- **Requirement:** Validate state parameter to prevent CSRF attacks
- **Our Implementation:**
  - Generates secure random CSRF token (32 bytes)
  - Stores CSRF token in database with expiration (10 minutes)
  - Validates CSRF token on callback
  - Deletes used state to prevent replay attacks

## Implementation Details

### State Parameter Encoding

**Format:**
```
base64(csrf_token + "|" + redirect_uri)
```

**Example:**
- CSRF Token: `abc123...`
- Redirect URI: `https://grainlify.0xo.in`
- Encoded State: `base64("abc123...|https://grainlify.0xo.in")`

### Flow

1. **Frontend initiates OAuth:**
   ```
   GET /auth/github/login/start?redirect=https://grainlify.0xo.in
   ```

2. **Backend generates state:**
   - Generates CSRF token: `randomState(32)`
   - Stores CSRF token in database
   - Encodes state: `base64(csrf_token|redirect_uri)`
   - Redirects to GitHub with encoded state

3. **GitHub redirects back:**
   ```
   GET /auth/github/login/callback?code=...&state=base64(csrf_token|redirect_uri)
   ```

4. **Backend validates and decodes:**
   - Decodes state parameter to extract CSRF token and redirect_uri
   - Validates CSRF token against database
   - Validates redirect_uri against allowed origins
   - Processes OAuth code exchange
   - Redirects to: `{redirect_uri}/auth/callback?token={jwt}`

## Security Measures

### 1. CSRF Protection
- ✅ Secure random state generation (32 bytes)
- ✅ State stored in database with expiration
- ✅ State validated on callback
- ✅ State deleted after use (prevents replay)

### 2. Open Redirect Prevention
- ✅ Redirect URI validated against whitelist:
  - localhost origins (development)
  - `*.vercel.app` domains (preview deployments)
  - Explicit `CORS_ORIGINS` config
  - `FRONTEND_BASE_URL` config
- ✅ Validation happens twice:
  - When received from frontend (LoginStart)
  - When extracted from state parameter (CallbackUnified)

### 3. State Parameter Security
- ✅ Base64 URL-safe encoding
- ✅ Opaque to clients (cannot be tampered with)
- ✅ Includes CSRF token for validation
- ✅ Includes redirect_uri for dynamic redirection

## Code Structure

### State Encoding Functions

```go
// Encodes CSRF token and redirect_uri in state parameter
encodeStateWithRedirect(csrfToken, redirectURI string) string

// Decodes state parameter to extract CSRF token and redirect_uri
decodeStateWithRedirect(encodedState string) (string, string, error)
```

### Validation Flow

1. **LoginStart Handler:**
   - Validates redirect_uri against allowed origins
   - Generates CSRF token
   - Stores CSRF token in database
   - Encodes state with CSRF token and redirect_uri
   - Redirects to GitHub

2. **CallbackUnified Handler:**
   - Decodes state parameter
   - Validates CSRF token against database
   - Validates redirect_uri from state against allowed origins
   - Processes OAuth
   - Redirects to validated redirect_uri

## Backward Compatibility

The implementation maintains backward compatibility:
- If state cannot be decoded, treats entire state as CSRF token
- Falls back to database-stored redirect_uri if not in state
- Falls back to config values if no redirect_uri available

## Benefits of This Approach

1. **OAuth 2.0 Spec Compliant** - Follows recommended pattern
2. **Secure** - CSRF protection + open redirect prevention
3. **Dynamic** - Supports multiple frontend deployments
4. **Stateless** - Redirect URI encoded in state (no database dependency for redirect)
5. **Flexible** - Works for production, preview, localhost

## Comparison: Database vs State Parameter

| Aspect | Database Storage | State Parameter (Current) |
|--------|------------------|---------------------------|
| **OAuth 2.0 Spec** | Not recommended | ✅ Recommended |
| **Security** | ✅ Secure | ✅ Secure |
| **CSRF Protection** | ✅ Yes | ✅ Yes |
| **Stateless** | ❌ Requires DB | ✅ Self-contained |
| **Scalability** | Requires DB lookup | No DB lookup needed |
| **Complexity** | Simpler | Slightly more complex |

## Conclusion

Our implementation now follows the OAuth 2.0 specification recommendation by encoding the redirect_uri in the state parameter, while maintaining all security measures (CSRF protection, open redirect prevention). This is the industry-standard approach used by major platforms.

