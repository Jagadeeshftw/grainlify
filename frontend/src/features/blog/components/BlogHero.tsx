import { BookOpen, Sparkles } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function BlogHero() {
  const { theme } = useTheme();

  return (
    <div className="relative backdrop-blur-[40px] bg-gradient-to-br from-white/[0.18] to-white/[0.12] rounded-[20px] md:rounded-[28px] border border-white/25 shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden">
      {/* Animated Background Elements */}
      <div className="absolute inset-0 opacity-20">
        <div className="absolute top-10 right-10 w-40 h-40 bg-[#c9983a]/30 rounded-full blur-[80px] animate-pulse" />
        <div className="absolute bottom-10 left-10 w-48 h-48 bg-[#d4af37]/25 rounded-full blur-[90px] animate-pulse" style={{ animationDelay: '1s' }} />
      </div>

      <div className="relative z-10 p-6 md:p-12 sm:p-8">
        <div className="flex flex-col md:flex-row md:items-start md:justify-between gap-4 md:gap-0">
          <div className="flex-1">
            <div className="flex items-start gap-2 md:gap-3 mb-4">
              <div className="w-12 h-12 md:w-14 md:h-14 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-lg border border-white/20 flex-shrink-0">
                <BookOpen className="w-6 h-6 md:w-7 md:h-7 text-white" />
              </div>
              <div className="min-w-0">
                <h1 className={`text-2xl md:text-[38px] font-bold transition-colors leading-tight ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>OnlyGrain Blog</h1>
                <p className={`text-xs md:text-[15px] transition-colors mt-1 md:mt-0 ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Insights, updates, and stories from the OnlyGrain ecosystem</p>
              </div>
            </div>
          </div>
          <div className="w-16 h-16 md:w-20 md:h-20 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.35)] border border-white/15 animate-bounce-slow flex-shrink-0">
            <Sparkles className="w-8 h-8 md:w-10 md:h-10 text-white" />
          </div>
        </div>
      </div>
    </div>
  );
}
