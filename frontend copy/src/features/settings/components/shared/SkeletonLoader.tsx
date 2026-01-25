interface SkeletonLoaderProps {
  className?: string;
}

export function SkeletonLoader({ className }: SkeletonLoaderProps) {
  return (
    <div className={`relative overflow-hidden bg-white/[0.08] rounded-[12px] ${className}`}>
      <div className="absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r from-transparent via-white/[0.15] to-transparent" />
    </div>
  );
}
