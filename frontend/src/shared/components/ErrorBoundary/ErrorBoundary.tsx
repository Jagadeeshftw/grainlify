import { Component, type ErrorInfo, type ReactNode, useState, useCallback } from 'react';
import { AlertTriangle, RefreshCw, Home, ExternalLink, Bug } from 'lucide-react';
import { useTheme, isDarkVariant, isA11yVariant } from '../../contexts/ThemeContext';
import { FOCUS_RING_SPEC } from '../../contexts/ThemeContext';

export type ErrorBoundaryVariant = 'full-page' | 'widget';

export interface ErrorBoundaryProps {
  children: ReactNode;
  variant?: ErrorBoundaryVariant;
  onReset?: () => void;
  onReportIssue?: (error: Error, errorInfo: ErrorInfo | null) => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  retryCount: number;
}

// ─── SVG Illustration ─────────────────────────────────────────────────────

function CrashIllustration({ isDark, isHighContrast }: { isDark: boolean; isHighContrast: boolean }) {
  const stroke = isHighContrast ? '#f5c842' : isDark ? '#c9983a' : '#a67c2e';
  const fill = isHighContrast ? 'rgba(245,200,66,0.20)' : isDark ? 'rgba(201,152,58,0.12)' : 'rgba(201,152,58,0.10)';
  const neutral = isDark ? 'rgba(255,255,255,0.18)' : 'rgba(44,36,28,0.14)';
  return (
    <svg width="96" height="96" viewBox="0 0 96 96" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="eb-crash-title">
      <title id="eb-crash-title">Error illustration — broken hexagon</title>
      {/* Hexagon outline */}
      <polygon points="48,10 80,28 80,68 48,86 16,68 16,28" fill={fill} stroke={stroke} strokeWidth="2.5" strokeLinejoin="round" />
      {/* Broken crack line */}
      <path d="M38 36 L48 52 L58 40" stroke={stroke} strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" fill="none" />
      <path d="M44 56 L52 66" stroke={neutral} strokeWidth="2" strokeLinecap="round" />
      {/* Exclamation mark */}
      <line x1="48" y1="44" x2="48" y2="58" stroke={isHighContrast ? '#f5c842' : isDark ? '#f5f5f5' : '#2d2820'} strokeWidth="2.5" strokeLinecap="round" />
      <circle cx="48" cy="70" r="2.5" fill={isHighContrast ? '#f5c842' : isDark ? '#f5f5f5' : '#2d2820'} />
      {/* Debris dots */}
      <circle cx="22" cy="46" r="2" fill={neutral} />
      <circle cx="74" cy="34" r="2.5" fill={neutral} />
      <circle cx="68" cy="72" r="2" fill={neutral} />
      <circle cx="28" cy="68" r="1.5" fill={neutral} />
    </svg>
  );
}

// ─── Report issue helpers ──────────────────────────────────────────────────

function buildReportUrl(error: Error, errorInfo: ErrorInfo | null): string {
  const lines = [
    `**Error:** ${error.name}: ${error.message}`,
    `**URL:** ${window.location.href}`,
    `**Timestamp:** ${new Date().toISOString()}`,
    `**User Agent:** ${navigator.userAgent}`,
    '',
    '**Stack:**',
    '```',
    error.stack || '(no stack)',
    '```',
  ];
  if (errorInfo?.componentStack) {
    lines.push('', '**Component Stack:**', '```', errorInfo.componentStack, '```');
  }
  const body = encodeURIComponent(lines.join('\n'));
  const title = encodeURIComponent(`[Error] ${error.message}`);
  return `https://github.com/Jagadeeshftw/grainlify/issues/new?title=${title}&body=${body}`;
}

// ─── Fallback UI ───────────────────────────────────────────────────────────

interface ErrorFallbackProps {
  error: Error;
  errorInfo: ErrorInfo | null;
  variant: ErrorBoundaryVariant;
  retryCount: number;
  isRetrying: boolean;
  onRetry: () => void;
  onReportIssue?: (error: Error, errorInfo: ErrorInfo | null) => void;
}

function ErrorFallback({ error, errorInfo, variant, retryCount, isRetrying, onRetry, onReportIssue }: ErrorFallbackProps) {
  const { theme } = useTheme();
  const dark = isDarkVariant(theme);
  const isHighContrast = theme === 'high-contrast';
  const isDev = process.env.NODE_ENV === 'development';
  const [stackOpen, setStackOpen] = useState(false);

  const handleReport = useCallback(() => {
    if (onReportIssue) {
      onReportIssue(error, errorInfo);
    } else {
      window.open(buildReportUrl(error, errorInfo), '_blank', 'noopener,noreferrer');
    }
  }, [error, errorInfo, onReportIssue]);

  const handleGoHome = useCallback(() => {
    window.location.href = '/';
  }, []);

  const focusRing = FOCUS_RING_SPEC.className(theme);

  if (variant === 'widget') {
    return (
      <div
        role="alert"
        aria-live="assertive"
        className={[
          'flex items-start gap-3 p-4 rounded-[12px] border',
          isHighContrast
            ? 'bg-black border-2 border-[#888888]'
            : dark
              ? 'bg-[#2d2820] border-[rgba(255,255,255,0.10)]'
              : 'bg-white border-[rgba(44,36,28,0.12)]',
        ].join(' ')}
      >
        <AlertTriangle
          className={[
            'w-5 h-5 flex-shrink-0 mt-0.5',
            isHighContrast ? 'text-[#ff6e6e]' : dark ? 'text-[#ef4444]' : 'text-[#dc2626]',
          ].join(' ')}
          aria-hidden="true"
        />
        <div className="flex-1 min-w-0">
          <p
            className={[
              'text-[14px] font-semibold mb-1',
              isHighContrast ? 'text-white' : dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]',
            ].join(' ')}
          >
            Widget failed to load
          </p>
          <p
            className={[
              'text-[12px] mb-2',
              isHighContrast ? 'text-[#ebebeb]' : dark ? 'text-[#b8a898]' : 'text-[#7a6b5a]',
            ].join(' ')}
          >
            {error.message || 'An unexpected error occurred in this section.'}
          </p>
          <button
            onClick={onRetry}
            disabled={isRetrying}
            className={[
              'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[8px] text-[12px] font-semibold',
              'transition-colors',
              isHighContrast
                ? 'bg-[#f5c842] text-black hover:bg-[#ffe680]'
                : dark
                  ? 'bg-[#c9983a] text-white hover:bg-[#e8c77f] hover:text-[#2d2820]'
                  : 'bg-[#a67c2e] text-white hover:bg-[#c9983a]',
              focusRing,
              isRetrying ? 'opacity-60 cursor-not-allowed' : '',
            ].join(' ')}
          >
            {isRetrying ? (
              <>
                <span className="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" aria-hidden="true" />
                <span>Retrying…</span>
              </>
            ) : (
              <>
                <RefreshCw className="w-3 h-3" aria-hidden="true" />
                <span>Retry</span>
              </>
            )}
          </button>
        </div>
      </div>
    );
  }

  // Full-page variant
  return (
    <div
      role="main"
      aria-label="Application error"
      className={[
        'fixed inset-0 z-[100] flex items-center justify-center px-6',
        isHighContrast
          ? 'bg-black'
          : dark
            ? 'bg-[#1a1714]'
            : 'bg-[#f5f0ea]',
      ].join(' ')}
    >
      <div role="alert" aria-live="assertive" className="sr-only">
        An unexpected error occurred. You can try again or go to the homepage.
      </div>

      <div
        className={[
          'w-full max-w-md rounded-[24px] border p-10 text-center',
          isHighContrast
            ? 'bg-[#0d0d0d] border-2 border-[#888888]'
            : dark
              ? 'backdrop-blur-[40px] bg-[#2d2820]/[0.4] border-[rgba(255,255,255,0.10)]'
              : 'backdrop-blur-[40px] bg-white/[0.55] border-[rgba(44,36,28,0.12)]',
        ].join(' ')}
      >
        {/* Illustration */}
        <div className="flex justify-center mb-6">
          <div className="w-[96px] h-[96px] flex-shrink-0">
            <CrashIllustration isDark={dark} isHighContrast={isHighContrast} />
          </div>
        </div>

        {/* Heading */}
        <h1
          tabIndex={-1}
          className={[
            'text-[24px] font-bold mb-3',
            isHighContrast ? 'text-white' : dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]',
          ].join(' ')}
        >
          Something went wrong
        </h1>

        {/* Message */}
        <p
          className={[
            'text-[14px] leading-relaxed mb-6 max-w-[400px] mx-auto',
            isHighContrast ? 'text-[#ebebeb]' : dark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]',
          ].join(' ')}
        >
          {retryCount > 0
            ? 'Still not working. You can try again or go back to the homepage.'
            : 'An unexpected error occurred. Our team has been notified. You can try again or go back to the homepage.'}
        </p>

        {/* Actions */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-3 mb-6">
          <button
            onClick={onRetry}
            disabled={isRetrying}
            className={[
              'inline-flex items-center gap-2 min-h-[44px] px-6 py-2.5 rounded-[12px] text-[14px] font-semibold w-full sm:w-auto justify-center',
              'transition-colors',
              isHighContrast
                ? 'bg-[#f5c842] text-black hover:bg-[#ffe680]'
                : dark
                  ? 'bg-[#c9983a] text-white hover:bg-[#e8c77f] hover:text-[#2d2820]'
                  : 'bg-[#a67c2e] text-white hover:bg-[#c9983a]',
              focusRing,
              isRetrying ? 'opacity-60 cursor-not-allowed' : '',
            ].join(' ')}
          >
            {isRetrying ? (
              <>
                <span className="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" aria-hidden="true" />
                <span>Retrying…</span>
              </>
            ) : (
              <>
                <RefreshCw className="w-4 h-4" aria-hidden="true" />
                <span>Try again</span>
              </>
            )}
          </button>

          <button
            onClick={handleGoHome}
            className={[
              'inline-flex items-center gap-2 min-h-[44px] px-6 py-2.5 rounded-[12px] text-[14px] font-semibold w-full sm:w-auto justify-center',
              'transition-colors',
              isHighContrast
                ? 'border-2 border-[#aaaaaa] text-white hover:bg-white/20'
                : dark
                  ? 'border border-[rgba(255,255,255,0.15)] text-[#c9983a] hover:bg-white/10'
                  : 'border border-[rgba(44,36,28,0.15)] text-[#a67c2e] hover:bg-black/5',
              focusRing,
            ].join(' ')}
          >
            <Home className="w-4 h-4" aria-hidden="true" />
            <span>Go to homepage</span>
          </button>
        </div>

        {/* Report issue link */}
        <button
          onClick={handleReport}
          className={[
            'inline-flex items-center gap-1.5 text-[13px] transition-colors',
            isHighContrast
              ? 'text-[#c8c8c8] hover:text-white'
              : dark
                ? 'text-[#b8a898] hover:text-[#e8dfd0]'
                : 'text-[#9f8b74] hover:text-[#2d2820]',
            focusRing,
          ].join(' ')}
        >
          <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" />
          <span>Report this issue</span>
        </button>

        {/* Dev-only stack trace */}
        {isDev && (
          <div className="mt-6 pt-6 border-t border-[rgba(255,255,255,0.08)] dark:border-[rgba(255,255,255,0.08)]">
            <button
              onClick={() => setStackOpen((o) => !o)}
              aria-expanded={stackOpen}
              aria-controls="error-boundary-stacktrace"
              className={[
                'inline-flex items-center gap-1.5 text-[12px] font-mono transition-colors',
                isHighContrast
                  ? 'text-[#c8c8c8] hover:text-white'
                  : dark
                    ? 'text-[#b8a898] hover:text-[#e8dfd0]'
                    : 'text-[#7a6b5a] hover:text-[#2d2820]',
                focusRing,
              ].join(' ')}
            >
              <Bug className="w-3.5 h-3.5" aria-hidden="true" />
              <span>{stackOpen ? 'Hide' : 'Show'} stack trace</span>
            </button>

            {stackOpen && (
              <pre
                id="error-boundary-stacktrace"
                className={[
                  'mt-3 p-4 rounded-[8px] text-[11px] font-mono leading-relaxed text-left overflow-x-auto max-h-[240px] overflow-y-auto whitespace-pre-wrap',
                  isHighContrast
                    ? 'bg-black text-[#c8c8c8] border border-[#555555]'
                    : dark
                      ? 'bg-[#1a1714] text-[#b8a898]'
                      : 'bg-[#f5f0ea] text-[#7a6b5a]',
                ].join(' ')}
              >
                <strong className={isHighContrast ? 'text-white' : dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}>
                  {error.name}: {error.message}
                </strong>
                {'\n\n'}
                {error.stack}
                {errorInfo?.componentStack && (
                  <>
                    {'\n\n'}
                    <strong className={isHighContrast ? 'text-white' : dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}>
                      Component Stack:
                    </strong>
                    {'\n'}
                    {errorInfo.componentStack}
                  </>
                )}
              </pre>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Error Boundary class component ────────────────────────────────────────

interface ErrorBoundaryClassProps extends ErrorBoundaryProps {
  onStateChange?: (hasError: boolean) => void;
}

export class ErrorBoundaryClass extends Component<ErrorBoundaryClassProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryClassProps) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null, retryCount: 0 };
    this.handleRetry = this.handleRetry.bind(this);
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    this.setState({ errorInfo });
    this.props.onStateChange?.(true);
    if (process.env.NODE_ENV !== 'test') {
      console.error('[ErrorBoundary]', error, errorInfo);
    }
  }

  handleRetry(): void {
    this.setState(
      (prev) => ({ hasError: false, error: null, errorInfo: null, retryCount: prev.retryCount + 1 }),
      () => {
        this.props.onStateChange?.(false);
        this.props.onReset?.();
      },
    );
  }

  render() {
    if (this.state.hasError && this.state.error) {
      return (
        <ErrorFallback
          error={this.state.error}
          errorInfo={this.state.errorInfo}
          variant={this.props.variant ?? 'full-page'}
          retryCount={this.state.retryCount}
          isRetrying={false}
          onRetry={this.handleRetry}
          onReportIssue={this.props.onReportIssue}
        />
      );
    }
    return this.props.children;
  }
}

// ─── Public API ────────────────────────────────────────────────────────────

export function ErrorBoundary(props: ErrorBoundaryProps) {
  return <ErrorBoundaryClass {...props} />;
}
