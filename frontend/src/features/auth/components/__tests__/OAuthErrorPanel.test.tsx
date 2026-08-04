import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import '@testing-library/jest-dom';
import { OAuthErrorPanel } from '../OAuthErrorPanel';
import type { OAuthErrorState } from '../../types/oauthErrors';

// Mock ThemeContext
vi.mock('../../../../shared/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

const deniedError: OAuthErrorState = {
  code: 'denied-scopes',
  heading: 'Permission Required',
  description: 'You declined the required GitHub permissions.',
  primaryCta: 'Try Again with GitHub',
  secondaryCta: 'Contact Support',
  icon: 'ShieldOff',
};

const rateLimitError: OAuthErrorState = {
  code: 'rate-limited',
  heading: 'Too Many Requests',
  description: 'GitHub is temporarily limiting requests.',
  primaryCta: 'Retry',
  secondaryCta: 'Contact Support',
  retryAfterSeconds: 5,
  icon: 'Clock',
};

const networkError: OAuthErrorState = {
  code: 'network-failure',
  heading: 'Connection Failed',
  description: 'We couldn\'t reach GitHub.',
  primaryCta: 'Retry Connection',
  icon: 'WifiOff',
};

const unknownError: OAuthErrorState = {
  code: 'unknown-error',
  heading: 'Something Went Wrong',
  description: 'An unexpected error occurred.',
  primaryCta: 'Try Again',
  secondaryCta: 'Contact Support',
  icon: 'AlertCircle',
};

function renderPanel(error: OAuthErrorState, props: Partial<React.ComponentProps<typeof OAuthErrorPanel>> = {}) {
  const onRetry = props.onRetry ?? vi.fn();
  const onContactSupport = props.onContactSupport ?? vi.fn();
  return render(
    <MemoryRouter>
      <OAuthErrorPanel
        error={error}
        onRetry={onRetry}
        onContactSupport={onContactSupport}
        {...props}
      />
    </MemoryRouter>,
  );
}

describe('OAuthErrorPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  // ── Rendering per error type ──────────────────────────────────
  it('renders denied-scopes error with correct heading and description', () => {
    renderPanel(deniedError);
    expect(screen.getByText('Permission Required')).toBeInTheDocument();
    expect(screen.getByText('You declined the required GitHub permissions.')).toBeInTheDocument();
  });

  it('renders rate-limited error with countdown', () => {
    renderPanel(rateLimitError);
    expect(screen.getByText('Too Many Requests')).toBeInTheDocument();
    expect(screen.getByText(/retry available in 5s/i)).toBeInTheDocument();
  });

  it('renders network-failure error', () => {
    renderPanel(networkError);
    expect(screen.getByText('Connection Failed')).toBeInTheDocument();
  });

  it('renders unknown-error', () => {
    renderPanel(unknownError);
    expect(screen.getByText('Something Went Wrong')).toBeInTheDocument();
  });

  // ── role="alert" ──────────────────────────────────────────────
  it('has role="alert" for screen-reader announcement', () => {
    renderPanel(deniedError);
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  // ── CTA interactions ──────────────────────────────────────────
  it('calls onRetry when primary CTA is clicked', () => {
    const onRetry = vi.fn();
    renderPanel(deniedError, { onRetry });
    fireEvent.click(screen.getByRole('button', { name: 'Try Again with GitHub' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('calls onContactSupport when secondary CTA is clicked', () => {
    const onContactSupport = vi.fn();
    renderPanel(deniedError, { onContactSupport });
    fireEvent.click(screen.getByRole('button', { name: 'Contact Support' }));
    expect(onContactSupport).toHaveBeenCalledTimes(1);
  });

  // ── Rate-limit countdown ──────────────────────────────────────
  it('disables retry during countdown', () => {
    renderPanel(rateLimitError);
    const btn = screen.getByRole('button', { name: /wait/i });
    expect(btn).toBeDisabled();
  });

  it('enables retry after countdown expires', () => {
    renderPanel(rateLimitError);
    act(() => {
      vi.advanceTimersByTime(5000);
    });
    const btn = screen.getByRole('button', { name: 'Retry' });
    expect(btn).not.toBeDisabled();
  });

  it('shows "You can retry now" after countdown', () => {
    renderPanel(rateLimitError);
    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(screen.getByText(/you can retry now/i)).toBeInTheDocument();
  });

  // ── "Back to Sign In" link ────────────────────────────────────
  it('renders a "Back to Sign In" link', () => {
    renderPanel(deniedError);
    const link = screen.getByRole('link', { name: /back to sign in/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute('href', '/signin');
  });

  // ── Accessibility IDs ─────────────────────────────────────────
  it('has id="oauth-error-panel" on the alert region', () => {
    renderPanel(deniedError);
    expect(document.getElementById('oauth-error-panel')).toBeInTheDocument();
  });

  it('has id="oauth-error-retry-cta" on the retry button', () => {
    renderPanel(deniedError);
    expect(document.getElementById('oauth-error-retry-cta')).toBeInTheDocument();
  });

  // ── No secondary CTA for network error (no secondaryCta defined) ──
  it('does not render secondary CTA when error has no secondaryCta', () => {
    renderPanel(networkError);
    expect(screen.queryByText('Contact Support')).not.toBeInTheDocument();
  });
});
