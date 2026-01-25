import { Calendar, Clock, User, ArrowRight } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogPost } from '../types';

interface FeaturedPostProps {
  post: BlogPost;
}

export function FeaturedPost({ post }: FeaturedPostProps) {
  const { theme } = useTheme();

  return (
    <div className="backdrop-blur-[40px] bg-gradient-to-br from-white/[0.18] to-white/[0.12] rounded-[28px] border border-white/25 shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden group hover:shadow-[0_12px_40px_rgba(0,0,0,0.12)] transition-all duration-500 cursor-pointer">
      <div className="relative">
        {/* Animated Glow Effects */}
        <div className="absolute inset-0 opacity-15">
          <div className="absolute top-1/4 left-1/4 w-32 h-32 bg-[#c9983a]/40 rounded-full blur-[60px] animate-pulse" />
          <div className="absolute bottom-1/4 right-1/4 w-32 h-32 bg-[#d4af37]/30 rounded-full blur-[70px] animate-pulse" style={{ animationDelay: '0.5s' }} />
        </div>

        <div className="relative z-10 p-10">
          <div className="flex items-start gap-8">
            {/* Icon/Image */}
            <div className="flex-shrink-0 w-32 h-32 rounded-[24px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-xl text-6xl border-2 border-white/20 group-hover:scale-110 transition-transform duration-500">
              {post.image}
            </div>

            {/* Content */}
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-4">
                <span className="px-4 py-1.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[10px] text-[12px] font-bold shadow-md border border-white/10">
                  FEATURED
                </span>
                <span className={`text-[13px] flex items-center gap-1.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                  <Calendar className="w-3.5 h-3.5" />
                  {post.date}
                </span>
                <span className={`text-[13px] flex items-center gap-1.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                  <Clock className="w-3.5 h-3.5" />
                  {post.readTime}
                </span>
              </div>

              <h2 className={`text-[32px] font-bold mb-4 leading-tight group-hover:text-[#c9983a] transition-colors duration-300 ${
                theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>
                {post.title}
              </h2>

              <p className={`text-[16px] mb-6 leading-relaxed transition-colors ${
                theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
              }`}>
                {post.excerpt}
              </p>

              <div className="flex items-center justify-between">
                {post.author && (
                  <div className={`flex items-center gap-2 text-[14px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>
                    <User className="w-4 h-4" />
                    <span className={`font-medium transition-colors ${
                      theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>{post.author}</span>
                  </div>
                )}
                <button className="px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[14px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all flex items-center gap-2 border border-white/10 group-hover:scale-105">
                  Read Full Story
                  <ArrowRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
