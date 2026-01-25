import { SkeletonLoader } from '../../../../shared/components/SkeletonLoader';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

export function StatsCardSkeleton() {
  const { theme } = useTheme();

  return (
    <div className={`backdrop-blur-[40px] rounded-[20px] border p-6 relative overflow-hidden transition-colors ${
      theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
    }`}>
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <SkeletonLoader className="h-4 w-32 mb-2" />
          <SkeletonLoader className="h-3 w-20" />
        </div>
        <SkeletonLoader variant="circle" className="w-10 h-10" />
      </div>
      <div className="mb-2">
        <SkeletonLoader className="h-8 w-16" />
      </div>
      <SkeletonLoader className="h-5 w-20 rounded-[6px]" />
    </div>
  );
}

