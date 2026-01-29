import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogFeature } from '../types';

interface BlogFeatureListProps {
  features: BlogFeature[];
}

export function BlogFeatureList({ features }: BlogFeatureListProps) {
  const { theme } = useTheme();

  return (
    <div className="space-y-4 md:space-y-5 lg:space-y-6">
      {features.map((feature) => (
        <div key={feature.number} className="flex gap-3 md:gap-4">
          <div className="flex-shrink-0 w-8 h-8 md:w-10 md:h-10 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center text-white font-bold shadow-md text-xs md:text-base">
            {feature.number}
          </div>
          <div className="min-w-0">
            <h4 className={`text-base md:text-lg lg:text-[20px] font-bold mb-1 md:mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>{feature.title}</h4>
            <p className={`text-xs md:text-sm lg:text-[15px] leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
            }`}>
              {feature.description}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}
