/**
 * HeatmapTooltip — unit & interaction tests
 *
 * Coverage:
 *  1. Renders nothing when day=null
 *  2. Renders nothing when anchorRef.current=null
 *  3. Populated day — date header, breakdown rows, streak badge
 *  4. Empty day (count=0) — "No activity" copy, no breakdown rows
 *  5. Fallback (no breakdown prop) — total count shown
 *  6. Loading skeleton rendered when isLoading=true
 *  7. Streak badge hidden when streakLength <= 1
 *  8. Zero-count breakdown rows not rendered
 *  9. Tap-outside dismissal calls onDismiss
 * 10. Tooltip does not dismiss on tap inside itself
 * 11. Tooltip does not dismiss on tap on the anchor element
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createRef } from 'react';
import { HeatmapTooltip, type TooltipDayData } from './HeatmapTooltip';

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Builds a real HTMLButtonElement and attaches it to the document so
 *  getBoundingClientRect returns a non-zero rect. */
function makeAnchor(): HTMLButtonElement {
  const el = document.createElement('button');
  el.style.cssText = 'position:fixed;top:200px;left:200px;width:16px;height:16px';
  document.body.appendChild(el);
  // jsdom doesn't implement layout; stub getBoundingClientRect manually
  vi.spyOn(el, 'getBoundingClientRect').mockReturnValue({
    top: 200, left: 200, bottom: 216, right: 216,
    width: 16, height: 16, x: 200, y: 200,
    toJSON: () => ({}),
  } as DOMRect);
  return el;
}

function renderTooltip(
  day: TooltipDayData | null,
  overrides: Partial<{
    isTouchOpen: boolean;
    isLoading: boolean;
    onDismiss: () => void;
  }> = {},
) {
  const anchorEl = makeAnchor();
  const anchorRef = createRef<HTMLElement | null>();
  // createRef gives a read-only .current — patch via Object.defineProperty
  Object.defineProperty(anchorRef, 'current', {
    get: () => anchorEl,
    configurable: true,
  });

  const onDismiss = overrides.onDismiss ?? vi.fn();

  const { rerender, unmount } = render(
    <HeatmapTooltip
      day={day}
      anchorRef={anchorRef as React.RefObject<HTMLElement | null>}
      isTouchOpen={overrides.isTouchOpen ?? false}
      onDismiss={onDismiss}
      isDark={true}
      isLoading={overrides.isLoading ?? false}
    />,
  );

  return { anchorEl, anchorRef, onDismiss, rerender, unmount };
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('HeatmapTooltip', () => {
  beforeEach(() => {
    // Clean up any lingering anchor elements between tests
    document.body.querySelectorAll('button').forEach((b) => b.remove());
  });

  // 1 ─────────────────────────────────────────────────────────────────────────
  it('renders nothing when day is null and isLoading is false', () => {
    const { anchorEl } = renderTooltip(null);
    expect(screen.queryByTestId('heatmap-tooltip')).toBeNull();
    anchorEl.remove();
  });

  // 2 ─────────────────────────────────────────────────────────────────────────
  it('renders nothing when anchorRef.current is null', () => {
    const anchorRef = createRef<HTMLElement | null>();
    // anchorRef.current is null by default
    const { container } = render(
      <HeatmapTooltip
        day={{ date: '2025-01-01', count: 3 }}
        anchorRef={anchorRef as React.RefObject<HTMLElement | null>}
        isTouchOpen={false}
        onDismiss={vi.fn()}
        isDark={true}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  // 3 ─────────────────────────────────────────────────────────────────────────
  it('renders date header, breakdown rows, and streak badge for a populated day', () => {
    const day: TooltipDayData = {
      date: '2025-01-16',
      count: 5,
      breakdown: { issuesClosed: 3, prsMerged: 1, bountiesWon: 1 },
      streak: { streakDay: 4, streakLength: 6 },
    };

    renderTooltip(day);

    // Date header — date-fns formats 2025-01-16 as "Thursday, January 16, 2025"
    expect(screen.getByText('Thursday, January 16, 2025')).toBeInTheDocument();

    // Breakdown rows
    expect(screen.getByText('Issues closed')).toBeInTheDocument();
    expect(screen.getByText('PRs merged')).toBeInTheDocument();
    expect(screen.getByText('Bounties won')).toBeInTheDocument();

    // Counts (rendered as individual text nodes via tabular-nums span)
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getAllByText('1')).toHaveLength(2); // prsMerged + bountiesWon

    // Streak badge
    expect(screen.getByText(/Day 4 of a 6-day streak/i)).toBeInTheDocument();
  });

  // 4 ─────────────────────────────────────────────────────────────────────────
  it('shows "No activity" for an empty day and omits breakdown rows', () => {
    const day: TooltipDayData = { date: '2025-06-01', count: 0 };

    renderTooltip(day);

    expect(screen.getByText('No activity')).toBeInTheDocument();
    expect(screen.queryByText('Issues closed')).toBeNull();
    expect(screen.queryByText('PRs merged')).toBeNull();
    expect(screen.queryByText('Bounties won')).toBeNull();
    expect(screen.queryByText(/streak/i)).toBeNull();
  });

  // 5 ─────────────────────────────────────────────────────────────────────────
  it('shows total count fallback when no breakdown is provided', () => {
    const day: TooltipDayData = { date: '2025-03-10', count: 7 };

    renderTooltip(day);

    // Should show "7 contributions" without any category rows
    expect(screen.getByText(/7/)).toBeInTheDocument();
    expect(screen.getByText(/contributions/i)).toBeInTheDocument();
    expect(screen.queryByText('Issues closed')).toBeNull();
  });

  // 6 ─────────────────────────────────────────────────────────────────────────
  it('renders loading skeleton when isLoading is true', () => {
    renderTooltip(null, { isLoading: true });

    const tooltip = screen.getByTestId('heatmap-tooltip');
    expect(tooltip).toBeInTheDocument();
    // The skeleton has an aria-label
    expect(screen.getByLabelText(/loading day details/i)).toBeInTheDocument();
  });

  // 7 ─────────────────────────────────────────────────────────────────────────
  it('hides streak badge when streakLength is 1 or absent', () => {
    const day: TooltipDayData = {
      date: '2025-04-01',
      count: 2,
      streak: { streakDay: 1, streakLength: 1 },
    };

    renderTooltip(day);

    expect(screen.queryByText(/streak/i)).toBeNull();
  });

  // 8 ─────────────────────────────────────────────────────────────────────────
  it('omits breakdown rows whose count is zero', () => {
    const day: TooltipDayData = {
      date: '2025-05-20',
      count: 2,
      breakdown: { issuesClosed: 2, prsMerged: 0, bountiesWon: 0 },
    };

    renderTooltip(day);

    expect(screen.getByText('Issues closed')).toBeInTheDocument();
    expect(screen.queryByText('PRs merged')).toBeNull();
    expect(screen.queryByText('Bounties won')).toBeNull();
  });

  // 9 ─────────────────────────────────────────────────────────────────────────
  it('calls onDismiss when user taps outside tooltip and anchor', () => {
    const onDismiss = vi.fn();
    const day: TooltipDayData = { date: '2025-07-04', count: 3 };

    renderTooltip(day, { isTouchOpen: true, onDismiss });

    // Simulate a tap on an unrelated element
    const outside = document.createElement('div');
    document.body.appendChild(outside);
    fireEvent.mouseDown(outside);

    expect(onDismiss).toHaveBeenCalledTimes(1);
    outside.remove();
  });

  // 10 ────────────────────────────────────────────────────────────────────────
  it('does NOT call onDismiss when user taps inside the tooltip', () => {
    const onDismiss = vi.fn();
    const day: TooltipDayData = {
      date: '2025-07-10',
      count: 2,
      breakdown: { issuesClosed: 2, prsMerged: 0, bountiesWon: 0 },
    };

    renderTooltip(day, { isTouchOpen: true, onDismiss });

    const tooltip = screen.getByTestId('heatmap-tooltip');
    fireEvent.mouseDown(tooltip);

    expect(onDismiss).not.toHaveBeenCalled();
  });

  // 11 ────────────────────────────────────────────────────────────────────────
  it('does NOT call onDismiss when user taps the anchor element', () => {
    const onDismiss = vi.fn();
    const day: TooltipDayData = { date: '2025-08-15', count: 1 };

    const { anchorEl } = renderTooltip(day, { isTouchOpen: true, onDismiss });

    fireEvent.mouseDown(anchorEl);

    expect(onDismiss).not.toHaveBeenCalled();
  });
});
