/**
 * @file Custom glassmorphism tooltip for the onboarding tour.
 *
 * Rendered by Joyride via the `tooltipComponent` prop. Using a custom component
 * (instead of inline `styles.tooltip`) is the only reliable way to apply the
 * app's `backdrop-blur` glass surface and to render the progress dots + step
 * counter the spec calls for.
 *
 * @accessibility
 * - The container spreads `tooltipProps` (`role="dialog"`, `aria-modal`) and is
 *   labelled / described by the title and body via `aria-labelledby` /
 *   `aria-describedby`, so screen readers announce it as a modal dialog with
 *   meaningful content.
 * - Every control spreads Joyride's `*Props` (which carry `aria-label`, `role`,
 *   and `onClick`) and adds `type="button"` plus a visible focus ring. Joyride's
 *   focus trap keeps Tab within these controls; ESC / Dismiss / Skip provide the
 *   required escape (WCAG 2.1.2 No Keyboard Trap).
 */

import type { TooltipRenderProps } from 'react-joyride';
import { X } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function TourTooltip({
  index,
  size,
  isLastStep,
  step,
  backProps,
  closeProps,
  primaryProps,
  skipProps,
  tooltipProps,
}: TooltipRenderProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const isFirst = index === 0;
  const titleId = `onboarding-tour-title-${index}`;
  const bodyId = `onboarding-tour-body-${index}`;

  return (
    <div
      {...tooltipProps}
      aria-labelledby={step.title ? titleId : undefined}
      aria-describedby={bodyId}
      style={{ width: 384, maxWidth: '90vw' }}
      className={`backdrop-blur-[40px] rounded-[24px] border p-6 shadow-[0_8px_32px_rgba(0,0,0,0.18)] ${
        isDark ? 'bg-[#2d2820]/[0.72] border-white/10' : 'bg-white/[0.55] border-white/40'
      }`}
    >
      {/* Header: dismissible step counter */}
      <div className="flex items-center justify-between mb-3">
        <span
          className={`text-[12px] font-semibold uppercase tracking-wide ${
            isDark ? 'text-[#e8c77f]' : 'text-[#a2792c]'
          }`}
        >
          Step {index + 1} of {size}
        </span>
        <button
          type="button"
          {...closeProps}
          className={`p-1.5 rounded-full transition-colors focus:outline-2 focus:outline-offset-2 focus:outline-[#c9983a] ${
            isDark ? 'text-[#b8a898] hover:bg-white/10' : 'text-[#7a6b5a] hover:bg-black/5'
          }`}
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Title + body */}
      {step.title ? (
        <h2
          id={titleId}
          className={`text-[18px] font-bold mb-2 ${isDark ? 'text-[#f5efe5]' : 'text-[#2d2820]'}`}
        >
          {step.title}
        </h2>
      ) : null}
      <div
        id={bodyId}
        className={`text-[14px] leading-relaxed mb-5 ${isDark ? 'text-[#d4c5b0]' : 'text-[#6b5d4d]'}`}
      >
        {step.content}
      </div>

      {/* Progress dots (decorative — the counter above is the SR-facing source) */}
      <div className="flex items-center gap-1.5 mb-5" aria-hidden="true">
        {Array.from({ length: size }).map((_, i) => (
          <span
            key={i}
            className={`h-1.5 rounded-full transition-all ${
              i === index
                ? 'w-5 bg-[#c9983a]'
                : `w-1.5 ${isDark ? 'bg-white/20' : 'bg-black/15'}`
            }`}
          />
        ))}
      </div>

      {/* Footer controls */}
      <div className="flex items-center justify-between">
        <button
          type="button"
          {...skipProps}
          className={`text-[13px] font-medium transition-colors focus:outline-2 focus:outline-offset-2 focus:outline-[#c9983a] ${
            isDark ? 'text-[#b8a898] hover:text-[#e8dfd0]' : 'text-[#7a6b5a] hover:text-[#2d2820]'
          }`}
        >
          Skip tour
        </button>

        <div className="flex items-center gap-2">
          {!isFirst ? (
            <button
              type="button"
              {...backProps}
              className={`px-4 py-2.5 rounded-[14px] text-[13px] font-semibold border transition-all focus:outline-2 focus:outline-offset-2 focus:outline-[#c9983a] ${
                isDark
                  ? 'bg-white/[0.08] border-white/20 text-[#e8dfd0] hover:bg-white/[0.12]'
                  : 'bg-white/[0.4] border-white/50 text-[#2d2820] hover:bg-white/[0.6]'
              }`}
            >
              Back
            </button>
          ) : null}
          <button
            type="button"
            {...primaryProps}
            className="px-5 py-2.5 rounded-[14px] text-[13px] font-semibold bg-[#c9983a] text-white shadow-[0_4px_16px_rgba(201,152,58,0.25)] transition-colors hover:bg-[#a67c2e] focus:outline-2 focus:outline-offset-2 focus:outline-[#c9983a]"
          >
            {isLastStep ? 'Finish' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  );
}
