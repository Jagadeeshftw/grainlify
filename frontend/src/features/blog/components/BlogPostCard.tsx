import { Calendar, Clock, ArrowRight } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogPost } from '../types';

interface BlogPostCardProps {
  post: BlogPost;
}

export function BlogPostCard({ post }: BlogPostCardProps) {
  const { theme } = useTheme();

  return (
    <div className="backdrop-blur-[30px] bg-white/[0.15] rounded-[16px] md:rounded-[20px] border border-white/25 p-4 md:p-6 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.08)] transition-all cursor-pointer group active:scale-95 md:active:scale-100 touch-highlight">
      <div className="w-14 h-14 md:w-16 md:h-16 rounded-[12px] md:rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-lg text-2xl md:text-3xl mb-3 md:mb-4 border border-white/15 group-hover:scale-110 transition-transform duration-300">
        {post.icon}
      </div>
      
      {post.category && (
        <div className="flex items-center gap-2 mb-2.5 md:mb-3">
          <span className="px-2 md:px-3 py-0.5 md:py-1 bg-[#c9983a]/20 border border-[#c9983a]/35 rounded-[6px] md:rounded-[8px] text-[10px] md:text-[11px] font-semibold text-[#8b6f3a]">
            {post.category}
          </span>
        </div>
      )}

      <h4 className={`text-base md:text-[18px] font-bold mb-2 md:mb-3 group-hover:text-[#c9983a] transition-colors line-clamp-2 ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>
        {post.title}
      </h4>

      <p className={`text-xs md:text-[14px] mb-3 md:mb-4 leading-relaxed line-clamp-2 md:line-clamp-3 transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
      }`}>
        {post.excerpt}
      </p>

      <div className={`flex items-center gap-2 md:gap-3 text-[10px] md:text-[12px] pb-3 md:pb-4 border-b border-white/10 mb-3 md:mb-4 transition-colors overflow-auto ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
      }`}>
        <span className="flex items-center gap-0.5 md:gap-1 whitespace-nowrap">
          <Calendar className="w-3 h-3 md:w-3.5 md:h-3.5" />
          <span className="hidden sm:inline">{post.date}</span>
          <span className="sm:hidden text-[9px]">{post.date.split(' ').pop()}</span>
        </span>
        <span className="hidden sm:inline">•</span>
        <span className="flex items-center gap-0.5 md:gap-1 whitespace-nowrap">
          <Clock className="w-3 h-3 md:w-3.5 md:h-3.5" />
          {post.readTime}
        </span>
      </div>

      <button className="text-xs md:text-[14px] font-semibold text-[#c9983a] hover:text-[#a67c2e] transition-colors flex items-center gap-1.5 md:gap-2 min-h-10 md:min-h-0">
        Read More
        <ArrowRight className="w-3.5 h-3.5 md:w-4 md:h-4 group-hover:translate-x-1 transition-transform" />
      </button>
    </div>
  );
}
