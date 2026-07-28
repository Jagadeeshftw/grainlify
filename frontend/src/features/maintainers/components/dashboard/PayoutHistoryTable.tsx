/**
 * PayoutHistoryTable — paginated payout history with skeleton + empty states.
 *
 * Design spec: design/specs/maintainers-bounty-analytics-dashboard.md §5
 * Issue: #1509
 *
 * Accessibility:
 *  - Proper <table> with scope="col" headers
 *  - Status pills carry aria-label
 *  - Pagination region uses aria-live="polite"
 *  - Below md: card-list replaces table (hidden md:table / md:hidden)
 */

import { useState } from 'react';
import { ChevronLeft, ChevronRight, DollarSign } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { SkeletonLoader } from '../../../../shared/components/SkeletonLoader';
import { PayoutRecord, PayoutStatus } from '../../types';

const PAGE_SIZE = 10;

// ─── Status pill ──────────────────────────────────────────────────────────────

interface StatusConfig {
  label: string;
  bg: string;
  border: string;
  text: string;
}

function getStatusConfig(status: PayoutStatus, isDark: boolean): StatusConfig {
  switch (status) {
    case 'paid':
      return {
        label: 'Paid',
        bg:     isDark ? 'bg-[#22c55e]/20' : 'bg-[#22c55e]/15',
        border: 'border-[#22c55e]/30',
        text:   isDark ? 'text-[#22c55e]'  : 'text-[#16a34a]',
      };
    case 'pending':
      return {
        label: 'Pending',
        bg:     isDark ? 'bg-[#f59e0b]/20' : 'bg-[#f59e0b]/15',
        border: 'border-[#f59e0b]/30',
        text:   isDark ? 'text-[#f59e0b]'  : 'text-[#d97706]',
      };
    case 'processing':
      return {
        label: 'Processing',
        bg:     isDark ? 'bg-[#3b82f6]/20' : 'bg-[#3b82f6]/15',
        border: 'border-[#3b82f6]/30',
        text:   isDark ? 'text-[#3b82f6]'  : 'text-[#2563eb]',
      };
    case 'failed':
      return {
        label: 'Failed',
        bg:     isDark ? 'bg-[#ef4444]/20' : 'bg-[#ef4444]/15',
        border: 'border-[#ef4444]/30',
        text:   isDark ? 'text-[#ef4444]'  : 'text-[#dc2626]',
      };
  }
}

function StatusPill({ status, isDark }: { status: PayoutStatus; isDark: boolean }) {
  const cfg = getStatusConfig(status, isDark);
  return (
    <span
      role="status"
      aria-label={cfg.label}
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-[11px] font-semibold border ${cfg.bg} ${cfg.border} ${cfg.text}`}
    >
      {cfg.label}
    </span>
  );
}

// ─── Avatar with initials fallback ───────────────────────────────────────────

function ContributorAvatar({ username, avatarUrl }: { username: string; avatarUrl?: string }) {
  const src = avatarUrl ?? `https://github.com/${username}.png?size=24`;
  const initials = username.slice(0, 2).toUpperCase();
  return (
    <img
      src={src}
      alt={username}
      className="w-6 h-6 rounded-full border border-[#c9983a]/40 flex-shrink-0"
      onError={(e) => {
        const t = e.currentTarget;
        t.style.display = 'none';
        const parent = t.parentElement;
        if (parent) {
          const fb = document.createElement('span');
          fb.className =
            'w-6 h-6 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center text-[9px] font-bold text-[#c9983a]';
          fb.textContent = initials;
          parent.insertBefore(fb, t);
        }
      }}
    />
  );
}

// ─── Skeleton rows ────────────────────────────────────────────────────────────

function SkeletonRows() {
  return (
    <>
      {Array.from({ length: 5 }).map((_, i) => (
        <tr key={i} aria-hidden="true">
          <td className="py-3 px-4"><SkeletonLoader className="h-4 w-24" /></td>
          <td className="py-3 px-4">
            <div className="flex items-center gap-2">
              <SkeletonLoader variant="circle" className="w-6 h-6" />
              <SkeletonLoader className="h-4 w-28" />
            </div>
          </td>
          <td className="py-3 px-4"><SkeletonLoader className="h-4 w-32" /></td>
          <td className="py-3 px-4 text-right"><SkeletonLoader className="h-4 w-16 ml-auto" /></td>
          <td className="py-3 px-4"><SkeletonLoader className="h-5 w-20 rounded-full" /></td>
        </tr>
      ))}
    </>
  );
}

// ─── Skeleton cards (mobile) ──────────────────────────────────────────────────

function SkeletonCards() {
  return (
    <ul className="space-y-3 md:hidden" aria-busy="true" aria-label="Loading payout history">
      {Array.from({ length: 5 }).map((_, i) => (
        <li
          key={i}
          className="rounded-[14px] border border-white/15 p-4 space-y-2 bg-white/[0.06]"
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <SkeletonLoader variant="circle" className="w-6 h-6" />
              <SkeletonLoader className="h-4 w-24" />
            </div>
            <SkeletonLoader className="h-3 w-20" />
          </div>
          <div className="flex items-center justify-between">
            <SkeletonLoader className="h-4 w-32" />
            <SkeletonLoader className="h-4 w-16" />
          </div>
          <SkeletonLoader className="h-5 w-20 rounded-full" />
        </li>
      ))}
    </ul>
  );
}

// ─── Empty state ──────────────────────────────────────────────────────────────

function EmptyState({ isDark }: { isDark: boolean }) {
  return (
    <tr>
      <td colSpan={5}>
        <div
          className={`flex flex-col items-center justify-center py-12 gap-3 ${
            isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}
        >
          <DollarSign className="w-10 h-10 opacity-30" aria-hidden />
          <p className="text-[14px] font-semibold">No payouts yet</p>
          <p className="text-[12px] text-center max-w-xs">
            When contributors complete bounties, payouts will appear here.
          </p>
        </div>
      </td>
    </tr>
  );
}

// ─── Payout row ───────────────────────────────────────────────────────────────

function PayoutRow({ record, isDark }: { record: PayoutRecord; isDark: boolean }) {
  const date = new Date(record.date).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
  return (
    <tr
      tabIndex={0}
      className={`border-b transition-colors focus:outline-none focus:ring-2 focus:ring-inset focus:ring-[#f1b400] ${
        isDark
          ? 'border-white/8 hover:bg-white/[0.04]'
          : 'border-black/5 hover:bg-black/[0.03]'
      }`}
    >
      <td className={`py-3 px-4 text-[13px] whitespace-nowrap ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
        {date}
      </td>
      <td className="py-3 px-4">
        <div className="flex items-center gap-2">
          <ContributorAvatar username={record.contributor} avatarUrl={record.avatarUrl} />
          <span className={`text-[13px] font-semibold ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
            {record.contributor}
          </span>
        </div>
      </td>
      <td className={`py-3 px-4 text-[12px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
        {record.repository}
      </td>
      <td className={`py-3 px-4 text-right text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`}>
        {record.amount.toLocaleString()} XLM
      </td>
      <td className="py-3 px-4">
        <StatusPill status={record.status} isDark={isDark} />
      </td>
    </tr>
  );
}

// ─── Mobile card ──────────────────────────────────────────────────────────────

function PayoutCard({ record, isDark }: { record: PayoutRecord; isDark: boolean }) {
  const date = new Date(record.date).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
  return (
    <li
      className={`rounded-[14px] border p-4 space-y-2 transition-colors ${
        isDark
          ? 'bg-white/[0.06] border-white/10 hover:bg-white/[0.10]'
          : 'bg-white/[0.12] border-white/20 hover:bg-white/[0.18]'
      }`}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ContributorAvatar username={record.contributor} avatarUrl={record.avatarUrl} />
          <span className={`text-[13px] font-semibold ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
            {record.contributor}
          </span>
        </div>
        <span className={`text-[11px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>{date}</span>
      </div>
      <div className="flex items-center justify-between">
        <span className={`text-[12px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
          {record.repository}
        </span>
        <span className={`text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`}>
          {record.amount.toLocaleString()} XLM
        </span>
      </div>
      <StatusPill status={record.status} isDark={isDark} />
    </li>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

interface PayoutHistoryTableProps {
  records: PayoutRecord[];
  isLoading?: boolean;
}

export function PayoutHistoryTable({
  records,
  isLoading = false,
}: PayoutHistoryTableProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [page, setPage] = useState(1);

  const totalPages = Math.max(1, Math.ceil(records.length / PAGE_SIZE));
  const pageRecords = records.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  const thClass = `py-3 px-4 text-left text-[11px] font-bold uppercase tracking-wide ${
    isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
  }`;

  return (
    <div
      className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
        isDark ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
      }`}
    >
      <h2 className={`text-[18px] font-bold mb-5 ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
        Payout History
      </h2>

      {/* ── Desktop table ── */}
      <div className="hidden md:block overflow-x-auto">
        <table
          role="table"
          aria-label="Payout history"
          aria-busy={isLoading}
          className="w-full border-collapse"
        >
          <thead>
            <tr className={`border-b ${isDark ? 'border-white/10' : 'border-black/8'}`}>
              <th scope="col" role="columnheader" className={thClass}>Date</th>
              <th scope="col" role="columnheader" className={thClass}>Contributor</th>
              <th scope="col" role="columnheader" className={thClass}>Repository</th>
              <th scope="col" role="columnheader" className={`${thClass} text-right`}>Amount</th>
              <th scope="col" role="columnheader" className={thClass}>Status</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              <SkeletonRows />
            ) : pageRecords.length === 0 ? (
              <EmptyState isDark={isDark} />
            ) : (
              pageRecords.map((r) => (
                <PayoutRow key={r.id} record={r} isDark={isDark} />
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* ── Mobile card list ── */}
      {isLoading ? (
        <SkeletonCards />
      ) : pageRecords.length === 0 ? (
        <div className={`md:hidden flex flex-col items-center py-10 gap-3 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
          <DollarSign className="w-10 h-10 opacity-30" aria-hidden />
          <p className="text-[13px]">No payouts yet</p>
        </div>
      ) : (
        <ul className="md:hidden space-y-3 mt-2" aria-label="Payout history">
          {pageRecords.map((r) => (
            <PayoutCard key={r.id} record={r} isDark={isDark} />
          ))}
        </ul>
      )}

      {/* ── Pagination ── */}
      {!isLoading && records.length > PAGE_SIZE && (
        <div className="flex items-center justify-between mt-5">
          <button
            type="button"
            aria-label="Previous page"
            disabled={page === 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            className={`flex items-center gap-1 px-3 py-1.5 rounded-[8px] text-[12px] font-semibold border transition-all
              focus:outline-none focus:ring-2 focus:ring-[#f1b400]
              disabled:opacity-40 disabled:cursor-not-allowed
              ${isDark
                ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.14]'
                : 'bg-white/[0.12] border-white/25 text-[#2d2820] hover:bg-white/[0.20]'
              }`}
          >
            <ChevronLeft className="w-3.5 h-3.5" aria-hidden />
            Previous
          </button>

          {/* Live page announcement */}
          <p
            aria-live="polite"
            aria-atomic="true"
            className={`text-[12px] font-medium ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}
          >
            Page {page} of {totalPages}
          </p>

          <button
            type="button"
            aria-label="Next page"
            disabled={page === totalPages}
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            className={`flex items-center gap-1 px-3 py-1.5 rounded-[8px] text-[12px] font-semibold border transition-all
              focus:outline-none focus:ring-2 focus:ring-[#f1b400]
              disabled:opacity-40 disabled:cursor-not-allowed
              ${isDark
                ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.14]'
                : 'bg-white/[0.12] border-white/25 text-[#2d2820] hover:bg-white/[0.20]'
              }`}
          >
            Next
            <ChevronRight className="w-3.5 h-3.5" aria-hidden />
          </button>
        </div>
      )}
    </div>
  );
}
