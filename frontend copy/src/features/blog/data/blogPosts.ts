import { BlogPost } from '../types';

export const featuredPost: BlogPost = {
  id: 0,
  title: "Bridging the Gap: How OnlyGrain is Revolutionizing Open Source Contribution",
  excerpt: "Discover how OnlyGrain connects talented developers with groundbreaking Web3 projects across all blockchain ecosystems, creating a seamless bridge between innovation and execution.",
  date: "December 27, 2024",
  readTime: "8 min read",
  author: "OnlyGrain Team",
  image: "🌐",
  isFeatured: true,
};

export const recentPosts: BlogPost[] = [
  {
    id: 1,
    title: "The Future of Decentralized Development",
    excerpt: "Exploring how blockchain technology is transforming the way developers collaborate on open-source projects.",
    date: "December 20, 2024",
    readTime: "6 min read",
    category: "Innovation",
    icon: "🚀"
  },
  {
    id: 2,
    title: "Cross-Chain Collaboration: Breaking Down Barriers",
    excerpt: "How OnlyGrain enables developers to contribute to projects across multiple blockchain ecosystems seamlessly.",
    date: "December 15, 2024",
    readTime: "7 min read",
    category: "Technology",
    icon: "🔗"
  },
  {
    id: 3,
    title: "Incentivizing Quality Contributions",
    excerpt: "Learn about our reward system that recognizes and compensates exceptional open-source contributions.",
    date: "December 10, 2024",
    readTime: "5 min read",
    category: "Community",
    icon: "💎"
  }
];

// Easy to add more blog posts here in the future
export const allBlogPosts: BlogPost[] = [
  featuredPost,
  ...recentPosts,
];
