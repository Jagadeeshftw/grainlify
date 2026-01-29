import { Calendar, Clock, User, ArrowRight } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogPost } from '../types';

interface FeaturedPostProps {
  post: BlogPost;
}

export function FeaturedPost({ post }: FeaturedPostProps) {
  const { theme } = useTheme();

  return (
    <div className="backdrop-blur-[40px] bg-gradient-to-br from-white/[0.18] to-white/[0.12] rounded-[20px] md:rounded-[28px] border border-white/25 shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden group hover:shadow-[0_12px_40px_rgba(0,0,0,0.12)] transition-all duration-500 cursor-pointer">
      <div className="relative">
        {/* Animated Glow Effects */}
        <div className="absolute inset-0 opacity-15">
          <div className="absolute top-1/4 left-1/4 w-32 h-32 bg-[#c9983a]/40 rounded-full blur-[60px] animate-pulse" />
          <div className="absolute bottom-1/4 right-1/4 w-32 h-32 bg-[#d4af37]/30 rounded-full blur-[70px] animate-pulse" style={{ animationDelay: '0.5s' }} />
        </div>

        <div className="relative z-10 p-4 md:p-10 sm:p-6">
          <div className="flex flex-col md:flex-row md:items-start md:gap-8 gap-4">
            {/* Icon/Image */}
            <div className="flex-shrink-0 w-24 h-24 md:w-32 md:h-32 rounded-[20px] md:rounded-[24px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-xl text-4xl md:text-6xl border-2 border-white/20 group-hover:scale-110 transition-transform duration-500">
              {post.image}
            </div>

            {/* Content */}
            <div className="flex-1 min-w-0">
              <div className="flex flex-wrap items-center gap-2 md:gap-3 mb-3 md:mb-4">
                <span className="px-3 md:px-4 py-1 md:py-1.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[8px] md:rounded-[10px] text-[10px] md:text-[12px] font-bold shadow-md border border-white/10 whitespace-nowrap">
                  FEATURED
                </span>
                <span className={`text-[11px] md:text-[13px] flex items-center gap-1 md:gap-1.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                  <Calendar className="w-3 h-3 md:w-3.5 md:h-3.5" />
                  {post.date}
                </span>
                <span className={`text-[11px] md:text-[13px] flex items-center gap-1 md:gap-1.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                  <Clock className="w-3 h-3 md:w-3.5 md:h-3.5" />
                  {post.readTime}
                </span>
              </div>

              <h2 className={`text-xl md:text-[32px] font-bold mb-3 md:mb-4 leading-tight group-hover:text-[#c9983a] transition-colors duration-300 ${
                theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>
                {post.title}
              </h2>

              <p className={`text-sm md:text-[16px] mb-4 md:mb-6 leading-relaxed transition-colors line-clamp-2 md:line-clamp-none ${
                theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
              }`}>
                {post.excerpt}
              </p>

              <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3 md:gap-0">
                {post.author && (
                  <div className={`flex items-center gap-2 text-xs md:text-[14px] transition-colors min-w-0 ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>
                    <User className="w-3.5 h-3.5 md:w-4 md:h-4 flex-shrink-0" />
                    <span className={`font-medium transition-colors truncate ${
                      theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>{post.author}</span>
                  </div>
                )}
                <button className="px-4 md:px-6 py-2.5 md:py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[12px] md:rounded-[14px] font-semibold text-xs md:text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all flex items-center gap-1.5 md:gap-2 border border-white/10 group-hover:scale-105 active:scale-95 md:active:scale-105 whitespace-nowrap">
                  Read Full Story
                  <ArrowRight className="w-3.5 h-3.5 md:w-4 md:h-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
