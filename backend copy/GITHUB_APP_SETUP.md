# GitHub App Setup Guide

This guide walks you through creating and configuring a GitHub App for Grainlify.

## Step 1: Create GitHub App

1. Go to: **GitHub → Settings → Developer settings → GitHub Apps**
2. Click **"New GitHub App"**

**Note:** The app will be "private" by default, which is fine. Private apps can still be installed by users - they just won't appear in the GitHub Marketplace. This is actually recommended for most use cases.

## Step 2: Fill in the Form

### Basic Information

**GitHub App name:**
```
Grainlify
```
(Or your preferred name - this is what users will see)

**Description:**
```markdown
Grainlify helps maintainers manage open-source projects, track contributions, and connect with contributors. Install this app to sync your repositories and manage issues and pull requests.
```

**Homepage URL:**
```
https://grainlify.com
```
(Or your production domain)

### Identifying and Authorizing Users

**Callback URL:**
```
https://your-backend-domain.com/auth/github/app/install/callback
```
**Important:** Replace `your-backend-domain.com` with your actual backend URL (e.g., `https://api.grainlify.com` or your Railway/Heroku URL)

⚠️ **For Local Development:** Use a tunneling service (same as webhook URL):
```
https://abc123.loclx.io/auth/github/app/install/callback
```
(Replace `abc123.loclx.io` with your actual tunnel URL)

**Expire user authorization tokens:**
- ✅ **Checked** (Recommended: enables refresh tokens)

**Request user authorization (OAuth) during installation:**
- ⬜ **Unchecked** (Not needed for our use case)

**Enable Device Flow:**
- ⬜ **Unchecked** (Not needed)

### Post Installation

**Setup URL (optional):**
```
https://your-backend-domain.com/auth/github/app/install/callback
```
⚠️ **Important:** 
- **Option 1 (Recommended):** Leave this **EMPTY** - GitHub will use the "Callback URL" from "Identifying and authorizing users" section
- **Option 2:** Set it to the same callback URL as above (must match exactly)
- **Don't set it to frontend URL** - This can cause `missing_installation_id` errors

**Redirect on update:**
- ✅ **Checked** (Recommended: redirect when repos are added/removed)

### Webhook

**Active:**
- ✅ **Checked**

**Webhook URL:**
```
https://your-backend-domain.com/webhooks/github
```
**Important:** Replace `your-backend-domain.com` with your actual backend URL

⚠️ **For Local Development:** GitHub cannot reach `localhost`. You MUST use a tunneling service:

**Option 1: Using loclx (Recommended)**
```bash
# Install loclx if not already installed
# Then create a tunnel
loclx tunnel http --to localhost:8080

# This will give you a URL like: https://abc123.loclx.io
# Use this for webhook URL:
https://abc123.loclx.io/webhooks/github
```

**Option 2: Using ngrok**
```bash
# Install ngrok, then:
ngrok http 8080

# Use the HTTPS URL provided:
https://abc123.ngrok.io/webhooks/github
```

**Important:** 
- Use the **HTTPS** URL (not HTTP)
- The tunnel must be running when GitHub tries to deliver webhooks
- Update the webhook URL in GitHub App settings whenever you restart the tunnel (URL changes)

**Secret:**
```
<Generate a NEW random secret string>
```
**⚠️ Important:** This is NOT the same as your GitHub OAuth Client Secret!

**Generate a new random secret:**
```bash
openssl rand -hex 32
```

**Or use any secure random string generator:**
- Should be at least 32 characters
- Random and unpredictable
- Different from your OAuth Client Secret

**Example output:**
```
a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6
```

**Save this value** - you'll need it for `GITHUB_WEBHOOK_SECRET` environment variable.

**Why a separate secret?**
- Webhook secret verifies that webhook requests are actually from GitHub
- OAuth Client Secret is for user authentication flows
- They serve different security purposes

### Permissions

#### Repository Permissions

**Contents:**
- **Read-only** ✅
- Description: "Access commits, branches, and code to sync project data"

**Issues:**
- **Read & write** ✅
- Description: "Manage contributor applications and issue assignments"

**Pull requests:**
- **Read-only** ✅
- Description: "Track contributions and project activity"

**Metadata:**
- **Read-only** ✅ (Usually auto-selected)
- Description: "Access repository metadata"

#### Organization Permissions

**Members:**
- **Read-only** ✅
- Description: "Verify organization membership and roles"

#### Account Permissions

**None required** - Leave all as "No access"

### Subscribe to Events

Based on the permissions selected, subscribe to:

- ✅ **Installation** - "Installation created, deleted, or new permissions accepted"
- ✅ **Installation repositories** - "Repositories added or removed from installation"
- ✅ **Issues** - "Issue opened, edited, deleted, transferred, pinned, unpinned, closed, reopened, assigned, unassigned, labeled, unlabeled, locked, unlocked, milestoned, or demilestoned"
- ✅ **Pull request** - "Pull request opened, edited, closed, merged, synchronized, ready for review, locked, unlocked, a pull request review was requested, or a review request was removed"
- ✅ **Push** - "One or more commits pushed to a repository"
- ✅ **Repository** - "Repository created, deleted, archived, unarchived, publicized, or privatized"

### Where can this GitHub App be installed?

**⚠️ IMPORTANT - Select:**
- ✅ **Any account** - "Allow this GitHub App to be installed by any user or organization"

**DO NOT select:**
- ❌ **Only on this account** - This restricts installation to only the app creator

**Why:** If you select "Only on this account", other users won't be able to install the app. You MUST select "Any account" to allow your users to install it.

## Step 3: Create the App

Click **"Create GitHub App"**

## Step 4: Generate and Save Private Key

1. **Generate a private key**
   - Click **"Generate a private key"** button
   - Download the `.pem` file (e.g., `grainlify.private-key.pem`)
   - **⚠️ Important:** This file can only be downloaded once! Save it securely.

2. **Base64 encode the private key**
   ```bash
   # On macOS/Linux:
   base64 -i grainlify.private-key.pem
   
   # Or using cat and base64:
   cat grainlify.private-key.pem | base64
   
   # The output will be a long base64 string - copy the entire output
   ```

3. **Save the base64 string** - You'll need this for `GITHUB_APP_PRIVATE_KEY` environment variable

## Step 5: Configure IP Allow List (Optional)

**When is this needed?**
- Only if organizations that install your app have IP allow lists enabled
- Usually **NOT required** for most use cases
- Skip this step if you're unsure

**If you need to add IPs:**

1. **For local development:** Skip (tunnels use dynamic IPs)
2. **For production:** Add your server's IP addresses
   - If using Railway/Heroku: They provide IP ranges (check their docs)
   - If using your own server: Add your server's public IP
   - Format: `192.168.1.1/32` (single IP) or `192.168.1.0/24` (IP range)

**Example for Railway:**
```
IP address: 0.0.0.0/0
Description: Allow all (if your hosting provider doesn't provide specific IPs)
```

**Note:** Most hosting providers (Railway, Heroku, AWS, etc.) don't require this unless you're behind a specific firewall.

## Step 6: Save Important Information

After creating the app, save these values:

1. **App ID** (numeric, e.g., `123456`)
   - Found in the app settings page
   - Save for `GITHUB_APP_ID` environment variable

2. **App slug** (e.g., `grainlify`)
   - Found in the URL: `github.com/apps/grainlify`
   - Save for `GITHUB_APP_SLUG` environment variable

3. **Private Key** (base64 encoded)
   - From Step 4 above
   - Save for `GITHUB_APP_PRIVATE_KEY` environment variable

## Step 7: Configure Environment Variables

Add these to your backend `.env` file or deployment platform:

```bash
# GitHub App Configuration
GITHUB_APP_ID=123456
GITHUB_APP_SLUG=grainlify
GITHUB_APP_PRIVATE_KEY=<base64-encoded-private-key>

# Webhook Secret (from Step 2)
GITHUB_WEBHOOK_SECRET=<your-webhook-secret>

# Public Base URL (for webhook URL)
PUBLIC_BASE_URL=https://your-backend-domain.com

# Frontend Base URL (for redirects)
FRONTEND_BASE_URL=https://grainlify.com
```

## Step 8: Update Callback URL in GitHub App Settings

After deploying your backend:

1. Go back to your GitHub App settings
2. Update the **Callback URL** to match your production URL:
   ```
   https://your-production-backend.com/auth/github/app/install/callback
   ```
3. Update the **Webhook URL** to match your production URL:
   ```
   https://your-production-backend.com/webhooks/github
   ```

## Testing Locally

### Step 1: Start Your Backend
```bash
cd backend
go run ./cmd/api
# Or use air for auto-reload:
make dev
```

### Step 2: Create a Tunnel

**Using loclx:**
```bash
# Start tunnel (keep this running)
loclx tunnel http --to localhost:8080

# You'll get output like:
# Forwarding https://abc123.loclx.io -> http://localhost:8080
```

**Using ngrok:**
```bash
ngrok http 8080
# You'll get output like:
# Forwarding https://abc123.ngrok.io -> http://localhost:8080
```

### Step 3: Update GitHub App Settings

1. Go to your GitHub App settings
2. Update **Callback URL** to:
   ```
   https://abc123.loclx.io/auth/github/app/install/callback
   ```
   (Replace with your actual tunnel URL)

3. Update **Webhook URL** to:
   ```
   https://abc123.loclx.io/webhooks/github
   ```

4. Click **"Update GitHub App"**

### Step 4: Test Installation

1. Keep both backend and tunnel running
2. Go to your frontend and click "Add a repository"
3. Click "Install GitHub App"
4. You should be redirected to GitHub
5. After installation, GitHub will redirect back to your callback URL

### Important Notes

- ⚠️ **Tunnel URL changes** each time you restart the tunnel (with free services)
- ⚠️ **Keep tunnel running** while testing - if it stops, webhooks will fail
- ✅ For production, use a permanent domain (Railway, Heroku, etc.)
- ✅ Production URLs don't change, so webhooks work reliably

## Verification Checklist

- [ ] GitHub App created
- [ ] App ID saved
- [ ] App slug saved
- [ ] Private key generated and base64 encoded
- [ ] Webhook secret generated
- [ ] Callback URL configured
- [ ] Webhook URL configured
- [ ] Permissions set correctly
- [ ] Events subscribed
- [ ] Environment variables set
- [ ] Backend deployed with new environment variables
- [ ] Test installation flow

## Troubleshooting

### "Invalid callback URL"
- Ensure the callback URL exactly matches what's in GitHub App settings
- Check for trailing slashes
- Verify HTTPS (required for production)

### "Webhook delivery failed"
- Verify webhook URL is accessible
- Check webhook secret matches `GITHUB_WEBHOOK_SECRET`
- Ensure backend is running and `/webhooks/github` endpoint exists

### "Installation failed"
- Verify `GITHUB_APP_ID` and `GITHUB_APP_SLUG` are correct
- Check private key is properly base64 encoded
- Ensure backend has access to GitHub API

## Next Steps

After setup:
1. Test the installation flow from the frontend
2. Verify repositories are synced after installation
3. Check webhook deliveries in GitHub App settings
4. Monitor backend logs for any errors

---

**Last Updated:** 2025-01-01

