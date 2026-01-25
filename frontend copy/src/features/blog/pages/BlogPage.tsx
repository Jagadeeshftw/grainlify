import { featuredPost, recentPosts } from '../data/blogPosts';
import { BlogHero } from '../components/BlogHero';
import { FeaturedPost } from '../components/FeaturedPost';
import { BlogArticle } from '../components/BlogArticle';
import { RecentPostsGrid } from '../components/RecentPostsGrid';
import { BlogStyles } from '../components/BlogStyles';

export function BlogPage() {
  return (
    <div className="space-y-8">
      {/* Header Hero Section */}
      <BlogHero />

      {/* Featured Article */}
      <FeaturedPost post={featuredPost} />

      {/* Main Content Article - About OnlyGrain */}
      <BlogArticle />

      {/* Recent Posts Grid */}
      <RecentPostsGrid posts={recentPosts} />

      {/* CSS Animations */}
      <BlogStyles />
    </div>
  );
}
