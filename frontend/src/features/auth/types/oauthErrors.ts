/**
 * GitHub OAuth Error Types & Classifier
 *
 * Centralises error classification so that AuthCallbackPage (full-page),
 * SignInPage and SignUpPage (inline banner) render the same copy and
 * recovery actions for every failure mode.
 */

export type OAuthErrorCode =
  | 'denied-scopes'
  | 'network-failure'
  | 'rate-limited'
  | 'unknown-error';

export interface OAuthErrorState {
  /** Machine-readable error code */
  code: OAuthErrorCode;
  /** User-facing heading */
  heading: string;
  /** User-facing description */
  description: string;
  /** Label for the primary recovery CTA */
  primaryCta: string;
  /** Optional secondary CTA label (e.g. "Contact Support") */
  secondaryCta?: string;
  /** Seconds until automatic retry is allowed (rate-limit only) */
  retryAfterSeconds?: number;
  /** Icon name from lucide-react */
  icon: 'ShieldOff' | 'WifiOff' | 'Clock' | 'AlertCircle';
}

/** Retry-After header value GitHub typically sends (seconds). */
const DEFAULT_RATE_LIMIT_RETRY_SECONDS = 60;

/**
 * Classify a raw error string / Error object into a structured OAuthErrorState.
 *
 * The classifier inspects URL `error` param values returned by GitHub's OAuth
 * flow, HTTP status codes surfaced via the backend, and generic JS errors.
 */
export function classifyOAuthError(
  raw: string | Error | null | undefined,
  retryAfterHeader?: number | null,
): OAuthErrorState {
  const message =
    raw instanceof Error ? raw.message : (raw ?? '').toLowerCase();

  // 1. User denied scopes / cancelled the authorisation dialog
  if (
    message.includes('access_denied') ||
    message.includes('denied') ||
    message.includes('cancelled') ||
    message.includes('canceled') ||
    message.includes('scope')
  ) {
    return {
      code: 'denied-scopes',
      heading: 'Permission Required',
      description:
        'You declined the required GitHub permissions. Grainlify needs access to your public profile and repositories to create your account.',
      primaryCta: 'Try Again with GitHub',
      secondaryCta: 'Contact Support',
      icon: 'ShieldOff',
    };
  }

  // 2. GitHub API rate-limit (403 / 429 or explicit "rate" mention)
  if (
    message.includes('rate') ||
    message.includes('429') ||
    message.includes('too many requests') ||
    message.includes('limit')
  ) {
    return {
      code: 'rate-limited',
      heading: 'Too Many Requests',
      description:
        'GitHub is temporarily limiting requests. Please wait before trying again.',
      primaryCta: 'Retry',
      secondaryCta: 'Contact Support',
      retryAfterSeconds: retryAfterHeader ?? DEFAULT_RATE_LIMIT_RETRY_SECONDS,
      icon: 'Clock',
    };
  }

  // 3. Network / timeout failure
  if (
    message.includes('network') ||
    message.includes('timeout') ||
    message.includes('fetch') ||
    message.includes('failed to fetch') ||
    message.includes('err_internet') ||
    message.includes('econnrefused') ||
    message.includes('enotfound') ||
    message.includes('abort')
  ) {
    return {
      code: 'network-failure',
      heading: 'Connection Failed',
      description:
        "We couldn't reach GitHub. Please check your internet connection and try again.",
      primaryCta: 'Retry Connection',
      icon: 'WifiOff',
    };
  }

  // 4. Catch-all unknown error
  return {
    code: 'unknown-error',
    heading: 'Something Went Wrong',
    description:
      'An unexpected error occurred during authentication. Please try again or contact support if the problem persists.',
    primaryCta: 'Try Again',
    secondaryCta: 'Contact Support',
    icon: 'AlertCircle',
  };
}
