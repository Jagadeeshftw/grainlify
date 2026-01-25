# Backend API Integration

This document describes the frontend integration with the Patchwork backend API.

## Backend URL

**Current Backend:** `http://7nonainmv1.loclx.io`

To change the backend URL, edit `/src/shared/config/api.ts`:

```typescript
export const API_BASE_URL = 'http://your-backend-url';
```

## Authentication Flow

The application uses **GitHub OAuth** for authentication. There is no traditional email/password signup or signin.

### How It Works

1. **User Clicks "Sign In/Sign Up"**
   - Both signin and signup redirect to the same GitHub OAuth flow
   - No role selection is needed upfront - roles are assigned by the backend

2. **GitHub OAuth Flow**
   - User is redirected to `http://7nonainmv1.loclx.io/auth/github/login/start`
   - User authorizes Grainlify on GitHub
   - GitHub redirects back to backend's callback endpoint
   - Backend processes OAuth and redirects to frontend: `/auth/callback?token=<jwt_token>`

3. **Frontend Callback Handling**
   - The `/auth/callback` route extracts the JWT token from URL
   - Token is stored in localStorage as `patchwork_jwt`
   - User info is fetched from `/me` endpoint
   - User is redirected to `/dashboard`

4. **Authenticated Requests**
   - All subsequent API calls include: `Authorization: Bearer <jwt_token>`
   - If token expires (401 response), user is redirected to signin

### Authentication State

The `AuthContext` manages authentication state:

- **`isAuthenticated`** - Whether user is logged in
- **`isLoading`** - Whether auth state is being checked
- **`userRole`** - User's role (contributor, maintainer, admin)
- **`userId`** - User's UUID
- **`login(token)`** - Store token and fetch user info
- **`logout()`** - Clear token and user state

## API Client

All API calls are centralized in `/src/shared/api/client.ts`.

### Example Usage

```typescript
import { getCurrentUser, getUserProfile } from '@/shared/api/client';

// Get current user (requires authentication)
const user = await getCurrentUser();
console.log(user.id, user.role);

// Get user profile
const profile = await getUserProfile();
console.log(profile.contributions_count);
```

### Available API Methods

#### Authentication
- `getCurrentUser()` - Get current user info (id, role)
- `getGitHubLoginUrl()` - Get GitHub OAuth URL
- `getGitHubStatus()` - Check if GitHub account is linked

#### User Profile
- `getUserProfile()` - Get contributions, languages, ecosystems
- `getProfileCalendar()` - Get 365-day contribution calendar
- `getProfileActivity(limit, offset)` - Get paginated activity feed

#### Projects
- `getPublicProjects(params)` - Get filtered list of projects
- `getProjectFilters()` - Get available filters (languages, tags, etc.)
- `getMyProjects()` - Get projects owned by user (maintainers)
- `createProject(data)` - Create a new project
- `verifyProject(id)` - Verify project ownership
- `syncProject(id)` - Sync project data from GitHub

#### Ecosystems
- `getEcosystems()` - Get list of ecosystems

#### KYC
- `startKYCVerification()` - Start KYC verification session
- `getKYCStatus()` - Get KYC verification status

## File Structure

```
/src/shared/
├── api/
│   ├── client.ts          # API client with all endpoints
│   └── index.ts           # Exports
├── config/
│   └── api.ts             # API configuration (base URL)
└── contexts/
    └── AuthContext.tsx    # Authentication state management

/src/app/pages/
├── AuthCallbackPage.tsx   # OAuth callback handler
└── ...

/src/features/auth/pages/
├── SignInPage.tsx         # GitHub OAuth signin
└── SignUpPage.tsx         # GitHub OAuth signup
```

## Token Storage

JWT tokens are stored in **localStorage** with the key `patchwork_jwt`.

⚠️ **Important Security Note:**
- This is suitable for development and prototyping
- For production, consider using httpOnly cookies for better security
- Never expose sensitive data in the JWT payload

## Error Handling

The API client automatically handles common errors:

- **401 Unauthorized** - Token expired/invalid → Clears token and redirects to signin
- **Other errors** - Throws error with message from backend

Example:
```typescript
try {
  const profile = await getUserProfile();
} catch (error) {
  console.error(error.message); // "Authentication failed. Please sign in again."
}
```

## CORS Configuration

The backend must allow requests from your frontend domain.

For development, make sure the backend allows:
- `http://localhost:5173` (or your Vite dev server port)
- Or configure CORS to allow all origins (development only)

## Next Steps

### To Complete Integration:

1. ✅ **Authentication** - Implemented (GitHub OAuth)
2. ⬜ **Profile Pages** - Fetch real data from `/profile` endpoints
3. ⬜ **Projects Browsing** - Fetch from `/projects` endpoint
4. ⬜ **Ecosystems** - Fetch from `/ecosystems` endpoint
5. ⬜ **Maintainer Dashboard** - Fetch from `/projects/mine` endpoint
6. ⬜ **KYC Verification** - Integrate KYC flow
7. ⬜ **Admin Panel** - Implement admin endpoints (if user is admin)

### Testing Authentication:

1. Start your frontend: `npm run dev`
2. Click "Sign In" or "Sign Up"
3. You'll be redirected to GitHub OAuth
4. After authorization, you'll be redirected back to `/auth/callback`
5. The app will extract the token and log you in
6. You'll be redirected to `/dashboard`

## Troubleshooting

### "Authentication failed" error
- Check if backend is running at the configured URL
- Verify CORS is configured correctly
- Check browser console for detailed errors

### OAuth redirect not working
- Verify backend's `PUBLIC_BASE_URL` includes your frontend URL
- Check that `/auth/callback` route is registered in your app

### Token expires immediately
- Check backend's JWT expiration settings
- Verify token is being stored correctly in localStorage

## Environment Variables (Optional)

For better configuration management, you can use environment variables:

```env
# .env.local
VITE_API_BASE_URL=http://7nonainmv1.loclx.io
```

Then update `/src/shared/config/api.ts`:
```typescript
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';
```
