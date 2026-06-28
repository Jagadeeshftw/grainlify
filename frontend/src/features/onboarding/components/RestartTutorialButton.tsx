/**
 * @file Persistent "restart tutorial" entry point for the Settings page.
 *
 * Renders a glassmorphism card with a button that replays the onboarding tour.
 * It is shown regardless of the active Settings tab, giving contributors a
 * durable way back into the walkthrough after the first-run experience.
 */

import { Compass } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { useOnboardingTour } from '../tour/useOnboardingTour';

export function RestartTutorialButton() {
  const { theme } = useTheme();
  const { restartTour, hasSeen } = useOnboardingTour();
  const isDark = theme === 'dark';

  return (
    <div
      className={`backdrop-blur-[40px] rounded-[24px] border p-6 shadow-[0_8px_32px_rgba(0,0,0,0.08)] flex items-center justify-between gap-4 transition-colors ${
        isDark ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
      }`}
    >
      <div>
        <h3 className={`text-[16px] font-semibold mb-1 ${isDark ? 'text-[#f5efe5]' : 'text-[#2d2820]'}`}>
          Product tour
        </h3>
        <p className={`text-[13px] ${isDark ? 'text-[#b8a898]' : 'text-[#6b5d4d]'}`}>
          {hasSeen
            ? 'Replay the 6-step walkthrough of finding bounties, claiming issues, and tracking rewards.'
            : 'Take the 6-step walkthrough of finding bounties, claiming issues, and tracking rewards.'}
        </p>
      </div>
      <button
        type="button"
        onClick={restartTour}
        className="flex items-center gap-2 px-5 py-3 rounded-[14px] bg-[#c9983a] text-white text-[14px] font-semibold whitespace-nowrap shadow-[0_4px_16px_rgba(201,152,58,0.25)] transition-colors hover:bg-[#a67c2e] focus:outline-2 focus:outline-offset-2 focus:outline-[#c9983a]"
      >
        <Compass className="w-4 h-4" />
        {hasSeen ? 'Restart tutorial' : 'Start tutorial'}
      </button>
    </div>
  );
}
