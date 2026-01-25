# Private Key Setup - Quick Guide

## You've Generated a Private Key - What's Next?

### Step 1: Locate Your Private Key File

The private key file was downloaded when you clicked "Generate a private key". It's typically named:
- `grainlify.private-key.pem`
- Or `grainlify-YYYY-MM-DD.private-key.pem`

**Location:** Usually in your Downloads folder

### Step 2: Base64 Encode the Private Key

Open terminal and run:

```bash
# Navigate to where you saved the .pem file
cd ~/Downloads  # or wherever you saved it

# Base64 encode the file
base64 -i grainlify.private-key.pem

# Or if that doesn't work:
cat grainlify.private-key.pem | base64
```

**Copy the entire output** - it will be a long string like:
```
LS0tLS1CRUdJTiBSU0EgUFJJVkFURSBLRVktLS0tLQpNSUlFcEFJQkFBS0NBUUVB...
(many more characters)
```

### Step 3: Add to Environment Variables

Add this to your `.env` file or deployment platform:

```bash
GITHUB_APP_PRIVATE_KEY=<paste-the-entire-base64-string-here>
```

**Important:**
- ✅ Paste the ENTIRE base64 string (it's very long)
- ✅ No line breaks or spaces
- ✅ No quotes needed (unless your env system requires them)

### Step 4: Verify It Works

After adding to environment variables and restarting your backend, check the logs. You should see:
- No errors about "private key" or "GITHUB_APP_PRIVATE_KEY"
- If there are errors, double-check the base64 encoding

### Troubleshooting

**Error: "invalid private key"**
- Make sure you copied the ENTIRE base64 string
- No line breaks or extra spaces
- Try re-encoding: `base64 -i your-key.pem`

**Error: "private key not configured"**
- Check environment variable name: `GITHUB_APP_PRIVATE_KEY`
- Restart your backend after adding the variable
- Verify the variable is loaded: `echo $GITHUB_APP_PRIVATE_KEY`

**Lost the private key?**
- You'll need to generate a new one in GitHub App settings
- Delete the old one and generate a new private key
- Re-encode and update your environment variables

---

## IP Allow List - Do You Need It?

### When You DON'T Need It:
- ✅ Most common use case
- ✅ Local development
- ✅ Most cloud hosting (Railway, Heroku, etc.)
- ✅ If organizations installing your app don't have IP allow lists

### When You DO Need It:
- ⚠️ Organizations that install your app have IP allow lists enabled
- ⚠️ Your backend is behind a specific firewall
- ⚠️ Your hosting provider requires it

### How to Add (If Needed):

1. **For Production Server:**
   - Find your server's public IP
   - Add: `YOUR_IP/32` (e.g., `203.0.113.1/32`)
   - Description: "Production server"

2. **For Cloud Hosting:**
   - Check your provider's documentation for IP ranges
   - Railway: Usually not needed (they handle it)
   - Heroku: Usually not needed
   - AWS: Check your VPC/security group IPs

3. **For Testing:**
   - You can skip this entirely
   - Only add if you get specific errors about IP allow lists

**Recommendation:** Start without adding any IPs. Only add them if you encounter issues or if organizations specifically require it.















