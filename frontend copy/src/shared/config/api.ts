/**
 * API Configuration
 * 
 * All configuration comes from environment variables (VITE_* prefix for Vite)
 * 
 * Required environment variables:
 * - VITE_API_BASE_URL: Backend API URL (e.g., http://localhost:8080 or https://your-backend.com)
 * 
 * Optional environment variables:
 * - VITE_FRONTEND_BASE_URL: Frontend base URL (defaults to window.location.origin)
 */

// Get API base URL from environment variable
// In Vite, environment variables must be prefixed with VITE_ to be exposed to the client
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';

// Get frontend base URL from environment variable or use current origin
export const FRONTEND_BASE_URL = import.meta.env.VITE_FRONTEND_BASE_URL || window.location.origin;

/**
 * Frontend callback URL for OAuth
 * This is where GitHub will redirect after authentication
 */
export const OAUTH_CALLBACK_URL = `${FRONTEND_BASE_URL}/auth/callback`;