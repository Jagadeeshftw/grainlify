/**
 * BountyFunnelChart — 4-stage application-to-payout conversion funnel.
 *
 * Design spec: design/specs/maintainers-bounty-analytics-dashboard.md §4
 * Issue: #1509
 *
 * Accessibility:
 *  - Chart wrapped in aria-hidden="true"; data exposed via sr-only <table>
 *  - Each stage color is paired with a distinct text label (never color-only)
 */

import { FunnelChart, Funnel, LabelList, Tooltip, ResponsiveContainer } from 'recharts';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { SkeletonLoader } from '../../../../shared/components/SkeletonLoader';
import { FunnelStage } from '../../types';

interface BountyFunnelChartProps {
  applied: number;
  assigned: number;
  submitted: number;
  paid: number;
  isLoading?: boolean;
}

// ─── helpers ─────────────────────────────────────────────────────────────────

function conversionRate(numerator: number, denominator: number): string {
  if (denominator === 0) return '—';
  return `${Math.round((numerator / denominator) * 100)}%`;
}

// ─── Tooltip ─────────────────────────────────────────────────────────────────

function FunnelTooltip({
  active,
  payload,
  isDark,
  applied,
}: {
  active?: boolean;
  payload?: any[];
  isDark: boolean;
  applied: number;
}) {
  if (!active || !payload?.length) return null;
  const { name, value } = payload[0].payload;
  return (
    <div
      className={`backdrop-blur-[40px] rounded-[14px] border px-4 py-3 ${
        isDark
          ? 'bg-neutral-900/80 border-white/10'
          : 'bg-[#e8dfd0]/95 border-white/25'
      }`}
    >
      <p className={`text-[13px] font-bold mb-1 ${isDark ? 'text-neutral-300' : 'text-[#7a6b5a]'}`}>
        {name}
      </p>
      <p className={`text-[14px] font-black ${isDark ? 'text-neutral-100' : 'text-[#2d2820]'}`}>
        {value.toLocaleString()}
      </p>
      <p className={`text-[11px] font-semibold mt-0.5 ${isDark ? 'text-neutral-400' : 'text-[#7a6b5a]'}`}>
        {conversionRate(value, applied)} of total
      </p>
    </div>
  );
}

// ─── Loading skeleton ─────────────────────────────────────────────────────────

function FunnelSkeleton() {
  const widths = ['w-full', 'w-4/5', 'w-3/5', 'w-2/5'];
  return (
    <div className="space-y-3 py-4" aria-busy="true" aria-label="Loading funnel data">
      {widths.map((w, i) => (
        <div key={i} className="flex items-center gap-3">
          <SkeletonLoader className={`h-10 ${w} rounded-[8px]`} />
          <SkeletonLoader className="h-4 w-16" />
        </div>
      ))}
    </div>
  );
}

// ─── Empty state ──────────────────────────────────────────────────────────────

function FunnelEmpty({ isDark }: { isDark: boolean }) {
  return (
    <div className={`flex flex-col items-center justify-center h-[260px] gap-3 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
      <svg aria-hidden className="w-12 h-12 opacity-40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5}
          d="M3 4h18l-7 8v5l-4 3V12L3 4z" />
      </svg>
      <p className="text-[13px] font-medium">No bounty activity yet in this period</p>
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

export function BountyFunnelChart({
  applied,
  assigned,
  submitted,
  paid,
  isLoading = false,
}: BountyFunnelChartProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const funnelData: FunnelStage[] = [
    { name: 'Applied',   value: applied,   fill: '#c9983a' },
    { name: 'Assigned',  value: assigned,  fill: '#3b82f6' },
    { name: 'Submitted', value: submitted, fill: '#f59e0b' },
    { name: 'Paid',      value: paid,      fill: '#22c55e' },
  ];

  const isEmpty = !isLoading && applied === 0;

  // Conversion rates between adjacent stages
  const rates = [
    { from: 'Applied',   to: 'Assigned',  rate: conversionRate(assigned,  applied)  },
    { from: 'Assigned',  to: 'Submitted', rate: conversionRate(submitted, assigned)  },
    { from: 'Submitted', to: 'Paid',      rate: conversionRate(paid,      submitted) },
  ];

  return (
    <div
      className={`backdrop-blur-[40px] rounded-[24px] border p-6 relative overflow-hidden transition-colors ${
        isDark ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
      }`}
    >
      {/* Background glow */}
      <div className="absolute top-0 right-0 w-64 h-64 bg-gradient-to-bl from-[#c9983a]/8 to-transparent rounded-full blur-3xl pointer-events-none" />

      <div className="relative">
        <h2 className={`text-[18px] font-bold mb-1 ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
          Conversion Funnel
        </h2>
        <p className={`text-[12px] font-medium mb-5 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
          Applied → Assigned → Submitted → Paid
        </p>

        {isLoading ? (
          <FunnelSkeleton />
        ) : isEmpty ? (
          <FunnelEmpty isDark={isDark} />
        ) : (
          <>
            {/* Chart — aria-hidden; data exposed via sr-only table below */}
            <div aria-hidden="true">
              <ResponsiveContainer width="100%" height={260}>
                <FunnelChart>
                  <Tooltip
                    content={(props) => (
                      <FunnelTooltip {...props} isDark={isDark} applied={applied} />
                    )}
                  />
                  <Funnel
                    dataKey="value"
                    data={funnelData}
                    isAnimationActive
                    animationDuration={800}
                  >
                    <LabelList
                      dataKey="name"
                      position="right"
                      style={{
                        fill: isDark ? '#e8dfd0' : '#2d2820',
                        fontSize: 13,
                        fontWeight: 600,
                      }}
                    />
                    <LabelList
                      dataKey="value"
                      position="left"
                      style={{
                        fill: isDark ? '#b8a898' : '#7a6b5a',
                        fontSize: 12,
                        fontWeight: 500,
                      }}
                    />
                  </Funnel>
                </FunnelChart>
              </ResponsiveContainer>
            </div>

            {/* Conversion rate legend */}
            <div className="flex justify-around mt-2">
              {rates.map((r) => (
                <div key={r.to} className="text-center">
                  <p className={`text-[10px] font-medium ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
                    {r.from} → {r.to}
                  </p>
                  <p className={`text-[13px] font-bold ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
                    {r.rate} converted
                  </p>
                </div>
              ))}
            </div>

            {/* ── Accessible data-table alternative (screen-reader only) ── */}
            <table className="sr-only" aria-label="Bounty conversion funnel data">
              <thead>
                <tr>
                  <th scope="col">Stage</th>
                  <th scope="col">Count</th>
                  <th scope="col">Conversion rate from previous stage</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>Applied</td>
                  <td>{applied}</td>
                  <td>100% (baseline)</td>
                </tr>
                <tr>
                  <td>Assigned</td>
                  <td>{assigned}</td>
                  <td>{conversionRate(assigned, applied)}</td>
                </tr>
                <tr>
                  <td>Submitted</td>
                  <td>{submitted}</td>
                  <td>{conversionRate(submitted, assigned)}</td>
                </tr>
                <tr>
                  <td>Paid</td>
                  <td>{paid}</td>
                  <td>{conversionRate(paid, submitted)}</td>
                </tr>
              </tbody>
            </table>
          </>
        )}
      </div>
    </div>
  );
}
