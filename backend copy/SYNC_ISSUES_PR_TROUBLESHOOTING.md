# Troubleshooting: Issues and PRs Not Syncing

## Problem

Repositories are showing as "verified" but issues and pull requests are not being fetched.

## Root Cause

The sync worker requires:
1. **User must have linked GitHub account** via OAuth (not just GitHub App installation)
2. **OAuth token must have access** to the repositories
3. **Sync worker must be running**

## Solution

### Step 1: Check if User Has Linked GitHub Account

The sync worker uses the user's OAuth token, not the GitHub App installation token. The user needs to:

1. **Link their GitHub account via OAuth:**
   - Go to your app
   - Sign in with GitHub (this links the account)
   - Or use the "Link GitHub Account" feature if available

2. **Verify the OAuth token has access:**
   - The token needs `repo` scope to access private repositories
   - For public repos, `public_repo` scope is sufficient

### Step 2: Check Sync Worker is Running

Check your backend logs for:
```
background worker started
```

If you don't see this, the sync worker is not running. The worker starts automatically when:
- `NATS_URL` is **NOT** set (runs in-process)
- Database is connected

### Step 3: Check Sync Jobs Status

Check if sync jobs are being created and processed:

**In backend logs, look for:**
```
enqueued sync jobs for existing project
starting sync job
sync job completed successfully
```

**Or check for errors:**
```
sync job failed: GitHub account not linked
sync job failed: github_not_linked
```

### Step 4: Check Backend Logs

After installing the GitHub App, you should see:

1. **Repository sync:**
   ```
   verified existing project from GitHub App installation
   enqueued sync jobs for existing project
   ```

2. **Sync worker processing:**
   ```
   starting sync job
   job_type=sync_issues
   repo=owner/repo
   ```

3. **Sync completion:**
   ```
   sync issues completed
   total_issues=X
   ```

## Common Issues

### Issue 1: "GitHub account not linked"

**Error in logs:**
```
sync job failed: GitHub account not linked
error=github_not_linked
```

**Fix:**
- User must sign in with GitHub (OAuth) to link their account
- The GitHub App installation alone is not enough

### Issue 2: "Token doesn't have access"

**Error in logs:**
```
failed to fetch issues page
error=github repo fetch failed: status 404
```

**Fix:**
- User's OAuth token needs `repo` scope for private repos
- Re-link GitHub account with proper scopes

### Issue 3: Sync Worker Not Running

**No logs about sync jobs being processed**

**Fix:**
- Check if `NATS_URL` is set (if set, worker runs separately)
- Restart backend server
- Check logs for "background worker started"

### Issue 4: Sync Jobs Stuck in "pending"

**Jobs are created but never processed**

**Fix:**
- Check if sync worker is running
- Check for errors in worker logs
- Verify database connection

## Manual Sync Trigger

You can manually trigger a sync for a project:

```bash
POST /projects/:id/sync
Authorization: Bearer <token>
```

This will enqueue sync jobs for issues and PRs.

## Check Sync Job Status

Check the status of sync jobs for a project:

```bash
GET /projects/:id/sync/jobs
Authorization: Bearer <token>
```

This shows:
- Job status (pending, running, completed, failed)
- Error messages if any
- Number of attempts

## Architecture Note

**Current Implementation:**
- GitHub App installation → Creates projects, verifies them
- Sync jobs → Use user's OAuth token (not installation token)
- Sync worker → Processes jobs using OAuth token

**Why:**
- The sync worker architecture was designed for OAuth tokens
- Installation tokens are app-level, OAuth tokens are user-level
- For now, users need both: GitHub App installation + OAuth account link

**Future Improvement:**
- Could modify sync worker to use installation tokens when available
- Would require storing installation_id with projects

---

## Quick Checklist

- [ ] User has signed in with GitHub (OAuth link exists)
- [ ] Sync worker is running (check logs for "background worker started")
- [ ] Sync jobs are being created (check logs for "enqueued sync jobs")
- [ ] Sync jobs are being processed (check logs for "starting sync job")
- [ ] No errors in sync worker logs
- [ ] OAuth token has `repo` scope for private repos















