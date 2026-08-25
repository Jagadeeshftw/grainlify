/**
 * ContributionHeatmap Component
 *
 * A responsive, accessible 365-day contribution heatmap with:
 * - Enriched hover/tap tooltip: date header, per-category breakdown
 *   (issues closed, PRs merged, bounties won), streak badge
 * - 150 ms hover-delay before tooltip appears (avoids flicker on fast scans)
 * - Touch: tap-to-open tooltip; tap-outside dismisses it
 * - Keyboard: arrow-key grid navigation (↑↓←→), Enter/Space toggles tooltip,
 *   Escape closes tooltip
 * - role="grid" / role="gridcell" on the heatmap for AT grid navigation
 * - aria-live region announces tooltip content to screen readers
 * - WCAG 2.1 AA: 14.7:1 text contrast on tooltip surface
 * - Respects prefers-reduced-motion (no scale transform, opacity-only)
 *
 * Design spec: design/specs/heatmap-tooltip-enrichment.md
 * Visualisation spec: design/profilepage-visualizations.md
 */

import {
  useState,
  useMemo,
  useRef,
  useCallback,
  useEffect,
  type KeyboardEvent,
  type MouseEvent,
  type TouchEvent as ReactTouchEvent,
} from 'react';
import { Sparkles } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import {
  HeatmapTooltip,
  type TooltipDayData,
  type DayBreakdown,
  type StreakContext,
} from './HeatmapTooltip';
// DayBreakdown and StreakContext are re-exported as part of the public API
// so consuming components can type-check `HeatmapData.breakdown` and `.streak`.
export type { DayBreakdown, StreakContext };

// ─── Public types ─────────────────────────────────────────────────────────────

export interface HeatmapData {
  date: string;
  count: number;
  level: number;
  /** Per-category breakdown (optional – enriches tooltip) */
  breakdown?: DayBreakdown;
  /** Streak context for this day (optional) */
  streak?: StreakContext;
}

export interface ContributionHeatmapProps {
  data: HeatmapData[];
  isLoading?: boolean;
  totalContributions?: number;
}

// ─── Constants ────────────────────────────────────────────────────────────────

const MONTHS = [
  'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
];
const DAYS_FULL = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const DAYS_SHORT = ['M', 'T', 'W', 'Th', 'F', 'Sa', 'Su'];
const WEEKS = 53;
const DAYS_PER_WEEK = 7;
/** ms to wait after pointer enters a cell before showing tooltip */
const HOVER_DELAY_MS = 150;

// ─── Colour helpers ───────────────────────────────────────────────────────────

function getHeatmapColor(level: number, isDark: boolean): string {
  const colors: Record<number, string> = {
    0: isDark
      ? 'bg-white/[0.08] border-white/[0.12]'
      : 'bg-[#efefef] border-[#d6d3d1]',
    1: 'bg-[#c9983a]/35 border-[#c9983a]/50',
    2: 'bg-[#c9983a]/55 border-[#c9983a]/75',
    3: 'bg-[#c9983a]/75 border-[#c9983a]/90',
    4: 'bg-gradient-to-br from-[#f1b400] to-[#c9983a] border-[#d4af37]',
  };
  return colors[Math.min(level, 4)];
}

function getLevelShadow(level: number): string {
  const shadows: Record<number, string> = {
    0: '',
    1: 'shadow-[0_2px_10px_rgba(201,152,58,0.2)]',
    2: 'shadow-[0_2px_12px_rgba(201,152,58,0.3)]',
    3: 'shadow-[0_3px_14px_rgba(201,152,58,0.45)]',
    4: 'shadow-[0_4px_20px_rgba(201,152,58,0.6),0_0_15px_rgba(241,180,0,0.4)]',
  };
  return shadows[Math.min(level, 4)];
}

// ─── Component ────────────────────────────────────────────────────────────────

export function ContributionHeatmap({
  data,
  isLoading = false,
  totalContributions = 0,
}: ContributionHeatmapProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  // ── Tooltip state ──────────────────────────────────────────────────────────
  /** The day data currently shown in the tooltip, null when hidden */
  const [tooltipDay, setTooltipDay] = useState<TooltipDayData | null>(null);
  /**
   * Whether the tooltip was opened by a touch tap (vs. hover/focus).
   * Controls tap-outside dismissal.
   */
  const [isTouchOpen, setIsTouchOpen] = useState(false);
  /** The cell element the tooltip is anchored to */
  const anchorRef = useRef<HTMLButtonElement | null>(null);

  // Pending timer for hover delay
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── Focus/keyboard navigation state ───────────────────────────────────────
  /** [weekIdx, dayIdx] of the currently keyboard-focused cell */
  const [focusPos, setFocusPos] = useState<[number, number] | null>(null);
  /** Whether the focused cell's tooltip is "pinned" open via Enter/Space */
  const [isPinned, setIsPinned] = useState(false);
  /**
   * Mirror of isPinned as a ref so event-handler closures always read the
   * current value without being invalidated on every render.
   */
  const isPinnedRef = useRef(false);

  // Grid ref for programmatic focus management
  const gridRef = useRef<HTMLDivElement>(null);

  // ── Build heatmap grid ─────────────────────────────────────────────────────
  const heatmapGrid = useMemo<HeatmapData[][]>(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const grid: HeatmapData[][] = [];

    for (let week = 0; week < WEEKS; week++) {
      grid[week] = [];
      for (let day = 0; day < DAYS_PER_WEEK; day++) {
        const daysAgo = 364 - (week * DAYS_PER_WEEK + day);
        const target = new Date(today);
        target.setDate(target.getDate() - daysAgo);
        const dateStr = target.toISOString().split('T')[0];
        const entry = data.find((d) => d.date === dateStr) ?? {
          date: dateStr,
          count: 0,
          level: 0,
        };
        grid[week][day] = entry;
      }
    }
    return grid;
  }, [data]);

  // ── Tooltip helpers ────────────────────────────────────────────────────────

  const clearHoverTimer = useCallback(() => {
    if (hoverTimerRef.current !== null) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
  }, []);

  const openTooltip = useCallback(
    (entry: HeatmapData, el: HTMLButtonElement) => {
      anchorRef.current = el;
      setTooltipDay({
        date: entry.date,
        count: entry.count,
        breakdown: entry.breakdown,
        streak: entry.streak,
      });
    },
    [],
  );

  const closeTooltip = useCallback(() => {
    clearHoverTimer();
    setTooltipDay(null);
    setIsTouchOpen(false);
    setIsPinned(false);
    isPinnedRef.current = false;
  }, [clearHoverTimer]);

  // ── Pointer handlers ───────────────────────────────────────────────────────

  const handleMouseEnter = useCallback(
    (entry: HeatmapData, e: MouseEvent<HTMLButtonElement>) => {
      clearHoverTimer();
      const el = e.currentTarget;
      hoverTimerRef.current = setTimeout(() => {
        openTooltip(entry, el);
      }, HOVER_DELAY_MS);
    },
    [clearHoverTimer, openTooltip],
  );

  const handleMouseLeave = useCallback(() => {
    if (isPinnedRef.current) return;
    clearHoverTimer();
    hoverTimerRef.current = setTimeout(() => {
      if (!isPinnedRef.current) setTooltipDay(null);
    }, 80);
  }, [clearHoverTimer]);

  // ── Touch handlers ─────────────────────────────────────────────────────────

  const handleTouchStart = useCallback(
    (entry: HeatmapData, e: ReactTouchEvent<HTMLButtonElement>) => {
      // Prevent the synthetic mouse event that would follow
      e.preventDefault();
      const el = e.currentTarget;
      if (isTouchOpen && tooltipDay?.date === entry.date) {
        // Second tap on same cell closes it
        closeTooltip();
        return;
      }
      openTooltip(entry, el);
      setIsTouchOpen(true);
    },
    [isTouchOpen, tooltipDay, openTooltip, closeTooltip],
  );

  // ── Click handler (pin/unpin for pointer users) ────────────────────────────

  const handleClick = useCallback(
    (entry: HeatmapData, e: MouseEvent<HTMLButtonElement>) => {
      // Touch events already handled above; guard against synthetic click
      if (e.detail === 0) return; // keyboard-generated click – ignore here
      const el = e.currentTarget;
      const isAlreadyPinned = isPinned && tooltipDay?.date === entry.date;
      if (isAlreadyPinned) {
        closeTooltip();
      } else {
        openTooltip(entry, el);
        setIsPinned(true);
        isPinnedRef.current = true;
      }
    },
    [isPinned, tooltipDay, openTooltip, closeTooltip],
  );

  /** Programmatically move focus to a specific cell in the grid */
  const focusCell = useCallback(
    (weekIdx: number, dayIdx: number) => {
      setFocusPos([weekIdx, dayIdx]);
      const selector = `[data-cell="${weekIdx}-${dayIdx}"]`;
      const el = gridRef.current?.querySelector<HTMLButtonElement>(selector);
      el?.focus();
    },
    // gridRef is a stable ref object; no need to list it
    [],
  );

  // ── Keyboard handler ───────────────────────────────────────────────────────

  const handleKeyDown = useCallback(
    (
      entry: HeatmapData,
      weekIdx: number,
      dayIdx: number,
      e: KeyboardEvent<HTMLButtonElement>,
    ) => {
      switch (e.key) {
        case 'Enter':
        case ' ': {
          e.preventDefault();
          const el = e.currentTarget;
          if (isPinned && tooltipDay?.date === entry.date) {
            closeTooltip();
          } else {
            openTooltip(entry, el);
            setIsPinned(true);
            isPinnedRef.current = true;
          }
          break;
        }
        case 'Escape': {
          e.preventDefault();
          closeTooltip();
          break;
        }
        // Arrow-key grid navigation
        case 'ArrowRight': {
          e.preventDefault();
          const nextWeek = Math.min(weekIdx + 1, WEEKS - 1);
          focusCell(nextWeek, dayIdx);
          break;
        }
        case 'ArrowLeft': {
          e.preventDefault();
          const prevWeek = Math.max(weekIdx - 1, 0);
          focusCell(prevWeek, dayIdx);
          break;
        }
        case 'ArrowDown': {
          e.preventDefault();
          const nextDay = Math.min(dayIdx + 1, DAYS_PER_WEEK - 1);
          focusCell(weekIdx, nextDay);
          break;
        }
        case 'ArrowUp': {
          e.preventDefault();
          const prevDay = Math.max(dayIdx - 1, 0);
          focusCell(weekIdx, prevDay);
          break;
        }
        default:
          break;
      }
    },
    [isPinned, tooltipDay, openTooltip, closeTooltip, focusCell],
  );

  // Keep focusPos in sync when a cell receives focus via Tab
  const handleCellFocus = useCallback(
    (entry: HeatmapData, weekIdx: number, dayIdx: number, e: React.FocusEvent<HTMLButtonElement>) => {
      setFocusPos([weekIdx, dayIdx]);
      // Show tooltip on focus (keyboard a11y) if not already pinned on another day
      if (!isPinned) {
        openTooltip(entry, e.currentTarget);
      }
    },
    [isPinned, openTooltip],
  );

  const handleCellBlur = useCallback(() => {
    if (!isPinnedRef.current) {
      hoverTimerRef.current = setTimeout(() => {
        if (!isPinnedRef.current) setTooltipDay(null);
      }, 100);
    }
  }, []);

  // Cleanup timer on unmount
  useEffect(() => () => clearHoverTimer(), [clearHoverTimer]);

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    // Position relative so the absolutely-positioned tooltip stays in the
    // document flow rather than escaping to a portal (avoids scroll jank)
    <div className="w-full space-y-4 relative">

      {/* ── Title bar ─────────────────────────────────────────────────────── */}
      <div className="flex items-center justify-between mb-4">
        <h2
          className={`text-lg sm:text-xl font-bold ${
            isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}
          id="heatmap-title"
        >
          {isLoading ? (
            <span className="inline-block h-8 w-32 bg-neutral-300/40 animate-pulse rounded" />
          ) : (
            <>
              <span className="text-2xl sm:text-3xl lg:text-4xl font-black">
                {totalContributions}
              </span>
              <span
                className={`text-sm sm:text-base ml-2 ${
                  isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}
              >
                contributions last year
              </span>
            </>
          )}
        </h2>

        <div className="flex items-center gap-2 text-sm">
          <span className={isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}>
            2025
          </span>
        </div>
      </div>

      {/* ── Scrollable container ───────────────────────────────────────────── */}
      <div
        className="w-full backdrop-blur-[20px] bg-white/[0.12] rounded-lg sm:rounded-xl lg:rounded-2xl border border-white/30 p-4 sm:p-6 overflow-x-auto lg:overflow-visible"
        role="region"
        aria-label="Contribution Heatmap"
        aria-describedby="heatmap-desc"
      >
        {/* Screen-reader description */}
        <p id="heatmap-desc" className="sr-only">
          A 365-day contribution heatmap. Color intensity indicates activity:
          light gray is no contributions, gold is maximum. Use Tab to enter the
          grid, arrow keys to navigate cells, Enter or Space to view details,
          Escape to close the preview.
        </p>

        {/* Month labels */}
        <div className="flex mb-4 lg:mb-6 min-w-max lg:min-w-full">
          <div className="w-12 sm:w-14 flex-shrink-0" aria-hidden="true" />
          <div className="flex-1 flex justify-between px-1" aria-hidden="true">
            {MONTHS.map((month, idx) => (
              <div
                key={idx}
                className={`text-xs sm:text-sm font-bold text-center ${
                  isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}
              >
                {month}
              </div>
            ))}
          </div>
        </div>

        {/* Grid + day labels */}
        <div className="flex gap-2 sm:gap-3 min-w-max lg:min-w-full">
          {/* Y-axis day labels */}
          <div
            className="flex flex-col justify-between py-0.5 flex-shrink-0"
            aria-hidden="true"
          >
            {/* Short labels: mobile */}
            <div className="flex flex-col justify-between py-0.5 md:hidden">
              {DAYS_SHORT.map((d, i) => (
                <div
                  key={i}
                  className={`h-4 sm:h-5 flex items-center justify-center text-xs font-semibold ${
                    isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}
                >
                  {d}
                </div>
              ))}
            </div>
            {/* Full labels: md+ */}
            <div className="hidden md:flex flex-col justify-between py-0.5 h-full">
              {DAYS_FULL.map((d, i) => (
                <div
                  key={i}
                  className={`h-5 flex items-center text-sm font-semibold ${
                    isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}
                >
                  {d}
                </div>
              ))}
            </div>
          </div>

          {/* ── Heatmap grid ──────────────────────────────────────────────── */}
          {isLoading ? (
            /* Skeleton */
            <div className="flex gap-2 sm:gap-3" aria-busy="true" aria-label="Loading heatmap">
              {Array.from({ length: 52 }).map((_, wi) => (
                <div key={wi} className="flex flex-col gap-2 sm:gap-3">
                  {Array.from({ length: 7 }).map((_, di) => (
                    <div
                      key={di}
                      className="w-4 h-4 sm:w-5 sm:h-5 lg:w-6 lg:h-6 bg-neutral-300/30 rounded animate-pulse"
                    />
                  ))}
                </div>
              ))}
            </div>
          ) : (
            <div
              ref={gridRef}
              role="grid"
              aria-label="Contribution activity grid"
              aria-rowcount={DAYS_PER_WEEK}
              aria-colcount={WEEKS}
              className="flex gap-1 sm:gap-2 lg:gap-3"
            >
              {heatmapGrid.map((week, weekIdx) => (
                <div
                  key={weekIdx}
                  role="row"
                  // Columns are visually vertical (week columns), but semantically
                  // each "row" here is one week column. We label it for AT.
                  aria-label={`Week ${weekIdx + 1}`}
                  className="flex flex-col gap-1 sm:gap-2 lg:gap-3"
                >
                  {week.map((entry, dayIdx) => {
                    const isFocused =
                      focusPos?.[0] === weekIdx && focusPos?.[1] === dayIdx;
                    const isActive = tooltipDay?.date === entry.date;

                    return (
                      <button
                        key={dayIdx}
                        data-cell={`${weekIdx}-${dayIdx}`}
                        role="gridcell"
                        aria-label={
                          entry.count === 0
                            ? `${entry.date}: No activity`
                            : `${entry.date}: ${entry.count} contribution${entry.count !== 1 ? 's' : ''}${
                                entry.streak
                                  ? `, day ${entry.streak.streakDay} of a ${entry.streak.streakLength}-day streak`
                                  : ''
                              }`
                        }
                        aria-selected={isActive}
                        // Only the first cell in the grid is in the tab sequence;
                        // arrow keys handle intra-grid navigation.
                        tabIndex={weekIdx === 0 && dayIdx === 0 ? 0 : isFocused ? 0 : -1}
                        onClick={(e) => handleClick(entry, e)}
                        onKeyDown={(e) => handleKeyDown(entry, weekIdx, dayIdx, e)}
                        onMouseEnter={(e) => handleMouseEnter(entry, e)}
                        onMouseLeave={handleMouseLeave}
                        onTouchStart={(e) => handleTouchStart(entry, e)}
                        onFocus={(e) => handleCellFocus(entry, weekIdx, dayIdx, e)}
                        onBlur={handleCellBlur}
                        className={[
                          'w-4 h-4 sm:w-5 sm:h-5 md:w-6 md:h-6 lg:w-7 lg:h-7',
                          'rounded border-2 cursor-pointer',
                          // Transition: scale for full-motion; reduced-motion
                          // overrides to opacity-only via global CSS
                          'transition-all duration-150 ease-out',
                          'motion-reduce:transition-none',
                          'hover:scale-110 hover:z-20',
                          // Focus ring – WCAG 2.4.7
                          'focus-visible:outline focus-visible:outline-2',
                          'focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]',
                          'focus-visible:shadow-[0_0_0_4px_rgba(241,180,0,0.25)]',
                          getHeatmapColor(entry.level, isDark),
                          getLevelShadow(entry.level),
                          isActive ? 'ring-2 ring-[#f1b400] scale-110 z-20' : '',
                        ]
                          .filter(Boolean)
                          .join(' ')}
                      >
                        {entry.level >= 3 && entry.count > 0 && (
                          <Sparkles
                            className="w-2 h-2 sm:w-2.5 sm:h-2.5 text-white drop-shadow-lg motion-safe:animate-pulse"
                            aria-hidden="true"
                          />
                        )}
                      </button>
                    );
                  })}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* ── Legend ────────────────────────────────────────────────────────── */}
        <div
          className="flex flex-wrap items-center justify-center lg:justify-end gap-3 lg:gap-4 mt-6 lg:mt-8 pt-6 lg:pt-8 border-t border-white/20"
          aria-label="Contribution intensity legend"
        >
          <span
            className={`text-xs sm:text-sm font-bold ${
              isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}
          >
            Less
          </span>
          <div className="flex items-center gap-1.5 sm:gap-2">
            {[0, 1, 2, 3, 4].map((level) => (
              <div
                key={level}
                className={`w-4 h-4 sm:w-5 sm:h-5 rounded border-2 ${getHeatmapColor(
                  level,
                  isDark,
                )}`}
                aria-label={
                  ['No contributions', 'Low', 'Medium', 'High', 'Maximum'][level]
                }
              />
            ))}
          </div>
          <span
            className={`text-xs sm:text-sm font-bold ${
              isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}
          >
            More
          </span>
        </div>
      </div>

      {/* ── Enriched tooltip ──────────────────────────────────────────────── */}
      <HeatmapTooltip
        day={tooltipDay}
        anchorRef={anchorRef}
        isTouchOpen={isTouchOpen}
        onDismiss={closeTooltip}
        isDark={isDark}
      />

      {/* ── Screen-reader data table alternative ──────────────────────────── */}
      <table className="sr-only" aria-label="365-Day Contribution Activity Table">
        <thead>
          <tr>
            <th scope="col">Date</th>
            <th scope="col">Day</th>
            <th scope="col">Contributions</th>
            <th scope="col">Intensity Level</th>
          </tr>
        </thead>
        <tbody>
          {heatmapGrid.flat().map((entry) => {
            const date = new Date(entry.date + 'T00:00:00');
            const dayName = date.toLocaleDateString('en-US', { weekday: 'long' });
            const levelNames = ['Empty', 'Low', 'Medium', 'High', 'Maximum'];
            return (
              <tr key={entry.date}>
                <td>{entry.date}</td>
                <td>{dayName}</td>
                <td>{entry.count}</td>
                <td>{levelNames[Math.min(entry.level, 4)]}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
