import { useEffect, useMemo, useState } from 'react';

import { getLandingStats, type LandingStats } from '../api/client';

type LandingStatsDisplay = {
  activeProjects: string;
  contributors: string;
  grantsDistributed: string;
};

const formatCount = (n: number) => n.toLocaleString();

const formatUSD = (n: number) =>
  new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 0,
  }).format(n);

export function useLandingStats() {
  const [stats, setStats] = useState<LandingStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let isMounted = true;

    (async () => {
      try {
        setIsLoading(true);
        const s = await getLandingStats();
        if (!isMounted) return;
        setStats(s);
        setError(null);
      } catch (e) {
        if (!isMounted) return;
        setError(e instanceof Error ? e.message : 'Failed to load stats');
      } finally {
        if (!isMounted) return;
        setIsLoading(false);
      }
    })();

    return () => {
      isMounted = false;
    };
  }, []);

  const display: LandingStatsDisplay = useMemo(() => {
    if (!stats) {
      return {
        activeProjects: '—',
        contributors: '—',
        grantsDistributed: '—',
      };
    }

    return {
      activeProjects: formatCount(stats.active_projects),
      contributors: formatCount(stats.contributors),
      grantsDistributed: formatUSD(stats.grants_distributed_usd),
    };
  }, [stats]);

  return { stats, display, isLoading, error };
}


