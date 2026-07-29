import { Medal, Trophy, Crown, Sparkles } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import type { ProjectData } from '../types';

interface ProjectsPodiumProps {
  topThree: ProjectData[];
  isLoaded: boolean;
}

function isLogoUrl(logo: string): boolean {
  return typeof logo === 'string' && (logo.startsWith('http://') || logo.startsWith('https://'));
}

const rankLabel = (rank: number): string => {
  if (rank === 1) return '1st place';
  if (rank === 2) return '2nd place';
  if (rank === 3) return '3rd place';
  return `${rank}th place`;
};

const rankElevation = (rank: number): string => {
  if (rank === 1) return '-mt-8';
  return '';
};

const rankCardWidth = (rank: number): string => {
  if (rank === 1) return 'w-[170px]';
  return 'w-[150px]';
};

const rankCardPadding = (rank: number): string => {
  if (rank === 1) return 'p-7';
  return 'p-6';
};

const rankBorder = (rank: number): string => {
  if (rank === 1) return 'border-2 border-[#c9983a]/60';
  return 'border-2 border-white/40';
};

const rankBackground = (rank: number, isDark: boolean): string => {
  if (rank === 1) return 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20';
  return isDark
    ? 'bg-gradient-to-br from-white/[0.25] to-white/[0.15]'
    : 'bg-gradient-to-br from-white/[0.5] to-white/[0.3]';
};

const rankShadow = (rank: number): string => {
  if (rank === 1) return 'shadow-[0_8px_32px_rgba(201,152,58,0.35)] hover:shadow-[0_12px_40px_rgba(201,152,58,0.5)]';
  return 'shadow-[0_6px_24px_rgba(0,0,0,0.1)] hover:shadow-[0_8px_28px_rgba(0,0,0,0.15)]';
};

const rankBadgeBg = (rank: number): string => {
  if (rank === 1) return 'bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-2 border-[#c9983a]/70';
  return 'bg-white/[0.2] border border-white/30';
};

const rankBadgeClass = (rank: number): string => {
  if (rank === 1) return 'px-4 py-2 rounded-[12px] shadow-md animate-slide-up';
  if (rank === 2) return 'px-3 py-1.5 rounded-[10px] shadow-sm animate-slide-up-delayed';
  return 'px-3 py-1.5 rounded-[10px] shadow-sm animate-slide-up-more-delayed';
};

const avatarSize = (rank: number): string => {
  if (rank === 1) return 'w-20 h-20';
  return 'w-16 h-16';
};

const avatarGradient = (rank: number): string => {
  if (rank === 1) return 'from-[#c9983a] to-[#a67c2e]';
  if (rank === 2) return 'from-[#c9983a]/80 to-[#a67c2e]/70';
  return 'from-[#b89968]/80 to-[#9a7d4f]/70';
};

const medalColor = (rank: number): string => {
  if (rank === 1) return 'text-[#c9983a]';
  if (rank === 2) return 'text-[#a89780]';
  return 'text-[#b89968]';
};

export function ProjectsPodium({ topThree, isLoaded }: ProjectsPodiumProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const placementOrder = [2, 1, 3];
  const rankMap: Record<number, ProjectData> = {
    1: topThree[0],
    2: topThree[1],
    3: topThree[2],
  };

  return (
    <div
      className="flex items-end justify-center gap-4 mt-8"
      role="group"
      aria-label="Top projects podium"
    >
      {placementOrder.map((rank) => {
        const data = rankMap[rank];
        if (!data) return null;
        const delay = rank === 1 ? 'delay-600' : rank === 2 ? 'delay-700' : 'delay-800';

        return (
          <div
            key={rank}
            className={`flex flex-col items-center transition-all duration-700 ${delay} ${
              isLoaded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-12'
            } ${rankElevation(rank)}`}
            role="article"
            aria-label={rankLabel(rank)}
          >
            <div
              className={`relative backdrop-blur-[30px] ${rankBackground(rank, isDark)} ${rankBorder(rank)} rounded-[18px] ${rankCardPadding(rank)} ${rankCardWidth(rank)} ${rankShadow(rank)} hover:scale-105 transition-all duration-300 group`}
            >
              {rank === 1 && (
                <>
                  <div className="absolute inset-0 bg-gradient-to-br from-[#c9983a]/10 to-transparent rounded-[18px] animate-pulse-glow" />
                  <div className="absolute inset-0 overflow-hidden rounded-[18px]">
                    {[...Array(8)].map((_, i) => (
                      <div
                        key={i}
                        className="absolute top-1/2 left-1/2 w-1 h-[120%] bg-gradient-to-t from-transparent via-[#c9983a]/20 to-transparent animate-ray-rotate"
                        style={{
                          transform: `translate(-50%, -50%) rotate(${i * 45}deg)`,
                          animationDelay: `${i * 0.2}s`,
                        }}
                      />
                    ))}
                  </div>
                  <div className="absolute inset-0 overflow-hidden rounded-[18px]">
                    {[...Array(12)].map((_, i) => (
                      <div
                        key={i}
                        className="absolute w-1.5 h-1.5 bg-[#c9983a] rounded-full animate-particle-float"
                        style={{
                          left: `${20 + (i * 7)}%`,
                          bottom: '10%',
                          animationDelay: `${i * 0.3}s`,
                          animationDuration: `${3 + (i % 3)}s`,
                        }}
                      />
                    ))}
                  </div>
                  <div className="absolute -inset-3 border-2 border-[#c9983a]/20 rounded-[22px] animate-ping-gentle" />
                </>
              )}

              <div className="relative">
                <div className="relative">
                  <div
                    className={`${avatarSize(rank)} rounded-full bg-gradient-to-br ${avatarGradient(rank)} flex items-center justify-center mx-auto mb-3 border-2 border-white/30 shadow-lg text-2xl group-hover:rotate-12 transition-transform duration-300 overflow-hidden`}
                  >
                    {isLogoUrl(data.logo) ? (
                      <img src={data.logo} alt="" className="w-full h-full object-cover" />
                    ) : (
                      data.logo
                    )}
                  </div>
                  {rank === 1 && (
                    <Crown
                      className="absolute -top-6 left-1/2 -translate-x-1/2 w-6 h-6 text-[#d4af37] animate-float"
                      aria-hidden="true"
                    />
                  )}
                  {rank !== 1 && (
                    <Sparkles
                      className="absolute -top-1 -right-1 w-4 h-4 text-[#c9983a] opacity-0 group-hover:opacity-100 transition-opacity"
                      aria-hidden="true"
                    />
                  )}
                </div>

                <div className="text-center">
                  <div className={`text-[${rank === 1 ? '14' : '13'}px] font-bold mb-1 truncate max-w-[140px] transition-colors ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                    {data.name}
                  </div>
                  <div className={`text-[${rank === 1 ? '26' : '20'}px] font-black text-[#c9983a] ${rank === 1 ? 'animate-number-glow' : ''}`} aria-label={`Score: ${data.score}`}>
                    {data.score}
                  </div>
                  <div className={`text-[11px] transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>pts</div>
                </div>
              </div>
            </div>

            <div className={`flex items-center justify-center gap-1.5 backdrop-blur-[20px] ${rankBadgeBg(rank)} ${rankBadgeClass(rank)}`}>
              {rank === 1 ? (
                <Trophy className="w-6 h-6 text-[#c9983a] animate-bounce-gentle" aria-hidden="true" />
              ) : (
                <Medal className={`w-5 h-5 ${medalColor(rank)}`} aria-hidden="true" />
              )}
              <span className={`text-[${rank === 1 ? '18' : '16'}px] font-bold transition-colors ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                #{rank}
              </span>
              <span className="sr-only">{rankLabel(rank)}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
