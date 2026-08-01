/**
 * AnalyticsTab — unit & interaction tests
 *
 * Coverage targets:
 *  - BountyFunnelChart: renders stages, sr-only table, empty state, loading skeleton
 *  - PayoutHistoryTable: renders rows, status pills, pagination, empty state, skeleton
 *  - TopContributorsModule: renders ranked list, trend icons, "View all" link, empty state
 *  - AnalyticsTab: period filter toggles, aria-pressed, data scoping, full composition
 *  - Accessibility: aria attributes on all interactive elements
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { BountyFunnelChart } from '../BountyFunnelChart';
import { PayoutHistoryTable } from '../PayoutHistoryTable';
import { TopContributorsModule } from '../TopContributorsModule';
import { AnalyticsTab } from '../AnalyticsTab';
import { PayoutRecord, TopContributor } from '../../../types';

/* ── Mock ThemeContext ───────────────────────────────────────────────────── */

vi.mock('../../../../../shared/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

/* ── Mock SkeletonLoader ─────────────────────────────────────────────────── */

vi.mock('../../../../../shared/components/SkeletonLoader', () => ({
  SkeletonLoader: ({ className, variant }: { className?: string; variant?: string }) => (
    <span data-testid="skeleton" data-variant={variant} className={className} />
  ),
}));

/* ── Mock Recharts (FunnelChart not available in jsdom) ──────────────────── */

vi.mock('recharts', () => ({
  FunnelChart:        ({ children }: any) => <div data-testid="funnel-chart">{children}</div>,
  Funnel:             ({ data }: any) => <div data-testid="funnel">{data?.map((d: any) => <span key={d.name}>{d.name}</span>)}</div>,
  LabelList:          () => null,
  Tooltip:            () => null,
  ResponsiveContainer: ({ children }: any) => <div>{children}</div>,
}));

/* ── Fixtures ────────────────────────────────────────────────────────────── */

const paidRecord: PayoutRecord = {
  id: '1',
  date: '2026-07-20T10:00:00Z',
  contributor: 'alice',
  repository: 'StelloPay/frontend',
  amount: 250,
  status: 'paid',
};

const pendingRecord: PayoutRecord = {
  id: '2',
  date: '2026-07-18T14:30:00Z',
  contributor: 'bob',
  repository: 'StelloPay/core',
  amount: 500,
  status: 'pending',
};

const processingRecord: PayoutRecord = {
  id: '3',
  date: '2026-07-15T09:00:00Z',
  contributor: 'carol',
  repository: 'QuickLendX/protocol',
  amount: 150,
  status: 'processing',
};

const failedRecord: PayoutRecord = {
  id: '4',
  date: '2026-07-12T16:45:00Z',
  contributor: 'dave',
  repository: 'StelloPay/core',
  amount: 300,
  status: 'failed',
};

const mockContributors: TopContributor[] = [
  { rank: 1, username: 'alice', totalEarned: 1240, trend: 'up',   trendValue: 2 },
  { rank: 2, username: 'bob',   totalEarned: 980,  trend: 'same', trendValue: 0 },
  { rank: 3, username: 'carol', totalEarned: 750,  trend: 'down', trendValue: 1 },
];

const mockProjects = [{ id: 'p1', github_full_name: 'org/repo', status: 'verified' }];

/* ══════════════════════════════════════════════════════════════════════════ */
/*  BountyFunnelChart                                                         */
/* ══════════════════════════════════════════════════════════════════════════ */

describe('BountyFunnelChart', () => {
  it('renders the chart heading', () => {
    render(<BountyFunnelChart applied={100} assigned={80} submitted={60} paid={50} />);
    expect(screen.getByText('Conversion Funnel')).toBeInTheDocument();
  });

  it('renders a Recharts FunnelChart when data is present', () => {
    render(<BountyFunnelChart applied={100} assigned={80} submitted={60} paid={50} />);
    expect(screen.getByTestId('funnel-chart')).toBeInTheDocument();
  });

  it('renders the sr-only accessible table', () => {
    render(<BountyFunnelChart applied={100} assigned={80} submitted={60} paid={50} />);
    const table = screen.getByRole('table', { hidden: true });
    expect(table).toHaveAttribute('aria-label', 'Bounty conversion funnel data');
  });

  it('sr-only table contains all four stage rows', () => {
    render(<BountyFunnelChart applied={100} assigned={80} submitted={60} paid={50} />);
    const table = screen.getByRole('table', { hidden: true });
    expect(within(table).getByText('Applied')).toBeInTheDocument();
    expect(within(table).getByText('Assigned')).toBeInTheDocument();
    expect(within(table).getByText('Submitted')).toBeInTheDocument();
    expect(within(table).getByText('Paid')).toBeInTheDocument();
  });

  it('sr-only table contains correct counts', () => {
    render(<BountyFunnelChart applied={142} assigned={98} submitted={71} paid={58} />);
    const table = screen.getByRole('table', { hidden: true });
    expect(within(table).getByText('142')).toBeInTheDocument();
    expect(within(table).getByText('98')).toBeInTheDocument();
    expect(within(table).getByText('71')).toBeInTheDocument();
    expect(within(table).getByText('58')).toBeInTheDocument();
  });

  it('shows empty state when applied is 0', () => {
    render(<BountyFunnelChart applied={0} assigned={0} submitted={0} paid={0} />);
    expect(screen.getByText('No bounty activity yet in this period')).toBeInTheDocument();
  });

  it('shows loading skeletons when isLoading is true', () => {
    render(<BountyFunnelChart applied={0} assigned={0} submitted={0} paid={0} isLoading />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
    expect(screen.queryByTestId('funnel-chart')).toBeNull();
  });

  it('shows conversion rate labels', () => {
    render(<BountyFunnelChart applied={100} assigned={80} submitted={60} paid={50} />);
    // "converted" appears for each adjacent-stage rate
    expect(screen.getAllByText(/converted/i).length).toBeGreaterThanOrEqual(3);
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */
/*  PayoutHistoryTable                                                        */
/* ══════════════════════════════════════════════════════════════════════════ */

describe('PayoutHistoryTable', () => {
  it('renders the table heading', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    expect(screen.getByText('Payout History')).toBeInTheDocument();
  });

  it('renders a <table> with aria-label', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    expect(screen.getByRole('table')).toHaveAttribute('aria-label', 'Payout history');
  });

  it('renders column headers with scope="col"', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    const headers = screen.getAllByRole('columnheader');
    const labels = headers.map((h) => h.textContent?.trim());
    expect(labels).toContain('Date');
    expect(labels).toContain('Contributor');
    expect(labels).toContain('Repository');
    expect(labels).toContain('Amount');
    expect(labels).toContain('Status');
  });

  it('renders contributor name', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    expect(screen.getByText('alice')).toBeInTheDocument();
  });

  it('renders amount with XLM suffix', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    expect(screen.getByText('250 XLM')).toBeInTheDocument();
  });

  it('renders repository name', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    expect(screen.getByText('StelloPay/frontend')).toBeInTheDocument();
  });

  it('renders "Paid" status pill with role=status', () => {
    render(<PayoutHistoryTable records={[paidRecord]} />);
    const pill = screen.getByRole('status', { name: 'Paid' });
    expect(pill).toBeInTheDocument();
  });

  it('renders "Pending" status pill', () => {
    render(<PayoutHistoryTable records={[pendingRecord]} />);
    expect(screen.getByRole('status', { name: 'Pending' })).toBeInTheDocument();
  });

  it('renders "Processing" status pill', () => {
    render(<PayoutHistoryTable records={[processingRecord]} />);
    expect(screen.getByRole('status', { name: 'Processing' })).toBeInTheDocument();
  });

  it('renders "Failed" status pill', () => {
    render(<PayoutHistoryTable records={[failedRecord]} />);
    expect(screen.getByRole('status', { name: 'Failed' })).toBeInTheDocument();
  });

  it('shows empty state when records is empty', () => {
    render(<PayoutHistoryTable records={[]} />);
    expect(screen.getByText('No payouts yet')).toBeInTheDocument();
  });

  it('shows skeleton rows when isLoading is true', () => {
    render(<PayoutHistoryTable records={[]} isLoading />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });

  it('does not render pagination when records fit on one page', () => {
    render(<PayoutHistoryTable records={[paidRecord, pendingRecord]} />);
    expect(screen.queryByRole('button', { name: 'Previous page' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Next page' })).toBeNull();
  });

  it('renders pagination when records exceed PAGE_SIZE (10)', () => {
    const many = Array.from({ length: 12 }, (_, i) => ({
      ...paidRecord,
      id: String(i),
      contributor: `user${i}`,
    }));
    render(<PayoutHistoryTable records={many} />);
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next page' })).toBeInTheDocument();
  });

  it('Previous button is disabled on first page', () => {
    const many = Array.from({ length: 12 }, (_, i) => ({ ...paidRecord, id: String(i), contributor: `u${i}` }));
    render(<PayoutHistoryTable records={many} />);
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
  });

  it('advances to page 2 when Next is clicked', async () => {
    const many = Array.from({ length: 12 }, (_, i) => ({ ...paidRecord, id: String(i), contributor: `user${i}` }));
    render(<PayoutHistoryTable records={many} />);
    await userEvent.click(screen.getByRole('button', { name: 'Next page' }));
    expect(screen.getByText('Page 2 of 2')).toBeInTheDocument();
  });

  it('Next button disabled on last page', async () => {
    const many = Array.from({ length: 12 }, (_, i) => ({ ...paidRecord, id: String(i), contributor: `u${i}` }));
    render(<PayoutHistoryTable records={many} />);
    await userEvent.click(screen.getByRole('button', { name: 'Next page' }));
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  it('page announcement region has aria-live=polite', () => {
    const many = Array.from({ length: 12 }, (_, i) => ({ ...paidRecord, id: String(i), contributor: `u${i}` }));
    render(<PayoutHistoryTable records={many} />);
    const region = screen.getByText(/Page \d+ of \d+/);
    expect(region).toHaveAttribute('aria-live', 'polite');
  });

  it('each data row is keyboard-focusable (tabIndex=0)', () => {
    render(<PayoutHistoryTable records={[paidRecord, pendingRecord]} />);
    const rows = screen.getAllByRole('row').filter((r) => r.getAttribute('tabindex') === '0');
    expect(rows.length).toBe(2);
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */
/*  TopContributorsModule                                                     */
/* ══════════════════════════════════════════════════════════════════════════ */

describe('TopContributorsModule', () => {
  it('renders the module heading', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByText('Top Contributors')).toBeInTheDocument();
  });

  it('renders an ordered list with aria-label', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByRole('list', { name: 'Top contributors by earnings' })).toBeInTheDocument();
  });

  it('renders all contributor usernames', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByText('alice')).toBeInTheDocument();
    expect(screen.getByText('bob')).toBeInTheDocument();
    expect(screen.getByText('carol')).toBeInTheDocument();
  });

  it('renders earned amounts with XLM suffix', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByText('1,240 XLM')).toBeInTheDocument();
    expect(screen.getByText('980 XLM')).toBeInTheDocument();
  });

  it('renders rank badges with aria-label', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByLabelText('Rank 1')).toBeInTheDocument();
    expect(screen.getByLabelText('Rank 2')).toBeInTheDocument();
    expect(screen.getByLabelText('Rank 3')).toBeInTheDocument();
  });

  it('renders trend "up" with correct aria-label', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByLabelText('Rank improved by 2')).toBeInTheDocument();
  });

  it('renders trend "down" with correct aria-label', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByLabelText('Rank dropped by 1')).toBeInTheDocument();
  });

  it('renders trend "same" with correct aria-label', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    expect(screen.getByLabelText('Rank unchanged')).toBeInTheDocument();
  });

  it('caps list at 5 even when more are provided', () => {
    const many = Array.from({ length: 8 }, (_, i) => ({
      rank: i + 1, username: `user${i}`, totalEarned: 100 * (8 - i),
      trend: 'same' as const, trendValue: 0,
    }));
    render(<TopContributorsModule contributors={many} />);
    expect(screen.getAllByRole('listitem').length).toBe(5);
  });

  it('renders "View all" link with descriptive aria-label', () => {
    const onNavigate = vi.fn();
    render(<TopContributorsModule contributors={mockContributors} onNavigateToLeaderboard={onNavigate} />);
    const link = screen.getByRole('link', { name: 'View all contributors on leaderboard' });
    expect(link).toBeInTheDocument();
  });

  it('calls onNavigateToLeaderboard when "View all" is clicked', async () => {
    const onNavigate = vi.fn();
    render(<TopContributorsModule contributors={mockContributors} onNavigateToLeaderboard={onNavigate} />);
    await userEvent.click(screen.getByRole('link', { name: 'View all contributors on leaderboard' }));
    expect(onNavigate).toHaveBeenCalledTimes(1);
  });

  it('shows empty state when contributors is empty', () => {
    render(<TopContributorsModule contributors={[]} />);
    expect(screen.getByText('No contributor data yet')).toBeInTheDocument();
  });

  it('shows loading skeletons when isLoading is true', () => {
    render(<TopContributorsModule contributors={[]} isLoading />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });

  it('each list item is keyboard-focusable (tabIndex=0)', () => {
    render(<TopContributorsModule contributors={mockContributors} />);
    const items = screen.getAllByRole('listitem').filter((el) => el.getAttribute('tabindex') === '0');
    expect(items.length).toBe(mockContributors.length);
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */
/*  AnalyticsTab (composition + period filter)                                */
/* ══════════════════════════════════════════════════════════════════════════ */

describe('AnalyticsTab', () => {
  it('renders all three module headings', () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    expect(screen.getByText('Conversion Funnel')).toBeInTheDocument();
    expect(screen.getByText('Top Contributors')).toBeInTheDocument();
    expect(screen.getByText('Payout History')).toBeInTheDocument();
  });

  it('renders all four period filter buttons', () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    expect(screen.getByRole('button', { name: 'Filter by Last 7 days' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Filter by Last 30 days' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Filter by Last 90 days' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Filter by All time' })).toBeInTheDocument();
  });

  it('period filter group has role="group" with aria-label', () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    expect(screen.getByRole('group', { name: 'Analytics time period' })).toBeInTheDocument();
  });

  it('"Last 30 days" is active (aria-pressed=true) by default', () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    const btn = screen.getByRole('button', { name: 'Filter by Last 30 days' });
    expect(btn).toHaveAttribute('aria-pressed', 'true');
  });

  it('inactive period buttons have aria-pressed=false', () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    const btn = screen.getByRole('button', { name: 'Filter by Last 7 days' });
    expect(btn).toHaveAttribute('aria-pressed', 'false');
  });

  it('clicking a period button makes it active', async () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    const btn = screen.getByRole('button', { name: 'Filter by Last 7 days' });
    await userEvent.click(btn);
    expect(btn).toHaveAttribute('aria-pressed', 'true');
  });

  it('clicking a period button deactivates the previous selection', async () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    await userEvent.click(screen.getByRole('button', { name: 'Filter by Last 7 days' }));
    const prev = screen.getByRole('button', { name: 'Filter by Last 30 days' });
    expect(prev).toHaveAttribute('aria-pressed', 'false');
  });

  it('calls onNavigateToLeaderboard when "View all" link is clicked', async () => {
    const onNavigate = vi.fn();
    render(<AnalyticsTab selectedProjects={mockProjects} onNavigateToLeaderboard={onNavigate} />);
    await userEvent.click(screen.getByRole('link', { name: 'View all contributors on leaderboard' }));
    expect(onNavigate).toHaveBeenCalledTimes(1);
  });

  it('shows skeletons for all three modules when isLoadingProjects=true', () => {
    render(<AnalyticsTab selectedProjects={mockProjects} isLoadingProjects />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });

  it('period filter buttons are keyboard-activatable', async () => {
    render(<AnalyticsTab selectedProjects={mockProjects} />);
    const btn = screen.getByRole('button', { name: 'Filter by All time' });
    btn.focus();
    await userEvent.keyboard(' ');
    expect(btn).toHaveAttribute('aria-pressed', 'true');
  });
});
