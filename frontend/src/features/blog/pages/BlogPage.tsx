import { featuredPost, recentPosts } from '../data/blogPosts';
import { BlogHero } from '../components/BlogHero';
import { FeaturedPost } from '../components/FeaturedPost';
import { BlogArticle } from '../components/BlogArticle';
import { RecentPostsGrid } from '../components/RecentPostsGrid';
import { BlogStyles } from '../components/BlogStyles';

export function BlogPage() {
  return (
    <div className="space-y-4 md:space-y-6 lg:space-y-8 px-4 md:px-6 lg:px-8 py-4 md:py-6 lg:py-8 max-w-7xl mx-auto">
      {/* Header Hero Section */}
      <BlogHero />

      {/* Featured Article */}
      <div className="mt-6 md:mt-8 lg:mt-10">
        <FeaturedPost post={featuredPost} />
      </div>

      {/* Main Content Article - About OnlyGrain */}
      <div className="mt-6 md:mt-8 lg:mt-10">
        <BlogArticle />
      </div>

      {/* Recent Posts Grid */}
      <div className="mt-6 md:mt-8 lg:mt-10">
        <RecentPostsGrid posts={recentPosts} />
      </div>

      {/* CSS Animations */}
      <BlogStyles />
    </div>
  );
}
