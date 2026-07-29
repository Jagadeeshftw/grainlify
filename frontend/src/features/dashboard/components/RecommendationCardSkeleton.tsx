import { useTheme } from "../../../shared/contexts/ThemeContext";
import { SkeletonLoader } from "../../../shared/components/SkeletonLoader";

export function RecommendationCardSkeleton() {
  const { theme } = useTheme();
  const isDark = theme === "dark";

  return (
    <div
      className={`rounded-[22px] border p-5 ${
        isDark
          ? "border-white/10 bg-white/[0.08]"
          : "border-white/20 bg-white/[0.16]"
      }`}
    >
      <div className="flex items-center gap-3 mb-4">
        <SkeletonLoader variant="default" className="h-11 w-11 rounded-[14px]" />
        <div className="flex-1">
          <SkeletonLoader className="h-3 w-20 mb-2" />
          <SkeletonLoader className="h-5 w-32" />
        </div>
      </div>

      <SkeletonLoader className="h-8 w-full mb-3" />
      <SkeletonLoader className="h-3 w-full mb-1" />
      <SkeletonLoader className="h-3 w-5/6 mb-4" />

      <div className="flex flex-wrap gap-2">
        <SkeletonLoader className="h-7 w-24 rounded-full" />
        <SkeletonLoader className="h-7 w-20 rounded-full" />
        <SkeletonLoader className="h-7 w-16 rounded-full" />
      </div>
    </div>
  );
}
