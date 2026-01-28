import React from 'react';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function ProjectsPodiumSkeleton() {
  const { theme } = useTheme();

  return (
    <div className="flex items-end justify-center gap-4 mt-8">
      {/* Second Place */}
      <div className="flex flex-col items-center">
        <SkeletonLoader variant="circle" className="w-16 h-16 mb-3" />
        <SkeletonLoader className="h-32 w-[150px] rounded-[18px] mb-3" />
        <SkeletonLoader className="h-8 w-12 rounded-[10px]" />
      </div>

      {/* First Place */}
      <div className="flex flex-col items-center -mt-8">
        <SkeletonLoader variant="circle" className="w-20 h-20 mb-4" />
        <SkeletonLoader className="h-40 w-[170px] rounded-[20px] mb-3" />
        <SkeletonLoader className="h-10 w-16 rounded-[12px]" />
      </div>

      {/* Third Place */}
      <div className="flex flex-col items-center">
        <SkeletonLoader variant="circle" className="w-16 h-16 mb-3" />
        <SkeletonLoader className="h-32 w-[150px] rounded-[18px] mb-3" />
        <SkeletonLoader className="h-8 w-12 rounded-[10px]" />
      </div>
    </div>
  );
}
