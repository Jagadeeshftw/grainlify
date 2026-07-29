import { useState, useEffect, useCallback, useRef } from 'react';
import { ShieldOff, WifiOff, Clock, AlertCircle, ArrowLeft } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import type { OAuthErrorState } from '../types/oauthErrors';

const ICON_MAP = { ShieldOff, WifiOff, Clock, AlertCircle } as const;

interface OAuthErrorPanelProps {
  error: OAuthErrorState;
  /** Called when the user clicks the primary CTA. */
  onRetry: () => void;
  /** Called when the user clicks "Contact Support". */
  onContactSupport?: () => void;
}

/**
 * Full-page error panel rendered on AuthCallbackPage when OAuth processing
 * encounters an error.
 *
 * Displays a distinct error icon, heading, description, and recovery CTAs
 * inside the existing glassmorphic card layout. Includes a countdown timer
 * for rate-limit errors.
 *
 * Accessibility: role="alert", focus moves to error heading on appearance,
 * all state communicated via icon + text (not colour alone).
 */
export function OAuthErrorPanel({
  error,
  onRetry,
  onContactSupport,
}: OAuthErrorPanelProps) {
  const { theme } = useTheme();
  const headingRef = useRef<HTMLHeadingElement>(null);
  const [countdown, setCountdown] = useState(error.retryAfterSeconds ?? 0);
  const isRateLimited = error.code === 'rate-limited' && countdown > 0;
  const isDark = theme === 'dark';

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

  // Per-error-type accent colours
  const accent = {
    'denied-scopes': { bg: 'bg-[#ef4444]/10', text: 'text-[#ef4444]', ring: '#ef4444' },
    'network-failure': { bg: 'bg-[#3b82f6]/10', text: 'text-[#3b82f6]', ring: '#3b82f6' },
    'rate-limited': { bg: 'bg-[#f59e0b]/10', text: 'text-[#f59e0b]', ring: '#f59e0b' },
    'unknown-error': { bg: 'bg-[#ef4444]/10', text: 'text-[#ef4444]', ring: '#ef4444' },
  }[error.code];

  return (
    <div
      role="alert"
      aria-live="assertive"
      id="oauth-error-panel"
      className="text-center animate-[fadeIn_300ms_ease-out]"
    >
      {/* Error icon */}
      <div className="mb-5 flex justify-center">
        <div
          className={`
            p-4 rounded-full ${accent.bg} ${accent.text}
            transition-all duration-300
          `}
          aria-hidden="true"
        >
          <Icon className="w-10 h-10" />
        </div>
      </div>

      {/* Heading */}
      <h2
        ref={headingRef}
        tabIndex={-1}
        className={`
          text-xl font-bold mb-2 outline-none transition-colors
          ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}
        `}
      >
        {error.heading}
      </h2>

      {/* Description */}
      <p
        className={`
          text-sm mb-4 max-w-xs mx-auto leading-relaxed transition-colors
          ${isDark ? 'text-[#d4d4d4]' : 'text-[#57534e]'}
        `}
      >
        {error.description}
      </p>

      {/* Rate-limit countdown */}
      {error.code === 'rate-limited' && (
        <div
          className={`
            inline-flex items-center gap-2 px-4 py-2 rounded-full mb-4
            text-xs font-mono
            ${isDark
              ? 'bg-[#f59e0b]/10 text-[#f59e0b]'
              : 'bg-[#f59e0b]/10 text-[#b45309]'
            }
          `}
          aria-live="polite"
        >
          <Clock className="w-3.5 h-3.5" aria-hidden="true" />
          {countdown > 0
            ? `Retry available in ${countdown}s`
            : 'You can retry now'}
        </div>
      )}

      {/* CTAs */}
      <div className="flex flex-col items-center gap-3 mt-2">
        <button
          id="oauth-error-retry-cta"
          onClick={handleRetry}
          disabled={isRateLimited}
          className={`
            w-full max-w-[260px] py-3 text-sm font-medium rounded-[12px]
            transition-all duration-150
            focus:outline-none focus:ring-2 focus:ring-offset-2
            ${isRateLimited
              ? 'opacity-50 cursor-not-allowed'
              : 'hover:scale-[1.02] active:scale-[0.98]'
            }
            bg-[#24292e] hover:bg-[#1b1f23] text-white
            border border-white/10 shadow-[0_4px_12px_rgba(0,0,0,0.15)]
          `}
          style={{ '--tw-ring-color': accent.ring } as React.CSSProperties}
        >
          {isRateLimited ? `Wait ${countdown}s…` : error.primaryCta}
        </button>

        {error.secondaryCta && onContactSupport && (
          <button
            onClick={onContactSupport}
            className={`
              w-full max-w-[260px] py-3 text-sm font-medium rounded-[12px] border
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

        <Link
          to="/signin"
          className={`
            inline-flex items-center gap-1.5 text-xs mt-2
            transition-colors
            focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50 rounded
            ${isDark
              ? 'text-[#b8a898] hover:text-[#f5f5f5]'
              : 'text-[#7a6b5a] hover:text-[#2d2820]'
            }
          `}
        >
          <ArrowLeft className="w-3.5 h-3.5" />
          Back to Sign In
        </Link>
      </div>
    </div>
  );
}
