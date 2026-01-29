import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AuthProvider, useAuth } from "../shared/contexts/AuthContext";
import { ThemeProvider } from "../shared/contexts/ThemeContext";
import { LandingPage } from "../features/landing";
import { SignInPage, SignUpPage, AuthCallbackPage } from "../features/auth";
import { Dashboard } from "../features/dashboard";
import { BlogPage } from "../features/blog/pages/BlogPage";
import Toast from "../shared/components/Toast";
// import { Activity } from "lucide-react";

function ProtectedRoute({ children }: { children: JSX.Element }) {
  const { isAuthenticated, isLoading } = useAuth();
  if (isLoading) return children; // let AuthProvider finish initial check
  if (!isAuthenticated) return <Navigate to="/" replace />;
  return children;
}

export default function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <AuthProvider>
          <div className="overflow-x-hidden">
            {/* <Dashboard /> */}
            <Routes>
              <Route path="/" element={<LandingPage />} />
              <Route path="/blog" element={<BlogPage />} />
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
