import { Calendar, Clock, Sparkles, Globe, Zap, Coins } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogStatistics } from './BlogStatistics';
import { BlogFeatureList } from './BlogFeatureList';
import { BlogHowItWorks } from './BlogHowItWorks';
import { BlogWhyChoose } from './BlogWhyChoose';
import { BlogCTA } from './BlogCTA';
import { BlogFeature } from '../types';

export function BlogArticle() {
  const { theme } = useTheme();

  const features: BlogFeature[] = [
    {
      number: 1,
      title: 'Connect Developers with Projects',
      description: 'We provide an intelligent matching system that connects developers with projects that align with their skills, interests, and experience level. Whether you\'re a seasoned blockchain architect or just starting your Web3 journey, there\'s a place for you on OnlyGrain.',
    },
    {
      number: 2,
      title: 'Multi-Chain Support',
      description: 'From Ethereum to Solana, Polkadot to Cosmos, and everything in between—OnlyGrain supports projects across all major blockchain ecosystems. We believe in a multi-chain future and our platform reflects that vision.',
    },
    {
      number: 3,
      title: 'Transparent Reward System',
      description: 'Quality contributions deserve recognition. Our platform features a comprehensive reward and recognition system that ensures developers are fairly compensated for their work. From bounties to grants, we make sure your efforts are valued.',
    },
    {
      number: 4,
      title: 'Build Your Reputation',
      description: 'Every contribution you make builds your on-chain reputation. Our leaderboard and profile system showcases your achievements, making it easier to stand out in the competitive Web3 job market.',
    },
    {
      number: 5,
      title: 'Community-Driven Development',
      description: 'At OnlyGrain, we believe in the power of community. Our platform facilitates collaboration, knowledge sharing, and networking among developers, project maintainers, and ecosystem stakeholders.',
    },
  ];

  return (
    <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[20px] md:rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-4 md:p-8 lg:p-10">
      <div className="max-w-4xl mx-auto">
        {/* Article Header */}
        <div className="text-center mb-6 md:mb-8 lg:mb-10">
          <h2 className={`text-xl md:text-3xl lg:text-[36px] font-bold mb-3 md:mb-4 transition-colors leading-tight ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>
            Welcome to OnlyGrain: The Future of Open Source Collaboration
          </h2>
          <div className={`flex flex-col md:flex-row md:items-center md:justify-center gap-2 md:gap-3 lg:gap-4 text-xs md:text-sm lg:text-[14px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>
            <span className="flex items-center gap-1 md:gap-1.5 justify-center">
              <Calendar className="w-3.5 h-3.5 md:w-4 md:h-4" />
              December 27, 2024
            </span>
            <span className="hidden md:inline">•</span>
            <span className="flex items-center gap-1 md:gap-1.5 justify-center">
              <Clock className="w-3.5 h-3.5 md:w-4 md:h-4" />
              10 min read
            </span>
          </div>
        </div>

        {/* Article Content */}
        <div className="prose prose-lg max-w-none">
          {/* Introduction */}
          <div className="mb-6 md:mb-8">
            <h3 className={`text-lg md:text-2xl lg:text-[28px] font-bold mb-3 md:mb-4 flex items-center gap-2 md:gap-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              <Sparkles className="w-5 h-5 md:w-6 md:h-6 lg:w-7 lg:h-7 text-[#c9983a] flex-shrink-0" />
              What is OnlyGrain?
            </h3>
            <p className={`text-sm md:text-base lg:text-[16px] leading-relaxed mb-3 md:mb-4 transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
            }`}>
              OnlyGrain is a revolutionary platform that bridges the gap between talented open-source developers and innovative Web3 projects across all blockchain ecosystems. We're not just another development platform—we're building the future of decentralized collaboration.
            </p>
            <p className={`text-sm md:text-base lg:text-[16px] leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
            }`}>
              In the rapidly evolving world of blockchain and Web3, finding the right talent for your project or discovering meaningful contribution opportunities can be challenging. OnlyGrain solves this by creating a unified ecosystem where developers and projects connect, collaborate, and thrive together.
            </p>
          </div>

          {/* Our Mission */}
          <div className="mb-6 md:mb-8 p-4 md:p-5 lg:p-6 backdrop-blur-[30px] bg-white/[0.15] rounded-[16px] md:rounded-[20px] border border-white/25">
            <h3 className={`text-lg md:text-2xl lg:text-[28px] font-bold mb-3 md:mb-4 flex items-center gap-2 md:gap-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              <Globe className="w-5 h-5 md:w-6 md:h-6 lg:w-7 lg:h-7 text-[#c9983a] flex-shrink-0" />
              Our Mission
            </h3>
            <p className={`text-sm md:text-base lg:text-[16px] leading-relaxed mb-3 md:mb-4 transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
            }`}>
              We believe that the best innovations happen when talented developers have access to meaningful projects and when projects can easily discover and engage with skilled contributors. Our mission is to democratize access to Web3 development opportunities while ensuring quality contributions are properly recognized and rewarded.
            </p>
            <BlogStatistics />
          </div>

          {/* What We Do */}
          <div className="mb-6 md:mb-8">
            <h3 className={`text-lg md:text-2xl lg:text-[28px] font-bold mb-3 md:mb-4 flex items-center gap-2 md:gap-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              <Zap className="w-5 h-5 md:w-6 md:h-6 lg:w-7 lg:h-7 text-[#c9983a] flex-shrink-0" />
              What We Do
            </h3>
            <BlogFeatureList features={features} />
          </div>

          {/* How It Works */}
          <BlogHowItWorks />

          {/* Why Choose OnlyGrain */}
          <div className="mb-6 md:mb-8">
            <h3 className={`text-lg md:text-2xl lg:text-[28px] font-bold mb-3 md:mb-4 flex items-center gap-2 md:gap-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              <Coins className="w-5 h-5 md:w-6 md:h-6 lg:w-7 lg:h-7 text-[#c9983a] flex-shrink-0" />
              Why Choose OnlyGrain?
            </h3>
            <BlogWhyChoose />
          </div>

          {/* Closing CTA */}
          <BlogCTA />
        </div>
      </div>
    </div>
  );
}
