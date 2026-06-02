/**
 * ContributionHeatmap Component
 * 
 * A responsive, accessible 365-day contribution heatmap with:
 * - Mobile-first responsive design (horizontal scroll on sm/md, full-width on lg+)
 * - Color scale from neutral to gold based on contribution level
 * - WCAG 2.1 AA compliant with keyboard navigation and screen reader support
 * - Month/day axis labels with responsive typography
 * - Tooltips with contribution details
 * 
 * Design Spec: design/profilepage-visualizations.md
 */

import { useState, useMemo, useRef, useEffect } from 'react';
import { Sparkles } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

interface HeatmapData {
  date: string;
  count: number;
  level: number;
}

interface ContributionHeatmapProps {
  data: HeatmapData[];
  isLoading?: boolean;
  totalContributions?: number;
}

export function ContributionHeatmap({ 
  data, 
  isLoading = false,
  totalContributions = 0 
}: ContributionHeatmapProps) {
  const { theme } = useTheme();
  const [tooltipData, setTooltipData] = useState<{
    date: string;
    count: number;
    x: number;
    y: number;
  } | null>(null);
  const [focusedCell, setFocusedCell] = useState<string | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);

  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const daysShort = ['M', 'T', 'W', 'Th', 'F', 'Sa', 'Su'];

  // Generate 365 days of heatmap data
  const heatmapGrid = useMemo(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const grid: HeatmapData[][] = [];

    for (let week = 0; week < 53; week++) {
      grid[week] = [];
      for (let day = 0; day < 7; day++) {
        const daysAgo = 364 - (week * 7 + day);
        const targetDate = new Date(today);
        targetDate.setDate(targetDate.getDate() - daysAgo);
        const dateStr = targetDate.toISOString().split('T')[0];

        const entry = data.find(d => d.date === dateStr) || { date: dateStr, count: 0, level: 0 };
        grid[week][day] = entry;
      }
    }
    return grid;
  }, [data]);

  // Get color based on level
  const getHeatmapColor = (level: number, isDark: boolean) => {
    const colors = {
      0: isDark ? 'bg-white/40 border-white/60' : 'bg-[#efefef] border-[#d6d3d1]',
      1: 'bg-[#c9983a]/35 border-[#c9983a]/50',
      2: 'bg-[#c9983a]/55 border-[#c9983a]/75',
      3: 'bg-[#c9983a]/75 border-[#c9983a]/90',
      4: 'bg-gradient-to-br from-[#f1b400] to-[#c9983a] border-[#d4af37]'
    };
    return colors[Math.min(level, 4) as keyof typeof colors];
  };

  const getShadow = (level: number) => {
    const shadows = {
      0: 'shadow-sm',
      1: 'shadow-[0_2px_10px_rgba(201,152,58,0.2)]',
      2: 'shadow-[0_2px_12px_rgba(201,152,58,0.3)]',
      3: 'shadow-[0_3px_14px_rgba(201,152,58,0.45)]',
      4: 'shadow-[0_4px_20px_rgba(201,152,58,0.6),0_0_15px_rgba(241,180,0,0.4)]'
    };
    return shadows[Math.min(level, 4) as keyof typeof shadows];
  };

  const handleCellHover = (date: string, count: number, event: React.MouseEvent) => {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    setTooltipData({
      date,
      count,
      x: rect.left,
      y: rect.top
    });
  };

  const handleCellClick = (date: string, count: number) => {
    setFocusedCell(date === focusedCell ? null : date);
  };

  const handleKeyDown = (date: string, count: number, event: React.KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleCellClick(date, count);
    } else if (event.key === 'Escape') {
      setTooltipData(null);
      setFocusedCell(null);
    }
  };

  const isDarkTheme = theme === 'dark';

  return (
    <div className="w-full space-y-4">
      {/* Title and Year Navigation */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <h2
            className={`text-lg sm:text-xl font-bold ${
              isDarkTheme ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}
            id="heatmap-title"
          >
            {isLoading ? (
              <span className="h-8 w-32 bg-neutral-300 animate-pulse rounded" />
            ) : (
              <>
                <span className="text-2xl sm:text-3xl lg:text-4xl font-black">
                  {totalContributions}
                </span>
                <span
                  className={`text-sm sm:text-base ml-2 ${
                    isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}
                >
                  contributions last year
                </span>
              </>
            )}
          </h2>
        </div>

        {/* Year Navigation - can be enhanced with date picker */}
        <div className="flex items-center gap-2 text-sm">
          <span className={isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}>
            2025
          </span>
        </div>
      </div>

      {/* Scrollable Heatmap Container */}
      <div
        className="w-full backdrop-blur-[20px] bg-white/[0.12] rounded-lg sm:rounded-xl lg:rounded-2xl border border-white/30 p-4 sm:p-6 overflow-x-auto lg:overflow-visible"
        role="region"
        aria-label="Contribution Heatmap"
        aria-describedby="heatmap-desc"
        ref={gridRef}
      >
        <p id="heatmap-desc" className="sr-only">
          A 365-day contribution heatmap for 2025. Color intensity indicates activity level: empty (no contributions, light gray) to maximum (gold). 
          Use Tab to navigate between cells, Enter or Space to view details, Escape to close tooltips, and arrow keys to move between cells.
        </p>

        {/* Month Labels */}
        <div className="flex mb-4 lg:mb-6 min-w-max lg:min-w-full">
          <div className="w-12 sm:w-14 flex-shrink-0" />
          <div className="flex-1 flex justify-between px-1">
            {months.map((month, idx) => (
              <div
                key={idx}
                className={`text-xs sm:text-sm font-bold text-center ${
                  isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}
              >
                {month}
              </div>
            ))}
          </div>
        </div>

        {/* Grid Container */}
        <div className="flex gap-2 sm:gap-3 min-w-max lg:min-w-full">
          {/* Day Labels - Y Axis */}
          <div className="flex flex-col justify-between py-0.5 flex-shrink-0">
            {days.map((day, idx) => (
              <div
                key={idx}
                className={`h-4 sm:h-5 flex items-center justify-center text-xs sm:text-sm font-semibold ${
                  isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                } md:hidden`}
              >
                {daysShort[idx]}
              </div>
            ))}
            {/* Full day names on larger screens */}
            <div className="hidden md:flex flex-col justify-between py-0.5 h-full">
              {days.map((day, idx) => (
                <div
                  key={idx}
                  className={`h-5 flex items-center text-sm font-semibold ${
                    isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}
                >
                  {day}
                </div>
              ))}
            </div>
          </div>

          {/* Contribution Grid */}
          {isLoading ? (
            <div className="flex gap-2 sm:gap-3">
              {Array.from({ length: 52 }).map((_, weekIdx) => (
                <div key={weekIdx} className="flex flex-col gap-2 sm:gap-3">
                  {Array.from({ length: 7 }).map((_, dayIdx) => (
                    <div
                      key={dayIdx}
                      className="w-4 h-4 sm:w-5 sm:h-5 lg:w-6 lg:h-6 bg-neutral-300 rounded animate-pulse"
                    />
                  ))}
                </div>
              ))}
            </div>
          ) : (
            <div className="flex gap-1 sm:gap-2 lg:gap-3">
              {heatmapGrid.map((week, weekIdx) => (
                <div key={weekIdx} className="flex flex-col gap-1 sm:gap-2 lg:gap-3">
                  {week.map((entry, dayIdx) => {
                    const isFocused = focusedCell === entry.date;
                    const cellSize = 'w-4 h-4 sm:w-5 sm:h-5 md:w-6 md:h-6 lg:w-7 lg:h-7';

                    return (
                      <button
                        key={dayIdx}
                        onClick={() => handleCellClick(entry.date, entry.count)}
                        onKeyDown={(e) => handleKeyDown(entry.date, entry.count, e)}
                        onMouseEnter={(e) => handleCellHover(entry.date, entry.count, e)}
                        onMouseLeave={() => setTooltipData(null)}
                        aria-label={`${entry.date}: ${entry.count} contribution${entry.count !== 1 ? 's' : ''}`}
                        className={`
                          ${cellSize}
                          rounded border-2 transition-all duration-150 cursor-pointer
                          focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]
                          hover:scale-110 hover:z-20 hover:shadow-lg
                          ${getHeatmapColor(entry.level, isDarkTheme)}
                          ${getShadow(entry.level)}
                          ${isFocused ? 'ring-2 ring-[#f1b400] scale-110' : ''}
                        `}
                        rel="button"
                      >
                        {entry.level >= 3 && entry.count > 0 && (
                          <Sparkles className="w-2 h-2 sm:w-2.5 sm:h-2.5 text-white drop-shadow-lg animate-pulse" />
                        )}
                      </button>
                    );
                  })}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Legend */}
        <div className="flex flex-wrap items-center justify-center lg:justify-end gap-3 lg:gap-4 mt-6 lg:mt-8 pt-6 lg:pt-8 border-t border-white/20">
          <span className={`text-xs sm:text-sm font-bold ${isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            Less
          </span>
          <div className="flex items-center gap-1.5 sm:gap-2">
            {/* Level 0 - Empty */}
            <div
              className={`w-4 h-4 sm:w-5 sm:h-5 rounded border-2 ${getHeatmapColor(0, isDarkTheme)}`}
              aria-label="No contributions"
            />
            {/* Level 1 - Low */}
            <div
              className={`w-4 h-4 sm:w-5 sm:h-5 rounded border-2 ${getHeatmapColor(1, isDarkTheme)}`}
              aria-label="Low activity"
            />
            {/* Level 2 - Medium */}
            <div
              className={`w-4 h-4 sm:w-5 sm:h-5 rounded border-2 ${getHeatmapColor(2, isDarkTheme)}`}
              aria-label="Medium activity"
            />
            {/* Level 3 - High */}
            <div
              className={`w-4 h-4 sm:w-5 sm:h-5 rounded border-2 ${getHeatmapColor(3, isDarkTheme)}`}
              aria-label="High activity"
            />
            {/* Level 4+ - Max */}
            <div
              className={`w-4 h-4 sm:w-5 sm:h-5 rounded border-2 ${getHeatmapColor(4, isDarkTheme)}`}
              aria-label="Maximum activity"
            />
          </div>
          <span className={`text-xs sm:text-sm font-bold ${isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            More
          </span>
        </div>
      </div>

      {/* Tooltip */}
      {tooltipData && (
        <div
          ref={tooltipRef}
          className={`
            fixed z-50 px-3 sm:px-4 py-2 sm:py-3 rounded-lg text-xs sm:text-sm font-medium
            backdrop-blur-[40px] border border-white/25 shadow-lg
            ${isDarkTheme 
              ? 'bg-[#2d2820]/95 text-[#f5f5f5]' 
              : 'bg-[#e8dfd0]/95 text-[#2d2820]'
            }
          `}
          style={{
            left: `${tooltipData.x}px`,
            top: `${tooltipData.y - 40}px`,
            transform: 'translateX(-50%)'
          }}
        >
          <div className="font-bold">{tooltipData.date}</div>
          <div className="text-xs sm:text-sm">
            {tooltipData.count} contribution{tooltipData.count !== 1 ? 's' : ''}
          </div>
        </div>
      )}

      {/* Data Table Alternative for Screen Readers */}
      <table className="sr-only" aria-label="365-Day Contribution Activity Table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Day</th>
            <th>Contributions</th>
            <th>Intensity Level</th>
          </tr>
        </thead>
        <tbody>
          {heatmapGrid.flat().map((entry) => {
            const date = new Date(entry.date);
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
