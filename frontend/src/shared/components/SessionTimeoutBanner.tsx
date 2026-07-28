import { useEffect, useRef, useState } from 'react';
import { AlertTriangle, X, LogIn } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { useAuth } from '../contexts/AuthContext';
import { useTheme } from '../contexts/ThemeContext';
import { isDarkVariant } from '../contexts/ThemeContext';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Format a number of seconds as "M:SS" (e.g. 304 → "5:04"). */
function formatCountdown(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}

/** Build the visible copy string for the banner. */
function buildCopy(state: 'warning-visible' | 'critical', secs: number): string {
  if (state === 'critical') {
    if (secs <= 10) {
      return `Session expiring in ${secs} second${secs !== 1 ? 's' : ''} — save your work now.`;
    }
    return `Session expiring in ${secs} second${secs !== 1 ? 's' : ''}.`;
  }
  // warning-visible
  if (secs < 120) {
    return `Your session expires in ${formatCountdown(secs)}.`;
  }
  return `Your session expires in ${formatCountdown(secs)} — stay signed in to keep working.`;
}

// ---------------------------------------------------------------------------
// Forced-logout screen
// ---------------------------------------------------------------------------

function SessionExpiredScreen() {
  const { theme } = useTheme();
  const navigate = useNavigate();
  const dark = isDarkVariant(theme);
  const isHighContrast = theme === 'high-contrast';

  const handleSignBackIn = () => {
    const lastRoute = window.location.pathname + window.location.search;
    if (lastRoute && lastRoute !== '/signin') {
      sessionStorage.setItem('authReturnTo', lastRoute);
    }
    navigate('/signin', { replace: true });
  };

  return (
    <div
      role="main"
      aria-label="Session ended"
      className={`fixed inset-0 z-[100] flex items-center justify-center px-6 transition-colors ${
        dark
          ? 'bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]'
          : 'bg-gradient-to-br from-[#e8dfd0] via-[#d4c5b0] to-[#c9b89a]'
      } ${isHighContrast ? '!bg-black' : ''}`}
    >
      {/* Single role="alert" — announced once on mount */}
      <div role="alert" className="sr-only">
        Your session has ended. Please sign back in.
      </div>

      <div
        className={`w-full max-w-sm rounded-[24px] border p-10 text-center shadow-[0_8px_32px_rgba(0,0,0,0.18)] transition-colors ${
          isHighContrast
            ? 'bg-black border-2 border-white'
            : dark
              ? 'backdrop-blur-[40px] bg-[#2d2820]/[0.4] border-white/10'
              : 'backdrop-blur-[40px] bg-white/[0.35] border-white'
        }`}
      >
        {/* Logo mark */}
        <div className="flex justify-center mb-6">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-[#c9983a] to-[#d4af37] shadow-[0_2px_8px_rgba(201,152,58,0.4)]" />
        </div>

        <h1
          className={`text-2xl font-bold mb-3 transition-colors ${
            isHighContrast ? 'text-white' : dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}
        >
          Your session has ended
        </h1>

        <p
          className={`text-sm mb-6 leading-relaxed transition-colors ${
            isHighContrast ? 'text-white' : dark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}
        >
          For your security, you've been signed out after a period of inactivity.
        </p>

        <button
          onClick={handleSignBackIn}
          aria-label="Sign back in and return to your previous page"
          className={`inline-flex items-center gap-2 px-6 py-3 rounded-[12px] font-semibold text-sm transition-all focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 ${
            isHighContrast
              ? 'bg-white text-black border-2 border-white hover:bg-[#eeeeee] focus-visible:outline-[#ffff00]'
              : 'bg-[#c9983a] hover:bg-[#a67c2e] text-white shadow-[0_4px_12px_rgba(201,152,58,0.35)] focus-visible:outline-[#a2792c] dark:focus-visible:outline-[#f1b400]'
          }`}
        >
          <LogIn className="w-4 h-4" aria-hidden="true" />
          Sign back in
        </button>

        <p
          className={`text-xs mt-4 transition-colors ${
            isHighContrast ? 'text-white' : dark ? 'text-[#b8a898]' : 'text-[#9f8b74]'
          }`}
        >
          You'll be taken back to the page you were on.
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Banner
// ---------------------------------------------------------------------------

/**
 * SessionTimeoutBanner
 *
 * Renders a fixed top-of-viewport strip when the user's session is within
 * 5 minutes of expiry. Transitions to a critical state under 1 minute, then
 * to a full forced-logout screen on expiry.
 *
 * Accessibility:
 * - role="alert" region announces once per state transition (not per tick).
 * - Focus is never stolen from the user's current task.
 * - Keyboard accessible: all interactive elements are <button>.
 * - WCAG 2.1 AA contrast in both light and dark themes.
 * - Reduced-motion: uses data-opacity-transition (opacity-only, ≤150 ms).
 * - High-contrast: opaque backgrounds, solid borders, yellow focus ring.
 */
export function SessionTimeoutBanner() {
  const { sessionTimeoutState, secondsRemaining, staySignedIn, dismissTimeoutBanner } = useAuth();
  const { theme } = useTheme();
  const dark = isDarkVariant(theme);
  const isHighContrast = theme === 'high-contrast';
  const isReducedMotion = theme === 'reduced-motion';

  // Track whether we've already announced the current state so role="alert"
  // only fires once per state transition.
  const announcedStateRef = useRef<string>('');
  const [announcement, setAnnouncement] = useState('');
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Update the one-shot ARIA announcement on state transitions only.
  useEffect(() => {
    if (sessionTimeoutState === announcedStateRef.current) return;
    announcedStateRef.current = sessionTimeoutState;

    if (sessionTimeoutState === 'warning-visible') {
      setAnnouncement('Your session will expire in about 5 minutes. Select "Stay signed in" to continue.');
    } else if (sessionTimeoutState === 'critical') {
      setAnnouncement('Your session is about to expire. Select "Stay signed in" immediately to avoid being signed out.');
    } else {
      setAnnouncement('');
    }
  }, [sessionTimeoutState]);

  // Expose banner height as a CSS custom property so sticky nav can offset itself.
  useEffect(() => {
    const root = document.documentElement;
    if (sessionTimeoutState === 'warning-visible' || sessionTimeoutState === 'critical') {
      root.style.setProperty('--session-banner-height', '52px');
    } else {
      root.style.setProperty('--session-banner-height', '0px');
    }
    return () => {
      root.style.setProperty('--session-banner-height', '0px');
    };
  }, [sessionTimeoutState]);

  const handleStaySignedIn = async () => {
    setIsRefreshing(true);
    try {
      await staySignedIn();
      toast.success("You're still signed in.", { duration: 3000 });
    } catch {
      // staySignedIn() transitions to expired on failure — no additional handling needed.
    } finally {
      setIsRefreshing(false);
    }
  };

  // Render forced-logout screen on expiry
  if (sessionTimeoutState === 'expired') {
    return <SessionExpiredScreen />;
  }

  // Nothing to show
  if (sessionTimeoutState === 'banner-hidden') {
    return null;
  }

  const isWarning = sessionTimeoutState === 'warning-visible';
  const isCritical = sessionTimeoutState === 'critical';

  // -------------------------------------------------------------------------
  // Theme-conditional class strings
  // -------------------------------------------------------------------------

  const bannerBg = isHighContrast
    ? isCritical ? 'bg-[#1a0000] border-b-2 border-white' : 'bg-black border-b-2 border-white'
    : isWarning
      ? dark
        ? 'bg-[#3a2b0d] border-b border-[#f59e0b]/50'
        : 'bg-[#fffaeb] border-b border-[#f59e0b]/30'
      : dark
        ? 'bg-[#2d1a1a] border-b border-[#ef4444]/60'
        : 'bg-[#fef2f2] border-b border-[#ef4444]/40';

  const textColor = isHighContrast
    ? 'text-white'
    : isWarning
      ? dark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
      : dark ? 'text-[#fca5a5]' : 'text-[#2d2820]';

  const iconColor = isHighContrast
    ? 'text-white'
    : isWarning
      ? dark ? 'text-[#f59e0b]' : 'text-[#b45309]'
      : dark ? 'text-[#f87171]' : 'text-[#dc2626]';

  const ctaColor = isHighContrast
    ? 'text-white border-2 border-white hover:bg-white/20 focus-visible:outline-[#ffff00]'
    : isWarning
      ? dark
        ? 'text-[#f1b400] border border-[#f1b400]/30 bg-[#f1b400]/10 hover:bg-[#f1b400]/20 focus-visible:outline-[#f1b400]'
        : 'text-[#2d2820] border border-[#c9983a]/40 bg-[#c9983a]/10 hover:bg-[#c9983a]/20 focus-visible:outline-[#a2792c]'
      : dark
        ? 'text-[#fca5a5] border border-[#ef4444]/40 bg-[#ef4444]/10 hover:bg-[#ef4444]/20 focus-visible:outline-[#f87171]'
        : 'text-[#dc2626] border border-[#ef4444]/30 bg-[#ef4444]/10 hover:bg-[#ef4444]/20 focus-visible:outline-[#dc2626]';

  const closeBtnColor = isHighContrast
    ? 'text-white hover:bg-white/20 focus-visible:outline-[#ffff00]'
    : isWarning
      ? dark ? 'text-[#b8a898] hover:text-[#e8dfd0] focus-visible:outline-[#f1b400]' : 'text-[#7a6b5a] hover:text-[#2d2820] focus-visible:outline-[#a2792c]'
      : dark ? 'text-[#fca5a5]/70 hover:text-[#fca5a5] focus-visible:outline-[#f87171]' : 'text-[#dc2626]/70 hover:text-[#dc2626] focus-visible:outline-[#dc2626]';

  const transitionClass = isReducedMotion
    ? 'data-[opacity-transition] transition-opacity duration-[150ms]'
    : 'transition-all duration-200';

  return (
    <>
      {/*
        role="alert" child — announced once on state entry, not on every tick.
        Content changes only when `announcement` updates (state transition).
      */}
      {announcement && (
        <div role="alert" className="sr-only" aria-live="assertive" aria-atomic="true">
          {announcement}
        </div>
      )}

      <div
        // tabIndex={-1} — container is not itself in the focus order
        tabIndex={-1}
        data-opacity-transition
        className={`
          fixed top-0 left-0 right-0 z-50
          h-[48px] md:h-[52px]
          flex items-center justify-between
          px-4 md:px-6
          ${bannerBg}
          ${transitionClass}
        `}
        aria-hidden="false"
      >
        {/* Left: icon + copy */}
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <AlertTriangle
            className={`w-4 h-4 flex-shrink-0 ${iconColor}`}
            aria-hidden="true"
          />

          {/* Visible countdown copy — not injected into role="alert" on every tick */}
          {/* aria-live="off" so screen readers do not announce every second */}
          <span
            aria-live="off"
            className={`text-sm font-medium truncate ${textColor}`}
          >
            {/* Mobile: shorter copy */}
            <span className="md:hidden">
              {isCritical
                ? `${secondsRemaining}s remaining`
                : `Expires ${formatCountdown(secondsRemaining)}`}
            </span>
            {/* Desktop: full copy */}
            <span className="hidden md:inline">
              {buildCopy(sessionTimeoutState, secondsRemaining)}
            </span>
          </span>
        </div>

        {/* Right: CTA + dismiss */}
        <div className="flex items-center gap-2 flex-shrink-0 ml-3">
          {/* "Stay signed in" CTA */}
          <button
            onClick={handleStaySignedIn}
            disabled={isRefreshing}
            aria-label="Stay signed in and extend your session"
            className={`
              hidden md:inline-flex items-center gap-1.5
              px-3 py-1 rounded-[8px]
              text-xs font-semibold
              transition-colors
              focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2
              disabled:opacity-50 disabled:cursor-not-allowed
              ${ctaColor}
            `}
          >
            {isRefreshing ? (
              <>
                <span
                  className="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"
                  aria-hidden="true"
                />
                <span>Refreshing…</span>
              </>
            ) : (
              'Stay signed in'
            )}
          </button>

          {/* Mobile: icon-only "stay signed in" */}
          <button
            onClick={handleStaySignedIn}
            disabled={isRefreshing}
            aria-label="Stay signed in and extend your session"
            className={`
              md:hidden
              p-1.5 rounded-[8px]
              transition-colors
              focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2
              disabled:opacity-50 disabled:cursor-not-allowed
              ${ctaColor}
            `}
          >
            {isRefreshing ? (
              <span
                className="inline-block w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"
                aria-hidden="true"
              />
            ) : (
              <LogIn className="w-3.5 h-3.5" aria-hidden="true" />
            )}
            <span className="sr-only">Stay signed in</span>
          </button>

          {/* Dismiss button — only visible in warning state */}
          {isWarning && (
            <button
              onClick={dismissTimeoutBanner}
              aria-label="Dismiss session warning banner"
              className={`
                p-1.5 rounded-[8px]
                transition-colors
                focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2
                ${closeBtnColor}
              `}
            >
              <X className="w-3.5 h-3.5" aria-hidden="true" />
              <span className="sr-only">Dismiss</span>
            </button>
          )}
        </div>
      </div>

      {/* Spacer so content below is not obscured by the fixed banner */}
      <div
        className="h-[48px] md:h-[52px]"
        aria-hidden="true"
      />
    </>
  );
}
