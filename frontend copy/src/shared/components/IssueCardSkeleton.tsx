import { SkeletonLoader } from './SkeletonLoader';

export function IssueCardSkeleton() {
  return (
    <div className="backdrop-blur-[25px] rounded-[16px] border p-4 bg-white/[0.08] border-white/10">
      {/* Issue Header */}
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2">
          {/* Circular Icon Skeleton */}
          <SkeletonLoader variant="circle" width="32px" height="32px" />
          {/* Number Badge Skeleton */}
          <SkeletonLoader variant="default" width="40px" height="24px" />
        </div>
      </div>

      {/* Issue Title Skeleton */}
      <div className="mb-3 space-y-2">
        <SkeletonLoader variant="text" width="90%" height="16px" />
        <SkeletonLoader variant="text" width="70%" height="16px" />
      </div>

      {/* Repository and Applicants Skeleton */}
      <div className="flex items-center gap-3 mb-3">
        <SkeletonLoader variant="text" width="100px" height="12px" />
        <SkeletonLoader variant="text" width="80px" height="12px" />
      </div>

      {/* Author and Time Skeleton */}
      <div className="flex items-center gap-2 pt-3 border-t border-white/10">
        {/* Avatar Skeleton */}
        <SkeletonLoader variant="circle" width="24px" height="24px" />
        {/* Name Skeleton */}
        <SkeletonLoader variant="text" width="80px" height="12px" />
        {/* Time Skeleton */}
        <SkeletonLoader variant="text" width="60px" height="12px" className="ml-auto" />
      </div>
    </div>
  );
}















