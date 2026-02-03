import { BrowserRouter, Routes, Route, Navigate, useLocation } from "react-router-dom";
import { AuthProvider, useAuth } from "../shared/contexts/AuthContext";
import { ThemeProvider } from "../shared/contexts/ThemeContext";
import { LandingPage } from "../features/landing";
import { SignInPage, SignUpPage, AuthCallbackPage } from "../features/auth";
import { Dashboard } from "../features/dashboard";
import Toast from "../shared/components/Toast";

function ProtectedRoute({ children }: { children: JSX.Element }) {
  const { isAuthenticated, isLoading } = useAuth();
  const location = useLocation();
  if (isLoading) return children; // let AuthProvider finish initial check
  if (!isAuthenticated) {
    const returnTo = location.pathname + (location.search || "");
    const signinUrl = returnTo ? `/signin?returnTo=${encodeURIComponent(returnTo)}` : "/signin";
    return <Navigate to={signinUrl} replace />;
  }
  return children;
}

export default function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <AuthProvider>
          <div className="overflow-x-hidden">
            <Routes>
              <Route path="/" element={<LandingPage />} />
              <Route path="/signin" element={<SignInPage />} />
              <Route path="/signup" element={<SignUpPage />} />
              <Route path="/auth/callback" element={<AuthCallbackPage />} />
              <Route
                path="/dashboard"
                element={
                  <ProtectedRoute>
                    <Dashboard />
                  </ProtectedRoute>
                }
              />
            </Routes>
            <Toast />
          </div>
        </AuthProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
