import { Crown, Trophy, Star } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { LeaderboardType } from "../types";

interface LeaderboardHeroProps {
  leaderboardType: LeaderboardType;
  isLoaded: boolean;
  children: React.ReactNode;
}

export function LeaderboardHero({
  leaderboardType,
  isLoaded,
  children,
}: LeaderboardHeroProps) {
  const { theme } = useTheme();

  return (
    <div
      className={`relative min-h-[450px] backdrop-blur-[40px] bg-gradient-to-br from-white/[0.18] to-white/[0.12] rounded-[28px] border border-white/25 shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden transition-all duration-1000 ${
        isLoaded ? "opacity-100 translate-y-0" : "opacity-0 translate-y-8"
      }`}
    >
      {/* Golden Glow Effects with Animation */}
      <div className="absolute inset-0 opacity-20">
        <div className="absolute top-10 left-10 w-40 h-40 bg-[#c9983a]/30 rounded-full blur-[80px] animate-glow-pulse" />
        <div className="absolute top-20 right-20 w-48 h-48 bg-[#d4af37]/25 rounded-full blur-[90px] animate-glow-pulse-delayed" />
        <div className="absolute bottom-10 left-1/3 w-44 h-44 bg-[#c9983a]/28 rounded-full blur-[85px] animate-glow-pulse" />
        <div className="absolute bottom-20 right-1/4 w-36 h-36 bg-[#b89968]/25 rounded-full blur-[70px] animate-glow-pulse-delayed" />
      </div>

      {/* Animated Sparkles */}
      <div className="absolute inset-0">
        {[...Array(35)].map((_, i) => (
          <div
            key={i}
            className="absolute w-1 h-1 bg-[#c9983a]/50 rounded-full animate-twinkle-slow"
            style={{
              top: `${Math.random() * 100}%`,
              left: `${Math.random() * 100}%`,
              animationDelay: `${Math.random() * 3}s`,
            }}
          />
        ))}
      </div>

      {/* Floating Golden Rings */}
      <div className="absolute inset-0 overflow-hidden">
        <div className="absolute top-1/4 left-10 w-32 h-32 border-2 border-[#c9983a]/20 rounded-full animate-float" />
        <div className="absolute top-1/3 right-16 w-24 h-24 border-2 border-[#d4af37]/15 rounded-full animate-float-delayed" />
        <div className="absolute bottom-1/4 left-1/4 w-20 h-20 border-2 border-[#c9983a]/20 rounded-full animate-float-slow" />
      </div>

      <div className="relative z-10 p-4 md:p-10">
        {/* Title Section with Entrance Animation */}
        <div
          className={`text-center mb-6 md:mb-10 transition-all duration-1000 delay-200 ${
            isLoaded ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-4"
          }`}
        >
          <div className="relative inline-block mb-3">
            <h1
              className={`text-[28px] md:text-[44px] font-bold drop-shadow-sm relative z-10 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              {leaderboardType === "contributors"
                ? "Seasonal Contributors"
                : "Top Projects"}
            </h1>
            {/* Golden Underline Animation */}
            <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-[#c9983a] to-transparent opacity-40 animate-shimmer" />
          </div>

          <div className="relative inline-block">
            <Crown className="absolute -top-6 md:-top-8 left-1/2 -translate-x-1/2 w-8 h-8 md:w-10 md:h-10 text-[#c9983a] animate-bounce-slow drop-shadow-[0_2px_8px_rgba(201,152,58,0.4)]" />
            <h2
              className={`text-[28px] md:text-[44px] font-bold mb-4 ml-2 pt-4 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
              style={{
                textShadow:
                  "0 2px 8px rgba(201, 152, 58, 0.3), 0 0 20px rgba(201, 152, 58, 0.2)",
              }}
            >
              Leaderboard
            </h2>
          </div>

          <p
            className={`text-[13px] md:text-[14px] max-w-2xl mx-auto leading-relaxed px-4 transition-colors ${
              theme === "dark" ? "text-[#d4d4d4]" : "text-[#6b5d4d]"
            }`}
          >
            {leaderboardType === "contributors"
              ? "These aren't just contributors—they're the backbone of this project. Raw numbers, real impact. Where do you stand?"
              : "Leading the innovation frontier. These projects are setting the standard. Is yours among them?"}
          </p>
        </div>

        {/* Podium Section */}
        <div
          className={`backdrop-blur-[40px] bg-white/[0.15] rounded-[24px] border border-white/30 p-4 md:p-8 max-w-3xl mx-auto shadow-[0_8px_32px_rgba(0,0,0,0.12)] transition-all duration-1000 delay-500 ${
            isLoaded ? "opacity-100 scale-100" : "opacity-0 scale-95"
          }`}
        >
          <div className="flex items-center justify-center gap-2 mb-4 md:mb-6">
            <h3
              className={`text-[16px] md:text-[18px] font-bold transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              {leaderboardType === "contributors"
                ? "Most Our Champions"
                : "Elite Projects"}
            </h3>
            <Trophy className="w-5 h-5 text-[#c9983a] drop-shadow-sm animate-wiggle" />
            <Trophy className="w-5 h-5 text-[#c9983a] drop-shadow-sm animate-wiggle-delayed" />
            <Trophy className="w-5 h-5 text-[#c9983a] drop-shadow-sm animate-wiggle" />
          </div>

          <div className="flex items-center justify-center gap-2 mb-6 md:mb-10">
            <Star className="w-4 h-4 text-[#c9983a] animate-pulse-slow" />
            <div
              className={`text-[13px] font-semibold transition-colors ${
                theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
              }`}
            >
              Season{" "}
              {leaderboardType === "contributors" ? "2025 Q1" : "Champions"}
            </div>
            <Star className="w-4 h-4 text-[#c9983a] animate-pulse-slow" />
          </div>

          {children}
        </div>
      </div>
    </div>
  );
}
