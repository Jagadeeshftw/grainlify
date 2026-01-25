import { SkeletonLoader } from './SkeletonLoader';

export function ChartSkeleton() {
  return (
    <div className="space-y-6">
      {/* Chart Title Skeleton */}
      <div className="space-y-2">
        <SkeletonLoader variant="text" width="200px" height="24px" />
        <SkeletonLoader variant="text" width="150px" height="14px" />
      </div>

      {/* Chart Area Skeleton */}
      <div className="space-y-4">
        {/* Y-axis labels skeleton */}
        <div className="flex items-end gap-4 h-[200px]">
          {[...Array(6)].map((_, idx) => (
            <div key={idx} className="flex-1 flex flex-col items-center gap-2">
              {/* Bar skeleton */}
              <SkeletonLoader 
                variant="default" 
                width="100%" 
                height={`${Math.random() * 60 + 40}%`}
                className="rounded-t-[8px]"
              />
              {/* X-axis label skeleton */}
              <SkeletonLoader variant="text" width="40px" height="12px" />
            </div>
          ))}
        </div>

        {/* Legend Skeleton */}
        <div className="flex items-center gap-4 justify-center pt-4">
          <div className="flex items-center gap-2">
            <SkeletonLoader variant="circle" width="12px" height="12px" />
            <SkeletonLoader variant="text" width="80px" height="12px" />
          </div>
          <div className="flex items-center gap-2">
            <SkeletonLoader variant="circle" width="12px" height="12px" />
            <SkeletonLoader variant="text" width="60px" height="12px" />
          </div>
        </div>
      </div>
    </div>
  );
}















