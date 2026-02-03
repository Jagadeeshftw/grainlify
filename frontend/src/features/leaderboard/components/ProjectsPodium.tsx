import { Medal, Trophy, Crown, Sparkles } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { ProjectData } from '../types';

function isLogoUrl(logo: string): boolean {
  return typeof logo === 'string' && (logo.startsWith('http://') || logo.startsWith('https://'));
}

interface ProjectsPodiumProps {
  topThree: ProjectData[];
  isLoaded: boolean;
}

export function ProjectsPodium({ topThree, isLoaded }: ProjectsPodiumProps) {
  const { theme } = useTheme();

  return (
    <div className="flex items-end justify-center gap-4 mt-8">
      {/* 2nd Place */}
      <div className={`flex flex-col items-center transition-all duration-700 delay-700 ${
        isLoaded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-12'
      }`}>
        <div className="backdrop-blur-[30px] bg-gradient-to-br from-white/[0.25] to-white/[0.15] border-2 border-white/40 rounded-[18px] p-6 w-[150px] shadow-[0_6px_24px_rgba(0,0,0,0.1)] mb-3 hover:shadow-[0_8px_28px_rgba(0,0,0,0.15)] hover:scale-105 transition-all duration-300 group">
          <div className="relative">
            <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#c9983a]/80 to-[#a67c2e]/70 flex items-center justify-center mx-auto mb-3 border-2 border-white/30 shadow-lg text-2xl overflow-hidden group-hover:rotate-12 transition-transform duration-300">
              {isLogoUrl(topThree[1].logo) ? (
                <img src={topThree[1].logo} alt="" className="w-full h-full object-cover" />
              ) : (
                topThree[1].logo
              )}
            </div>
            <Sparkles className="absolute -top-1 -right-1 w-4 h-4 text-[#c9983a] opacity-0 group-hover:opacity-100 transition-opacity" />
          </div>
          <div className="text-center">
            <div className={`text-[13px] font-bold mb-1 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>{topThree[1].name}</div>
            <div className="text-[20px] font-black text-[#c9983a]">{topThree[1].score}</div>
            <div className={`text-[11px] transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>pts</div>
          </div>
        </div>
        <div className="flex items-center justify-center gap-1.5 backdrop-blur-[20px] bg-white/[0.2] border border-white/30 rounded-[10px] px-3 py-1.5 shadow-sm animate-slide-up-delayed">
          <Medal className="w-5 h-5 text-[#a89780]" />
          <span className={`text-[16px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>#2</span>
        </div>
      </div>

      {/* 1st Place */}
      <div className={`flex flex-col items-center -mt-8 transition-all duration-700 delay-600 ${
        isLoaded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-12'
      }`}>
        <div className="relative backdrop-blur-[30px] bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/60 rounded-[20px] p-7 w-[170px] shadow-[0_8px_32px_rgba(201,152,58,0.35)] mb-3 hover:shadow-[0_12px_40px_rgba(201,152,58,0.5)] hover:scale-110 transition-all duration-300 group">
          <div className="absolute inset-0 bg-gradient-to-br from-[#c9983a]/10 to-transparent rounded-[20px] animate-pulse-glow" />
          
          <div className="absolute inset-0 overflow-hidden rounded-[20px]">
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
          
          <div className="absolute inset-0 overflow-hidden rounded-[20px]">
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
          
          <div className="absolute -inset-3 border-2 border-[#c9983a]/20 rounded-[24px] animate-ping-gentle" />
          
          <div className="relative">
            <div className="relative w-20 h-20 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center mx-auto mb-3 border-2 border-[#d4af37] shadow-xl text-3xl overflow-hidden group-hover:rotate-[360deg] transition-transform duration-700">
              {isLogoUrl(topThree[0].logo) ? (
                <img src={topThree[0].logo} alt="" className="w-full h-full object-cover" />
              ) : (
                topThree[0].logo
              )}
              <Crown className="absolute -top-6 left-1/2 -translate-x-1/2 w-6 h-6 text-[#d4af37] animate-float" />
            </div>
            <div className="text-center">
              <div className={`text-[14px] font-bold mb-1 transition-colors ${
                theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>{topThree[0].name}</div>
              <div className="text-[26px] font-black text-[#c9983a] animate-number-glow">{topThree[0].score}</div>
              <div className={`text-[11px] transition-colors ${
                theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>pts</div>
            </div>
          </div>
        </div>
        <div className="flex items-center justify-center gap-1.5 backdrop-blur-[20px] bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-2 border-[#c9983a]/70 rounded-[12px] px-4 py-2 shadow-md animate-slide-up">
          <Trophy className="w-6 h-6 text-[#c9983a] animate-bounce-gentle" />
          <span className={`text-[18px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>#1</span>
        </div>
      </div>

      {/* 3rd Place */}
      <div className={`flex flex-col items-center transition-all duration-700 delay-800 ${
        isLoaded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-12'
      }`}>
        <div className="backdrop-blur-[30px] bg-gradient-to-br from-white/[0.25] to-white/[0.15] border-2 border-white/40 rounded-[18px] p-6 w-[150px] shadow-[0_6px_24px_rgba(0,0,0,0.1)] mb-3 hover:shadow-[0_8px_28px_rgba(0,0,0,0.15)] hover:scale-105 transition-all duration-300 group">
          <div className="relative">
            <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#b89968]/80 to-[#9a7d4f]/70 flex items-center justify-center mx-auto mb-3 border-2 border-white/30 shadow-lg text-2xl overflow-hidden group-hover:rotate-12 transition-transform duration-300">
              {isLogoUrl(topThree[2].logo) ? (
                <img src={topThree[2].logo} alt="" className="w-full h-full object-cover" />
              ) : (
                topThree[2].logo
              )}
            </div>
            <Sparkles className="absolute -top-1 -right-1 w-4 h-4 text-[#b89968] opacity-0 group-hover:opacity-100 transition-opacity" />
          </div>
          <div className="text-center">
            <div className={`text-[13px] font-bold mb-1 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>{topThree[2].name}</div>
            <div className="text-[20px] font-black text-[#c9983a]">{topThree[2].score}</div>
            <div className={`text-[11px] transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>pts</div>
          </div>
        </div>
        <div className="flex items-center justify-center gap-1.5 backdrop-blur-[20px] bg-white/[0.2] border border-white/30 rounded-[10px] px-3 py-1.5 shadow-sm animate-slide-up-more-delayed">
          <Medal className="w-5 h-5 text-[#b89968]" />
          <span className={`text-[16px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>#3</span>
        </div>
      </div>
    </div>
  );
}