# Verify GitHub App Callback URL

## Current Status

✅ **Installation Start Working:**
- Backend is generating installation URL correctly
- Expected callback URL: `https://ybbppz5u6r.loclx.io/auth/github/app/install/callback`

❌ **Callback Not Received:**
- GitHub is not redirecting back after installation
- No callback logs in backend

## Step 1: Verify Callback URL in GitHub App Settings

1. Go to: **https://github.com/settings/apps**
2. Click on **"Grainlify"** app
3. Scroll to **"Identifying and authorizing users"** section
4. Check the **"Callback URL"** field

**It MUST be set to:**
```
https://ybbppz5u6r.loclx.io/auth/github/app/install/callback
```

**Important:**
- ✅ Must be HTTPS (not HTTP)
- ✅ Exact path: `/auth/github/app/install/callback`
- ✅ No trailing slash
- ✅ Must match exactly (case-sensitive)

## Step 2: Test if Callback Endpoint is Reachable

Run this command to test if GitHub can reach your callback URL:

```bash
curl -I "https://ybbppz5u6r.loclx.io/auth/github/app/install/callback?installation_id=test&state=test"
```

**Expected response:**
- Status code: `200` or `302` (redirect)
- If you get `404` or connection error, the tunnel might be down

## Step 3: Check Tunnel Status

Make sure your tunnel is still running:

```bash
# Check if tunnel is active
loclx tunnel list

# If not running, start it:
loclx tunnel http --to localhost:8080
```

**Note:** Tunnel URLs can change if you restart the tunnel. If the URL changed, update it in:
1. GitHub App settings (Callback URL)
2. Backend `.env` file (`PUBLIC_BASE_URL`)

## Step 4: Verify Setup URL (Optional)

In GitHub App settings → **"Post Installation"** section:

**"Setup URL (optional)":**
- ⬜ **Leave EMPTY** (recommended)
- OR set to same as callback URL: `https://ybbppz5u6r.loclx.io/auth/github/app/install/callback`

**"Redirect on update":**
- ✅ **Checked**

## Step 5: Test Installation Flow

1. **Start backend** (if not running):
   ```bash
   cd backend
   go run ./cmd/api
   ```

2. **Start tunnel** (if not running):
   ```bash
   loclx tunnel http --to localhost:8080
   ```

3. **Verify callback URL in GitHub App settings** matches tunnel URL

4. **Try installation again:**
   - Go to your frontend
   - Click "Install GitHub App"
   - Complete installation on GitHub
   - Watch backend logs for callback

## Expected Backend Logs

**When installation starts:**
```
GitHub App installation started
  user_id=...
  expected_callback_url=https://ybbppz5u6r.loclx.io/auth/github/app/install/callback
```

**After installing on GitHub (if callback URL is correct):**
```
GitHub App installation callback received
  method=GET
  path=/auth/github/app/install/callback
  query_params=map[installation_id:123 state:...]
```

## Troubleshooting

### Issue: Callback URL Not Set

**Symptom:** No callback received, GitHub doesn't redirect

**Fix:** Set callback URL in GitHub App settings → "Identifying and authorizing users" → "Callback URL"

### Issue: Callback URL Mismatch

**Symptom:** GitHub redirects but to wrong URL

**Fix:** Make sure callback URL in GitHub App settings matches exactly:
```
https://ybbppz5u6r.loclx.io/auth/github/app/install/callback
```

### Issue: Tunnel Down

**Symptom:** `curl` command fails, connection refused

**Fix:** Restart tunnel:
```bash
loclx tunnel http --to localhost:8080
```

**Note:** If tunnel URL changes, update it in GitHub App settings!

### Issue: Backend Not Running

**Symptom:** `curl` returns connection error

**Fix:** Start backend:
```bash
cd backend
go run ./cmd/api
```

## Quick Checklist

- [ ] Callback URL is set in GitHub App settings
- [ ] Callback URL is: `https://ybbppz5u6r.loclx.io/auth/github/app/install/callback`
- [ ] Tunnel is running and accessible
- [ ] Backend is running on `localhost:8080`
- [ ] `curl` test to callback URL succeeds
- [ ] Try installation again and watch for callback logs

---

**Most Common Issue:** Callback URL is not set in GitHub App settings. GitHub requires this to be explicitly configured - it won't use a URL from your code.















