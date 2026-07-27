/**
 * TopContributorsModule — ranked list of top 5 contributors by earnings.
 *
 * Design spec: design/specs/maintainers-bounty-analytics-dashboard.md §6
 * Issue: #1509
 *
 * Accessibility:
 *  - List is a <ol> with aria-label
 *  - Trend icons carry aria-label text
 *  - "View all" link includes descriptive aria-label
 */

import { TrendingUp, TrendingDown, Minus } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { SkeletonLoader } from '../../../../shared/components/SkeletonLoader';
import { TopContributor } from '../../types';

interface TopContributorsModuleProps {
  contributors: TopContributor[];
  isLoading?: boolean;
  onNavigateToLeaderboard?: () => void;
}

// ─── Rank badge ───────────────────────────────────────────────────────────────

function RankBadge({ rank }: { rank: number }) {
  const config =
    rank === 1 ? { bg: 'from-[#c9983a] to-[#d4af37]', text: 'text-white' } :
    rank === 2 ? { bg: 'from-[#a8a29e] to-[#78716c]', text: 'text-white' } :
    rank === 3 ? { bg: 'from-[#cd7f32] to-[#a0522d]', text: 'text-white' } :
                 { bg: 'from-white/20 to-white/10',    text: 'text-[#b8a898]' };

  return (
    <span
      aria-label={`Rank ${rank}`}
      className={`w-7 h-7 rounded-full flex-shrink-0 flex items-center justify-center
        bg-gradient-to-br ${config.bg} ${config.text}
        text-[11px] font-black border border-white/20`}
    >
      {rank}
    </span>
  );
}

// ─── Contributor avatar ───────────────────────────────────────────────────────

function ContribAvatar({ username, avatarUrl }: { username: string; avatarUrl?: string }) {
  const src = avatarUrl ?? `https://github.com/${username}.png?size=32`;
  const initials = username.slice(0, 2).toUpperCase();
  return (
    <img
      src={src}
      alt={username}
      className="w-8 h-8 rounded-full border border-[#c9983a]/40 flex-shrink-0"
      onError={(e) => {
        const t = e.currentTarget;
        t.style.display = 'none';
        const parent = t.parentElement;
        if (parent) {
          const fb = document.createElement('span');
          fb.className =
            'w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center text-[10px] font-bold text-[#c9983a]';
          fb.textContent = initials;
          parent.insertBefore(fb, t);
        }
      }}
    />
  );
}

// ─── Trend indicator ──────────────────────────────────────────────────────────

function TrendIndicator({ trend, value }: { trend: TopContributor['trend']; value: number }) {
  if (trend === 'up') {
    return (
      <span className="flex items-center gap-0.5 text-[#22c55e]" aria-label={`Rank improved by ${value}`}>
        <TrendingUp className="w-3.5 h-3.5" aria-hidden />
        <span className="text-[11px] font-semibold">+{value}</span>
      </span>
    );
  }
  if (trend === 'down') {
    return (
      <span className="flex items-center gap-0.5 text-[#ef4444]" aria-label={`Rank dropped by ${value}`}>
        <TrendingDown className="w-3.5 h-3.5" aria-hidden />
        <span className="text-[11px] font-semibold">-{value}</span>
      </span>
    );
  }
  return (
    <span className="flex items-center gap-0.5 text-[#a8a29e]" aria-label="Rank unchanged">
      <Minus className="w-3.5 h-3.5" aria-hidden />
    </span>
  );
}

// ─── Skeleton rows ────────────────────────────────────────────────────────────

function SkeletonList() {
  return (
    <ul className="space-y-3" aria-busy="true" aria-label="Loading top contributors">
      {Array.from({ length: 5 }).map((_, i) => (
        <li key={i} className="flex items-center gap-3">
          <SkeletonLoader variant="circle" className="w-7 h-7" />
          <SkeletonLoader variant="circle" className="w-8 h-8" />
          <SkeletonLoader className="h-4 w-28 flex-1" />
          <SkeletonLoader className="h-4 w-16" />
        </li>
      ))}
    </ul>
  );
}

// ─── Empty state ──────────────────────────────────────────────────────────────

function EmptyState({ isDark }: { isDark: boolean }) {
  return (
    <div className={`flex flex-col items-center justify-center py-10 gap-2 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
      <svg aria-hidden className="w-10 h-10 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5}
          d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0" />
      </svg>
      <p className="text-[13px] font-medium">No contributor data yet</p>
      <p className="text-[12px] text-center">Earn data will appear here once payouts are processed.</p>
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

export function TopContributorsModule({
  contributors,
  isLoading = false,
  onNavigateToLeaderboard,
}: TopContributorsModuleProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div
      className={`backdrop-blur-[40px] rounded-[24px] border p-6 relative overflow-hidden transition-colors ${
        isDark ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
      }`}
    >
      {/* Background glow */}
      <div className="absolute top-0 left-0 w-48 h-48 bg-gradient-to-br from-[#c9983a]/8 to-transparent rounded-full blur-3xl pointer-events-none" />

      <div className="relative">
        {/* Header */}
        <div className="flex items-center justify-between mb-5">
          <h2 className={`text-[18px] font-bold ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
            Top Contributors
          </h2>
          {onNavigateToLeaderboard && (
            <a
              href="#"
              onClick={(e) => { e.preventDefault(); onNavigateToLeaderboard(); }}
              aria-label="View all contributors on leaderboard"
              className={`text-[12px] font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-[#f1b400] rounded px-1
                ${isDark ? 'text-[#c9983a] hover:text-[#e8c77f]' : 'text-[#a67c2e] hover:text-[#c9983a]'}`}
            >
              View all →
            </a>
          )}
        </div>

        {/* Divider */}
        <div className={`h-px mb-4 ${isDark ? 'bg-white/10' : 'bg-black/8'}`} />

        {/* Content */}
        {isLoading ? (
          <SkeletonList />
        ) : contributors.length === 0 ? (
          <EmptyState isDark={isDark} />
        ) : (
          <ol aria-label="Top contributors by earnings" className="space-y-3">
            {contributors.slice(0, 5).map((c) => (
              <li
                key={c.username}
                tabIndex={0}
                className={`flex items-center gap-3 rounded-[10px] px-2 py-1.5 transition-colors
                  focus:outline-none focus:ring-2 focus:ring-[#f1b400]
                  ${isDark ? 'hover:bg-white/[0.05]' : 'hover:bg-black/[0.03]'}`}
              >
                <RankBadge rank={c.rank} />
                <ContribAvatar username={c.username} avatarUrl={c.avatarUrl} />
                <span className={`flex-1 text-[13px] font-semibold truncate ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
                  {c.username}
                </span>
                <span className={`text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`}>
                  {c.totalEarned.toLocaleString()} XLM
                </span>
                <TrendIndicator trend={c.trend} value={c.trendValue} />
              </li>
            ))}
          </ol>
        )}
      </div>
    </div>
  );
}
