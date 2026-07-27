import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import React from 'react';
import { ErrorBoundary, ErrorBoundaryClass } from './ErrorBoundary';
import { ThemeProvider } from '../../contexts/ThemeContext';

// ─── Helpers ───────────────────────────────────────────────────────────────

const ThrowError = ({ message }: { message?: string }) => {
  throw new Error(message ?? 'Test render error');
};

function GoodChild() {
  return <div data-testid="good-child">Hello</div>;
}

function BrokenChild() {
  const [shouldThrow, setShouldThrow] = React.useState(false);
  if (shouldThrow) throw new Error('Conditional throw');
  return (
    <button data-testid="trigger-error" onClick={() => setShouldThrow(true)}>
      Break
    </button>
  );
}

function renderWithTheme(ui: React.ReactElement) {
  return render(<ThemeProvider>{ui}</ThemeProvider>);
}

// ─── Mocks ─────────────────────────────────────────────────────────────────

function mockLocalStorage() {
  const store: Record<string, string> = {};
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { Object.keys(store).forEach(k => delete store[k]); },
    get length() { return Object.keys(store).length; },
    key: (index: number) => Object.keys(store)[index] ?? null,
  });
}

beforeEach(() => {
  mockLocalStorage();
  vi.stubGlobal('process', { ...process, env: { ...process.env, NODE_ENV: 'test' } });
});

afterEach(() => {
  vi.restoreAllMocks();
});

// ─── ErrorBoundaryClass ────────────────────────────────────────────────────

describe('ErrorBoundaryClass', () => {
  it('renders children when there is no error', () => {
    renderWithTheme(
      <ErrorBoundaryClass>
        <GoodChild />
      </ErrorBoundaryClass>,
    );
    expect(screen.getByTestId('good-child')).toBeInTheDocument();
  });

  it('renders fallback UI when a child throws', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundaryClass>
        <ThrowError />
      </ErrorBoundaryClass>,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Try again')).toBeInTheDocument();
    expect(screen.getByText('Go to homepage')).toBeInTheDocument();
    expect(screen.getByText('Report this issue')).toBeInTheDocument();
  });

  it('renders widget variant fallback when variant="widget"', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundaryClass variant="widget">
        <ThrowError />
      </ErrorBoundaryClass>,
    );
    expect(screen.getByText('Widget failed to load')).toBeInTheDocument();
    expect(screen.getByText('Retry')).toBeInTheDocument();
  });

  it('calls onStateChange with true on error', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const onStateChange = vi.fn();
    renderWithTheme(
      <ErrorBoundaryClass onStateChange={onStateChange}>
        <ThrowError />
      </ErrorBoundaryClass>,
    );
    expect(onStateChange).toHaveBeenCalledWith(true);
  });

  it('resets error state when retry is clicked', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundaryClass>
        <BrokenChild />
      </ErrorBoundaryClass>,
    );
    fireEvent.click(screen.getByTestId('trigger-error'));
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Try again'));
    expect(screen.getByTestId('trigger-error')).toBeInTheDocument();
  });

  it('shows retry-failed message on second failure', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundaryClass>
        <BrokenChild />
      </ErrorBoundaryClass>,
    );
    // First error
    fireEvent.click(screen.getByTestId('trigger-error'));
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    // Retry
    fireEvent.click(screen.getByText('Try again'));
    expect(screen.getByTestId('trigger-error')).toBeInTheDocument();
    // Second error
    fireEvent.click(screen.getByTestId('trigger-error'));
    expect(screen.getByText((t) => t.startsWith('Still not working'))).toBeInTheDocument();
  });

  it('calls onReset when retry is clicked', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const onReset = vi.fn();
    renderWithTheme(
      <ErrorBoundaryClass onReset={onReset}>
        <ThrowError />
      </ErrorBoundaryClass>,
    );
    fireEvent.click(screen.getByText('Try again'));
    expect(onReset).toHaveBeenCalledOnce();
  });

  it('displays error message in fallback for widget variant', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundaryClass variant="widget">
        <ThrowError message="Custom widget error" />
      </ErrorBoundaryClass>,
    );
    expect(screen.getByText('Custom widget error')).toBeInTheDocument();
  });

  it('calls onStateChange with false on retry', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const onStateChange = vi.fn();
    renderWithTheme(
      <ErrorBoundaryClass onStateChange={onStateChange}>
        <ThrowError />
      </ErrorBoundaryClass>,
    );
    expect(onStateChange).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByText('Try again'));
    expect(onStateChange).toHaveBeenCalledWith(false);
  });
});

// ─── ErrorBoundary (default export wrapper) ────────────────────────────────

describe('ErrorBoundary', () => {
  it('renders children when there is no error', () => {
    renderWithTheme(
      <ErrorBoundary>
        <GoodChild />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('good-child')).toBeInTheDocument();
  });

  it('renders full-page fallback on error', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('main', { name: 'Application error' })).toBeInTheDocument();
    expect(screen.getByText('Try again')).toBeInTheDocument();
  });

  it('renders widget fallback with variant prop', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundary variant="widget">
        <ThrowError />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Widget failed to load')).toBeInTheDocument();
  });
});

// ─── Accessibility ─────────────────────────────────────────────────────────

describe('ErrorBoundary — accessibility', () => {
  it('has role="alert" region in full-page variant', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );
    const alerts = screen.getAllByRole('alert');
    expect(alerts.length).toBeGreaterThanOrEqual(1);
  });

  it('has role="alert" region in widget variant', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundary variant="widget">
        <ThrowError />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('renders SVG with role="img" in full-page variant', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );
    const svg = document.querySelector('svg[role="img"]');
    expect(svg).toBeInTheDocument();
  });

  it('does not show stack trace when NODE_ENV is test', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithTheme(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );
    expect(screen.queryByText('Show stack trace')).not.toBeInTheDocument();
  });
});

// ─── Report issue ──────────────────────────────────────────────────────────

describe('ErrorBoundary — report issue', () => {
  it('opens default report URL when onReportIssue is not provided', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const open = vi.fn();
    const originalOpen = window.open;
    window.open = open;

    renderWithTheme(
      <ErrorBoundary>
        <ThrowError message="Test error" />
      </ErrorBoundary>,
    );
    fireEvent.click(screen.getByText('Report this issue'));
    expect(open).toHaveBeenCalledOnce();
    const url = open.mock.calls[0][0] as string;
    expect(url).toContain('github.com/Jagadeeshftw/grainlify/issues/new');
    expect(url).toContain(encodeURIComponent('Test error'));

    window.open = originalOpen;
  });

  it('calls custom onReportIssue when provided', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const onReportIssue = vi.fn();
    renderWithTheme(
      <ErrorBoundary onReportIssue={onReportIssue}>
        <ThrowError message="Custom report" />
      </ErrorBoundary>,
    );
    fireEvent.click(screen.getByText('Report this issue'));
    expect(onReportIssue).toHaveBeenCalledOnce();
    const errorArg = onReportIssue.mock.calls[0][0] as Error;
    expect(errorArg.message).toBe('Custom report');
  });
});
