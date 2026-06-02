import { useState, useEffect, useRef, useCallback } from "react";
import type { LeaderboardType, FilterType, Petal, LeaderData, ProjectData, TimePeriod, RoleFilter, EcosystemOption } from "../types";
import { getLeaderboard, getRecommendedProjects } from "../../../shared/api/client";
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

const POLL_INTERVAL_MS = 30_000; // 30s polling for real-time updates

export function LeaderboardPage() {
  const { theme } = useTheme();
  const [activeFilter, setActiveFilter] = useState<FilterType>("overall");
  const [leaderboardType, setLeaderboardType] = useState<LeaderboardType>("contributors");
  const [showEcosystemDropdown, setShowEcosystemDropdown] = useState(false);
  const [selectedEcosystem, setSelectedEcosystem] = useState<EcosystemOption>({
    label: "All Ecosystems",
    value: "all",
  });
  const [timePeriod, setTimePeriod] = useState<TimePeriod>("all-time");
  const [roleFilter, setRoleFilter] = useState<RoleFilter>("all");
  const [petals, setPetals] = useState<Petal[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);
  const [leaderboardData, setLeaderboardData] = useState<LeaderData[]>([]);
  const [projectsData, setProjectsData] = useState<ProjectData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingProjects, setIsLoadingProjects] = useState(true);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const prevDataRef = useRef<Map<string, number>>(new Map());
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const getProjectIcon = (githubFullName: string) => {
    const [owner] = githubFullName.split("/");
    return `https://github.com/${owner}.png?size=200`;
  };

  const computePreviousRanks = useCallback((data: LeaderData[]): LeaderData[] => {
    const prevMap = prevDataRef.current;
    if (prevMap.size === 0) {
      data.forEach((d) => prevMap.set(d.username, d.rank));
      return data;
    }
    const enriched = data.map((d) => {
      const prevRank = prevMap.get(d.username);
      return { ...d, previousRank: prevRank ?? d.rank };
    });
    data.forEach((d) => prevMap.set(d.username, d.rank));
    return enriched;
  }, []);

  const fetchLeaderboard = useCallback(async (isPoll = false) => {
    if (leaderboardType !== "contributors") return;
    if (!isPoll) {
      setIsLoading(true);
      setOffset(0);
    }
    setError(null);
    try {
      const data = await getLeaderboard(
        10,
        0,
        selectedEcosystem.value !== "all" ? selectedEcosystem.value : undefined,
      );
      const transformedData: LeaderData[] = data.map((item: { rank: number; rank_tier?: string; rank_tier_name?: string; username: string; avatar?: string; user_id?: string; score: number; trend: "up" | "down" | "same"; trendValue: number; contributions?: number; ecosystems?: string[] }) => ({
        rank: item.rank,
        rank_tier: item.rank_tier,
        rank_tier_name: item.rank_tier_name,
        username: item.username,
        avatar: item.avatar || `https://github.com/${item.username}.png?size=200`,
        user_id: item.user_id || "",
        score: item.score,
        trend: item.trend,
        trendValue: item.trendValue,
        contributions: item.contributions,
        ecosystems: item.ecosystems || [],
      }));
      const enriched = computePreviousRanks(transformedData);
      setLeaderboardData(enriched);
      setHasMore(data.length === 10);
      setLastUpdated(new Date());
    } catch (err) {
      console.error("Failed to fetch leaderboard:", err);
      if (!isPoll) {
        setLeaderboardData([]);
        setError("Failed to load leaderboard. Please try again.");
      }
    } finally {
      if (!isPoll) setIsLoading(false);
    }
  }, [leaderboardType, selectedEcosystem.value, computePreviousRanks]);

  const fetchProjects = useCallback(async () => {
    if (leaderboardType !== "projects") return;
    setIsLoadingProjects(true);
    try {
      const res = await getRecommendedProjects(50);
      const projects = res?.projects ?? [];
      const mapped: ProjectData[] = projects
        .filter((p: { github_full_name: string }) => (p.github_full_name.split("/")[1] || "") !== ".github")
        .map((p: { github_full_name: string; contributors_count?: number; open_issues_count?: number; ecosystem_name?: string }, idx: number) => {
          const repoName = p.github_full_name.split("/")[1] || p.github_full_name;
          const contributors = p.contributors_count ?? 0;
          const openIssues = p.open_issues_count ?? 0;
          const activity = openIssues > 10 ? "Very High" : openIssues > 5 ? "High" : openIssues > 2 ? "Medium" : "Low";
          return {
            rank: idx + 1,
            name: repoName,
            logo: getProjectIcon(p.github_full_name),
            score: contributors,
            trend: "same" as const,
            trendValue: 0,
            contributors,
            ecosystems: p.ecosystem_name ? [p.ecosystem_name] : [],
            activity,
          };
        });
      setProjectsData(mapped);
      setLastUpdated(new Date());
    } catch (err) {
      console.error("Failed to fetch projects:", err);
      setProjectsData([]);
    } finally {
      setIsLoadingProjects(false);
    }
  }, [leaderboardType]);

  useEffect(() => {
    fetchLeaderboard();
  }, [fetchLeaderboard]);

  useEffect(() => {
    fetchProjects();
  }, [fetchProjects]);

  // Polling for real-time updates
  useEffect(() => {
    if (leaderboardType !== "contributors") {
      if (pollRef.current) clearInterval(pollRef.current);
      return;
    }
    pollRef.current = setInterval(() => {
      fetchLeaderboard(true);
    }, POLL_INTERVAL_MS);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [leaderboardType, fetchLeaderboard]);

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
        return;
      }
      const transformedData: LeaderData[] = data.map((item: { rank: number; rank_tier?: string; rank_tier_name?: string; username: string; avatar?: string; user_id?: string; score: number; trend: "up" | "down" | "same"; trendValue: number; contributions?: number; ecosystems?: string[] }) => ({
        rank: item.rank,
        rank_tier: item.rank_tier,
        rank_tier_name: item.rank_tier_name,
        username: item.username,
        avatar: item.avatar || `https://github.com/${item.username}.png?size=200`,
        user_id: item.user_id || "",
        score: item.score,
        trend: item.trend,
        trendValue: item.trendValue,
        contributions: item.contributions,
        ecosystems: item.ecosystems || [],
      }));
      setLeaderboardData((prev) => [...prev, ...transformedData]);
      setOffset(nextOffset);
      setHasMore(data.length === 10);
    } catch (err) {
      console.error("Failed to load more leaderboard:", err);
      setHasMore(false);
    } finally {
      setIsLoadingMore(false);
    }
  };

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
    const interval = setInterval(generatePetals, 15000);
    return () => clearInterval(interval);
  }, []);

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

  const projectTopThree: ProjectData[] = [
    ...projectsData.slice(0, 3),
    ...Array(Math.max(0, 3 - projectsData.length))
      .fill(null)
      .map((_, i) => ({
        rank: projectsData.length + i + 1,
        name: "-",
        logo: "📦",
        score: 0,
        trend: "same" as const,
        trendValue: 0,
        contributors: 0,
        ecosystems: [] as string[],
        activity: "Low",
      })),
  ].slice(0, 3) as ProjectData[];

  return (
    <div className="space-y-6 relative">
      <FallingPetals petals={petals} />

      <LeaderboardTypeToggle
        leaderboardType={leaderboardType}
        onToggle={setLeaderboardType}
        isLoaded={isLoaded}
      />

      <LeaderboardHero leaderboardType={leaderboardType} isLoaded={isLoaded}>
        {leaderboardType === "contributors" && isLoading && (
          <ContributorsPodiumSkeleton />
        )}
        {leaderboardType === "contributors" && !isLoading && (
          leaderboardData.length > 0
            ? <ContributorsPodium topThree={contributorTopThree} isLoaded={isLoaded} actualCount={leaderboardData.length} />
            : (
              <div className={`text-center py-8 transition-colors ${theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                No contributors yet. Be the first to contribute!
              </div>
            )
        )}

        {leaderboardType === "projects" && isLoadingProjects && (
          <ContributorsPodiumSkeleton />
        )}
        {leaderboardType === "projects" && !isLoadingProjects && (
          projectsData.length > 0
            ? <ProjectsPodium topThree={projectTopThree} isLoaded={isLoaded} />
            : (
              <div className={`text-center py-8 transition-colors ${theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                No projects yet. Complete project setup to appear here.
              </div>
            )
        )}
      </LeaderboardHero>

      <FiltersSection
        activeFilter={activeFilter}
        onFilterChange={setActiveFilter}
        selectedEcosystem={selectedEcosystem}
        onEcosystemChange={(ecosystem) => setSelectedEcosystem(ecosystem)}
        showDropdown={showEcosystemDropdown}
        onToggleDropdown={() => setShowEcosystemDropdown(!showEcosystemDropdown)}
        isLoaded={isLoaded}
        timePeriod={timePeriod}
        onTimePeriodChange={setTimePeriod}
        roleFilter={roleFilter}
        onRoleFilterChange={setRoleFilter}
      />

      {/* Real-time update indicator */}
      {lastUpdated && leaderboardType === "contributors" && (
        <div
          className={`text-[11px] text-right px-2 transition-colors ${theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
          aria-live="polite"
          aria-atomic="true"
        >
          Last updated: {lastUpdated.toLocaleTimeString()}
          <span className="inline-block w-2 h-2 rounded-full bg-[#c9983a] ml-1.5 animate-pulse-slow" aria-hidden="true" />
        </div>
      )}

      {/* Error state */}
      {error && (
        <div
          className="backdrop-blur-[40px] bg-red-500/10 rounded-[20px] border border-red-500/30 p-6 text-center"
          role="alert"
        >
          <p className="text-red-600 font-semibold text-[14px]">{error}</p>
          <button
            onClick={() => fetchLeaderboard()}
            className="mt-3 px-4 py-2 rounded-[10px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white text-[12px] font-semibold shadow-md hover:shadow-lg transition-all"
          >
            Retry
          </button>
        </div>
      )}

      {/* Contributors section */}
      {leaderboardType === "contributors" && !error && (
        <div aria-live="polite" aria-atomic="false">
          {isLoading ? (
            <ContributorsTableSkeleton />
          ) : (
            <>
              <ContributorsTable
                data={leaderboardData}
                activeFilter={activeFilter}
                isLoaded={isLoaded}
                onUserClick={(username, userId) => {
                  const identifier = userId || username;
                  window.location.href = `/dashboard?tab=profile&user=${identifier}`;
                }}
              />
              {hasMore && (
                <div className="flex justify-center mt-6">
                  <button
                    onClick={loadMore}
                    disabled={isLoadingMore}
                    className={`px-6 py-3 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[14px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 focus-visible:outline-2 focus-visible:outline-white`}
                  >
                    {isLoadingMore ? (
                      <>
                        <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" role="status" aria-label="Loading" />
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
        </div>
      )}

      {/* Projects section */}
      {leaderboardType === "projects" && (
        <>
          {isLoadingProjects ? (
            <ContributorsTableSkeleton />
          ) : (
            <ProjectsTable
              data={projectsData}
              activeFilter={activeFilter}
              isLoaded={isLoaded}
            />
          )}
        </>
      )}

      <LeaderboardStyles />
    </div>
  );
}
