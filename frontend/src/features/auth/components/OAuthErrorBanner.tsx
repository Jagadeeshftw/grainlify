import { useState, useEffect, useCallback, useRef } from 'react';
import { ShieldOff, WifiOff, Clock, AlertCircle, X } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import type { OAuthErrorState } from '../types/oauthErrors';

const ICON_MAP = { ShieldOff, WifiOff, Clock, AlertCircle } as const;

interface OAuthErrorBannerProps {
  error: OAuthErrorState;
  /** Called when the user clicks the primary CTA (e.g. "Try Again with GitHub"). */
  onRetry: () => void;
  /** Called when the user clicks "Contact Support" (secondary CTA). */
  onContactSupport?: () => void;
  /** Called when the user dismisses the banner. */
  onDismiss?: () => void;
}

/**
 * Inline error banner shown on SignInPage and SignUpPage when an OAuth
 * error is detected (e.g. via URL params after a redirect back from GitHub).
 *
 * Accessibility: role="alert", auto-focuses heading, icon + text (not colour-only).
 */
export function OAuthErrorBanner({
  error,
  onRetry,
  onContactSupport,
  onDismiss,
}: OAuthErrorBannerProps) {
  const { theme } = useTheme();
  const headingRef = useRef<HTMLHeadingElement>(null);
  const [countdown, setCountdown] = useState(error.retryAfterSeconds ?? 0);
  const isRateLimited = error.code === 'rate-limited' && countdown > 0;

  // Auto-focus heading on mount for screen-reader announcement
  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  // Countdown timer for rate-limit errors
  useEffect(() => {
    if (error.code !== 'rate-limited' || !error.retryAfterSeconds) return;
    setCountdown(error.retryAfterSeconds);
    const interval = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) {
          clearInterval(interval);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, [error.code, error.retryAfterSeconds]);

  const handleRetry = useCallback(() => {
    if (isRateLimited) return;
    onRetry();
  }, [isRateLimited, onRetry]);

  const Icon = ICON_MAP[error.icon];
  const isDark = theme === 'dark';

  return (
    <div
      role="alert"
      aria-live="assertive"
      id="oauth-error-banner"
      className={`
        relative w-full rounded-[16px] border p-4 mb-4
        transition-all duration-300 ease-[cubic-bezier(0,0,0.2,1)]
        animate-[slideDown_300ms_ease-out]
        ${isDark
          ? 'bg-[#2a1f1f] border-[#ef4444]/20'
          : 'bg-[#fef2f2] border-[#dc2626]/20'
        }
      `}
      style={{
        boxShadow: isDark
          ? '0 4px 6px -1px rgba(0,0,0,0.3), 0 2px 4px -2px rgba(0,0,0,0.3)'
          : '0 4px 6px -1px rgba(0,0,0,0.08), 0 2px 4px -2px rgba(0,0,0,0.08)',
      }}
    >
      {/* Dismiss button */}
      {onDismiss && (
        <button
          onClick={onDismiss}
          aria-label="Dismiss error"
          className={`
            absolute top-3 right-3 p-1 rounded-full transition-colors
            focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50
            ${isDark
              ? 'text-[#b8a898] hover:text-[#f5f5f5] hover:bg-white/10'
              : 'text-[#7a6b5a] hover:text-[#2d2820] hover:bg-black/5'
            }
          `}
        >
          <X className="w-4 h-4" />
        </button>
      )}

      <div className="flex items-start gap-3">
        {/* Icon */}
        <div
          className={`
            flex-shrink-0 mt-0.5 p-2 rounded-[10px]
            ${isDark
              ? 'bg-[#ef4444]/10 text-[#ef4444]'
              : 'bg-[#dc2626]/10 text-[#dc2626]'
            }
          `}
          aria-hidden="true"
        >
          <Icon className="w-5 h-5" />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <h3
            ref={headingRef}
            tabIndex={-1}
            className={`
              text-sm font-semibold mb-1 outline-none
              ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}
            `}
          >
            {error.heading}
          </h3>
          <p
            className={`
              text-xs leading-relaxed mb-3
              ${isDark ? 'text-[#d4d4d4]' : 'text-[#57534e]'}
            `}
          >
            {error.description}
          </p>

          {/* Rate-limit countdown */}
          {error.code === 'rate-limited' && (
            <p
              className={`
                text-xs font-mono mb-3
                ${isDark ? 'text-[#f59e0b]' : 'text-[#b45309]'}
              `}
              aria-live="polite"
            >
              {countdown > 0
                ? `Retry available in ${countdown}s`
                : 'You can retry now'}
            </p>
          )}

          {/* CTAs */}
          <div className="flex flex-wrap gap-2">
            <button
              id="oauth-error-retry-cta"
              onClick={handleRetry}
              disabled={isRateLimited}
              className={`
                px-4 py-2 text-xs font-medium rounded-[8px]
                transition-all duration-150
                focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50
                ${isRateLimited
                  ? 'opacity-50 cursor-not-allowed'
                  : 'hover:scale-[1.02] active:scale-[0.98]'
                }
                ${isDark
                  ? 'bg-[#ef4444] text-white hover:bg-[#dc2626]'
                  : 'bg-[#dc2626] text-white hover:bg-[#b91c1c]'
                }
              `}
            >
              {isRateLimited ? `Wait ${countdown}s…` : error.primaryCta}
            </button>

            {error.secondaryCta && onContactSupport && (
              <button
                onClick={onContactSupport}
                className={`
                  px-4 py-2 text-xs font-medium rounded-[8px] border
                  transition-all duration-150
                  hover:scale-[1.02] active:scale-[0.98]
                  focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50
                  ${isDark
                    ? 'border-white/15 text-[#d4d4d4] hover:bg-white/5'
                    : 'border-[#d6d3d1] text-[#57534e] hover:bg-black/5'
                  }
                `}
              >
                {error.secondaryCta}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
