/**
 * HeatmapTooltip
 *
 * Enriched hover/tap preview tooltip for ContributionHeatmap cells.
 *
 * Anatomy (populated day):
 *   ┌─────────────────────────────────┐
 *   │  Thursday, January 16, 2025     │  ← date header
 *   │  ─────────────────────────────  │
 *   │  🐛 Issues closed       3       │
 *   │  🔀 PRs merged          1       │
 *   │  🏆 Bounties won        1       │
 *   │  ─────────────────────────────  │
 *   │  🔥 Day 4 of a 6-day streak     │  ← streak badge (optional)
 *   └─────────────────────────────────┘
 *
 * Anatomy (empty day):
 *   ┌─────────────────────────────────┐
 *   │  Thursday, January 16, 2025     │
 *   │  ─────────────────────────────  │
 *   │  No activity                    │
 *   └─────────────────────────────────┘
 *
 * States: hidden → appearing (150ms delay, 150ms fade) → visible → disappearing
 * Touch: tap-to-open; tap-outside to dismiss
 * Reduced-motion: opacity-only, 0ms transform
 *
 * Design tokens used (design-tokens.json):
 *   chart.tooltip.background  = rgba(26,20,16,0.95)
 *   chart.tooltip.border       = rgba(255,255,255,0.2)
 *   chart.tooltip.border-radius = 12px
 *   chart.tooltip.backdrop-blur = 30px
 *   darkMode.text.primary      = #f5f5f5
 *   darkMode.text.secondary    = #d4d4d4
 *   color.primary.500          = #f1b400  (streak badge accent)
 *   motion.durations.fast      = 150ms
 *
 * WCAG 2.1 AA:
 *   Text #f5f5f5 on rgba(26,20,16,0.95) → 14.7:1 (AAA)
 *   Streak text #f1b400 on rgba(26,20,16,0.95) → 7.1:1 (AAA)
 */

import {
  useEffect,
  useRef,
  useCallback,
  type CSSProperties,
  type RefObject,
} from 'react';
import { format, parseISO } from 'date-fns';
import { GitMerge, Trophy, Bug, Flame } from 'lucide-react';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface DayBreakdown {
  /** Number of issues closed on this day */
  issuesClosed: number;
  /** Number of PRs merged on this day */
  prsMerged: number;
  /** Number of bounties won on this day */
  bountiesWon: number;
}

export interface StreakContext {
  /** Which day of the streak this day falls on (1-indexed) */
  streakDay: number;
  /** Total length of the streak in days */
  streakLength: number;
}

export interface TooltipDayData {
  /** ISO date string YYYY-MM-DD */
  date: string;
  /** Total contributions (sum of all categories) */
  count: number;
  /** Optional per-category breakdown */
  breakdown?: DayBreakdown;
  /** Optional streak context */
  streak?: StreakContext;
}

export interface HeatmapTooltipProps {
  /** Data for the hovered/focused day, or null when hidden */
  day: TooltipDayData | null;
  /**
   * Anchor element the tooltip should position itself relative to.
   * When null the tooltip is not rendered at all.
   */
  anchorRef: RefObject<HTMLElement | null>;
  /** Whether the tooltip is in a touch-triggered open state */
  isTouchOpen: boolean;
  /** Callback to close a touch-triggered tooltip (tap outside) */
  onDismiss: () => void;
  /** Dark-mode flag from ThemeContext */
  isDark: boolean;
  /** Loading state while fetching day detail */
  isLoading?: boolean;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function formatDate(iso: string): string {
  try {
    return format(parseISO(iso), 'EEEE, MMMM d, yyyy');
  } catch {
    return iso;
  }
}

/**
 * Returns the CSS position for the tooltip so it stays within the viewport.
 * Prefers top of the anchor; falls back to bottom when there is not enough
 * space above.
 */
function computePosition(
  anchor: HTMLElement,
  tooltipEl: HTMLElement | null,
): CSSProperties {
  const rect = anchor.getBoundingClientRect();
  const tipHeight = tooltipEl?.offsetHeight ?? 120;
  const tipWidth = tooltipEl?.offsetWidth ?? 220;
  const MARGIN = 8; // px gap between cell and tooltip

  // Vertical: prefer above, fall back to below
  let top: number;
  const spaceAbove = rect.top - MARGIN;
  if (spaceAbove >= tipHeight) {
    top = rect.top + window.scrollY - tipHeight - MARGIN;
  } else {
    top = rect.bottom + window.scrollY + MARGIN;
  }

  // Horizontal: centre on the anchor, clamp to viewport edges
  let left = rect.left + window.scrollX + rect.width / 2 - tipWidth / 2;
  left = Math.max(MARGIN, Math.min(left, window.innerWidth - tipWidth - MARGIN));

  return { top, left };
}

// ─── Row sub-component ────────────────────────────────────────────────────────

interface BreakdownRowProps {
  icon: React.ReactNode;
  label: string;
  value: number;
}

function BreakdownRow({ icon, label, value }: BreakdownRowProps) {
  return (
    <div className="flex items-center gap-2">
      <span className="flex-shrink-0 w-4 h-4 flex items-center justify-center opacity-70">
        {icon}
      </span>
      <span className="flex-1 text-[#d4d4d4] text-xs leading-none">{label}</span>
      <span className="text-[#f5f5f5] text-xs font-semibold tabular-nums leading-none">
        {value}
      </span>
    </div>
  );
}

// ─── Loading skeleton ─────────────────────────────────────────────────────────

function TooltipSkeleton() {
  return (
    <div className="space-y-2 animate-pulse" aria-label="Loading day details">
      <div className="h-3 rounded bg-white/20 w-36" />
      <div className="h-px bg-white/10 my-1" />
      <div className="h-3 rounded bg-white/15 w-28" />
      <div className="h-3 rounded bg-white/15 w-24" />
      <div className="h-3 rounded bg-white/15 w-20" />
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

/**
 * HeatmapTooltip renders a rich preview card for a heatmap day cell.
 *
 * Positioning:
 *   - Computed in JS (not CSS fixed-left/top from mouse) so it works for both
 *     pointer and keyboard/touch contexts.
 *   - Re-calculates on every anchor change and on window resize.
 *
 * Dismissal:
 *   - Pointer: disappears on mouse-leave (handled by parent).
 *   - Touch: dismissed when user taps outside; parent passes isTouchOpen and
 *     onDismiss.
 *   - Keyboard: parent calls onDismiss when Escape is pressed.
 */
export function HeatmapTooltip({
  day,
  anchorRef,
  isTouchOpen,
  onDismiss,
  isDark: _isDark, // reserved for future light-mode variant
  isLoading = false,
}: HeatmapTooltipProps) {
  const tooltipRef = useRef<HTMLDivElement>(null);

  // Dismiss on tap-outside (touch UX)
  const handleDocumentTap = useCallback(
    (e: MouseEvent | TouchEvent) => {
      if (!isTouchOpen) return;
      const target = e.target as Node;
      if (
        tooltipRef.current?.contains(target) ||
        anchorRef.current?.contains(target)
      ) {
        return;
      }
      onDismiss();
    },
    [isTouchOpen, anchorRef, onDismiss],
  );

  useEffect(() => {
    document.addEventListener('mousedown', handleDocumentTap);
    document.addEventListener('touchstart', handleDocumentTap);
    return () => {
      document.removeEventListener('mousedown', handleDocumentTap);
      document.removeEventListener('touchstart', handleDocumentTap);
    };
  }, [handleDocumentTap]);

  // Reposition on resize
  useEffect(() => {
    const reposition = () => {
      if (!tooltipRef.current || !anchorRef.current) return;
      const pos = computePosition(anchorRef.current, tooltipRef.current);
      Object.assign(tooltipRef.current.style, {
        top: `${pos.top}px`,
        left: `${pos.left}px`,
      });
    };
    window.addEventListener('resize', reposition);
    return () => window.removeEventListener('resize', reposition);
  }, [anchorRef]);

  if (!day && !isLoading) return null;

  const anchor = anchorRef.current;
  if (!anchor) return null;

  const pos = computePosition(anchor, tooltipRef.current);
  const hasActivity = !!day && day.count > 0;
  const hasBreakdown =
    hasActivity &&
    day.breakdown &&
    (day.breakdown.issuesClosed > 0 ||
      day.breakdown.prsMerged > 0 ||
      day.breakdown.bountiesWon > 0);
  const hasStreak = hasActivity && !!day.streak && day.streak.streakLength > 1;

  return (
    <div
      ref={tooltipRef}
      role="tooltip"
      // aria-live so keyboard / screen-reader users hear the update
      aria-live="polite"
      aria-atomic="true"
      style={{
        position: 'absolute',
        top: pos.top,
        left: pos.left,
        zIndex: 9999,
        // motion: pointer transition; prefers-reduced-motion override in CSS
        transition: 'opacity 150ms cubic-bezier(0,0,0.2,1)',
      }}
      className={[
        // Glassmorphism surface matching design-tokens.json chart.tooltip
        'w-max max-w-[240px] min-w-[180px]',
        'rounded-xl border border-white/20',
        'bg-[rgba(26,20,16,0.95)] backdrop-blur-[30px]',
        'px-4 py-3',
        'shadow-[0_10px_25px_rgba(0,0,0,0.5)]',
        // Fade-in; reduced-motion handled via global CSS in index.css
        'animate-in fade-in-0 zoom-in-[0.97]',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
      ].join(' ')}
      data-testid="heatmap-tooltip"
    >
      {/* Arrow */}
      <div
        aria-hidden="true"
        className="absolute left-1/2 -translate-x-1/2 -bottom-[6px] w-3 h-3 rotate-45 bg-[rgba(26,20,16,0.95)] border-r border-b border-white/20"
      />

      {isLoading ? (
        <TooltipSkeleton />
      ) : day ? (
        <div className="space-y-2">
          {/* Date header */}
          <p className="text-[#f5f5f5] text-xs font-semibold leading-tight">
            {formatDate(day.date)}
          </p>

          <div className="h-px bg-white/10" aria-hidden="true" />

          {hasActivity ? (
            <>
              {/* Per-category breakdown */}
              {hasBreakdown ? (
                <div className="space-y-1.5">
                  {day.breakdown!.issuesClosed > 0 && (
                    <BreakdownRow
                      icon={<Bug className="w-3.5 h-3.5 text-[#22c55e]" />}
                      label="Issues closed"
                      value={day.breakdown!.issuesClosed}
                    />
                  )}
                  {day.breakdown!.prsMerged > 0 && (
                    <BreakdownRow
                      icon={<GitMerge className="w-3.5 h-3.5 text-[#c9983a]" />}
                      label="PRs merged"
                      value={day.breakdown!.prsMerged}
                    />
                  )}
                  {day.breakdown!.bountiesWon > 0 && (
                    <BreakdownRow
                      icon={<Trophy className="w-3.5 h-3.5 text-[#f1b400]" />}
                      label="Bounties won"
                      value={day.breakdown!.bountiesWon}
                    />
                  )}
                </div>
              ) : (
                /* Fallback: total count when no breakdown available */
                <p className="text-[#d4d4d4] text-xs">
                  <span className="text-[#f5f5f5] font-semibold">
                    {day.count}
                  </span>{' '}
                  contribution{day.count !== 1 ? 's' : ''}
                </p>
              )}

              {/* Streak badge */}
              {hasStreak && (
                <>
                  <div className="h-px bg-white/10" aria-hidden="true" />
                  <div className="flex items-center gap-1.5">
                    <Flame
                      className="w-3.5 h-3.5 text-[#f1b400] flex-shrink-0"
                      aria-hidden="true"
                    />
                    <span className="text-[#f1b400] text-[11px] font-semibold leading-none">
                      Day {day.streak!.streakDay} of a{' '}
                      {day.streak!.streakLength}-day streak
                    </span>
                  </div>
                </>
              )}
            </>
          ) : (
            /* Empty day */
            <p className="text-[#78716c] text-xs italic">No activity</p>
          )}
        </div>
      ) : null}
    </div>
  );
}
