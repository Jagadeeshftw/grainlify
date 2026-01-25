import { createContext, useContext, useState, ReactNode } from 'react';

export type UserRole = 'contributor' | 'maintainer' | 'admin' | null;

interface AuthContextType {
  userRole: UserRole;
  isAuthenticated: boolean;
  login: (role: UserRole) => void;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [userRole, setUserRole] = useState<UserRole>(null);

  const login = (role: UserRole) => {
    setUserRole(role);
  };

  const logout = () => {
    setUserRole(null);
  };

  return (
    <AuthContext.Provider
      value={{
        userRole,
        isAuthenticated: userRole !== null,
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