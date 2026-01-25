import { Calendar, Clock, ArrowRight } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogPost } from '../types';

interface BlogPostCardProps {
  post: BlogPost;
}

export function BlogPostCard({ post }: BlogPostCardProps) {
  const { theme } = useTheme();

  return (
    <div className="backdrop-blur-[30px] bg-white/[0.15] rounded-[20px] border border-white/25 p-6 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.08)] transition-all cursor-pointer group">
      <div className="w-16 h-16 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-lg text-3xl mb-4 border border-white/15 group-hover:scale-110 transition-transform duration-300">
        {post.icon}
      </div>
      
      {post.category && (
        <div className="flex items-center gap-2 mb-3">
          <span className="px-3 py-1 bg-[#c9983a]/20 border border-[#c9983a]/35 rounded-[8px] text-[11px] font-semibold text-[#8b6f3a]">
            {post.category}
          </span>
        </div>
      )}

      <h4 className={`text-[18px] font-bold mb-3 group-hover:text-[#c9983a] transition-colors ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>
        {post.title}
      </h4>

      <p className={`text-[14px] mb-4 leading-relaxed line-clamp-3 transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
      }`}>
        {post.excerpt}
      </p>

      <div className={`flex items-center gap-3 text-[12px] pb-4 border-b border-white/10 mb-4 transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
      }`}>
        <span className="flex items-center gap-1">
          <Calendar className="w-3.5 h-3.5" />
          {post.date}
        </span>
        <span>•</span>
        <span className="flex items-center gap-1">
          <Clock className="w-3.5 h-3.5" />
          {post.readTime}
        </span>
      </div>

      <button className="text-[14px] font-semibold text-[#c9983a] hover:text-[#a67c2e] transition-colors flex items-center gap-2">
        Read More
        <ArrowRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
      </button>
    </div>
  );
}
