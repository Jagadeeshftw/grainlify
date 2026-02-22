import { useEffect, useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../../shared/contexts/AuthContext';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function AuthCallbackPage() {
  const navigate = useNavigate();
  const { login, isAuthenticated } = useAuth();
  const { theme } = useTheme();
  const [error, setError] = useState<string | null>(null);
  const [isProcessing, setIsProcessing] = useState(true);
  const hasProcessed = useRef(false);

  // Redirect to dashboard (or returnTo from "review their application" link) once authenticated
  useEffect(() => {
    if (isAuthenticated && !error) {
      const returnTo = sessionStorage.getItem('authReturnTo');
      sessionStorage.removeItem('authReturnTo');
      if (returnTo && returnTo.startsWith('/dashboard')) {
        console.log('User is authenticated, redirecting to', returnTo);
        navigate(returnTo, { replace: true });
      } else {
        console.log('User is authenticated, redirecting to dashboard...');
        navigate('/dashboard', { replace: true });
      }
    }
  }, [isAuthenticated, error, navigate]);

  useEffect(() => {
    // Prevent running twice in React Strict Mode
    if (hasProcessed.current) {
      return;
    }
    
    const handleCallback = async () => {
      hasProcessed.current = true;
                                                                                                
                                                                                                                                                  
          

                                                                                                                                                            
                                                                                                                                          
                                                                                                                                                                                      

                                                                                                                                                                
                                                                                                                                                                                            
                                                                                                                                                                                                                  
                                                                                                                                                                                                                        
                                                                                                                                                                                                                                    
                                                                                                                                                                                                                                                      

                                                                                                                                                                                                                                          
                                                                                                                                                                                                                                                          
                                                                                                                                                                                                                                                                
                                                                                                                                                                                                                                                                    
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              
                                                                                                                                                                                                                                                                                          
                                                                                                                                                                                                                                                                                            
                                                                     
                                                                                                                      
                                                                                                                                        
                                                            
      
  try {
        // Get the token from URL parameters
        const params = new URLSearchParams(window.location.search);
        const token = params.get('token');
        const github = params.get('github');
        const errorParam = params.get('error');

        console.log('OAuth Callback - URL:', window.location.href);
        console.log('OAuth Callback - Token:', token ? 'Present' : 'Missing');
        console.log('OAuth Callback - GitHub Username:', github);
        console.log('OAuth Callback - Error:', errorParam);

        if (errorParam) {
          console.error('OAuth Error:', errorParam);
          if (errorParam === 'access_denied') {
                setError('Login was cancelled. Please try again.');
                } else {
                    setError(errorParam || 'An unexpected error occurred');
                    }
          }
          setIsProcessing(false);
          // Redirect to signin after 3 seconds
          setTimeout(() => navigate('/signin'), 3000);
          return;
        

        if (!token) {
          console.error('No token found in URL');
          setError('No authentication token received');
          setIsProcessing(false);
          setTimeout(() => navigate('/signin'), 3000);
          return;
        }

        // Login with the token
        console.log('Attempting login with token...');
        await login(token);
        console.log('Login successful! Auth state should update shortly...');
        setIsProcessing(false);
        // The redirect will happen via the useEffect watching isAuthenticated
      } catch (err) {
        console.error('Authentication failed:', err);
        setError(err instanceof Error ? err.message : 'Authentication failed');
        setIsProcessing(false);
        setTimeout(() => navigate('/signin'), 3000);
      }
    } 
  
    handleCallback();
 }, [login, navigate]);

  return (
    <div className={`min-h-screen flex items-center justify-center transition-colors ${
      theme === 'dark'
        ? 'bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]'
        : 'bg-gradient-to-br from-[#e8dfd0] via-[#d4c5b0] to-[#c9b89a]'
    }`}>
      <div className={`p-8 rounded-[24px] backdrop-blur-[40px] border max-w-md w-full mx-4 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.35] border-white'
      }`}>
        {error ? (
          <div className="text-center">
            <div className="mb-4">
              <svg
                className="mx-auto h-12 w-12 text-red-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </div>
            <h2 className={`text-xl mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Authentication Failed
            </h2>
            <p className={`text-sm transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              {error}
            </p>
            <p className={`text-xs mt-4 transition-colors ${
              theme === 'dark' ? 'text-[#a3a3a3]' : 'text-[#8a7d6f]'
            }`}>
              Redirecting to sign in...
            </p>
          </div>
        ) : isProcessing ? (
          <div className="text-center">
            <div className="mb-4">
              <div className="mx-auto h-12 w-12 border-4 border-[#c9983a] border-t-transparent rounded-full animate-spin"></div>
            </div>
            <h2 className={`text-xl mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Completing Authentication
            </h2>
            <p className={`text-sm transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Please wait while we sign you in...
            </p>
          </div>
        ) : (
          <div className="text-center">
            <div className="mb-4">
              <svg
                className="mx-auto h-12 w-12 text-green-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <h2 className={`text-xl mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Authentication Successful
            </h2>
            <p className={`text-sm transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Redirecting to dashboard...
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
