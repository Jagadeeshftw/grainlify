# Environment Variables: Redirect URLs Guide

## Quick Answer

**For Local Development with Tunnel:**

```bash
# ✅ Use tunnel URL for PUBLIC_BASE_URL (GitHub needs to reach your backend)
PUBLIC_BASE_URL=https://abc123.loclx.io

# ✅ Use tunnel URL for GITHUB_OAUTH_REDIRECT_URL (full callback URL)
GITHUB_OAUTH_REDIRECT_URL=https://abc123.loclx.io/auth/github/login/callback

# ✅ Use localhost for FRONTEND_BASE_URL (where backend redirects users after login)
FRONTEND_BASE_URL=http://localhost:5173
```

**For Production:**

```bash
# ✅ Use production backend domain
PUBLIC_BASE_URL=https://api.grainlify.com

# ✅ Use production backend domain
GITHUB_OAUTH_REDIRECT_URL=https://api.grainlify.com/auth/github/login/callback

# ✅ Use production frontend domain
FRONTEND_BASE_URL=https://grainlify.com
```

---

## Detailed Explanation

### 1. `PUBLIC_BASE_URL`

**Purpose:** The public URL of your backend that external services (GitHub, Didit) can reach.

**Local Development:**
- ✅ **Use tunnel URL:** `https://abc123.loclx.io`
- ❌ **Don't use:** `http://localhost:8080` (GitHub can't reach localhost)

**Production:**
- ✅ **Use production backend:** `https://api.grainlify.com`

**Used for:**
- GitHub webhook URLs
- GitHub App callback URLs (if not explicitly set)
- Any URL that needs to be accessible from the internet

---

### 2. `GITHUB_OAUTH_REDIRECT_URL`

**Purpose:** The full callback URL that GitHub redirects to after OAuth login.

**Local Development:**
- ✅ **Use tunnel URL:** `https://abc123.loclx.io/auth/github/login/callback`
- ❌ **Don't use:** `http://localhost:8080/auth/github/login/callback` (GitHub can't reach localhost)

**Production:**
- ✅ **Use production backend:** `https://api.grainlify.com/auth/github/login/callback`

**Important:**
- This must match exactly what you register in GitHub OAuth App settings
- Must be HTTPS (GitHub requires HTTPS)
- Must include the full path: `/auth/github/login/callback`

**Auto-construction:**
If `GITHUB_OAUTH_REDIRECT_URL` is not set, the backend will try to construct it from `PUBLIC_BASE_URL`:
```
PUBLIC_BASE_URL + "/auth/github/login/callback"
```

---

### 3. `FRONTEND_BASE_URL`

**Purpose:** Where your frontend is running. The backend redirects users here after successful login.

**Local Development:**
- ✅ **Use localhost:** `http://localhost:5173`
- ❌ **Don't use tunnel URL** (unless your frontend is also on the tunnel)

**Production:**
- ✅ **Use production frontend:** `https://grainlify.com`

**Used for:**
- Redirecting users after OAuth login
- Redirecting users after GitHub App installation
- CORS configuration (if `CORS_ORIGINS` is not set)

---

## Example .env File

### Local Development (with tunnel)

```bash
# Backend Configuration
PORT=8080
APP_ENV=dev

# Database
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5432/grainlify?sslmode=disable

# GitHub OAuth
GITHUB_OAUTH_CLIENT_ID=your_client_id
GITHUB_OAUTH_CLIENT_SECRET=your_client_secret

# ✅ Use tunnel URL (GitHub needs to reach your backend)
PUBLIC_BASE_URL=https://abc123.loclx.io
GITHUB_OAUTH_REDIRECT_URL=https://abc123.loclx.io/auth/github/login/callback

# ✅ Use localhost (your actual frontend)
FRONTEND_BASE_URL=http://localhost:5173

# GitHub App
GITHUB_APP_ID=123456
GITHUB_APP_SLUG=grainlify
GITHUB_APP_PRIVATE_KEY=base64_encoded_key
GITHUB_WEBHOOK_SECRET=your_webhook_secret

# JWT
JWT_SECRET=your_jwt_secret
```

### Production

```bash
# Backend Configuration
PORT=8080
APP_ENV=production

# Database
DB_URL=postgresql://user:pass@host:5432/grainlify?sslmode=require

# GitHub OAuth
GITHUB_OAUTH_CLIENT_ID=your_client_id
GITHUB_OAUTH_CLIENT_SECRET=your_client_secret

# ✅ Use production backend
PUBLIC_BASE_URL=https://api.grainlify.com
GITHUB_OAUTH_REDIRECT_URL=https://api.grainlify.com/auth/github/login/callback

# ✅ Use production frontend
FRONTEND_BASE_URL=https://grainlify.com

# GitHub App
GITHUB_APP_ID=123456
GITHUB_APP_SLUG=grainlify
GITHUB_APP_PRIVATE_KEY=base64_encoded_key
GITHUB_WEBHOOK_SECRET=your_webhook_secret

# JWT
JWT_SECRET=your_jwt_secret
```

---

## Common Mistakes

### ❌ Mistake 1: Using localhost for PUBLIC_BASE_URL

```bash
# ❌ WRONG - GitHub can't reach localhost
PUBLIC_BASE_URL=http://localhost:8080
```

**Fix:** Use tunnel URL for local development.

### ❌ Mistake 2: Using tunnel URL for FRONTEND_BASE_URL

```bash
# ❌ WRONG - Your frontend is on localhost, not tunnel
FRONTEND_BASE_URL=https://abc123.loclx.io
```

**Fix:** Use `http://localhost:5173` for local development.

### ❌ Mistake 3: Mismatch between .env and GitHub settings

```bash
# .env
GITHUB_OAUTH_REDIRECT_URL=https://abc123.loclx.io/auth/github/login/callback
```

But in GitHub OAuth App settings, you registered:
```
http://localhost:8080/auth/github/login/callback  # ❌ WRONG
```

**Fix:** Make sure they match exactly.

---

## How It Works

### OAuth Login Flow:

1. User clicks "Sign In" → Frontend calls `POST /auth/github/login/start`
2. Backend constructs OAuth URL using `GITHUB_OAUTH_REDIRECT_URL`
3. User redirected to GitHub → `https://github.com/login/oauth/authorize?...&redirect_uri=https://abc123.loclx.io/auth/github/login/callback`
4. User authorizes → GitHub redirects to: `https://abc123.loclx.io/auth/github/login/callback?code=...`
5. Backend processes callback → Exchanges code for token
6. Backend redirects user to: `FRONTEND_BASE_URL + "/dashboard"` → `http://localhost:5173/dashboard`

### GitHub App Installation Flow:

1. User clicks "Install GitHub App" → Frontend calls `POST /auth/github/app/install/start`
2. Backend generates installation URL
3. User redirected to GitHub → `https://github.com/apps/grainlify/installations/new`
4. User installs app → GitHub redirects to: `https://abc123.loclx.io/auth/github/app/install/callback?installation_id=123`
5. Backend processes callback
6. Backend redirects user to: `FRONTEND_BASE_URL + "/dashboard"` → `http://localhost:5173/dashboard`

---

## Summary

| Variable | Local Dev | Production | Used By |
|----------|-----------|------------|---------|
| `PUBLIC_BASE_URL` | Tunnel URL | Production backend | GitHub (webhooks, callbacks) |
| `GITHUB_OAUTH_REDIRECT_URL` | Tunnel URL + path | Production backend + path | GitHub OAuth |
| `FRONTEND_BASE_URL` | localhost | Production frontend | Backend (redirects users) |

**Key Rule:** If GitHub needs to reach it → use tunnel/production URL. If it's just for internal redirects → use localhost (local) or production frontend.















