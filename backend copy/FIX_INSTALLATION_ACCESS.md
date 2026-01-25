# Fix: Allow Other Users to Install GitHub App

## Problem

GitHub says "private app can't be installed by other users apart from the creator."

## Solution

You need to change the **"Where can this GitHub App be installed?"** setting.

## Steps to Fix

1. **Go to GitHub App Settings:**
   - GitHub → Settings → Developer settings → GitHub Apps
   - Click on your "Grainlify" app

2. **Scroll to "Where can this GitHub App be installed?" section**

3. **Change the setting:**
   - Currently: ⚠️ "Only on this account" (restricts to creator only)
   - Change to: ✅ **"Any account"** (allows any user/organization)

4. **Click "Update GitHub App"** at the bottom

## After Making the Change

- ✅ Other users can now install the app
- ✅ Organizations can install it
- ✅ The app remains "private" (not in marketplace) but is installable
- ✅ Installation flow will work for all users

## Verification

After updating:
1. Try the installation flow from your frontend
2. Or visit: `https://github.com/apps/grainlify/installations/new`
3. You should be able to select any organization/account to install it

## Important Notes

- **"Private"** (not in marketplace) is fine - it just means not listed publicly
- **"Any account"** setting is what allows others to install it
- These are two separate settings:
  - **Installation target** = Who can install (must be "Any account")
  - **Marketplace listing** = Private/Public (doesn't affect installation)

---

**TL;DR:** Change "Where can this GitHub App be installed?" from "Only on this account" to "Any account" in your GitHub App settings.















