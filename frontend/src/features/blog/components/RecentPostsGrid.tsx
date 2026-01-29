import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogPost } from '../types';
import { BlogPostCard } from './BlogPostCard';

interface RecentPostsGridProps {
  posts: BlogPost[];
}

export function RecentPostsGrid({ posts }: RecentPostsGridProps) {
  const { theme } = useTheme();

  return (
    <div>
      <h3 className={`text-lg md:text-[28px] font-bold mb-4 md:mb-6 transition-colors ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>Recent Articles</h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 md:gap-4 lg:gap-6">
        {posts.map((post) => (
          <BlogPostCard key={post.id} post={post} />
        ))}
      </div>
    </div>
  );
}
