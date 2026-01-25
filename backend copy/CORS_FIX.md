# CORS Error Fix for Production

## Error

```
Access to fetch at 'https://api.grainlify.0xo.in/stats/landing' from origin 'https://grainlify.0xo.in' 
has been blocked by CORS policy: No 'Access-Control-Allow-Origin' header is present on the requested resource.
```

## Cause

The backend CORS configuration is not allowing requests from `https://grainlify.0xo.in` because:
- It's not a localhost origin
- It's not a `*.vercel.app` domain
- It's not in the `CORS_ORIGINS` environment variable
- `FRONTEND_BASE_URL` is not set to `https://grainlify.0xo.in`

## Solution

Set the `FRONTEND_BASE_URL` environment variable in your production backend:

```bash
FRONTEND_BASE_URL=https://grainlify.0xo.in
```

**OR** add it to `CORS_ORIGINS`:

```bash
CORS_ORIGINS=https://grainlify.0xo.in
```

## How CORS Works in This Backend

The backend allows requests from:

1. **localhost origins** - `http://localhost:*` or `https://localhost:*`
2. **Vercel preview deployments** - Any `*.vercel.app` domain
3. **Explicit CORS origins** - Origins listed in `CORS_ORIGINS` env var (comma-separated)
4. **FrontendBaseURL** - If `FRONTEND_BASE_URL` is set

## Production Environment Variables

Make sure your production backend has:

```bash
# Required for CORS to allow production frontend
FRONTEND_BASE_URL=https://grainlify.0xo.in

# OR use CORS_ORIGINS (comma-separated for multiple origins)
CORS_ORIGINS=https://grainlify.0xo.in,https://www.grainlify.0xo.in
```

## Verification

After setting the environment variable and restarting the backend:

1. Open browser console on `https://grainlify.0xo.in`
2. Try to fetch from API: `fetch('https://api.grainlify.0xo.in/stats/landing')`
3. Should not see CORS errors

## Multiple Origins

If you need to allow multiple production domains:

```bash
CORS_ORIGINS=https://grainlify.0xo.in,https://www.grainlify.0xo.in,https://app.grainlify.0xo.in
```

Note: `FRONTEND_BASE_URL` only supports one URL, but `CORS_ORIGINS` supports multiple (comma-separated).

