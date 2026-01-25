# GitHub App Callback Not Working - Debug Guide

## Problem

After clicking "Install GitHub App", the user is redirected to GitHub, but GitHub is **not redirecting back** to your callback URL. No debug lines appear in the backend logs.

## Root Cause

GitHub App installations use a **different redirect mechanism** than OAuth:

- **OAuth:** You specify `redirect_uri` in the authorization URL
- **GitHub App:** GitHub uses the **"Callback URL"** from your GitHub App settings

## Solution: Verify Callback URL in GitHub App Settings

### Step 1: Check GitHub App Settings

1. Go to: **GitHub → Settings → Developer settings → GitHub Apps**
2. Click on your **"Grainlify"** app
3. Scroll to **"Identifying and authorizing users"** section
4. Check the **"Callback URL"** field

### Step 2: Required Callback URL

The callback URL **MUST** be set to:

```
https://ybbppz5u6r.loclx.io/auth/github/app/install/callback
```

(Replace with your actual tunnel URL)

### Step 3: Important Notes

1. **Must be HTTPS** - GitHub requires HTTPS (not HTTP)
2. **Exact path** - Must end with `/auth/github/app/install/callback`
3. **No trailing slash** - Don't add a trailing slash
4. **Must be reachable** - GitHub will verify the URL is accessible

### Step 4: Update GitHub App

1. If the callback URL is missing or incorrect, **update it**
2. Click **"Update GitHub App"** at the bottom
3. GitHub will verify the URL is reachable

## How GitHub App Installation Works

```
1. User clicks "Install GitHub App" in your frontend
   ↓
2. Frontend calls: POST /auth/github/app/install/start
   ↓
3. Backend generates installation URL: https://github.com/apps/grainlify/installations/new?state=abc123
   ↓
4. Frontend redirects user to GitHub installation page
   ↓
5. User selects org/repos and clicks "Install"
   ↓
6. GitHub redirects to: [Callback URL from GitHub App settings]?installation_id=123&state=abc123
   ↓
7. Backend processes callback at: /auth/github/app/install/callback
   ↓
8. Backend redirects user to: FRONTEND_BASE_URL/dashboard?github_app_installed=true
```

## Debugging Steps

### 1. Check Backend Logs

After clicking "Install GitHub App", you should see:

```
GitHub App installation started
  user_id=...
  app_slug=grainlify
  app_id=...
  state=...
  install_url=https://github.com/apps/grainlify/installations/new?state=...
  expected_callback_url=https://ybbppz5u6r.loclx.io/auth/github/app/install/callback
```

### 2. Check if Callback is Received

After installing on GitHub, check backend logs for:

```
GitHub App installation callback received
  method=GET
  path=/auth/github/app/install/callback
  query_params=...
```

**If you don't see this log:**
- GitHub is not redirecting to your callback URL
- The callback URL in GitHub App settings is incorrect or missing
- The tunnel might be down

### 3. Verify Tunnel is Running

```bash
# Check if tunnel is running
loclx tunnel list

# If not running, start it:
loclx tunnel http --to localhost:8080
```

### 4. Test Callback URL Directly

Try accessing the callback URL directly in your browser:

```
https://ybbppz5u6r.loclx.io/auth/github/app/install/callback?installation_id=test&state=test
```

You should see:
- Either a redirect to your frontend (if `installation_id` is valid)
- Or an error message (if `installation_id` is missing)

This confirms the endpoint is reachable.

## Common Issues

### Issue 1: Callback URL Not Set

**Symptom:** No callback received, no logs

**Fix:** Set the callback URL in GitHub App settings

### Issue 2: Callback URL Mismatch

**Symptom:** GitHub redirects but to wrong URL

**Fix:** Make sure callback URL in GitHub App settings matches exactly:
```
https://ybbppz5u6r.loclx.io/auth/github/app/install/callback
```

### Issue 3: Setup URL Interfering

**Symptom:** GitHub redirects to Setup URL instead of Callback URL

**Fix:** In GitHub App settings → "Post Installation" section:
- **"Setup URL (optional)"** - Leave **EMPTY** or set to same as callback URL
- Don't set it to your frontend URL

### Issue 4: Tunnel Not Running

**Symptom:** GitHub can't reach callback URL

**Fix:** Start your tunnel:
```bash
loclx tunnel http --to localhost:8080
```

## Verification Checklist

- [ ] Callback URL is set in GitHub App settings
- [ ] Callback URL is HTTPS (not HTTP)
- [ ] Callback URL path is: `/auth/github/app/install/callback`
- [ ] Tunnel is running and accessible
- [ ] Backend is running on `localhost:8080`
- [ ] Backend logs show "GitHub App installation started"
- [ ] After installation, backend logs show "GitHub App installation callback received"

## Next Steps

1. **Verify callback URL in GitHub App settings**
2. **Check backend logs** for "GitHub App installation started"
3. **Install the app on GitHub** and watch for callback logs
4. **If still not working**, check tunnel status and backend accessibility

---

**TL;DR:** The callback URL **MUST** be set in GitHub App settings → "Identifying and authorizing users" → "Callback URL". GitHub uses this URL, not a URL from your code.















