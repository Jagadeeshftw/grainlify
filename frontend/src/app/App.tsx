import React from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AuthProvider, useAuth } from "../shared/contexts/AuthContext";
import { ThemeProvider } from "../shared/contexts/ThemeContext";
import { LandingPage } from "../features/landing";
import { SignInPage, SignUpPage, AuthCallbackPage } from "../features/auth";
import { Dashboard } from "../features/dashboard";
import { ProfilePage } from "../features/dashboard/pages/ProfilePage";
import { LeaderboardPage } from "../features/leaderboard/pages/LeaderboardPage";
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider, useAuth } from '../shared/contexts/AuthContext';
import { ThemeProvider } from '../shared/contexts/ThemeContext';
import { LandingPage } from '../features/landing';
import { SignInPage, SignUpPage, AuthCallbackPage } from '../features/auth';
import { Dashboard } from '../features/dashboard';
import Toast from '../shared/components/Toast';

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
          <Routes>
            <Route path="/" element={<LandingPage />} />
            <Route path="/signin" element={<SignInPage />} />
            <Route path="/signup" element={<SignUpPage />} />
            <Route path="/auth/callback" element={<AuthCallbackPage />} />
            <Route path="/leaderboard" element={<LeaderboardPage />} />
            <Route
              path="/profile"
              element={
                <ProtectedRoute>
                  <ProfilePage />
                </ProtectedRoute>
              }
            />
            <Route
              path="/profile/:username"
              element={<ProfilePage />}
            />
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
        </AuthProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}