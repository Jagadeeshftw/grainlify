import React, { useState, useEffect } from 'react';
import { Clock } from 'lucide-react';
import {
  getFormattedTimestamp,
  FormattedTimestamp,
} from '../utils/timestamp';
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
  TooltipProvider,
} from '../../app/components/ui/tooltip';

export interface TimestampDisplayProps {
  /** The ISO string, Date object, or numeric epoch timestamp */
  timestamp?: string | Date | number | null;
  /** Fallback text if timestamp is unavailable or a relative string */
  fallbackText?: string;
  /** Custom CSS classes for the container */
  className?: string;
  /** Whether to render a small clock icon before the timestamp string */
  showIcon?: boolean;
  /** Custom test ID for unit testing */
  'data-testid'?: string;
}

export function TimestampDisplay({
  timestamp,
  fallbackText = 'Recently',
  className = '',
  showIcon = false,
  'data-testid': testId = 'timestamp-display',
}: TimestampDisplayProps) {
  const [formatted, setFormatted] = useState<FormattedTimestamp>(() =>
    getFormattedTimestamp(timestamp, fallbackText)
  );

  useEffect(() => {
    // Initial compute on prop change
    const nextFormatted = getFormattedTimestamp(timestamp, fallbackText);
    setFormatted(nextFormatted);

    // If no periodic update interval is needed (e.g. event > 7 days old or missing date), skip timer
    if (!nextFormatted.updateIntervalMs) {
      return;
    }

    let timerId: ReturnType<typeof setTimeout>;

    const scheduleTick = (delay: number) => {
      timerId = setTimeout(() => {
        const updated = getFormattedTimestamp(timestamp, fallbackText);
        setFormatted(updated);

        if (updated.updateIntervalMs) {
          scheduleTick(updated.updateIntervalMs);
        }
      }, delay);
    };

    scheduleTick(nextFormatted.updateIntervalMs);

    return () => {
      if (timerId) clearTimeout(timerId);
    };
  }, [timestamp, fallbackText]);

  // Screen reader accessible aria-label containing full details
  const ariaLabel = formatted.isoString
    ? `${formatted.display} - ${formatted.localFull} (${formatted.utcFull})`
    : fallbackText;

  return (
    <TooltipProvider delayDuration={150}>
      <Tooltip>
        <TooltipTrigger asChild>
          <time
            data-testid={testId}
            dateTime={formatted.isoString || undefined}
            tabIndex={0}
            aria-label={ariaLabel}
            className={`inline-flex items-center gap-1 cursor-help transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a] rounded ${className}`}
          >
            {showIcon && <Clock className="w-3 h-3 text-current opacity-70 flex-shrink-0" />}
            <span>{formatted.display}</span>
          </time>
        </TooltipTrigger>

        <TooltipContent
          side="top"
          align="center"
          className="bg-[#1f1b15]/95 border border-[#c9983a]/30 text-[#e8dfd0] backdrop-blur-md px-3 py-2 shadow-lg max-w-xs z-50 rounded-lg text-left"
        >
          {formatted.isoString ? (
            <div className="space-y-1 text-xs">
              <div className="flex items-center gap-1.5 font-medium text-[#f5efe5]">
                <Clock className="w-3.5 h-3.5 text-[#c9983a] flex-shrink-0" />
                <span>{formatted.localFull}</span>
              </div>
              <div className="text-[11px] text-[#b8a898] font-mono pl-5">
                {formatted.utcFull}
              </div>
            </div>
          ) : (
            <div className="text-xs text-[#e8dfd0]">{fallbackText}</div>
          )}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
