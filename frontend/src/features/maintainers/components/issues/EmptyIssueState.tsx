import { Inbox } from 'lucide-react';

interface EmptyIssueStateProps {
  issueCount: number;
}

export function EmptyIssueState({ issueCount }: EmptyIssueStateProps) {
  const displayCount = Number.isFinite(issueCount) ? issueCount : 0;
  return (
    <>
      {/* Background decorative circles (smaller on mobile) */}
      <div className="hidden md:block absolute top-[18%] left-[12%] w-96 h-96 bg-gradient-to-br from-[#c9983a]/5 to-transparent rounded-full blur-3xl pointer-events-none" />
      <div className="hidden sm:block absolute bottom-[18%] right-[12%] w-80 h-80 bg-gradient-to-tl from-[#d4af37]/5 to-transparent rounded-full blur-3xl pointer-events-none" />

      <div className="absolute inset-0 flex items-center justify-center px-4 sm:px-6">
        {/* Placeholder Illustration */}
        <div className="text-center relative w-full max-w-md sm:max-w-lg md:max-w-xl">
          {/* Animated Icon Container */}
          <div className="relative mx-auto mb-6 sm:mb-8 w-20 h-20 sm:w-24 sm:h-24 md:w-32 md:h-32 group/icon">
            {/* Outer glow ring */}
            <div className="absolute inset-0 rounded-full bg-gradient-to-br from-[#c9983a]/20 to-[#d4af37]/10 blur-xl animate-pulse" />

            {/* Main icon circle */}
            <div className="relative w-full h-full rounded-full bg-gradient-to-br from-[#c9983a]/15 to-[#d4af37]/10 border-2 border-[#c9983a]/30 flex items-center justify-center backdrop-blur-[20px] group-hover/icon:scale-110 group-hover/icon:border-[#c9983a]/50 transition-all duration-500">
              <Inbox className="w-10 h-10 sm:w-12 sm:h-12 md:w-16 md:h-16 text-[#c9983a]/60 group-hover/icon:text-[#c9983a]/80 transition-colors duration-300" strokeWidth={1.5} />
            </div>

            {/* Decorative dots (subtle on small screens) */}
            <div className="hidden sm:block absolute -top-2 -right-2 w-4 h-4 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] opacity-70 animate-pulse" style={{ animationDelay: '0.2s' }} />
            <div className="hidden sm:block absolute -bottom-2 -left-2 w-3 h-3 rounded-full bg-gradient-to-br from-[#d4af37] to-[#c9983a] opacity-70 animate-pulse" style={{ animationDelay: '0.4s' }} />
          </div>

          {/* Text content */}
          <h3 className="text-[18px] sm:text-[20px] md:text-[24px] font-bold text-[#4a3f2f] mb-2 sm:mb-3 tracking-tight">Select an issue</h3>
          <p className="text-[14px] sm:text-[15px] md:text-[16px] font-medium text-[#7a6b5a]/80 max-w-xs sm:max-w-md mx-auto leading-relaxed">
            Choose an issue from the list to view details
          </p>

          {/* Subtle hint */}
          <div className="mt-5 sm:mt-6 inline-flex items-center gap-2 px-3 sm:px-4 py-1.5 sm:py-2 rounded-full bg-white/20 border border-white/30">
            <div className="w-2 h-2 rounded-full bg-[#c9983a] animate-pulse" />
            <span className="text-[12px] sm:text-[13px] font-semibold text-[#7a6b5a]">{displayCount} issue{displayCount === 1 ? '' : 's'} available</span>
          </div>
        </div>
      </div>
    </>
  );
}
