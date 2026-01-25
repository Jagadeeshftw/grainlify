import { SkeletonLoader } from './SkeletonLoader';

export function ActivityItemSkeleton() {
  return (
    <div className="backdrop-blur-[25px] rounded-[14px] border p-4 bg-white/[0.08] border-white/10">
      <div className="flex items-start justify-between gap-4">
        {/* Left: Icon + Badge + Title */}
        <div className="flex items-start gap-3 flex-1 min-w-0">
          {/* Icon Skeleton */}
          <SkeletonLoader variant="circle" width="32px" height="32px" />
          
          {/* Badge Skeleton */}
          <SkeletonLoader variant="text" width="50px" height="24px" />
          
          {/* Title and Label */}
          <div className="flex-1 min-w-0 pt-0.5 space-y-2">
            {/* Title Skeleton */}
            <SkeletonLoader variant="text" width="80%" height="16px" />
            
            {/* Label and Time Skeleton */}
            <div className="flex items-center gap-3">
              <SkeletonLoader variant="text" width="60px" height="20px" />
              <SkeletonLoader variant="text" width="80px" height="14px" />
            </div>
          </div>
        </div>

        {/* Right: Button Skeleton */}
        <SkeletonLoader variant="default" width="80px" height="36px" />
      </div>
    </div>
  );
}















