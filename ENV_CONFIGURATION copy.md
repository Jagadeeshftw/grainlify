# Environment Configuration Guide

All hardcoded URLs have been removed. All configuration now comes from environment variables.

## Backend Environment Variables

Create a `.env` file in the `backend/` directory with the following variables:

### Required Variables

```bash
# Application Environment
APP_ENV=dev

# Database
DB_URL=postgresql://user:password@localhost:5432/grainlify?sslmode=disable

# JWT Secret (generate a secure random string)
JWT_SECRET=your-secret-key-here

# GitHub OAuth
GITHUB_OAUTH_CLIENT_ID=your-github-oauth-client-id
GITHUB_OAUTH_CLIENT_SECRET=your-github-oauth-client-secret
# IMPORTANT: This must be the FULL callback URL and must match EXACTLY what's registered in your GitHub OAuth app
# The callback route is: /auth/github/login/callback
# Development: http://localhost:8080/auth/github/login/callback
# Production: https://your-backend-domain.com/auth/github/login/callback
# If not set, will be constructed from PUBLIC_BASE_URL + /auth/github/login/callback
GITHUB_OAUTH_REDIRECT_URL=http://localhost:8080/auth/github/login/callback
```

### Frontend Configuration (IMPORTANT)

```bash
# Frontend Base URL - Used for OAuth redirects
# Development: http://localhost:5173
# Production: https://your-frontend-domain.com
FRONTEND_BASE_URL=http://localhost:5173

# Optional: Explicit OAuth success redirect (if different from FRONTEND_BASE_URL/auth/callback)
GITHUB_LOGIN_SUCCESS_REDIRECT_URL=http://localhost:5173/auth/callback

# CORS Origins (comma-separated, optional - defaults to FRONTEND_BASE_URL)
# Development: http://localhost:5173,http://localhost:3000
# Production: https://your-frontend-domain.com
CORS_ORIGINS=http://localhost:5173
```

### Optional Variables

```bash
# Server Configuration
PORT=8080
LOG_LEVEL=info
AUTO_MIGRATE=true

# Public Base URL (for webhooks)
PUBLIC_BASE_URL=http://localhost:8080

# Token Encryption Key (32 bytes base64 encoded)
TOKEN_ENC_KEY_B64=your-32-byte-base64-encryption-key

# GitHub Webhook Secret
GITHUB_WEBHOOK_SECRET=your-github-webhook-secret

# Didit KYC
DIDIT_API_KEY=your-didit-api-key
DIDIT_WORKFLOW_ID=your-didit-workflow-id
DIDIT_WEBHOOK_SECRET=your-didit-webhook-secret

# NATS (optional, for event bus)
NATS_URL=
```

## Frontend Environment Variables

Create a `.env` file in the `frontend/` directory with the following variables:

### Required Variables

```bash
# Backend API URL
# Development: http://localhost:8080
# Production: https://your-backend-domain.com
VITE_API_BASE_URL=http://localhost:8080
```

### Optional Variables

```bash
# Frontend Base URL (optional, defaults to window.location.origin)
# Development: http://localhost:5173
# Production: https://your-frontend-domain.com
VITE_FRONTEND_BASE_URL=http://localhost:5173
```

## How It Works

### Backend Redirect Logic

1. **GitHub Login Success Redirect:**
   - First checks `GITHUB_LOGIN_SUCCESS_REDIRECT_URL` (if set)
   - Otherwise, constructs from `FRONTEND_BASE_URL` + `/auth/callback`
   - Example: `FRONTEND_BASE_URL=http://localhost:5173` → redirects to `http://localhost:5173/auth/callback`

2. **CORS Configuration:**
   - If `CORS_ORIGINS` is set, uses those exact origins (comma-separated)
   - Otherwise, dynamically allows:
     - All `http://localhost:*` and `http://127.0.0.1:*` origins
     - The `FRONTEND_BASE_URL` origin

### Frontend Configuration

- `VITE_API_BASE_URL`: Points to your backend server
- `VITE_FRONTEND_BASE_URL`: Used for constructing callback URLs (defaults to current origin)

## Local Development Setup

1. **Backend `.env`:**
   ```bash
   FRONTEND_BASE_URL=http://localhost:5173
   GITHUB_LOGIN_SUCCESS_REDIRECT_URL=http://localhost:5173/auth/callback
   CORS_ORIGINS=http://localhost:5173
   ```

2. **Frontend `.env`:**
   ```bash
   VITE_API_BASE_URL=http://localhost:8080
   VITE_FRONTEND_BASE_URL=http://localhost:5173
   ```

## Production Setup

1. **Backend `.env`:**
   ```bash
   FRONTEND_BASE_URL=https://your-frontend-domain.com
   GITHUB_LOGIN_SUCCESS_REDIRECT_URL=https://your-frontend-domain.com/auth/callback
   CORS_ORIGINS=https://your-frontend-domain.com
   ```

2. **Frontend `.env`:**
   ```bash
   VITE_API_BASE_URL=https://your-backend-domain.com
   VITE_FRONTEND_BASE_URL=https://your-frontend-domain.com
   ```

## Important Notes

- **No hardcoded URLs**: All URLs are now configurable via environment variables
- **Vite prefix**: Frontend environment variables must be prefixed with `VITE_` to be accessible in the browser
- **CORS**: The backend automatically allows localhost origins in development mode
- **Fallbacks**: If `FRONTEND_BASE_URL` is not set, redirects may fail - always set it!

## GitHub OAuth Redirect URI Configuration

**CRITICAL**: The `GITHUB_OAUTH_REDIRECT_URL` must match EXACTLY what's registered in your GitHub OAuth app settings.

### Steps to Fix "redirect_uri is not associated with this application" Error:

1. **Check your current `GITHUB_OAUTH_REDIRECT_URL` value:**
   ```bash
   # In your backend .env file, it should be:
   GITHUB_OAUTH_REDIRECT_URL=http://localhost:8080/auth/github/login/callback
   ```

2. **Register the exact URL in GitHub:**
   - Go to GitHub → Settings → Developer settings → OAuth Apps
   - Select your OAuth app
   - In "Authorization callback URL", add: `http://localhost:8080/auth/github/login/callback`
   - The URL must match EXACTLY (including http vs https, localhost vs 127.0.0.1, port number, and path)

3. **Common mistakes:**
   - ❌ `http://127.0.0.1:8080` (should be `localhost`)
   - ❌ `http://localhost:8080` (missing `/auth/github/login/callback` path)
   - ❌ `http://localhost:8080/callback` (wrong path)
   - ✅ `http://localhost:8080/auth/github/login/callback` (correct)

4. **Auto-construction fallback:**
   - If `GITHUB_OAUTH_REDIRECT_URL` is not set, it will be constructed from `PUBLIC_BASE_URL + /auth/github/login/callback`
   - Example: `PUBLIC_BASE_URL=http://localhost:8080` → `http://localhost:8080/auth/github/login/callback`

### For Production:

Make sure to:
1. Set `GITHUB_OAUTH_REDIRECT_URL` to your production backend URL
2. Register the same URL in your GitHub OAuth app settings
3. Use `https://` (not `http://`) for production

