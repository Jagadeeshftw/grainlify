# OAuth Redirect Debugging Guide

## Issue: User redirected to localhost instead of production/preview URL

If users are being redirected to localhost (GitHub OAuth App Homepage URL) instead of the correct frontend URL, follow these steps:

## Step 1: Verify Migration is Applied

The `redirect_uri` column must exist in the `oauth_states` table. Check if migration `000024` has been applied:

```sql
-- Check if column exists
SELECT column_name 
FROM information_schema.columns 
WHERE table_name = 'oauth_states' AND column_name = 'redirect_uri';
```

If the column doesn't exist, run the migration:
```bash
cd backend
./cmd/migrate
```

Or if `AUTO_MIGRATE=true`, restart the backend to apply migrations.

## Step 2: Check Backend Logs

After implementing the logging changes, check backend logs when OAuth flow happens:

**Expected log messages:**

1. **When user clicks "Sign in":**
   ```
   OAuth login start - received redirect parameter redirect=https://grainlify.0xo.in
   OAuth login start - stored redirect_uri in state redirect_uri=https://grainlify.0xo.in state=...
   ```

2. **When GitHub redirects back:**
   ```
   OAuth callback - retrieved redirect_uri from state redirect_uri=https://grainlify.0xo.in kind=github_login
   OAuth redirect - using stored redirect_uri redirect_url=https://grainlify.0xo.in/auth/callback
   OAuth redirect - redirecting user final_redirect_url=https://grainlify.0xo.in/auth/callback?token=...
   ```

**If you see:**
- `OAuth callback - no redirect_uri in state` → The redirect parameter wasn't passed or stored
- `OAuth redirect - no redirect URL configured` → No fallback URL is configured

## Step 3: Verify Frontend is Passing Redirect

Check browser Network tab when clicking "Sign in with GitHub":

1. Look for request to: `/auth/github/login/start?redirect=...`
2. Verify the `redirect` parameter contains the correct frontend URL
3. Should be: `redirect=https%3A%2F%2Fgrainlify.0xo.in` (URL encoded)

**If redirect parameter is missing:**
- Check `frontend/src/shared/api/client.ts` - `getGitHubLoginUrl()` function
- Verify `window.location.origin` is working correctly

## Step 4: Verify Database Storage

Check if redirect_uri is being stored:

```sql
-- Check recent OAuth states
SELECT state, kind, redirect_uri, expires_at, created_at
FROM oauth_states
WHERE kind = 'github_login'
ORDER BY created_at DESC
LIMIT 5;
```

**Expected:** `redirect_uri` column should contain the frontend URL (e.g., `https://grainlify.0xo.in`)

**If NULL:** The redirect parameter wasn't passed from frontend or validation failed.

## Step 5: Check GitHub OAuth App Settings

**Homepage URL** should be set to production (but backend redirect should override it):
- ✅ `https://grainlify.0xo.in` (production)
- ❌ `http://localhost:5173` (wrong for production)

**Authorization callback URL** should be:
- ✅ `https://api.grainlify.0xo.in/auth/github/login/callback`

## Step 6: Test the Flow

1. **Clear browser cache/cookies**
2. **Open browser DevTools → Network tab**
3. **Click "Sign in with GitHub"**
4. **Check the request URL** - should include `?redirect=https://grainlify.0xo.in`
5. **After GitHub authorization**, check backend logs
6. **Verify final redirect** - should go to `https://grainlify.0xo.in/auth/callback?token=...`

## Common Issues

### Issue 1: Migration not applied
**Symptom:** `redirect_uri` column doesn't exist
**Fix:** Run migration `000024_add_redirect_uri_to_oauth_states.up.sql`

### Issue 2: Redirect parameter not passed
**Symptom:** Backend logs show no redirect parameter
**Fix:** Check frontend code, ensure `getGitHubLoginUrl()` includes redirect parameter

### Issue 3: Redirect URI validation failed
**Symptom:** Backend returns `redirect_uri_not_allowed` error
**Fix:** Ensure production domain is in `CORS_ORIGINS` or `FRONTEND_BASE_URL` is set

### Issue 4: Backend not redirecting
**Symptom:** User sees JSON response instead of redirect
**Fix:** Check backend logs, ensure redirect URL is constructed correctly

## Quick Fix Checklist

- [ ] Migration `000024` applied (redirect_uri column exists)
- [ ] Frontend passes `redirect` parameter in login URL
- [ ] Backend logs show redirect_uri being stored and retrieved
- [ ] Backend logs show redirect happening
- [ ] `FRONTEND_BASE_URL` or `GITHUB_LOGIN_SUCCESS_REDIRECT_URL` is set as fallback
- [ ] GitHub OAuth App Homepage URL is set to production (not localhost)

