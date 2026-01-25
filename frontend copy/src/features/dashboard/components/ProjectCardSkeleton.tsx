import { useTheme } from '../../../shared/contexts/ThemeContext';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';

export function ProjectCardSkeleton() {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div
      className={`backdrop-blur-[30px] rounded-[18px] border p-5 ${
        isDark
          ? 'bg-white/[0.08] border-white/15'
          : 'bg-white/[0.15] border-white/25'
      }`}
    >
      {/* Icon */}
      <div className="flex items-start justify-between mb-4">
        <SkeletonLoader variant="default" className="w-11 h-11 rounded-[12px]" />
      </div>

      {/* Title */}
      <SkeletonLoader className="h-5 w-3/4 mb-2" />
      
      {/* Description */}
      <SkeletonLoader className="h-3 w-full mb-1" />
      <SkeletonLoader className="h-3 w-5/6 mb-4" />

      {/* Stars and Forks */}
      <div className="flex items-center space-x-3 mb-4">
        <SkeletonLoader className="h-4 w-16" />
        <SkeletonLoader className="h-4 w-16" />
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-3 gap-2 mb-4 pb-4 border-b border-white/10">
        <div className="text-center">
          <SkeletonLoader className="h-6 w-8 mx-auto mb-1" />
          <SkeletonLoader className="h-3 w-16 mx-auto" />
        </div>
        <div className="text-center">
          <SkeletonLoader className="h-6 w-8 mx-auto mb-1" />
          <SkeletonLoader className="h-3 w-12 mx-auto" />
        </div>
        <div className="text-center">
          <SkeletonLoader className="h-6 w-8 mx-auto mb-1" />
          <SkeletonLoader className="h-3 w-10 mx-auto" />
        </div>
      </div>

      {/* Tags */}
      <div className="flex flex-wrap gap-1.5">
        <SkeletonLoader className="h-6 w-20 rounded-[8px]" />
        <SkeletonLoader className="h-6 w-24 rounded-[8px]" />
        <SkeletonLoader className="h-6 w-16 rounded-[8px]" />
      </div>
    </div>
  );
}














