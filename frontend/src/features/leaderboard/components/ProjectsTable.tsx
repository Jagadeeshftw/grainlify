import { TrendingUp, TrendingDown, Minus, Award, ArrowUp, ArrowDown } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import type { ProjectData, FilterType } from '../types';
import { getAvatarGradient } from '../data/leaderboardData';

interface ProjectsTableProps {
  data: ProjectData[];
  activeFilter: FilterType;
  isLoaded: boolean;
}

const getTrendIcon = (trend: 'up' | 'down' | 'same') => {
  if (trend === 'up') return <TrendingUp className="w-4 h-4 text-green-600" aria-hidden="true" />;
  if (trend === 'down') return <TrendingDown className="w-4 h-4 text-red-600" aria-hidden="true" />;
  return <Minus className="w-4 h-4 text-[#7a6b5a]" aria-hidden="true" />;
};

function isLogoUrl(logo: string): boolean {
  return typeof logo === 'string' && (logo.startsWith('http://') || logo.startsWith('https://'));
}

const getRankChangeBadge = (currentRank: number, previousRank?: number) => {
  if (previousRank === undefined || previousRank === null) return null;
  const diff = previousRank - currentRank;
  if (diff === 0) {
    return (
      <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded-[6px] bg-[#7a6b5a]/15 text-[#7a6b5a] text-[11px] font-semibold" aria-label="Rank unchanged">
        <Minus className="w-3 h-3" aria-hidden="true" />
      </span>
    );
  }
  if (diff > 0) {
    return (
      <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded-[6px] bg-green-600/15 text-green-700 text-[11px] font-semibold animate-delta-up" aria-label={`Moved up ${diff} positions`}>
        <ArrowUp className="w-3 h-3" aria-hidden="true" />
        {diff}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded-[6px] bg-red-600/15 text-red-700 text-[11px] font-semibold animate-delta-down" aria-label={`Moved down ${Math.abs(diff)} positions`}>
      <ArrowDown className="w-3 h-3" aria-hidden="true" />
      {Math.abs(diff)}
    </span>
  );
};

export function ProjectsTable({ data, activeFilter, isLoaded }: ProjectsTableProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div
      className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden transition-all duration-700 delay-1000 ${
        isLoaded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'
      }`}
      role="region"
      aria-label="Project rankings"
    >
      <div className="grid grid-cols-12 gap-4 px-8 py-4 border-b border-white/10 backdrop-blur-[30px] bg-white/[0.08]">
        <div className={`col-span-1 text-[12px] font-bold uppercase tracking-wider transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>Rank</div>
        <div className={`col-span-1 text-[12px] font-bold uppercase tracking-wider transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>Trend</div>
        <div className={`col-span-5 text-[12px] font-bold uppercase tracking-wider transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>Project</div>
        <div className={`col-span-2 text-[12px] font-bold uppercase tracking-wider text-right flex items-center justify-end gap-1 transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
          Score
          <Award className="w-3.5 h-3.5 animate-wiggle-slow" aria-hidden="true" />
        </div>
        <div className="col-span-3"></div>
      </div>

      <div className="divide-y divide-white/10" role="list" aria-label="Project rankings">
        {data.map((project, index) => {
          const rowAnimation = isLoaded ? `slideInLeft 0.5s ease-out ${1.1 + index * 0.1}s both` : 'none';

          return (
            <div
              key={`${project.rank}-${project.name}`}
              role="listitem"
              tabIndex={0}
              className="grid grid-cols-12 gap-4 px-8 py-5 hover:bg-white/[0.08] transition-all duration-300 cursor-pointer group outline-2 outline-offset-2 outline-transparent focus-visible:outline-[#c9983a] focus-visible:outline"
              style={{ animation: rowAnimation }}
              aria-label={`Rank ${project.rank}: ${project.name}, score ${project.score}`}
            >
              <div className="col-span-1 flex items-center gap-1.5">
                <div className="flex items-center justify-center w-8 h-8 rounded-[10px] bg-gradient-to-br from-white/[0.15] to-white/[0.08] border border-white/20 shadow-sm group-hover:scale-110 group-hover:shadow-md transition-all duration-300">
                  <span className={`text-[15px] font-bold transition-colors ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>{project.rank}</span>
                </div>
                {getRankChangeBadge(project.rank, project.previousRank)}
              </div>

              <div className="col-span-1 flex items-center">
                <div className="flex items-center justify-center w-8 h-8 rounded-[10px] bg-gradient-to-br from-white/[0.15] to-white/[0.08] border border-white/20 shadow-sm group-hover:scale-110 transition-all duration-300">
                  {getTrendIcon(project.trend)}
                </div>
              </div>

              <div className="col-span-5 flex items-center gap-3">
                <div className={`relative w-12 h-12 rounded-full bg-gradient-to-br ${getAvatarGradient(index)} flex items-center justify-center text-white font-bold text-[18px] shadow-md border-2 border-white/25 overflow-hidden shrink-0 group-hover:scale-125 group-hover:shadow-lg group-hover:rotate-12 transition-all duration-300`} aria-hidden="true">
                  {isLogoUrl(project.logo) ? (
                    <img src={project.logo} alt="" className="w-full h-full object-cover" />
                  ) : (
                    project.logo
                  )}
                  <div className="absolute inset-0 rounded-full border-2 border-[#c9983a]/0 group-hover:border-[#c9983a]/50 transition-all duration-300" />
                </div>
                <div className="min-w-0">
                  <div className={`text-[15px] font-bold group-hover:text-[#c9983a] transition-colors duration-300 truncate ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                    {project.name}
                  </div>
                  {activeFilter === 'contributions' && project.contributors !== undefined && (
                    <div className={`text-[12px] transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                      {project.contributors} contributor{project.contributors !== 1 ? 's' : ''}
                    </div>
                  )}
                  {project.ecosystems && project.ecosystems.length > 0 && (
                    <div className="flex gap-1.5 mt-1 flex-wrap">
                      {project.ecosystems.map((eco, idx) => (
                        <span key={idx} className="px-2 py-0.5 bg-[#c9983a]/20 border border-[#c9983a]/30 rounded-[6px] text-[10px] font-semibold text-[#8b6f3a] hover:bg-[#c9983a]/30 transition-colors">
                          {eco}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>

              <div className="col-span-2 flex items-center justify-end">
                <div className="relative px-5 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a]/25 to-[#d4af37]/15 border border-[#c9983a]/40 shadow-sm group-hover:shadow-lg group-hover:border-[#c9983a]/70 group-hover:from-[#c9983a]/35 group-hover:to-[#d4af37]/25 group-hover:scale-110 transition-all duration-300">
                  <div className={`text-[17px] font-black transition-colors ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>{project.score}</div>
                </div>
              </div>

              <div className="col-span-3 flex items-center justify-end opacity-0 group-hover:opacity-100 transition-all duration-300 gap-2">
                {project.activity && (
                  <div className={`px-3 py-1.5 rounded-[8px] text-[11px] font-semibold ${
                    project.activity === 'Very High' ? 'bg-green-500/20 text-green-700 border border-green-500/30' :
                    project.activity === 'High' ? 'bg-blue-500/20 text-blue-700 border border-blue-500/30' :
                    project.activity === 'Medium' ? 'bg-yellow-500/20 text-yellow-700 border border-yellow-500/30' :
                    'bg-gray-500/20 text-gray-700 border border-gray-500/30'
                  }`}>
                    {project.activity}
                  </div>
                )}
                <button className="px-4 py-2 rounded-[10px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white text-[12px] font-semibold shadow-md hover:shadow-lg hover:scale-105 transition-all duration-300 border border-white/10 focus-visible:outline-2 focus-visible:outline-white">
                  View Project
                </button>
              </div>
            </div>
          );
        })}
      </div>

      {data.length === 0 && (
        <div className={`text-center py-12 transition-colors ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
          <p className="text-[16px] font-semibold">No projects found</p>
          <p className="text-[13px] mt-1">Try adjusting your filters or check back later.</p>
        </div>
      )}
    </div>
  );
}
