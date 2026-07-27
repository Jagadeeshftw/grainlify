import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LeaderboardPage } from './LeaderboardPage';

const mockGetLeaderboard = vi.fn();
const mockGetRecommendedProjects = vi.fn();

vi.mock('../../../shared/api/client', () => ({
  getLeaderboard: (...args: unknown[]) => mockGetLeaderboard(...args),
  getRecommendedProjects: (...args: unknown[]) => mockGetRecommendedProjects(...args),
}));

vi.mock('../../../shared/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'light' }),
}));

vi.mock('../components/FallingPetals', () => ({
  FallingPetals: () => <div data-testid="falling-petals" />,
}));

vi.mock('../components/LeaderboardTypeToggle', () => ({
  LeaderboardTypeToggle: ({ leaderboardType }: { leaderboardType: string }) => <div>{leaderboardType}</div>,
}));

vi.mock('../components/LeaderboardHero', () => ({
  LeaderboardHero: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('../components/ContributorsPodium', () => ({
  ContributorsPodium: () => <div data-testid="contributors-podium" />,
}));

vi.mock('../components/ProjectsPodium', () => ({
  ProjectsPodium: () => <div data-testid="projects-podium" />,
}));

vi.mock('../components/FiltersSection', () => ({
  FiltersSection: () => <div data-testid="filters-section" />,
}));

vi.mock('../components/ContributorsTable', () => ({
  ContributorsTable: ({ data }: { data: Array<{ username: string }> }) => <div data-testid="contributors-table">{data[0]?.username ?? 'empty'}</div>,
}));

vi.mock('../components/ProjectsTable', () => ({
  ProjectsTable: () => <div data-testid="projects-table" />,
}));

vi.mock('../components/ContributorsPodiumSkeleton', () => ({
  ContributorsPodiumSkeleton: () => <div data-testid="podium-skeleton" />,
}));

vi.mock('../components/ContributorsTableSkeleton', () => ({
  ContributorsTableSkeleton: () => <div data-testid="table-skeleton" />,
}));

vi.mock('../../../shared/components/EmptyState', () => ({
  EmptyState: () => <div data-testid="empty-state" />,
}));

describe('LeaderboardPage export controls', () => {
  beforeEach(() => {
    mockGetLeaderboard.mockReset();
    mockGetRecommendedProjects.mockReset();
    mockGetLeaderboard.mockResolvedValue([
      {
        rank: 1,
        username: 'alice',
        avatar: 'https://example.com/alice.png',
        score: 120,
        trend: 'up',
        trendValue: 1,
        contributions: 6,
        ecosystems: ['Solana'],
      },
    ]);
    mockGetRecommendedProjects.mockResolvedValue({ projects: [] });
    Object.defineProperty(window, 'print', {
      configurable: true,
      value: vi.fn(),
    });
  });

  it('renders an accessible export trigger and invokes print for the selected format', async () => {
    render(<LeaderboardPage />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /export ranking as/i })).toBeInTheDocument();
    });

    const formatSelect = screen.getByLabelText(/export format/i);
    await userEvent.selectOptions(formatSelect, 'pdf');

    await userEvent.click(screen.getByRole('button', { name: /export ranking as/i }));

    expect(window.print).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/top contributors/i)).toBeInTheDocument();
  });
});
