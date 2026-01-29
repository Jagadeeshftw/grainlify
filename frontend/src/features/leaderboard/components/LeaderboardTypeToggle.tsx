import { Users, FolderGit2 } from "lucide-react";
import { LeaderboardType } from "../types";

interface LeaderboardTypeToggleProps {
  leaderboardType: LeaderboardType;
  onToggle: (type: LeaderboardType) => void;
  isLoaded: boolean;
}

export function LeaderboardTypeToggle({
  leaderboardType,
  onToggle,
  isLoaded,
}: LeaderboardTypeToggleProps) {
  return (
    <div
      className={`sticky top-6 z-[200] flex justify-center transition-all duration-1000 ${
        isLoaded ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-4"
      }`}
    >
      <div className="backdrop-blur-[40px] bg-gradient-to-br from-white/[0.25] to-white/[0.15] rounded-[20px] border-2 border-white/30 shadow-[0_12px_48px_rgba(0,0,0,0.15)] p-1 md:p-2 flex gap-1 md:gap-2 mx-4 md:mx-0">
        <button
          onClick={() => onToggle("contributors")}
          className={`flex items-center gap-1.5 md:gap-2 px-4 py-2.5 md:px-6 md:py-3 rounded-[14px] font-bold text-[13px] md:text-[15px] transition-all duration-300 ${
            leaderboardType === "contributors"
              ? "bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_6px_20px_rgba(201,152,58,0.4)] scale-105 border-2 border-white/20"
              : "text-[#6b5d4d] hover:bg-white/[0.15] hover:scale-105"
          }`}
        >
          <Users className="w-5 h-5" />
          Contributors
        </button>
        <button
          onClick={() => onToggle("projects")}
          className={`flex items-center gap-1.5 md:gap-2 px-4 py-2.5 md:px-6 md:py-3 rounded-[14px] font-bold text-[13px] md:text-[15px] transition-all duration-300 ${
            leaderboardType === "projects"
              ? "bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_6px_20px_rgba(201,152,58,0.4)] scale-105 border-2 border-white/20"
              : "text-[#6b5d4d] hover:bg-white/[0.15] hover:scale-105"
          }`}
        >
          <FolderGit2 className="w-5 h-5" />
          Projects
        </button>
      </div>
    </div>
  );
}
