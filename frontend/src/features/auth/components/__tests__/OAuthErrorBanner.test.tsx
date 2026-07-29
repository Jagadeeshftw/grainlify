import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import { OAuthErrorBanner } from '../OAuthErrorBanner';
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
  retryAfterSeconds: 3,
  icon: 'Clock',
};

const networkError: OAuthErrorState = {
  code: 'network-failure',
  heading: 'Connection Failed',
  description: 'We couldn\'t reach GitHub.',
  primaryCta: 'Retry Connection',
  icon: 'WifiOff',
};

describe('OAuthErrorBanner', () => {
  const onRetry = vi.fn();
  const onContactSupport = vi.fn();
  const onDismiss = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  // ── Rendering ──────────────────────────────────────────────────
  it('renders with role="alert" for screen readers', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('displays the error heading', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(screen.getByText('Permission Required')).toBeInTheDocument();
  });

  it('displays the error description', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(
      screen.getByText('You declined the required GitHub permissions.'),
    ).toBeInTheDocument();
  });

  it('displays primary CTA button', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(
      screen.getByRole('button', { name: 'Try Again with GitHub' }),
    ).toBeInTheDocument();
  });

  it('displays secondary CTA when provided', () => {
    render(
      <OAuthErrorBanner
        error={deniedError}
        onRetry={onRetry}
        onContactSupport={onContactSupport}
      />,
    );
    expect(
      screen.getByRole('button', { name: 'Contact Support' }),
    ).toBeInTheDocument();
  });

  it('does not display secondary CTA when onContactSupport is not provided', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(
      screen.queryByRole('button', { name: 'Contact Support' }),
    ).not.toBeInTheDocument();
  });

  // ── Dismiss button ────────────────────────────────────────────
  it('renders dismiss button when onDismiss is provided', () => {
    render(
      <OAuthErrorBanner
        error={deniedError}
        onRetry={onRetry}
        onDismiss={onDismiss}
      />,
    );
    expect(
      screen.getByRole('button', { name: 'Dismiss error' }),
    ).toBeInTheDocument();
  });

  it('calls onDismiss when dismiss button is clicked', () => {
    render(
      <OAuthErrorBanner
        error={deniedError}
        onRetry={onRetry}
        onDismiss={onDismiss}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss error' }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  // ── CTA interactions ──────────────────────────────────────────
  it('calls onRetry when primary CTA is clicked', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Try Again with GitHub' }),
    );
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('calls onContactSupport when secondary CTA is clicked', () => {
    render(
      <OAuthErrorBanner
        error={deniedError}
        onRetry={onRetry}
        onContactSupport={onContactSupport}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Contact Support' }));
    expect(onContactSupport).toHaveBeenCalledTimes(1);
  });

  // ── Rate-limit countdown ──────────────────────────────────────
  it('disables retry button during rate-limit countdown', () => {
    render(
      <OAuthErrorBanner error={rateLimitError} onRetry={onRetry} />,
    );
    const retryBtn = screen.getByRole('button', { name: /wait/i });
    expect(retryBtn).toBeDisabled();
  });

  it('shows countdown text for rate-limited errors', () => {
    render(
      <OAuthErrorBanner error={rateLimitError} onRetry={onRetry} />,
    );
    expect(
      screen.getByText(/retry available in 3s/i),
    ).toBeInTheDocument();
  });

  it('enables retry button after countdown reaches zero', () => {
    render(
      <OAuthErrorBanner error={rateLimitError} onRetry={onRetry} />,
    );

    // Fast-forward 3 seconds
    act(() => {
      vi.advanceTimersByTime(3000);
    });

    expect(
      screen.getByText(/you can retry now/i),
    ).toBeInTheDocument();
    const retryBtn = screen.getByRole('button', { name: 'Retry' });
    expect(retryBtn).not.toBeDisabled();
  });

  // ── Network error (no secondary CTA by default) ───────────────
  it('renders network error without secondary CTA', () => {
    render(
      <OAuthErrorBanner error={networkError} onRetry={onRetry} />,
    );
    expect(screen.getByText('Connection Failed')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Contact Support' }),
    ).not.toBeInTheDocument();
  });

  // ── Accessibility ─────────────────────────────────────────────
  it('has an element with id="oauth-error-banner"', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(document.getElementById('oauth-error-banner')).toBeInTheDocument();
  });

  it('has a retry CTA with id="oauth-error-retry-cta"', () => {
    render(
      <OAuthErrorBanner error={deniedError} onRetry={onRetry} />,
    );
    expect(document.getElementById('oauth-error-retry-cta')).toBeInTheDocument();
  });
});
