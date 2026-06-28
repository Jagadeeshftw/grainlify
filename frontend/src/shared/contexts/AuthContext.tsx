import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
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

interface AuthContextType {
  userRole: UserRole;
  userId: string | null;
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (token: string) => Promise<void>;
  logout: () => void;
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

export function AuthProvider({ children }: { children: ReactNode }) {
  const [userRole, setUserRole] = useState<UserRole>(AUTH_BYPASS_ENABLED ? 'contributor' : null);
  const [userId, setUserId] = useState<string | null>(AUTH_BYPASS_ENABLED ? BYPASS_USER.id : null);
  const [user, setUser] = useState<User | null>(AUTH_BYPASS_ENABLED ? BYPASS_USER : null);
  const [isLoading, setIsLoading] = useState(!AUTH_BYPASS_ENABLED);

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
  }, []);

  // Keep auth state in sync when token changes (logout in same tab, 401s, etc).
  useEffect(() => {
    const onTokenEvent = (e: Event) => {
      const ce = e as CustomEvent<{ token: string | null }>;
      const token = ce.detail?.token ?? null;
      if (!token) {
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
  }, []);

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
    } catch (error) {
      console.error('AuthContext - Login failed:', error);
      removeAuthToken();
      throw error;
    }
  };

  const logout = () => {
    removeAuthToken();
    setUser(null);
    setUserRole(null);
    setUserId(null);
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
