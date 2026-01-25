import { useTheme } from '../../../shared/contexts/ThemeContext';
import { BlogFeature } from '../types';

interface BlogFeatureListProps {
  features: BlogFeature[];
}

export function BlogFeatureList({ features }: BlogFeatureListProps) {
  const { theme } = useTheme();

  return (
    <div className="space-y-6">
      {features.map((feature) => (
        <div key={feature.number} className="flex gap-4">
          <div className="flex-shrink-0 w-10 h-10 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center text-white font-bold shadow-md">
            {feature.number}
          </div>
          <div>
            <h4 className={`text-[20px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>{feature.title}</h4>
            <p className={`text-[15px] leading-relaxed transition-colors ${
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
