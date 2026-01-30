import React, { useState, useEffect } from 'react';
import { LeaderboardType, FilterType, Petal, LeaderData, ProjectData } from '../types';
import { getLeaderboard, getProjectLeaderboard } from '../../../shared/api/client';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { FallingPetals } from '../components/FallingPetals';
import { LeaderboardTypeToggle } from '../components/LeaderboardTypeToggle';
import { LeaderboardHero } from '../components/LeaderboardHero';
import { ContributorsPodium } from '../components/ContributorsPodium';
import { ProjectsPodium } from '../components/ProjectsPodium';
import { FiltersSection } from '../components/FiltersSection';
import { ContributorsTable } from '../components/ContributorsTable';
import { ProjectsTable } from '../components/ProjectsTable';
import { LeaderboardStyles } from '../components/LeaderboardStyles';
import { ContributorsPodiumSkeleton } from '../components/ContributorsPodiumSkeleton';
import { ContributorsTableSkeleton } from '../components/ContributorsTableSkeleton';
import { ProjectsPodiumSkeleton } from '../components/ProjectsPodiumSkeleton';
import { EcosystemOption } from '../components/FiltersSection';

export function LeaderboardPage() {
  const { theme } = useTheme();
  const [activeFilter, setActiveFilter] = useState<FilterType>('overall');
  const [leaderboardType, setLeaderboardType] = useState<LeaderboardType>('contributors');
  const [showEcosystemDropdown, setShowEcosystemDropdown] = useState(false);
  const [selectedEcosystem, setSelectedEcosystem] = useState<EcosystemOption>({ label: 'All Ecosystems', value: 'all' });
  const [petals, setPetals] = useState<Petal[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);
  const [leaderboardData, setLeaderboardData] = useState<LeaderData[]>([]);
  const [projectsData, setProjectsData] = useState<ProjectData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);

  // Fetch leaderboard data
  useEffect(() => {
    const fetchLeaderboard = async () => {
      if (leaderboardType === 'contributors') {
        setIsLoading(true);
        setOffset(0); // Reset offset when switching types
        try {
          const data = await getLeaderboard(10, 0);
          // Transform API data to match LeaderData type
          const transformedData: LeaderData[] = data.map((item) => ({
            rank: item.rank,
            rank_tier: item.rank_tier,
            rank_tier_name: item.rank_tier_name,
            username: item.username,
            avatar: item.avatar || `https://github.com/${item.username}.png?size=200`,
            user_id: item.user_id || '',
            score: item.score,
            trend: item.trend,
            trendValue: item.trendValue,
            contributions: item.contributions,
            ecosystems: item.ecosystems || [],
          }));
          setLeaderboardData(transformedData);
          setHasMore(data.length === 10); // If we got 10 items, there might be more
          setIsLoading(false);
        } catch (err) {
          console.error('Failed to fetch leaderboard:', err);
          setLeaderboardData([]);
          setIsLoading(false); // Set loading to false to show empty state instead of skeleton
        }
      } else {
        // For projects, fetch from API
        setIsLoading(true);
        setOffset(0); // Reset offset when switching types
        try {
          // Use ecosystem slug from the selected option
          let ecosystemFilter: string | undefined = undefined;
          if (selectedEcosystem.value !== 'all') {
            ecosystemFilter = selectedEcosystem.value;
          }
          console.log('Fetching project leaderboard with filter:', ecosystemFilter);
          const data = await getProjectLeaderboard(100, 0, ecosystemFilter);
          console.log('Project leaderboard data received:', data);
          // Transform API data to match ProjectData type
          const transformedData: ProjectData[] = data.map((item) => ({
            rank: item.rank,
            name: item.name,
            logo: item.logo,
            score: item.score,
            trend: item.trend,
            trendValue: item.trendValue,
            contributors: item.contributors,
            ecosystems: item.ecosystems || [],
            activity: item.activity,
          }));
          setProjectsData(transformedData);
          setIsLoading(false);
        } catch (err) {
          console.error('Failed to fetch project leaderboard:', err);
          setProjectsData([]);
          setIsLoading(false); // Set loading to false to show empty state instead of skeleton
        }
      }
    };

    fetchLeaderboard();
  }, [leaderboardType, selectedEcosystem]);

  // Load more leaderboard data
  const loadMore = async () => {
    if (isLoadingMore || !hasMore) return;
    
    setIsLoadingMore(true);
    try {
      const nextOffset = offset + 10;
      const data = await getLeaderboard(10, nextOffset);
      
      if (data.length === 0) {
        setHasMore(false);
        setIsLoadingMore(false);
        return;
      }

      // Transform and append new data
      const transformedData: LeaderData[] = data.map((item) => ({
        rank: item.rank,
        rank_tier: item.rank_tier,
        rank_tier_name: item.rank_tier_name,
        username: item.username,
        avatar: item.avatar || `https://github.com/${item.username}.png?size=200`,
        user_id: item.user_id || '',
        score: item.score,
        trend: item.trend,
        trendValue: item.trendValue,
        contributions: item.contributions,
        ecosystems: item.ecosystems || [],
      }));
      
      setLeaderboardData((prev) => [...prev, ...transformedData]);
      setOffset(nextOffset);
      setHasMore(data.length === 10); // If we got less than 10, no more data
    } catch (err) {
      console.error('Failed to load more leaderboard:', err);
      setHasMore(false);
    } finally {
      setIsLoadingMore(false);
    }
  };

  // Generate falling petals on mount
  useEffect(() => {
    const generatePetals = () => {
      const newPetals: Petal[] = [];
      for (let i = 0; i < 30; i++) {
        newPetals.push({
          id: i,
          left: Math.random() * 100,
          delay: Math.random() * 5,
          duration: 8 + Math.random() * 6,
          rotation: Math.random() * 360,
          size: 0.6 + Math.random() * 0.8,
        });
      }
      setPetals(newPetals);
    };

    generatePetals();
    setTimeout(() => setIsLoaded(true), 100);

    // Regenerate petals every 15 seconds for continuous effect
    const interval = setInterval(generatePetals, 15000);
    return () => clearInterval(interval);
  }, []);

  // Ensure we have at least 3 items for the podium (pad with empty data if needed)
  const contributorTopThree: LeaderData[] = [
    ...leaderboardData.slice(0, 3),
    ...Array(Math.max(0, 3 - leaderboardData.length)).fill(null).map((_, i) => ({
      rank: leaderboardData.length + i + 1,
      username: '-',
      avatar: '👤',
      score: 0,
      trend: 'same' as const,
      trendValue: 0,
      contributions: 0,
      ecosystems: [],
    })),
  ].slice(0, 3) as LeaderData[];
  
  // Ensure we have at least 3 items for the project podium (pad with empty data if needed)
  const projectTopThree: ProjectData[] = [
    ...projectsData.slice(0, 3),
    ...Array(Math.max(0, 3 - projectsData.length)).fill(null).map((_, i) => ({
      rank: projectsData.length + i + 1,
      name: '-',
      logo: '📦',
      score: 0,
      trend: 'same' as const,
      trendValue: 0,
      contributors: 0,
      ecosystems: [],
    })),
  ].slice(0, 3) as ProjectData[];

  return (
    <div className="space-y-6 relative">
      {/* Falling Golden Petals - Full Page */}
      <FallingPetals petals={petals} />

      {/* Leaderboard Type Toggle - Floating Above Everything */}
      <LeaderboardTypeToggle
        leaderboardType={leaderboardType}
        onToggle={setLeaderboardType}
        isLoaded={isLoaded}
      />

      {/* Hero Header Section */}
      <LeaderboardHero leaderboardType={leaderboardType} isLoaded={isLoaded}>
        {/* Top 3 Podium - Contributors */}
        {leaderboardType === 'contributors' && isLoading && (
          <ContributorsPodiumSkeleton />
        )}
        {leaderboardType === 'contributors' && !isLoading && leaderboardData.length > 0 && (
          <ContributorsPodium 
            topThree={contributorTopThree} 
            isLoaded={isLoaded} 
            actualCount={leaderboardData.length}
          />
        )}
        {leaderboardType === 'contributors' && !isLoading && leaderboardData.length === 0 && (
          <div className={`text-center py-8 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>
            No contributors yet. Be the first to contribute!
          </div>
        )}

        {/* Top 3 Podium - Projects */}
        {leaderboardType === 'projects' && isLoading && (
          <ProjectsPodiumSkeleton />
        )}
        {leaderboardType === 'projects' && !isLoading && projectsData.length > 0 && (
          <ProjectsPodium topThree={projectTopThree} isLoaded={isLoaded} />
        )}
        {leaderboardType === 'projects' && !isLoading && projectsData.length === 0 && (
          <div className={`text-center py-8 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>
            No projects yet. Be the first to add a project!
          </div>
        )}
      </LeaderboardHero>

      {/* Filters Section */}
      <FiltersSection
        activeFilter={activeFilter}
        onFilterChange={setActiveFilter}
        selectedEcosystem={selectedEcosystem}
        onEcosystemChange={setSelectedEcosystem}
        showDropdown={showEcosystemDropdown}
        onToggleDropdown={() => setShowEcosystemDropdown(!showEcosystemDropdown)}
        isLoaded={isLoaded}
      />

      {/* Leaderboard Table - Contributors */}
      {leaderboardType === 'contributors' && (
        <>
          {isLoading ? (
            <ContributorsTableSkeleton />
          ) : (
            <>
              <ContributorsTable
                data={leaderboardData}
                activeFilter={activeFilter}
                isLoaded={isLoaded}
                onUserClick={(username, userId) => {
                  // Navigate to profile page with user identifier
                  const identifier = userId || username;
                  window.location.href = `/dashboard?page=profile&user=${encodeURIComponent(identifier)}`;
                }}
              />
              {hasMore && (
                <div className="flex justify-center mt-6">
                  <button
                    onClick={loadMore}
                    disabled={isLoadingMore}
                    className={`px-6 py-3 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[14px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2`}
                  >
                    {isLoadingMore ? (
                      <>
                        <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                        Loading...
                      </>
                    ) : (
                      'View All'
                    )}
                  </button>
                </div>
              )}
            </>
          )}
        </>
      )}

      {/* Leaderboard Table - Projects */}
      {leaderboardType === 'projects' && (
        <>
          {isLoading ? (
            <div className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              <div className="text-center">Loading projects...</div>
            </div>
          ) : projectsData.length === 0 ? (
            <div className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 text-center ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              No projects found. Be the first to add a project!
            </div>
          ) : (
            <ProjectsTable
              data={projectsData}
              activeFilter={activeFilter}
              isLoaded={isLoaded}
            />
          )}
        </>
      )}

      {/* CSS Animations */}
      <LeaderboardStyles />
    </div>
  );
}
