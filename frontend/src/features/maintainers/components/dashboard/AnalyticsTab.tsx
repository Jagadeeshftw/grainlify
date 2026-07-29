/**
 * AnalyticsTab — bounty analytics dashboard for MaintainersPage.
 *
 * Design spec: design/specs/maintainers-bounty-analytics-dashboard.md
 * Issue: #1509
 *
 * Layout:
 *  ┌─ period filter ──────────────────────────────────┐
 *  ├─ [BountyFunnelChart 60%] [TopContributors 40%] ──┤
 *  └─ [PayoutHistoryTable full-width] ────────────────┘
 */

import { useState, useMemo } from 'react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { BountyFunnelChart } from './BountyFunnelChart';
import { PayoutHistoryTable } from './PayoutHistoryTable';
import { TopContributorsModule } from './TopContributorsModule';
import { AnalyticsPeriod, PayoutRecord, TopContributor } from '../../types';

// ─── Mock data (replace with real API calls) ─────────────────────────────────

const MOCK_PAYOUTS: PayoutRecord[] = [
  { id: '1',  date: '2026-07-20T10:00:00Z', contributor: 'alice',        repository: 'StelloPay/frontend',  amount: 250,  status: 'paid'       },
  { id: '2',  date: '2026-07-18T14:30:00Z', contributor: 'bob',          repository: 'StelloPay/core',      amount: 500,  status: 'paid'       },
  { id: '3',  date: '2026-07-15T09:00:00Z', contributor: 'carol',        repository: 'QuickLendX/protocol', amount: 150,  status: 'processing' },
  { id: '4',  date: '2026-07-12T16:45:00Z', contributor: 'dave',         repository: 'StelloPay/frontend',  amount: 300,  status: 'pending'    },
  { id: '5',  date: '2026-07-10T11:00:00Z', contributor: 'eve',          repository: 'StelloPay/core',      amount: 200,  status: 'paid'       },
  { id: '6',  date: '2026-07-08T08:00:00Z', contributor: 'frank',        repository: 'QuickLendX/protocol', amount: 175,  status: 'failed'     },
  { id: '7',  date: '2026-07-05T13:00:00Z', contributor: 'grace',        repository: 'StelloPay/frontend',  amount: 400,  status: 'paid'       },
  { id: '8',  date: '2026-07-01T10:00:00Z', contributor: 'heidi',        repository: 'StelloPay/core',      amount: 320,  status: 'paid'       },
  { id: '9',  date: '2026-06-28T15:00:00Z', contributor: 'ivan',         repository: 'QuickLendX/protocol', amount: 210,  status: 'paid'       },
  { id: '10', date: '2026-06-25T09:30:00Z', contributor: 'judy',         repository: 'StelloPay/frontend',  amount: 180,  status: 'pending'    },
  { id: '11', date: '2026-06-20T11:00:00Z', contributor: 'alice',        repository: 'StelloPay/core',      amount: 270,  status: 'paid'       },
  { id: '12', date: '2026-06-15T14:00:00Z', contributor: 'bob',          repository: 'QuickLendX/protocol', amount: 340,  status: 'paid'       },
];

const MOCK_CONTRIBUTORS: TopContributor[] = [
  { rank: 1, username: 'alice',  totalEarned: 1240, trend: 'up',   trendValue: 2 },
  { rank: 2, username: 'bob',    totalEarned: 980,  trend: 'same', trendValue: 0 },
  { rank: 3, username: 'carol',  totalEarned: 750,  trend: 'down', trendValue: 1 },
  { rank: 4, username: 'dave',   totalEarned: 620,  trend: 'up',   trendValue: 5 },
  { rank: 5, username: 'eve',    totalEarned: 410,  trend: 'same', trendValue: 0 },
];

// ─── Period filter config ─────────────────────────────────────────────────────

interface PeriodOption {
  value: AnalyticsPeriod;
  label: string;
  days: number | null;
}

const PERIOD_OPTIONS: PeriodOption[] = [
  { value: '7d',  label: 'Last 7 days',  days: 7  },
  { value: '30d', label: 'Last 30 days', days: 30 },
  { value: '90d', label: 'Last 90 days', days: 90 },
  { value: 'all', label: 'All time',     days: null },
];

function filterByPeriod<T extends { date: string }>(
  records: T[],
  period: AnalyticsPeriod,
): T[] {
  const opt = PERIOD_OPTIONS.find((p) => p.value === period)!;
  if (!opt.days) return records;
  const cutoff = Date.now() - opt.days * 86_400_000;
  return records.filter((r) => new Date(r.date).getTime() >= cutoff);
}

// ─── Funnel counts derived from payout records ───────────────────────────────

function deriveFunnelCounts(records: PayoutRecord[]) {
  // applied  = all records (every payout started as an application)
  // assigned = those that progressed (not failed at first step)
  // submitted = processing + paid
  // paid     = paid only
  const applied   = records.length;
  const assigned  = records.filter((r) => r.status !== 'failed').length;
  const submitted = records.filter((r) => r.status === 'processing' || r.status === 'paid').length;
  const paid      = records.filter((r) => r.status === 'paid').length;
  return { applied, assigned, submitted, paid };
}

// ─── Props ────────────────────────────────────────────────────────────────────

interface Project {
  id: string;
  github_full_name: string;
  status: string;
}

interface AnalyticsTabProps {
  selectedProjects: Project[];
  isLoadingProjects?: boolean;
  onNavigateToLeaderboard?: () => void;
}

// ─── Main component ───────────────────────────────────────────────────────────

export function AnalyticsTab({
  isLoadingProjects = false,
  onNavigateToLeaderboard,
}: AnalyticsTabProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [period, setPeriod] = useState<AnalyticsPeriod>('30d');

  // Filter mock data by selected period
  // (In production, pass `period` to API and receive pre-filtered data)
  const filteredPayouts = useMemo(
    () => filterByPeriod(MOCK_PAYOUTS, period),
    [period],
  );

  const funnelCounts = useMemo(
    () => deriveFunnelCounts(filteredPayouts),
    [filteredPayouts],
  );

  const isLoading = isLoadingProjects;

  return (
    <div className="space-y-6">
      {/* ── Period filter ── */}
      <div
        role="group"
        aria-label="Analytics time period"
        className={`flex items-center gap-2 flex-wrap`}
      >
        {PERIOD_OPTIONS.map((opt) => {
          const isActive = period === opt.value;
          return (
            <button
              key={opt.value}
              type="button"
              aria-pressed={isActive}
              aria-label={`Filter by ${opt.label}`}
              onClick={() => setPeriod(opt.value)}
              className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold border transition-all
                focus:outline-none focus:ring-2 focus:ring-[#f1b400] focus:ring-offset-1
                ${isActive
                  ? isDark
                    ? 'bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-[#c9983a]/60 text-[#fef5e7]'
                    : 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-[#c9983a]/50 text-[#2d2820]'
                  : isDark
                    ? 'bg-white/[0.08] border-white/20 text-[#b8a898] hover:bg-white/[0.14] hover:text-[#e8dfd0]'
                    : 'bg-white/[0.10] border-white/25 text-[#7a6b5a] hover:bg-white/[0.18] hover:text-[#2d2820]'
                }`}
            >
              {opt.label}
            </button>
          );
        })}
      </div>

      {/* ── Funnel + Top Contributors ── */}
      <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-6">
        <BountyFunnelChart
          applied={funnelCounts.applied}
          assigned={funnelCounts.assigned}
          submitted={funnelCounts.submitted}
          paid={funnelCounts.paid}
          isLoading={isLoading}
        />
        <TopContributorsModule
          contributors={MOCK_CONTRIBUTORS}
          isLoading={isLoading}
          onNavigateToLeaderboard={onNavigateToLeaderboard}
        />
      </div>

      {/* ── Payout History ── */}
      <PayoutHistoryTable
        records={filteredPayouts}
        isLoading={isLoading}
      />
    </div>
  );
}
