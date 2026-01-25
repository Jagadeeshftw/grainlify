# GitHub App Callback URL Setup

## Problem: Not Redirecting After Installation

After installing the GitHub App, GitHub should redirect back to your website, but it's not happening.

## Solution: Configure Callback URL in GitHub App Settings

### Step 1: Go to GitHub App Settings

1. Go to: **GitHub → Settings → Developer settings → GitHub Apps**
2. Click on your **"Grainlify"** app
3. Scroll to **"Identifying and authorizing users"** section

### Step 2: Add Callback URL

In the **"Callback URL"** field, add:

**For Production:**
```
https://your-backend-domain.com/auth/github/app/install/callback
```

**For Local Development (with tunnel):**
```
https://abc123.loclx.io/auth/github/app/install/callback
```
(Replace `abc123.loclx.io` with your actual tunnel URL)

### Step 3: Important Notes

1. **Use HTTPS** - GitHub requires HTTPS for callbacks (not HTTP)
2. **Use Backend URL** - The callback goes to your **backend**, not frontend
3. **Exact Match** - The URL must match exactly what's in GitHub settings
4. **No Trailing Slash** - Don't add a trailing slash

### Step 4: Update and Save

1. Click **"Update GitHub App"** at the bottom
2. GitHub will verify the URL is reachable

## How It Works

1. User clicks "Install GitHub App" in your frontend
2. Frontend calls backend: `POST /auth/github/app/install/start`
3. Backend generates installation URL with state
4. User is redirected to GitHub: `https://github.com/apps/grainlify/installations/new`
5. User selects org/repos and clicks "Install"
6. **GitHub redirects to your callback URL:** `https://your-backend.com/auth/github/app/install/callback?installation_id=123&state=abc`
7. Backend processes the callback
8. **Backend redirects to frontend:** `https://your-frontend.com/dashboard?github_app_installed=true`

## Troubleshooting

### "Callback URL is not reachable"

- ✅ Make sure your backend is running
- ✅ Make sure the tunnel is running (for local dev)
- ✅ Use HTTPS (not HTTP)
- ✅ Check the URL matches exactly

### "Redirects to wrong place"

- ✅ Check `FRONTEND_BASE_URL` in your backend `.env`
- ✅ Backend redirects to: `FRONTEND_BASE_URL + "/dashboard"`

### "No redirect after installation"

- ✅ Verify callback URL is set in GitHub App settings
- ✅ Check backend logs for callback requests
- ✅ Make sure backend is accessible from the internet (use tunnel for local)

### "missing_installation_id" Error

If you see `{"error":"missing_installation_id"}` in the callback:

**Possible causes:**
1. **User cancelled installation** - User clicked "Cancel" on GitHub
2. **Direct URL access** - Someone accessed the callback URL directly (not from GitHub)
3. **Setup URL misconfiguration** - If "Setup URL" is set in GitHub App settings, it might interfere

**Solutions:**

1. **Check GitHub App Settings:**
   - Go to: GitHub → Settings → Developer settings → GitHub Apps → Your App
   - Scroll to **"Post Installation"** section
   - **"Setup URL (optional)"** - Leave this **EMPTY** or set it to your callback URL
   - **"Redirect on update"** - ✅ Check this

2. **Verify Callback URL:**
   - Make sure the callback URL in GitHub App settings matches exactly:
     ```
     https://your-tunnel-url.com/auth/github/app/install/callback
     ```
   - No trailing slash
   - Must be HTTPS

3. **Check Backend Logs:**
   - The backend now logs all query parameters when callback is received
   - Look for: `"GitHub App installation callback received"`
   - This will show what GitHub actually sent

4. **Try Installation Again:**
   - Go back to your dashboard
   - Click "Install GitHub App" again
   - Complete the installation on GitHub
   - Don't cancel or close the window

**Note:** The backend now handles missing `installation_id` gracefully by redirecting to the dashboard with a cancellation message.

## Environment Variables

Make sure these are set in your backend:

```bash
# Frontend URL (where backend redirects after installation)
FRONTEND_BASE_URL=http://localhost:5173  # or https://grainlify.com

# Public backend URL (for callback URL in GitHub)
PUBLIC_BASE_URL=http://localhost:8080  # or https://api.grainlify.com
```

## Testing

1. Set callback URL in GitHub App settings
2. Start your backend (and tunnel if local)
3. Try installation flow from frontend
4. After clicking "Install" on GitHub, you should be redirected back
5. Check backend logs to see if callback was received

---

**TL;DR:** Add the callback URL `https://your-backend.com/auth/github/app/install/callback` in GitHub App settings → "Identifying and authorizing users" section.

