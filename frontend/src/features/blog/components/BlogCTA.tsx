import { ArrowRight, Github } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function BlogCTA() {
  const { theme } = useTheme();

  return (
    <div className="text-center p-4 md:p-6 lg:p-8 backdrop-blur-[30px] bg-gradient-to-br from-[#c9983a]/15 to-[#d4af37]/10 rounded-[16px] md:rounded-[20px] border-2 border-[#c9983a]/30">
      <h3 className={`text-lg md:text-2xl lg:text-[28px] font-bold mb-2 md:mb-3 transition-colors ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>Ready to Get Started?</h3>
      <p className={`text-xs md:text-sm lg:text-[16px] mb-4 md:mb-5 lg:mb-6 max-w-2xl mx-auto transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
      }`}>
        Whether you're a developer looking for your next challenge or a project seeking talented contributors, OnlyGrain is your gateway to the future of open-source collaboration.
      </p>
      <div className="flex flex-col sm:flex-row items-center justify-center gap-2 md:gap-3 lg:gap-4">
        <button className="w-full sm:w-auto px-5 md:px-8 py-2.5 md:py-3 lg:py-4 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[12px] md:rounded-[16px] font-bold text-xs md:text-sm lg:text-[16px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all flex items-center justify-center gap-1.5 md:gap-2 border border-white/10 active:scale-95 md:active:scale-100 min-h-10 md:min-h-0">
          Join as Contributor
          <ArrowRight className="w-3.5 h-3.5 md:w-4 md:h-5" />
        </button>
        <button className={`w-full sm:w-auto px-5 md:px-8 py-2.5 md:py-3 lg:py-4 backdrop-blur-[30px] bg-white/[0.25] border-2 border-[#c9983a] rounded-[12px] md:rounded-[16px] font-bold text-xs md:text-sm lg:text-[16px] hover:bg-white/[0.35] transition-all flex items-center justify-center gap-1.5 md:gap-2 active:scale-95 md:active:scale-100 min-h-10 md:min-h-0 ${
          theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
        }`}>
          Submit Your Project
          <Github className="w-3.5 h-3.5 md:w-4 md:h-5" />
        </button>
      </div>
    </div>
  );
}
