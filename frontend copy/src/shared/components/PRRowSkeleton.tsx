import { SkeletonLoader } from './SkeletonLoader';

export function PRRowSkeleton() {
  return (
    <div className="grid grid-cols-[2fr_1.5fr_1fr_0.5fr] gap-6 px-6 py-5 rounded-[16px] backdrop-blur-[25px] border bg-white/[0.08] border-white/15">
      {/* Pull Request Info */}
      <div>
        <div className="flex items-start gap-3 mb-2">
          {/* Icon Skeleton */}
          <SkeletonLoader variant="circle" width="16px" height="16px" />
          <div className="flex-1 space-y-2">
            {/* Title Skeleton */}
            <SkeletonLoader variant="text" width="85%" height="16px" />
            {/* Number and Status Skeleton */}
            <SkeletonLoader variant="text" width="120px" height="12px" />
          </div>
        </div>
      </div>

      {/* Author Info */}
      <div>
        <div className="flex items-center gap-2 mb-2">
          {/* Avatar Skeleton */}
          <SkeletonLoader variant="circle" width="28px" height="28px" />
          {/* Name Skeleton */}
          <SkeletonLoader variant="text" width="80px" height="14px" />
        </div>
        {/* Badges Skeleton */}
        <div className="flex gap-1.5">
          <SkeletonLoader variant="text" width="50px" height="20px" />
          <SkeletonLoader variant="text" width="60px" height="20px" />
        </div>
      </div>

      {/* Repository Info */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          {/* Repo Avatar Skeleton */}
          <SkeletonLoader variant="default" width="20px" height="20px" className="rounded-md" />
          {/* Repo Name Skeleton */}
          <SkeletonLoader variant="text" width="70px" height="14px" />
        </div>
        {/* Org Skeleton */}
        <SkeletonLoader variant="text" width="60px" height="11px" className="ml-7" />
      </div>

      {/* Indicators Skeleton */}
      <div className="flex items-center gap-2">
        <SkeletonLoader variant="circle" width="28px" height="28px" />
        <SkeletonLoader variant="circle" width="28px" height="28px" />
      </div>
    </div>
  );
}















