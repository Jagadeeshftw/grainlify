import React from 'react';
import { render, screen, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { TimestampDisplay } from '../TimestampDisplay';

describe('TimestampDisplay component', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Freeze current date to 2026-07-26T20:30:00.000Z
    vi.setSystemTime(new Date('2026-07-26T20:30:00.000Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders relative time string for recent event', () => {
    const recentDate = new Date(Date.now() - 10 * 60 * 1000).toISOString(); // 10m ago
    render(<TimestampDisplay timestamp={recentDate} />);

    const timeElement = screen.getByTestId('timestamp-display');
    expect(timeElement).toBeInTheDocument();
    expect(timeElement).toHaveTextContent('10m ago');
    expect(timeElement).toHaveAttribute('datetime', recentDate);
    expect(timeElement).toHaveAttribute('tabindex', '0');
  });

  it('renders absolute local date for event older than 7 days', () => {
    const oldDate = new Date(Date.now() - 10 * 24 * 60 * 60 * 1000).toISOString(); // 10 days ago
    render(<TimestampDisplay timestamp={oldDate} />);

    const timeElement = screen.getByTestId('timestamp-display');
    expect(timeElement).toBeInTheDocument();
    expect(timeElement.textContent).not.toContain('ago');
    expect(timeElement).toHaveAttribute('datetime', oldDate);
  });

  it('renders fallback text when timestamp is invalid', () => {
    render(<TimestampDisplay timestamp="invalid" fallbackText="3 hours ago" />);

    const timeElement = screen.getByTestId('timestamp-display');
    expect(timeElement).toBeInTheDocument();
    expect(timeElement).toHaveTextContent('3 hours ago');
  });

  it('includes aria-label with full date-time and UTC details', () => {
    const isoDate = '2026-07-26T20:00:00.000Z';
    render(<TimestampDisplay timestamp={isoDate} />);

    const timeElement = screen.getByTestId('timestamp-display');
    const ariaLabel = timeElement.getAttribute('aria-label');
    expect(ariaLabel).toContain('30m ago');
    expect(ariaLabel).toContain('2026');
    expect(ariaLabel).toContain('UTC');
  });

  it('updates relative display live when fake timers advance', () => {
    const eventTime = new Date('2026-07-26T20:29:40.000Z'); // 20s ago => "Just now"
    render(<TimestampDisplay timestamp={eventTime} />);

    const timeElement = screen.getByTestId('timestamp-display');
    expect(timeElement).toHaveTextContent('Just now');

    // Advance 50 seconds -> now 70s ago -> should update to "1m ago"
    act(() => {
      vi.advanceTimersByTime(50 * 1000);
    });

    expect(timeElement).toHaveTextContent('1m ago');
  });
});
