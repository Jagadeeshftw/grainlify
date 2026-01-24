import { TrendingUp, TrendingDown, Minus, Award } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { ProjectData, FilterType } from "../types";
import { getAvatarGradient } from "../data/leaderboardData";

interface ProjectsTableProps {
  data: ProjectData[];
  activeFilter: FilterType;
  isLoaded: boolean;
}

const getTrendIcon = (trend: "up" | "down" | "same") => {
  if (trend === "up") return <TrendingUp className="w-4 h-4 text-green-600" />;
  if (trend === "down")
    return <TrendingDown className="w-4 h-4 text-red-600" />;
  return <Minus className="w-4 h-4 text-[#7a6b5a]" />;
};

export function ProjectsTable({
  data,
  activeFilter,
  isLoaded,
}: ProjectsTableProps) {
  const { theme } = useTheme();
  // Unifying layout to table for all devices as requested
  // Unifying layout to table for all devices as requested
  return (
    <div
      className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden transition-all duration-700 delay-1000 ${
        isLoaded ? "opacity-100 translate-y-0" : "opacity-0 translate-y-8"
      }`}
    >
      {/* Table Header - Desktop Only */}
      <div className="hidden md:grid grid-cols-12 gap-4 px-8 py-4 border-b border-white/10 backdrop-blur-[30px] bg-white/[0.08]">
        <div
          className={`col-span-1 text-[12px] font-bold uppercase tracking-wider transition-colors ${
            theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
          }`}
        >
          Rank
        </div>
        <div
          className={`col-span-1 text-[12px] font-bold uppercase tracking-wider transition-colors ${
            theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
          }`}
        >
          Trend
        </div>
        <div
          className={`col-span-5 text-[12px] font-bold uppercase tracking-wider transition-colors ${
            theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
          }`}
        >
          Project
        </div>
        <div
          className={`col-span-2 text-[12px] font-bold uppercase tracking-wider text-right flex items-center justify-end gap-1 transition-colors ${
            theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
          }`}
        >
          Score
          <Award className="w-3.5 h-3.5 animate-wiggle-slow" />
        </div>
        <div className="col-span-3"></div>
      </div>

      {/* Table Rows */}
      <div className="divide-y divide-white/10">
        {data.map((project, index) => (
          <div
            key={project.rank}
            className="flex flex-wrap md:grid md:grid-cols-12 gap-y-3 gap-x-4 px-4 py-4 md:px-8 md:py-5 hover:bg-white/[0.08] transition-all duration-300 cursor-pointer group items-center"
            style={{
              animation: isLoaded
                ? `slideInLeft 0.5s ease-out ${1.1 + index * 0.1}s both`
                : "none",
            }}
          >
            {/* Rank & Trend & Logo & Name - Mobile Grouping */}
            <div className="flex items-center gap-3 flex-1 md:contents">
              {/* Rank */}
              <div className="flex items-center justify-center w-8 h-8 rounded-[10px] bg-gradient-to-br from-white/[0.15] to-white/[0.08] border border-white/20 shadow-sm flex-shrink-0">
                <span
                  className={`text-[14px] md:text-[15px] font-bold transition-colors ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.rank}
                </span>
              </div>

              {/* Trend */}
              <div className="flex items-center justify-center w-8 h-8 rounded-[10px] bg-gradient-to-br from-white/[0.15] to-white/[0.08] border border-white/20 shadow-sm flex-shrink-0">
                {getTrendIcon(project.trend)}
              </div>

              {/* Project Info */}
              <div className="flex items-center gap-3 flex-1 md:col-span-5 md:flex-initial">
                <div
                  className={`relative w-10 h-10 md:w-12 md:h-12 rounded-full bg-gradient-to-br ${getAvatarGradient(index)} flex items-center justify-center text-white font-bold text-[16px] md:text-[18px] shadow-md border-2 border-white/25 group-hover:scale-125 group-hover:shadow-lg group-hover:rotate-12 transition-all duration-300 flex-shrink-0`}
                >
                  {project.logo}
                  {/* Glow ring on hover */}
                  <div className="absolute inset-0 rounded-full border-2 border-[#c9983a]/0 group-hover:border-[#c9983a]/50 transition-all duration-300 animate-ping-on-hover" />
                </div>
                <div className="min-w-0 flex-1">
                  <div
                    className={`text-[14px] md:text-[15px] font-bold group-hover:text-[#c9983a] transition-colors duration-300 truncate ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {project.name}
                  </div>
                  {/* Tags / Ecosystems */}
                  <div className="flex flex-wrap gap-1.5 mt-1">
                    {activeFilter === "contributions" &&
                      project.contributors && (
                        <span
                          className={`hidden md:inline-flex text-[11px] transition-colors mr-2 items-center ${
                            theme === "dark"
                              ? "text-[#d4d4d4]"
                              : "text-[#7a6b5a]"
                          }`}
                        >
                          {project.contributors} contributors
                        </span>
                      )}

                    {project.ecosystems &&
                      project.ecosystems.map((eco, idx) => (
                        <span
                          key={idx}
                          className="px-1.5 py-0.5 md:px-2 bg-[#c9983a]/20 border border-[#c9983a]/30 rounded-[6px] text-[9px] md:text-[10px] font-semibold text-[#8b6f3a] hover:bg-[#c9983a]/30 transition-colors whitespace-nowrap"
                        >
                          {eco}
                        </span>
                      ))}
                  </div>
                </div>
              </div>
            </div>

            {/* Score - Mobile adjustment */}
            <div className="flex items-center justify-end md:col-span-2">
              <div className="relative px-3 py-1.5 md:px-5 md:py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a]/25 to-[#d4af37]/15 border border-[#c9983a]/40 shadow-sm group-hover:shadow-lg group-hover:border-[#c9983a]/70 group-hover:from-[#c9983a]/35 group-hover:to-[#d4af37]/25 group-hover:scale-110 transition-all duration-300">
                <div
                  className={`text-[14px] md:text-[17px] font-black transition-colors ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.score}
                </div>
              </div>
            </div>

            {/* Action & Activity - Mobile adjustment */}
            <div className="flex items-center justify-end gap-2 w-full md:w-auto md:col-span-3">
              {activeFilter === "contributions" && project.contributors && (
                <span
                  className={`md:hidden text-[11px] transition-colors mr-auto ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  {project.contributors} contribs
                </span>
              )}

              {project.activity && (
                <div
                  className={`px-2.5 py-1.5 rounded-[8px] text-[10px] font-semibold whitespace-nowrap hidden lg:block ${
                    project.activity === "Very High"
                      ? "bg-green-500/20 text-green-700 border border-green-500/30"
                      : project.activity === "High"
                        ? "bg-blue-500/20 text-blue-700 border border-blue-500/30"
                        : project.activity === "Medium"
                          ? "bg-yellow-500/20 text-yellow-700 border border-yellow-500/30"
                          : "bg-gray-500/20 text-gray-700 border border-gray-500/30"
                  }`}
                >
                  {project.activity}
                </div>
              )}
              <button className="px-3 py-1.5 md:px-4 md:py-2 rounded-[10px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white text-[11px] md:text-[12px] font-semibold shadow-md hover:shadow-lg hover:scale-105 transition-all duration-300 border border-white/10 whitespace-nowrap ml-auto md:ml-0">
                View Project
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
