/**
 * @file Glassmorphism tooltip for feature-discovery coach marks.
 *
 * @accessibility
 * - `role="note"` with `aria-label="Feature hint"` — informational, not modal
 * - `aria-live="polite"` on wrapper so screen readers announce the hint
 * - "Got it" button is keyboard-focusable (Tab + Enter/Space)
 * - Escape key dismisses the coach mark (handled by CoachMarkProvider)
 * - Never auto-dismisses — only explicit user action
 */

import { useTheme } from '../../../shared/contexts/ThemeContext';

export interface CoachMarkTooltipProps {
  title: string;
  body: string;
  placement: 'top' | 'right' | 'bottom' | 'left';
  onDismiss: () => void;
}

export function CoachMarkTooltip({
  title,
  body,
  placement,
  onDismiss,
}: CoachMarkTooltipProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const titleId = `coach-mark-title-${title.replace(/\s+/g, '-').toLowerCase()}`;
  const bodyId = `coach-mark-body-${title.replace(/\s+/g, '-').toLowerCase()}`;

  // Position the pointer relative to the bubble
  const pointerStyles: Record<string, string> = {
    top: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
    bottom: 'top-full left-1/2 -translate-x-1/2 mt-2',
    left: 'right-full top-1/2 -translate-y-1/2 mr-2',
    right: 'left-full top-1/2 -translate-y-1/2 ml-2',
  };

  const pointerTriangle: Record<string, string> = {
    top: 'rotate-180',
    bottom: '',
    left: '-rotate-90',
    right: 'rotate-90',
  };

  return (
    <div
      role="note"
      aria-label="Feature hint"
      aria-live="polite"
      className={`
        relative z-50 w-[320px] max-w-[90vw]
        backdrop-blur-[40px] rounded-[24px] border p-5
        shadow-[0_8px_32px_rgba(0,0,0,0.18)]
        animate-coach-mark-in
        ${isDark
          ? 'bg-[#2d2820]/[0.72] border-white/10'
          : 'bg-white/[0.55] border-white/40'
        }
      `}
    >
      {/* Pointer triangle */}
      <div
        className={`absolute ${pointerStyles[placement]} pointer-events-none`}
        aria-hidden="true"
      >
        <svg
          width="16"
          height="10"
          viewBox="0 0 16 10"
          fill="none"
          className={pointerTriangle[placement]}
        >
          <path
            d="M8 10L0 0H16L8 10Z"
            fill={isDark ? '#c9983a' : '#f1b400'}
          />
        </svg>
      </div>

      {/* Highlight ring indicator (decorative) */}
      <div
        className="absolute -top-1 -left-1 w-3 h-3 rounded-full border-2"
        style={{
          borderColor: isDark ? '#c9983a' : '#f1b400',
          backgroundColor: isDark ? 'rgba(201,152,58,0.2)' : 'rgba(241,180,0,0.2)',
        }}
        aria-hidden="true"
      />

      {/* Title */}
      <h3
        id={titleId}
        className={`text-[15px] font-bold mb-1.5 ${
          isDark ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}
      >
        {title}
      </h3>

      {/* Body */}
      <div
        id={bodyId}
        className={`text-[13px] leading-relaxed mb-4 ${
          isDark ? 'text-[#d4c5b0]' : 'text-[#6b5d4d]'
        }`}
      >
        {body}
      </div>

      {/* Got it button */}
      <button
        type="button"
        onClick={onDismiss}
        className="
          w-full px-4 py-2.5 rounded-[14px] text-[13px] font-semibold
          bg-[#f1b400] text-white
          shadow-[0_4px_16px_rgba(201,152,58,0.25)]
          transition-colors hover:bg-[#a67c2e]
          focus:outline-2 focus:outline-offset-2 focus:outline-[#c9983a]
          min-h-[44px]
        "
        aria-label="Dismiss hint"
      >
        Got it
      </button>
    </div>
  );
}
