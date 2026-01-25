import { TrendingUp, TrendingDown } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { StatCard as StatCardType } from '../../types';

interface StatsCardProps {
  stat: StatCardType;
  index: number;
}

export function StatsCard({ stat, index }: StatsCardProps) {
  const { theme } = useTheme();
  const Icon = stat.icon;
  const isPositive = stat.change > 0;
  const isNegative = stat.change < 0;
  const isNeutral = stat.change === 0;

  return (
    <div
      className={`backdrop-blur-[40px] rounded-[18px] border p-6 hover:scale-105 transition-all duration-300 group relative overflow-hidden ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10 hover:bg-[#2d2820]/[0.5]'
          : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2]'
      }`}
      style={{ animationDelay: `${index * 50}ms` }}
    >
      {/* Background Glow */}
      <div className="absolute top-0 right-0 w-32 h-32 bg-gradient-to-br from-[#c9983a]/10 to-transparent rounded-full blur-2xl opacity-0 group-hover:opacity-100 transition-opacity duration-500" />

      {/* Header */}
      <div className="relative flex items-start justify-between mb-4">
        <div className="flex-1">
          <h3 className={`text-[12px] font-bold uppercase tracking-wide mb-1 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>
            {stat.title}
          </h3>
          <p className={`text-[10px] transition-colors ${
            theme === 'dark' ? 'text-[#9a8b7a]' : 'text-[#9a8b7a]'
          }`}>{stat.subtitle}</p>
        </div>
        
        {/* Icon with trend indicator */}
        <div className="relative">
          <div className="w-10 h-10 rounded-[12px] bg-gradient-to-br from-[#c9983a]/25 to-[#d4af37]/15 border border-[#c9983a]/30 flex items-center justify-center group-hover:scale-110 transition-transform">
            <Icon className="w-5 h-5 text-[#c9983a]" />
          </div>
          
          {/* Trend Arrow */}
          {!isNeutral && (
            <div className={`absolute -top-1 -right-1 w-5 h-5 rounded-full flex items-center justify-center ${
              isPositive ? 'bg-gradient-to-br from-[#4ade80] to-[#22c55e]' : 'bg-gradient-to-br from-[#f87171] to-[#ef4444]'
            }`}>
              {isPositive ? (
                <TrendingUp className="w-3 h-3 text-white" strokeWidth={3} />
              ) : (
                <TrendingDown className="w-3 h-3 text-white" strokeWidth={3} />
              )}
            </div>
          )}
        </div>
      </div>

      {/* Value */}
      <div className="relative">
        <div className={`text-[42px] font-black leading-none mb-2 group-hover:scale-105 transition-all origin-left ${
          theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
        }`}>
          {stat.value}
        </div>
        
        {/* Change Percentage */}
        <div className="flex items-center gap-2">
          <div className={`px-2.5 py-1 rounded-[6px] text-[11px] font-bold ${
            isPositive 
              ? 'bg-green-500/20 text-green-700 border border-green-500/30' 
              : isNegative 
              ? 'bg-red-500/20 text-red-700 border border-red-500/30'
              : 'bg-[#7a6b5a]/20 text-[#7a6b5a] border border-[#7a6b5a]/30'
          }`}>
            {isPositive && '+'}{stat.change}%
          </div>
        </div>
      </div>
    </div>
  );
}