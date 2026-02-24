import { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { ArrowLeft, Github } from 'lucide-react';
import { getGitHubLoginUrl } from '../../../shared/api/client';

export function SignInPage() {
  const { theme } = useTheme();
  const navigate = useNavigate();
  const [isRedirecting, setIsRedirecting] = useState(false);

  // Persist returnTo so after OAuth we can redirect back to the intended page (e.g. dashboard?tab=browse&project=...&issue=...)
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const returnTo = params.get('returnTo');
    if (returnTo) sessionStorage.setItem('authReturnTo', returnTo);
  }, []);

  // Check for OAuth callback token in URL (fallback for wrong redirect URL)
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const token = params.get('token');
    
    if (token) {
      // If there's a token in the URL, redirect to the proper callback handler
      navigate(`/auth/callback?token=${token}`, { replace: true });
    }
  }, [navigate]);

  const handleGithubSign = async () => {
        setLoading(true);
            try {
                    const provider = new GithubAuthProvider();
                            console.log("sign in false ",false)
                                    const github1 = await signInWithPopup(auth, provider);
                                            console.log("Redirecting to :", github1);
                                                    // subject to github login
                                                            window.location.href = github1;

                                                                } catch (error) {
                                                                        console.log(error);
                                                                            }
                                                                            };

  

  return (
    <div className={`min-h-screen flex items-center justify-center px-6 relative overflow-hidden transition-colors ${
      theme === 'dark'
        ? 'bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]'
        : 'bg-gradient-to-br from-[#e8dfd0] via-[#d4c5b0] to-[#c9b89a]'
    }`}>
      {/* Background Effects */}
      <div className="absolute top-1/4 left-1/4 w-96 h-96 rounded-full bg-[#c9983a]/30 blur-3xl animate-pulse" />
      <div className="absolute bottom-1/4 right-1/4 w-96 h-96 rounded-full bg-[#d4af37]/20 blur-3xl animate-pulse" />

      {/* Back Button */}
      <Link
        to="/"
        className={`absolute top-6 left-6 flex items-center space-x-2 hover:text-[#c9983a] transition-colors font-medium ${
          theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
        }`}
      >
        <ArrowLeft className="w-5 h-5" />
        <span>Back to Home</span>
      </Link>

      {/* Sign In Form */}
      <div className="relative z-10 w-full max-w-md">
        <div className={`backdrop-blur-[40px] border rounded-[28px] p-8 shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.08] border-white/15'
            : 'bg-white/[0.15] border-white/25'
        }`}>
          {/* Header */}
          <div className="text-center mb-8">
            <div className="flex items-center space-x-3 justify-center mb-8">
              <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-[#c9983a] to-[#d4af37] shadow-[0_2px_8px_rgba(201,152,58,0.4)]" />
              <span className={`text-2xl font-semibold transition-colors ${
                theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
              }`}>Grainlify</span>
            </div>
            <h2 className={`text-3xl font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Welcome Back</h2>
            <p className={`transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Sign in with your GitHub account</p>
          </div>

          {/* GitHub Sign In */}
          <div className="space-y-6">
            <button
              onClick={handleGitHubSignIn}
              disabled={isRedirecting}
              className="w-full py-4 rounded-[12px] bg-[#24292e] hover:bg-[#1b1f23] disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium transition-all flex items-center justify-center space-x-3 border border-white/10 shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
            >
              {isRedirecting ? (
                <>
                  <div className="h-5 w-5 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                  <span>Redirecting...</span>
                </>
              ) : (
                <>
                  <Github className="w-6 h-6" />
                  <span>Sign in with GitHub</span>
                </>
              )}
            </button>

            <div className="relative">
              <div className="absolute inset-0 flex items-center">
                <div className="w-full border-t border-white/20"></div>
              </div>
              <div className="relative flex justify-center text-sm">
                <span className={`px-4 bg-transparent transition-colors ${
                  theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
                }`}>
                  Secure authentication via GitHub OAuth
                </span>
              </div>
            </div>

            <div className={`backdrop-blur-[25px] border rounded-[12px] p-4 transition-colors ${
              theme === 'dark'
                ? 'bg-white/[0.06] border-white/10'
                : 'bg-white/[0.12] border-white/20'
            }`}>
              <p className={`text-xs text-center transition-colors ${
                theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
              }`}>
                By signing in, you agree to share your public GitHub profile information.
                We never access your private repositories without explicit permission.
              </p>
            </div>
          </div>

          {/* Sign Up Link */}
          <p className={`text-center mt-6 transition-colors ${
            theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
          }`}>
            Don't have an account?{' '}
            <Link to="/signup" className="text-[#c9983a] hover:text-[#d4af37] font-medium">
              Sign Up
            </Link>
          </p>
        </div>
      </div>
    </div>
  );
}