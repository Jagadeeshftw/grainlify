import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function ContributorsPodiumSkeleton() {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div className="flex items-end justify-center gap-4 mt-8">
      {/* Second Place */}
      <div className="flex flex-col items-center">
        <SkeletonLoader variant="circle" className="w-16 h-16 mb-3" />
        <SkeletonLoader className="h-32 w-24 rounded-t-[20px] mb-2" />
        <SkeletonLoader className="h-4 w-20 mb-1" />
        <SkeletonLoader className="h-3 w-16" />
      </div>

      {/* First Place */}
      <div className="flex flex-col items-center">
        <SkeletonLoader variant="circle" className="w-20 h-20 mb-4" />
        <SkeletonLoader className="h-40 w-28 rounded-t-[20px] mb-2" />
        <SkeletonLoader className="h-4 w-24 mb-1" />
        <SkeletonLoader className="h-3 w-20" />
      </div>

      {/* Third Place */}
      <div className="flex flex-col items-center">
        <SkeletonLoader variant="circle" className="w-16 h-16 mb-3" />
        <SkeletonLoader className="h-28 w-24 rounded-t-[20px] mb-2" />
        <SkeletonLoader className="h-4 w-20 mb-1" />
        <SkeletonLoader className="h-3 w-16" />
      </div>
    </div>
  );
}

