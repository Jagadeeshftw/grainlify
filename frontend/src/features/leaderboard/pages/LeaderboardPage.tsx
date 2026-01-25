import { useState, useEffect } from "react";
import { LeaderboardType, FilterType, Petal, LeaderData } from "../types";
import { projectsData } from "../data/leaderboardData";
import { getLeaderboard } from "../../../shared/api/client";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { FallingPetals } from "../components/FallingPetals";
import { LeaderboardTypeToggle } from "../components/LeaderboardTypeToggle";
import { LeaderboardHero } from "../components/LeaderboardHero";
import { ContributorsPodium } from "../components/ContributorsPodium";
import { ProjectsPodium } from "../components/ProjectsPodium";
import { FiltersSection } from "../components/FiltersSection";
import { ContributorsTable } from "../components/ContributorsTable";
import { ProjectsTable } from "../components/ProjectsTable";
import { LeaderboardStyles } from "../components/LeaderboardStyles";
import { ContributorsPodiumSkeleton } from "../components/ContributorsPodiumSkeleton";
import { ContributorsTableSkeleton } from "../components/ContributorsTableSkeleton";

export function LeaderboardPage() {
  const { theme } = useTheme();
  const [activeFilter, setActiveFilter] = useState<FilterType>("overall");
  const [leaderboardType, setLeaderboardType] =
    useState<LeaderboardType>("contributors");
  const [showEcosystemDropdown, setShowEcosystemDropdown] = useState(false);
  const [selectedEcosystem, setSelectedEcosystem] = useState({
    label: "All Ecosystems",
    value: "all",
  });
  const [petals, setPetals] = useState<Petal[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);
  const [leaderboardData, setLeaderboardData] = useState<LeaderData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);

  // Fetch leaderboard data
  useEffect(() => {
    const fetchLeaderboard = async () => {
      if (leaderboardType === "contributors") {
        setIsLoading(true);
        setOffset(0); // Reset offset when switching types
        try {
          const data = await getLeaderboard(
            10,
            0,
            selectedEcosystem.value !== "all"
              ? selectedEcosystem.value
              : undefined,
          );
          // Transform API data to match LeaderData type
          const transformedData: LeaderData[] = data.map((item) => ({
            rank: item.rank,
            rank_tier: item.rank_tier,
            rank_tier_name: item.rank_tier_name,
            username: item.username,
            avatar:
              item.avatar || `https://github.com/${item.username}.png?size=200`,
            user_id: item.user_id || "",
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
          console.error("Failed to fetch leaderboard:", err);
          setLeaderboardData([]);
          setIsLoading(false); // Set loading to false to show empty state instead of skeleton
        }
      } else {
        // For projects, we don't fetch from API, so set loading to false
        setIsLoading(false);
      }
    };

    fetchLeaderboard();
  }, [leaderboardType, activeFilter, selectedEcosystem.value]);

  // Load more leaderboard data
  const loadMore = async () => {
    if (isLoadingMore || !hasMore) return;

    setIsLoadingMore(true);
    try {
      const nextOffset = offset + 10;
      const data = await getLeaderboard(
        10,
        nextOffset,
        selectedEcosystem.value !== "all" ? selectedEcosystem.value : undefined,
      );

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
        avatar:
          item.avatar || `https://github.com/${item.username}.png?size=200`,
        user_id: item.user_id || "",
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
      console.error("Failed to load more leaderboard:", err);
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
    ...Array(Math.max(0, 3 - leaderboardData.length))
      .fill(null)
      .map((_, i) => ({
        rank: leaderboardData.length + i + 1,
        username: "-",
        avatar: "👤",
        score: 0,
        trend: "same" as const,
        trendValue: 0,
        contributions: 0,
        ecosystems: [],
      })),
  ].slice(0, 3) as LeaderData[];

  const projectTopThree = projectsData.slice(0, 3);

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
        {leaderboardType === "contributors" && isLoading && (
          <ContributorsPodiumSkeleton />
        )}
        {leaderboardType === "contributors" &&
          !isLoading &&
          leaderboardData.length > 0 && (
            <ContributorsPodium
              topThree={contributorTopThree}
              isLoaded={isLoaded}
              actualCount={leaderboardData.length}
            />
          )}
        {leaderboardType === "contributors" &&
          !isLoading &&
          leaderboardData.length === 0 && (
            <div
              className={`text-center py-8 transition-colors ${
                theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
              }`}
            >
              No contributors yet. Be the first to contribute!
            </div>
          )}

        {/* Top 3 Podium - Projects */}
        {leaderboardType === "projects" && (
          <ProjectsPodium topThree={projectTopThree} isLoaded={isLoaded} />
        )}
      </LeaderboardHero>

      {/* Filters Section */}
      <FiltersSection
        activeFilter={activeFilter}
        onFilterChange={setActiveFilter}
        selectedEcosystem={selectedEcosystem}
        onEcosystemChange={(ecosystem) => {
          setSelectedEcosystem(ecosystem);
        }}
        showDropdown={showEcosystemDropdown}
        onToggleDropdown={() =>
          setShowEcosystemDropdown(!showEcosystemDropdown)
        }
        isLoaded={isLoaded}
      />

      {/* Leaderboard Table - Contributors */}
      {leaderboardType === "contributors" && (
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
                  window.location.href = `/dashboard?tab=profile&user=${identifier}`;
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
                      "View All"
                    )}
                  </button>
                </div>
              )}
            </>
          )}
        </>
      )}

      {/* Leaderboard Table - Projects */}
      {leaderboardType === "projects" && (
        <ProjectsTable
          data={projectsData}
          activeFilter={activeFilter}
          isLoaded={isLoaded}
        />
      )}

      {/* CSS Animations */}
      <LeaderboardStyles />
    </div>
  );
}
