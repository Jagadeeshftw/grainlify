import { createContext, useContext, useState, useEffect, useRef, useCallback, ReactNode } from 'react';
import { getCurrentUser, getAuthToken, setAuthToken, removeAuthToken } from '../api/client';
import { AUTH_BYPASS_ENABLED } from '../config/devAuth';

export type UserRole = 'contributor' | 'maintainer' | 'admin' | null;

export interface User {
  id: string;
  role: string;
  github: {
    login: string;
    avatar_url: string;
  };
}

// ---------------------------------------------------------------------------
// Session timeout types
// ---------------------------------------------------------------------------

export type SessionTimeoutState =
  | 'banner-hidden'
  | 'warning-visible'
  | 'critical'
  | 'expired';

/** Seconds before expiry at which the warning banner first appears. */
const WARNING_THRESHOLD_SECS = 5 * 60; // 5 minutes
/** Seconds before expiry at which the banner escalates to critical. */
const CRITICAL_THRESHOLD_SECS = 60; // 1 minute

// ---------------------------------------------------------------------------
// JWT helpers
// ---------------------------------------------------------------------------

/**
 * Decode the `exp` claim from a JWT without verifying the signature.
 * Returns the Unix timestamp (seconds) or null if the token is malformed.
 */
function getTokenExpiry(token: string): number | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    // Base64url → base64 → JSON
    const payload = atob(parts[1].replace(/-/g, '+').replace(/_/g, '/'));
    const json = JSON.parse(payload);
    if (typeof json.exp !== 'number') return null;
    return json.exp;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Context shape
// ---------------------------------------------------------------------------

interface AuthContextType {
  userRole: UserRole;
  userId: string | null;
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (token: string) => Promise<void>;
  logout: () => void;
  // Session timeout
  sessionTimeoutState: SessionTimeoutState;
  secondsRemaining: number;
  staySignedIn: () => Promise<void>;
  dismissTimeoutBanner: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

/** Synthetic contributor used only while the DEV auth bypass is enabled. */
const BYPASS_USER: User = {
  id: 'dev-bypass-user',
  role: 'contributor',
  github: {
    login: 'dev-contributor',
    avatar_url: 'https://avatars.githubusercontent.com/u/0?v=4',
  },
};

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export function AuthProvider({ children }: { children: ReactNode }) {
  const [userRole, setUserRole] = useState<UserRole>(AUTH_BYPASS_ENABLED ? 'contributor' : null);
  const [userId, setUserId] = useState<string | null>(AUTH_BYPASS_ENABLED ? BYPASS_USER.id : null);
  const [user, setUser] = useState<User | null>(AUTH_BYPASS_ENABLED ? BYPASS_USER : null);
  const [isLoading, setIsLoading] = useState(!AUTH_BYPASS_ENABLED);

  // Session timeout state
  const [sessionTimeoutState, setSessionTimeoutState] = useState<SessionTimeoutState>('banner-hidden');
  const [secondsRemaining, setSecondsRemaining] = useState(0);

  // Refs for timers so we can clear them reliably
  const warningTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const criticalTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const expiryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const countdownIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  // Track whether the user dismissed the banner during the warning window so we
  // do not re-show it until the critical window begins.
  const dismissedDuringWarningRef = useRef(false);

  // ---------------------------------------------------------------------------
  // Timer management
  // ---------------------------------------------------------------------------

  const clearSessionTimers = useCallback(() => {
    if (warningTimerRef.current) clearTimeout(warningTimerRef.current);
    if (criticalTimerRef.current) clearTimeout(criticalTimerRef.current);
    if (expiryTimerRef.current) clearTimeout(expiryTimerRef.current);
    if (countdownIntervalRef.current) clearInterval(countdownIntervalRef.current);
    warningTimerRef.current = null;
    criticalTimerRef.current = null;
    expiryTimerRef.current = null;
    countdownIntervalRef.current = null;
  }, []);

  /**
   * Start a 1-second countdown interval that keeps `secondsRemaining` in sync
   * and drives the banner copy.
   */
  const startCountdown = useCallback((expiryTimestamp: number) => {
    if (countdownIntervalRef.current) clearInterval(countdownIntervalRef.current);

    const tick = () => {
      const nowSecs = Math.floor(Date.now() / 1000);
      const remaining = Math.max(0, expiryTimestamp - nowSecs);
      setSecondsRemaining(remaining);
    };

    tick(); // immediate first tick
    countdownIntervalRef.current = setInterval(tick, 1000);
  }, []);

  /**
   * Given an expiry Unix timestamp, arm the three session-timeout timers.
   * Safe to call multiple times — clears previous timers first.
   */
  const armSessionTimers = useCallback(
    (expiryTimestamp: number) => {
      clearSessionTimers();
      dismissedDuringWarningRef.current = false;

      const nowSecs = Math.floor(Date.now() / 1000);
      const totalSecs = expiryTimestamp - nowSecs;

      if (totalSecs <= 0) {
        // Token already expired
        setSessionTimeoutState('expired');
        setSecondsRemaining(0);
        return;
      }

      // Warning timer (5 min before expiry)
      const warningDelaySecs = totalSecs - WARNING_THRESHOLD_SECS;
      if (warningDelaySecs > 0) {
        warningTimerRef.current = setTimeout(() => {
          if (!dismissedDuringWarningRef.current) {
            setSessionTimeoutState('warning-visible');
          }
          startCountdown(expiryTimestamp);
        }, warningDelaySecs * 1000);
      } else {
        // Already inside the warning window
        if (totalSecs > CRITICAL_THRESHOLD_SECS) {
          setSessionTimeoutState('warning-visible');
        }
        startCountdown(expiryTimestamp);
      }

      // Critical timer (1 min before expiry)
      const criticalDelaySecs = totalSecs - CRITICAL_THRESHOLD_SECS;
      if (criticalDelaySecs > 0) {
        criticalTimerRef.current = setTimeout(() => {
          setSessionTimeoutState('critical');
          // Ensure countdown is running if it wasn't already
          startCountdown(expiryTimestamp);
        }, criticalDelaySecs * 1000);
      } else if (totalSecs > 0) {
        // Already inside the critical window
        setSessionTimeoutState('critical');
        startCountdown(expiryTimestamp);
      }

      // Expiry timer
      expiryTimerRef.current = setTimeout(() => {
        clearSessionTimers();
        setSessionTimeoutState('expired');
        setSecondsRemaining(0);
        // Force logout — clear token and user state
        removeAuthToken();
        setUser(null);
        setUserRole(null);
        setUserId(null);
      }, totalSecs * 1000);
    },
    [clearSessionTimers, startCountdown],
  );

  // ---------------------------------------------------------------------------
  // Auth helpers
  // ---------------------------------------------------------------------------

  const checkAuth = async () => {
    // DEV-only: skip the real auth check and load a mock contributor.
    if (AUTH_BYPASS_ENABLED) {
      console.warn(
        '[AuthContext] DEV auth bypass active (VITE_AUTH_BYPASS=true) — signed in as a mock contributor. This branch is stripped from production builds.',
      );
      setUser(BYPASS_USER);
      setUserRole('contributor');
      setUserId(BYPASS_USER.id);
      setIsLoading(false);
      return;
    }

    const token = getAuthToken();
    console.log('AuthContext - Checking authentication on mount');
    console.log('AuthContext - Token found:', token ? 'Yes' : 'No');

    if (token) {
      try {
        console.log('AuthContext - Fetching user profile...');
        const userData = await getCurrentUser();
        console.log('AuthContext - User profile:', userData);
        setUser(userData);
        setUserRole(userData.role as UserRole);
        setUserId(userData.id);
        console.log('AuthContext - User authenticated:', {
          role: userData.role,
          id: userData.id,
          githubLogin: userData.github.login
        });

        // Arm session timeout timers
        const expiry = getTokenExpiry(token);
        if (expiry !== null) {
          armSessionTimers(expiry);
        }
      } catch (error) {
        // Token is invalid, remove it
        console.error('AuthContext - Auth check failed:', error);
        removeAuthToken();
        setUser(null);
        setUserRole(null);
        setUserId(null);
      }
    } else {
      console.log('AuthContext - No token found, user not authenticated');
      setUser(null);
      setUserRole(null);
      setUserId(null);
    }
    setIsLoading(false);
    console.log('AuthContext - Loading complete');
  };

  // Check for existing token on mount
  useEffect(() => {
    checkAuth();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Keep auth state in sync when token changes (logout in same tab, 401s, etc).
  useEffect(() => {
    const onTokenEvent = (e: Event) => {
      const ce = e as CustomEvent<{ token: string | null }>;
      const token = ce.detail?.token ?? null;
      if (!token) {
        clearSessionTimers();
        setSessionTimeoutState('banner-hidden');
        setUser(null);
        setUserRole(null);
        setUserId(null);
        return;
      }
      // Token was set/changed: refresh user.
      checkAuth();
    };

    const onStorage = (e: StorageEvent) => {
      if (e.key !== 'patchwork_jwt') return;
      if (!e.newValue) {
        clearSessionTimers();
        setSessionTimeoutState('banner-hidden');
        setUser(null);
        setUserRole(null);
        setUserId(null);
        return;
      }
      checkAuth();
    };

    window.addEventListener('patchwork-auth-token', onTokenEvent);
    window.addEventListener('storage', onStorage);
    return () => {
      window.removeEventListener('patchwork-auth-token', onTokenEvent);
      window.removeEventListener('storage', onStorage);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clearSessionTimers]);

  // Clean up timers on unmount
  useEffect(() => {
    return () => {
      clearSessionTimers();
    };
  }, [clearSessionTimers]);

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  const login = async (token: string) => {
    console.log('AuthContext - login() called with token');
    setAuthToken(token);
    console.log('AuthContext - Token saved to localStorage');

    try {
      console.log('AuthContext - Fetching user profile after login...');
      const userData = await getCurrentUser();
      console.log('AuthContext - User profile received:', userData);
      setUser(userData);
      setUserRole(userData.role as UserRole);
      setUserId(userData.id);
      console.log('AuthContext - Login successful:', {
        role: userData.role,
        id: userData.id,
        isAuthenticated: true,
        githubLogin: userData.github.login
      });

      // Arm session timeout timers on login
      const expiry = getTokenExpiry(token);
      if (expiry !== null) {
        armSessionTimers(expiry);
      }
    } catch (error) {
      console.error('AuthContext - Login failed:', error);
      removeAuthToken();
      throw error;
    }
  };

  const logout = () => {
    clearSessionTimers();
    setSessionTimeoutState('banner-hidden');
    setSecondsRemaining(0);
    removeAuthToken();
    setUser(null);
    setUserRole(null);
    setUserId(null);
  };

  /**
   * Refresh the session by re-validating the current token with the API.
   * On success the countdown timers reset; on failure the session expires.
   */
  const staySignedIn = async () => {
    try {
      const userData = await getCurrentUser();
      setUser(userData);
      setUserRole(userData.role as UserRole);
      setUserId(userData.id);

      // Re-read token (back-end may have rotated it via a response header)
      const currentToken = getAuthToken();
      const expiry = currentToken ? getTokenExpiry(currentToken) : null;
      if (expiry !== null) {
        armSessionTimers(expiry);
      }

      setSessionTimeoutState('banner-hidden');
    } catch {
      // Refresh failed — expire session immediately
      clearSessionTimers();
      setSessionTimeoutState('expired');
      setSecondsRemaining(0);
      removeAuthToken();
      setUser(null);
      setUserRole(null);
      setUserId(null);
    }
  };

  /**
   * Dismiss the warning banner without refreshing the token.
   * The banner will reappear when the critical threshold is reached.
   */
  const dismissTimeoutBanner = () => {
    if (sessionTimeoutState === 'warning-visible') {
      dismissedDuringWarningRef.current = true;
      setSessionTimeoutState('banner-hidden');
    }
  };

  return (
    <AuthContext.Provider
      value={{
        userRole,
        userId,
        user,
        isAuthenticated: AUTH_BYPASS_ENABLED || (!!user && !!getAuthToken()),
        isLoading,
        login,
        logout,
        sessionTimeoutState,
        secondsRemaining,
        staySignedIn,
        dismissTimeoutBanner,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
