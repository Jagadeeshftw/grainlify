import { useState } from 'react';
import { X, CheckCircle2, FileText, Code, GitBranch, Users, Loader2 } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { API_BASE_URL } from '../../../shared/config/api';
import { getAuthToken } from '../../../shared/api/client';

interface InstallGitHubAppModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export function InstallGitHubAppModal({ isOpen, onClose, onSuccess }: InstallGitHubAppModalProps) {
  const { theme } = useTheme();
  const darkTheme = theme === 'dark';
  const [isInstalling, setIsInstalling] = useState(false);
  const [dontShowAgain, setDontShowAgain] = useState(false);

  const handleInstall = async () => {
    setIsInstalling(true);
    try {
      const token = getAuthToken();
      if (!token) {
        throw new Error('Please sign in to install the GitHub App');
      }

      // Get installation URL from backend
      const response = await fetch(`${API_BASE_URL}/auth/github/app/install/start`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || error.error || 'Failed to start installation');
      }

      const data = await response.json();
      
      // Store "don't show again" preference
      if (dontShowAgain) {
        localStorage.setItem('github_app_modal_dismissed', 'true');
      }

      // Redirect to GitHub App installation page
      window.location.href = data.install_url;
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to start installation');
      setIsInstalling(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[10000] flex items-center justify-center p-4">
      {/* Backdrop */}
      <div 
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className={`relative w-full max-w-[480px] rounded-[24px] border-2 shadow-[0_16px_64px_rgba(0,0,0,0.4)] transition-colors ${
        darkTheme
          ? 'bg-[#3a3228] border-white/30'
          : 'bg-[#d4c5b0] border-white/40'
      }`}>
        {/* Header */}
        <div className={`flex items-center justify-between px-5 py-4 border-b-2 transition-colors ${
          darkTheme ? 'border-white/20' : 'border-white/30'
        }`}>
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center">
              <span className="text-white text-base font-bold">G</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="text-[16px] font-bold text-[#c9983a]">→</span>
              <div className="w-7 h-7 rounded-full bg-[#24292e] flex items-center justify-center">
                <svg className="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
                </svg>
              </div>
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={isInstalling}
            className={`p-2 rounded-[10px] transition-all ${
              darkTheme
                ? 'hover:bg-white/10 text-[#b8a898] hover:text-[#e8dfd0]'
                : 'hover:bg-white/20 text-[#7a6b5a] hover:text-[#2d2820]'
            } ${isInstalling ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 space-y-4">
          {/* Title and Description */}
          <div>
            <h2 className={`text-[20px] font-bold mb-1.5 transition-colors ${
              darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
            }`}>
              Install Grainlify GitHub App
            </h2>
            <p className={`text-[13px] transition-colors ${
              darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              Install the Grainlify app on your GitHub organizations to manage repositories and sync data.
            </p>
          </div>

          {/* Required Permissions */}
          <div>
            <h3 className={`text-[15px] font-semibold mb-2.5 transition-colors ${
              darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
            }`}>
              Required permissions
            </h3>
            <div className="space-y-2.5">
              {/* Repository Contents */}
              <div className={`flex items-start gap-2.5 p-2.5 rounded-[10px] ${
                darkTheme ? 'bg-white/5' : 'bg-white/20'
              }`}>
                <FileText className={`w-4 h-4 flex-shrink-0 mt-0.5 ${
                  darkTheme ? 'text-[#c9983a]' : 'text-[#a2792c]'
                }`} />
                <div className="flex-1">
                  <p className={`text-[13px] font-semibold mb-0.5 transition-colors ${
                    darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                    Repository contents (read-only)
                  </p>
                  <p className={`text-[11px] transition-colors ${
                    darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    Access commits, branches, and code to sync project data
                  </p>
                </div>
              </div>

              {/* Issues */}
              <div className={`flex items-start gap-2.5 p-2.5 rounded-[10px] ${
                darkTheme ? 'bg-white/5' : 'bg-white/20'
              }`}>
                <Code className={`w-4 h-4 flex-shrink-0 mt-0.5 ${
                  darkTheme ? 'text-[#c9983a]' : 'text-[#a2792c]'
                }`} />
                <div className="flex-1">
                  <p className={`text-[13px] font-semibold mb-0.5 transition-colors ${
                    darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                    Issues (read & write)
                  </p>
                  <p className={`text-[11px] transition-colors ${
                    darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    Manage contributor applications and issue assignments
                  </p>
                </div>
              </div>

              {/* Pull Requests */}
              <div className={`flex items-start gap-2.5 p-2.5 rounded-[10px] ${
                darkTheme ? 'bg-white/5' : 'bg-white/20'
              }`}>
                <GitBranch className={`w-4 h-4 flex-shrink-0 mt-0.5 ${
                  darkTheme ? 'text-[#c9983a]' : 'text-[#a2792c]'
                }`} />
                <div className="flex-1">
                  <p className={`text-[13px] font-semibold mb-0.5 transition-colors ${
                    darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                    Pull requests (read-only)
                  </p>
                  <p className={`text-[11px] transition-colors ${
                    darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    Track contributions and project activity
                  </p>
                </div>
              </div>

              {/* Organization Members */}
              <div className={`flex items-start gap-2.5 p-2.5 rounded-[10px] ${
                darkTheme ? 'bg-white/5' : 'bg-white/20'
              }`}>
                <Users className={`w-4 h-4 flex-shrink-0 mt-0.5 ${
                  darkTheme ? 'text-[#c9983a]' : 'text-[#a2792c]'
                }`} />
                <div className="flex-1">
                  <p className={`text-[13px] font-semibold mb-0.5 transition-colors ${
                    darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                    Organization members (read-only)
                  </p>
                  <p className={`text-[11px] transition-colors ${
                    darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    Verify organization membership and roles
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Why GitHub App */}
          <div className={`p-3 rounded-[10px] ${
            darkTheme ? 'bg-[#c9983a]/10 border border-[#c9983a]/30' : 'bg-yellow-50 border border-yellow-200'
          }`}>
            <p className={`text-[12px] font-medium transition-colors ${
              darkTheme ? 'text-[#e8c77f]' : 'text-[#856404]'
            }`}>
              <strong>Why GitHub App installation?</strong> Verifies your organization membership and requires admin approval.
            </p>
          </div>

          {/* You stay in control */}
          <div className={`p-3 rounded-[10px] ${
            darkTheme ? 'bg-[#c9983a]/10 border border-[#c9983a]/30' : 'bg-yellow-50 border border-yellow-200'
          }`}>
            <p className={`text-[12px] font-medium transition-colors ${
              darkTheme ? 'text-[#e8c77f]' : 'text-[#856404]'
            }`}>
              <strong>You stay in control:</strong> Choose which repositories to connect during installation. Update permissions whenever you want.
            </p>
          </div>

          {/* Don't show again */}
          <label className="flex items-center gap-2 cursor-pointer">
            <div className="relative">
            <input
              type="checkbox"
              checked={dontShowAgain}
              onChange={(e) => setDontShowAgain(e.target.checked)}
              disabled={isInstalling}
                className={`w-4 h-4 rounded border-2 checked:bg-[#c9983a] checked:border-[#c9983a] focus:ring-2 focus:ring-[#c9983a]/40 transition-all cursor-pointer appearance-none ${
                darkTheme
                  ? 'border-[#b8a898]/50 bg-[#2d2820]'
                  : 'border-[#7a6b5a]/50 bg-[#e8dfd0]'
                } ${isInstalling ? 'opacity-50 cursor-not-allowed' : ''} ${
                  dontShowAgain ? 'bg-[#c9983a] border-[#c9983a]' : ''
                }`}
            />
              {dontShowAgain && (
                <svg 
                  className="absolute top-0 left-0 w-4 h-4 pointer-events-none"
                  fill="none" 
                  viewBox="0 0 16 16"
                >
                  <path 
                    d="M13.5 4.5L6 12L2.5 8.5" 
                    stroke="white" 
                    strokeWidth="2.5" 
                    strokeLinecap="round" 
                    strokeLinejoin="round"
                  />
                </svg>
              )}
            </div>
            <span className={`text-[12px] transition-colors ${
              darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              Don't show this message again
            </span>
          </label>

          {/* Actions */}
          <div className="flex items-center gap-2.5 pt-1">
            <button
              type="button"
              onClick={onClose}
              disabled={isInstalling}
              className={`flex-1 px-4 py-2.5 rounded-[10px] border-2 font-semibold text-[13px] transition-all ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] hover:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] hover:bg-white/60'
              } ${isInstalling ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              Cancel
            </button>
            <button
              onClick={handleInstall}
              disabled={isInstalling}
              className={`flex-1 px-4 py-2.5 rounded-[10px] border-2 font-semibold text-[13px] transition-all ${
                darkTheme
                  ? 'bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-[#c9983a]/70 text-[#fef5e7] hover:from-[#c9983a]/50 hover:to-[#d4af37]/40 shadow-[0_4px_16px_rgba(201,152,58,0.4)]'
                  : 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/50 text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 shadow-[0_4px_16px_rgba(201,152,58,0.25)]'
              } ${isInstalling ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              {isInstalling ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Redirecting...
                </span>
              ) : (
                'Install GitHub App'
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

