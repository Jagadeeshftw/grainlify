import { useState, useEffect, useRef } from 'react';
import { Search, ChevronDown, Award, Briefcase, GitPullRequest, FolderGit2, Trophy, Github, Code, Globe, Sparkles, TrendingUp, Star, Users, GitFork, DollarSign, GitMerge, Calendar, ChevronRight, Filter, Circle, Eye, Crown, Link, ArrowLeft, Medal, Shield, LucideIcon } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { useAuth } from '../../../shared/contexts/AuthContext';
import { getUserProfile, getProjectsContributed, getProjectsLed, getProfileCalendar, getProfileActivity, getPublicProfile } from '../../../shared/api/client';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';
import { LanguageIcon } from '../../../shared/components/LanguageIcon';
import { ContributionHeatmap } from '../components/ContributionHeatmap';
import { RewardsChart } from '../components/RewardsChart';
import { ReferralLink } from '../components/ReferralLink';

/**
 * @notice Represents a badge minted to a contributor.
 * @dev Retrieved from the badge contract indexer/API.
 */
export interface Badge {
  id: string;
  tokenId: string;

  name: string;

  description: string;

  image: string;

  rarity:
    | "Common"
    | "Rare"
    | "Epic"
    | "Legendary";

  mintedAt: string;

  txHash?: string;

  ecosystem?: string;
}

/**
 * Design contract for the proposed ProfilePage skill endorsement UI.
 * See: design/specs/skill-endorsement-ui.md
 */

interface ProfileData {
  contributions_count: number;
  languages: Array<{ language: string; contribution_count: number }>;
  ecosystems: Array<{ ecosystem_name: string; contribution_count: number }>;
  projects_contributed_to_count?: number;
  projects_led_count?: number;
  rewards_count?: number;
  bio?: string;
  website?: string;
  telegram?: string;
  linkedin?: string;
  whatsapp?: string;
  twitter?: string;
  discord?: string;
   kyc_verified?: boolean;
  rank: {
    position: number | null;
    tier: string;
    tier_name: string;
    tier_color: string;
  };
  farcaster?: string;
  badges?: Badge[];
}

interface Project {
  id: string;
  github_full_name: string;
  status: string;
  ecosystem_name?: string;
  language?: string;
  owner_avatar_url?: string;
  stars_count?: number;
  forks_count?: number;
  contributors_count?: number;
  merged_prs_count?: number;
  rewards_amount?: number;
}

interface ProfilePageProps {
  viewingUserId?: string | null;
  viewingUserLogin?: string | null;
  onBack?: () => void;
  onProjectClick?: (projectId: string) => void;
  onIssueClick?: (issueId: string, projectId: string) => void;
}

const mockBadges = [
  {
    id: "1",
    name: "Top Contributor",
    description: "Awarded for exceptional OSS contributions",
    image:
      "data:image/png;base64,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",
    rarity: "Legendary",
    mintedAt: "2026-01-15",
  },
  {
    id: "2",
    name: "100 PR Club",
    description: "Merged over 100 pull requests",
    image:
      "https://images.unsplash.com/photo-1642104704074-907c0698cbd9?w=400",
    rarity: "Epic",
    mintedAt: "2026-02-01",
  },
  {
    id: "3",
    name: "Hackathon Winner",
    description: "Won an ecosystem hackathon",
    image:
      "https://images.unsplash.com/photo-1622630998477-20aa696ecb05?w=400",
    rarity: "Rare",
    mintedAt: "2026-03-12",
  },
  {
    id: "4",
    name: "Core Maintainer",
    description: "Maintained a major ecosystem project",
    image:
      "https://images.unsplash.com/photo-1639322537228-f710d846310a?w=400",
    rarity: "Legendary",
    mintedAt: "2026-04-08",
  },
  {
    id: "5",
    name: "Community Champion",
    description: "Outstanding community engagement",
    image:
      "https://images.unsplash.com/photo-1640161704729-cbe966a08476?w=400",
    rarity: "Epic",
    mintedAt: "2026-05-19",
  },
];

// const mockBadges = [];

export function ProfilePage({ viewingUserId, viewingUserLogin, onBack, onProjectClick, onIssueClick }: ProfilePageProps) {
  const { theme } = useTheme();
  const { user, userRole } = useAuth();
  const [profileData, setProfileData] = useState<ProfileData | null>(null);
  const [viewingUser, setViewingUser] = useState<{ login: string; avatar_url?: string } | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [contributionCalendar, setContributionCalendar] = useState<Array<{ date: string; count: number; level: number }>>([]);
  const [contributionActivity, setContributionActivity] = useState<Array<{
    type: 'pull_request' | 'issue';
    id: string;
    number: number;
    title: string;
    url: string;
    state?: string;
    date: string;
    month_year: string;
    project_name: string;
    project_id: string;
  }>>([]);
  const [isLoadingProfile, setIsLoadingProfile] = useState(true);
  const [isLoadingProjects, setIsLoadingProjects] = useState(true);
  const [isLoadingCalendar, setIsLoadingCalendar] = useState(true);
  const [isLoadingActivity, setIsLoadingActivity] = useState(true);
  const [selectedMonth, setSelectedMonth] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedMonths, setExpandedMonths] = useState<{ [key: string]: boolean }>({});
  const [contributorModalOpen, setContributorModalOpen] = useState(false);
  const [leadModalOpen, setLeadModalOpen] = useState(false);
  const [showAllProjects, setShowAllProjects] = useState(false);
  const [showAllBadges, setShowAllBadges] = useState(false);
  const [projectsLed, setProjectsLed] = useState<Project[]>([]);
  const [isLoadingProjectsLed, setIsLoadingProjectsLed] = useState(true);

  // Ref to avoid applying stale fetch results when the viewed user changes mid-request
  const viewingRef = useRef({ viewingUserId, viewingUserLogin });
  viewingRef.current = { viewingUserId, viewingUserLogin };

  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  const isSameView = (uid: string | null | undefined, login: string | null | undefined) =>
    (uid === viewingRef.current.viewingUserId && login === viewingRef.current.viewingUserLogin) ||
    (uid == null && login == null && viewingRef.current.viewingUserId == null && viewingRef.current.viewingUserLogin == null);

  // Fetch profile data for the viewed user (or own profile when viewingUserId/viewingUserLogin are null)
  useEffect(() => {
    const isViewingOther = !!(viewingUserId || viewingUserLogin);
    const requestedUserId = viewingUserId;
    const requestedLogin = viewingUserLogin;
    if (isViewingOther) {
      setProfileData(null);
      setViewingUser(null);
    } else {
      // Viewing own profile: clear viewingUser so name/avatar use user?.github, not last viewed user
      setViewingUser(null);
    }
    const fetchProfile = async () => {
      setIsLoadingProfile(true);
      try {
        let data;
        if (isViewingOther) {
          data = await getPublicProfile(requestedUserId || undefined, requestedLogin || undefined);
          if (!isSameView(requestedUserId, requestedLogin)) return;
          setViewingUser({
            login: data.login,
            avatar_url: data.avatar_url || `https://github.com/${data.login}.png?size=200`
          });
        } else {
          data = await getUserProfile();
          if (!isSameView(requestedUserId, requestedLogin)) return;
        }
        setProfileData(data);
      } catch (error) {
        if (isSameView(requestedUserId, requestedLogin)) console.error('Failed to fetch profile:', error);
      } finally {
        if (isSameView(requestedUserId, requestedLogin)) setIsLoadingProfile(false);
      }
    };
    fetchProfile();
  }, [viewingUserId, viewingUserLogin]);

  // Fetch user's contributed projects (for viewed user or self)
  useEffect(() => {
    const requestedUserId = viewingUserId;
    const requestedLogin = viewingUserLogin;
    if (requestedUserId || requestedLogin) setProjects([]);
    const fetchProjects = async () => {
      setIsLoadingProjects(true);
      try {
        const data = await getProjectsContributed(requestedUserId || undefined, requestedLogin || undefined);
        if (!isSameView(requestedUserId, requestedLogin)) return;
        const contributedProjects = data.map((p: any) => ({
          id: p.id,
          github_full_name: p.github_full_name,
          status: p.status,
          ecosystem_name: p.ecosystem_name,
          language: p.language,
          owner_avatar_url: p.owner_avatar_url,
          stars_count: 0,
          forks_count: 0,
          contributors_count: 0,
        }));
        setProjects(contributedProjects);
      } catch (error) {
        if (isSameView(requestedUserId, requestedLogin)) console.error('Failed to fetch projects:', error);
      } finally {
        if (isSameView(requestedUserId, requestedLogin)) setIsLoadingProjects(false);
      }
    };
    fetchProjects();
  }, [viewingUserId, viewingUserLogin]);

  // Fetch projects led (for viewed user or self)
  useEffect(() => {
    const requestedUserId = viewingUserId;
    const requestedLogin = viewingUserLogin;
    setProjectsLed([]);
    const fetchLed = async () => {
      setIsLoadingProjectsLed(true);
      try {
        const data = await getProjectsLed(requestedUserId || undefined, requestedLogin || undefined);
        if (!isSameView(requestedUserId, requestedLogin)) return;
        setProjectsLed(data.map((p: any) => ({
          id: p.id,
          github_full_name: p.github_full_name,
          status: p.status,
          ecosystem_name: p.ecosystem_name,
          language: p.language,
          owner_avatar_url: p.owner_avatar_url,
          stars_count: 0,
          forks_count: 0,
          contributors_count: 0,
        })));
      } catch (error) {
        if (isSameView(requestedUserId, requestedLogin)) console.error('Failed to fetch projects led:', error);
      } finally {
        if (isSameView(requestedUserId, requestedLogin)) setIsLoadingProjectsLed(false);
      }
    };
    fetchLed();
  }, [viewingUserId, viewingUserLogin]);

  // Fetch contribution calendar (for viewed user or self)
  useEffect(() => {
    const requestedUserId = viewingUserId;
    const requestedLogin = viewingUserLogin;
    if (requestedUserId || requestedLogin) setContributionCalendar([]);
    const fetchCalendar = async () => {
      setIsLoadingCalendar(true);
      try {
        const data = await getProfileCalendar(requestedUserId || undefined, requestedLogin || undefined);
        if (!isSameView(requestedUserId, requestedLogin)) return;
        setContributionCalendar(data.calendar || []);
      } catch (error) {
        if (isSameView(requestedUserId, requestedLogin)) console.error('Failed to fetch calendar:', error);
      } finally {
        if (isSameView(requestedUserId, requestedLogin)) setIsLoadingCalendar(false);
      }
    };
    fetchCalendar();
  }, [viewingUserId, viewingUserLogin]);

  // Fetch contribution activity
  useEffect(() => {
    const requestedUserId = viewingUserId;
    const requestedLogin = viewingUserLogin;
    if (requestedUserId || requestedLogin) setContributionActivity([]);
    const fetchActivity = async () => {
      setIsLoadingActivity(true);
      try {
        const data = await getProfileActivity(100, 0, requestedUserId || undefined, requestedLogin || undefined);
        if (!isSameView(requestedUserId, requestedLogin)) return;
        setContributionActivity(data.activities || []);
        const monthsSet = new Set<string>();
        data.activities?.forEach((activity: any) => {
          if (activity.month_year) monthsSet.add(activity.month_year);
        });
        const monthsObj: { [key: string]: boolean } = {};
        Array.from(monthsSet).forEach((month, idx) => {
          monthsObj[month] = idx === 0;
        });
        setExpandedMonths(monthsObj);
      } catch (error) {
        if (isSameView(requestedUserId, requestedLogin)) console.error('Failed to fetch activity:', error);
      } finally {
        if (isSameView(requestedUserId, requestedLogin)) setIsLoadingActivity(false);
      }
    };
    fetchActivity();
  }, [viewingUserId, viewingUserLogin]);

  const toggleMonth = (month: string) => {
    setExpandedMonths(prev => ({
      ...prev,
      [month]: !prev[month],
    }));
  };

  // Get rank tier icon
  const getRankIcon = (tierName: string) => {
    const iconClass = "w-5 h-5 text-white drop-shadow-md";
    switch (tierName.toLowerCase()) {
      case 'conqueror':
        return <Crown className={iconClass} />;
      case 'ace':
        return <Trophy className={iconClass} />;
      case 'crown':
        return <Medal className={iconClass} />;
      case 'diamond':
        return <Sparkles className={iconClass} />;
      case 'gold':
        return <Award className={iconClass} />;
      case 'silver':
        return <Circle className={iconClass} />;
      case 'bronze':
        return <Eye className={iconClass} />;
      default:
        return <Award className={iconClass} />;
    }
  };

  // Calculate activity level (1-3) based on contribution count
  const getActivityLevel = (count: number, maxCount: number): number => {
    if (maxCount === 0) return 0;
    if (count >= maxCount * 0.67) return 3;
    if (count >= maxCount * 0.33) return 2;
    return 1;
  };

  // Get real languages data from profileData
  const activeLanguages = profileData?.languages?.slice(0, 3).map((lang) => {
    const maxCount = Math.max(...(profileData.languages?.map(l => l.contribution_count) || [0]));
    return {
      name: lang.language,
      contribution_count: lang.contribution_count,
      activityLevel: getActivityLevel(lang.contribution_count, maxCount),
    };
  }) || [];

  // Get real ecosystems data from profileData
  const activeEcosystems = profileData?.ecosystems?.slice(0, 2).map((eco) => {
    const maxCount = Math.max(...(profileData.ecosystems?.map(e => e.contribution_count) || [0]));
    return {
      name: eco.ecosystem_name,
      contribution_count: eco.contribution_count,
      activityLevel: getActivityLevel(eco.contribution_count, maxCount),
    };
  }) || [];

  // Group contribution activity by month, filtering to only show open issues
  const contributionsByMonth: { [key: string]: any[] } = {};
  contributionActivity.forEach((activity) => {
    // Only include issues if they are open, or include all PRs
    if (activity.type === 'issue' && activity.state !== 'open') {
      return; // Skip closed issues
    }

    const month = activity.month_year || 'Unknown';
    if (!contributionsByMonth[month]) {
      contributionsByMonth[month] = [];
    }
    contributionsByMonth[month].push({
      id: activity.id,
      type: activity.type,
      number: activity.number,
      badgeColor: activity.type === 'issue' ? '#c9983a' : '#d4af37',
      title: activity.title,
      project: activity.project_name,
      project_id: activity.project_id,
      date: new Date(activity.date).toLocaleDateString('en-US', { day: 'numeric', month: 'short' }),
      url: activity.url,
    });
  });

  // Empty rewards data (no rewards yet)
  const rewardsData: Array<{ name: string; value: number; color: string; amount: number }> = [];
  const totalRewards = 0;

  return (
    <div className="space-y-6">
      {/* Back Button (only when viewing another user's profile) */}
      {onBack && (viewingUserId || viewingUserLogin) && (
        <button
          onClick={onBack}
          className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] hover:bg-white/[0.2] transition-all ${theme === 'dark'
              ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#d4c5b0]'
              : 'bg-white/[0.15] border-white/25 text-[#2d2820]'
            }`}
        >
          <ArrowLeft className="w-4 h-4" />
          Back to Leaderboard
        </button>
      )}

      {/* Profile Header */}
      <div className="backdrop-blur-[40px] bg-gradient-to-br from-white/[0.18] to-white/[0.10] rounded-[32px] border-2 border-white/30 shadow-[0_20px_60px_rgba(0,0,0,0.15),0_0_80px_rgba(201,152,58,0.08)] p-2 md:p-12 relative overflow-visible z-20 group">
        {/* Ambient Background Glow - Enhanced */}
        <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-gradient-to-br from-[#c9983a]/15 via-[#d4af37]/10 to-transparent rounded-full blur-3xl pointer-events-none group-hover:scale-110 transition-transform duration-1000" />
        <div className="absolute bottom-0 left-0 w-[400px] h-[400px] bg-gradient-to-tr from-[#d4af37]/12 to-transparent rounded-full blur-3xl pointer-events-none group-hover:scale-110 transition-transform duration-1000" />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[300px] bg-gradient-to-r from-[#c9983a]/5 via-transparent to-[#d4af37]/5 rounded-full blur-3xl pointer-events-none" />

        <div className="relative flex flex-col md:flex-row items-center md:items-start justify-between gap-10">
          {/* Left Section - Profile Info */}
          <div className="flex flex-col items-start gap-7">
            {/* Avatar with Enhanced Effects */}
            <div className="relative group/avatar">
              {isLoadingProfile ? (
                <>
                  <SkeletonLoader variant="circle" width="128px" height="128px" className="border-[6px] border-white/40" />
                  <SkeletonLoader variant="circle" width="48px" height="48px" className="absolute -bottom-3 -right-3" />
                </>
              ) : (
                <>
                  <div className="absolute inset-0 bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/25 rounded-full blur-2xl group-hover/avatar:blur-3xl transition-all duration-700 animate-pulse" />
                  <div className="absolute inset-0 bg-gradient-to-br from-[#ffd700]/20 to-[#c9983a]/10 rounded-full blur-xl" />
                  {(viewingUser?.avatar_url || user?.github?.avatar_url) ? (
                    <img
                      src={viewingUser?.avatar_url || user?.github?.avatar_url}
                      alt={viewingUser?.login || user?.github?.login}
                      className="relative w-32 h-32 rounded-full border-[6px] border-white/40 shadow-[0_12px_40px_rgba(0,0,0,0.25),inset_0_2px_8px_rgba(255,255,255,0.3)] flex-shrink-0 group-hover/avatar:scale-105 transition-transform duration-500 object-cover"
                      onError={(e) => {
                        // Fallback to GitHub avatar if image fails to load
                        const target = e.target as HTMLImageElement;
                        const login = viewingUser?.login || user?.github?.login;
                        if (login) {
                          target.src = `https://github.com/${login}.png?size=200`;
                        }
                      }}
                    />
                  ) : (
                    <div className="relative w-32 h-32 rounded-full bg-gradient-to-br from-purple-500 via-pink-500 to-purple-600 border-[6px] border-white/40 shadow-[0_12px_40px_rgba(0,0,0,0.25),inset_0_2px_8px_rgba(255,255,255,0.3)] flex-shrink-0 group-hover/avatar:scale-105 transition-transform duration-500" />
                  )}
                  <div className="absolute -bottom-3 -right-3 w-12 h-12 rounded-full backdrop-blur-[25px] bg-gradient-to-br from-[#ffd700] via-[#c9983a] to-[#b8873a] border-[4px] border-white/50 shadow-[0_6px_20px_rgba(201,152,58,0.5),0_0_20px_rgba(255,215,0,0.3)] flex items-center justify-center group-hover/avatar:rotate-12 transition-transform duration-500">
                    <Award className="w-6 h-6 text-white drop-shadow-[0_2px_4px_rgba(0,0,0,0.3)]" />
                  </div>
                </>
              )}
            </div>

            {/* User Details */}
            <div className="flex-1 pt-1">
              {/* Username with Glow */}
              {isLoadingProfile ? (
                <SkeletonLoader variant="text" width="200px" height="42px" className="mb-3" />
              ) : (
                <h1 className={`text-[26px] md:text-[42px] font-black mb-4 tracking-tight transition-colors ${theme === 'dark'
                    ? 'text-[#f5f5f5]'
                    : 'bg-gradient-to-r from-[#1a1410] via-[#2d2820] to-[#4a3f2f] bg-clip-text text-transparent drop-shadow-[0_2px_4px_rgba(0,0,0,0.1)]'
                  }`}>
                  {viewingUser?.login || user?.github?.login || 'Loading...'}
                </h1>
              )}

              {/* Bio and Website */}
              {/* {!isLoadingProfile && (profileData?.bio || profileData?.website) && (
                <div className="mb-4 space-y-3">
                  {profileData.bio && (
                    <p className={`text-[15px] leading-relaxed transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`}>
                      {profileData.bio}
                    </p>
                  )}
                  {profileData.website && (
                    <div className="flex items-center gap-2">
                      <Link className="w-4 h-4 text-[#c9983a]" />
                      <a
                        href={profileData.website.startsWith('http') ? profileData.website : `https://${profileData.website}`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className={`text-[14px] font-medium hover:text-[#c9983a] transition-colors underline decoration-[#c9983a]/30 hover:decoration-[#c9983a]/60 ${theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
                          }`}
                      >
                        {profileData.website.replace(/^https?:\/\//, '').replace(/^www\./, '')}
                      </a>
                    </div>
                  )}
                </div>
              )} */}

              {/* Social Media Links - Show all icons, dimmed if no link */}
              {!isLoadingProfile && (
                <div className="flex items-center justify-center md:justify-start gap-1 md:gap-3 flex-wrap mb-4">
                  {/* GitHub - always enabled */}
                  <a
                    href={`https://github.com/${viewingUser?.login || user?.github?.login || ''}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                    title="GitHub"
                  >
                    <Github className="w-4 h-4 text-[#c9983a]" />
                  </a>

                  {/* KYC badge - always shown, enabled only when verified */}
                  <div
                    className={
                      profileData?.kyc_verified
                        ? "w-8 h-8 rounded-full bg-gradient-to-br from-[#4ade80]/30 to-[#16a34a]/30 border-2 border-[#22c55e]/60 flex items-center justify-center shadow-[0_4px_12px_rgba(34,197,94,0.5)]"
                        : `w-8 h-8 rounded-full border-2 flex items-center justify-center ${
                            theme === 'dark'
                              ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-60'
                              : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-70'
                          }`
                    }
                    title={profileData?.kyc_verified ? "KYC verified" : "KYC not verified"}
                  >
                    <Shield
                      className={`w-4 h-4 ${
                        profileData?.kyc_verified
                          ? 'text-[#16a34a]'
                          : theme === 'dark'
                            ? 'text-[#9ca3af]'
                            : 'text-[#6b7280]'
                      }`}
                    />
                  </div>
                  {/* Telegram */}
                  {profileData?.telegram ? (
                    <a
                      href={`https://t.me/${profileData.telegram.replace(/^@/, '')}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                      title="Telegram"
                    >
                      <svg className="w-4 h-4" fill="#c9983a" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z" />
                      </svg>
                    </a>
                  ) : (
                    <div className={`w-8 h-8 rounded-full border-2 flex items-center justify-center cursor-not-allowed ${theme === 'dark'
                        ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-40'
                        : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-60'
                      }`} title="Telegram">
                      <svg className={`w-4 h-4 ${theme === 'dark' ? 'fill-[#9ca3af]' : 'fill-[#6b7280]'}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z" />
                      </svg>
                    </div>
                  )}

                  {/* LinkedIn */}
                  {profileData?.linkedin ? (
                    <a
                      href={profileData.linkedin.startsWith('http') ? profileData.linkedin : `https://www.linkedin.com/in/${profileData.linkedin.replace(/^@/, '').replace(/^in\//, '')}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="w-8 h-8 mb-1 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                      title="LinkedIn"
                    >
                      <svg className="w-4 h-4" fill="#c9983a" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z" />
                      </svg>
                    </a>
                  ) : (
                    <div className={`w-8 h-8 mb-1 rounded-full border-2 flex items-center justify-center cursor-not-allowed ${theme === 'dark'
                        ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-40'
                        : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-60'
                      }`} title="LinkedIn">
                      <svg className={`w-4 h-4 ${theme === 'dark' ? 'fill-[#9ca3af]' : 'fill-[#6b7280]'}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z" />
                      </svg>
                    </div>
                  )}

                  {/* WhatsApp */}
                  {profileData?.whatsapp ? (
                    <a
                      href={`https://wa.me/${profileData.whatsapp.replace(/[^0-9]/g, '')}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                      title="WhatsApp"
                    >
                      <svg className="w-4 h-4" fill="#c9983a" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 01-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 01-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 012.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0012.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 005.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 00-3.48-8.413Z" />
                      </svg>
                    </a>
                  ) : (
                    <div className={`w-8 h-8 rounded-full border-2 flex items-center justify-center cursor-not-allowed ${theme === 'dark'
                        ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-40'
                        : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-60'
                      }`} title="WhatsApp">
                      <svg className={`w-4 h-4 ${theme === 'dark' ? 'fill-[#9ca3af]' : 'fill-[#6b7280]'}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 01-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 01-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 012.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0012.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 005.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 00-3.48-8.413Z" />
                      </svg>
                    </div>
                  )}

                  {/* Twitter */}
                  {profileData?.twitter ? (
                    <a
                      href={`https://twitter.com/${profileData.twitter.replace(/^@/, '')}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                      title="Twitter"
                    >
                      <svg className="w-4 h-4" fill="#c9983a" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M23.953 4.57a10 10 0 01-2.825.775 4.958 4.958 0 002.163-2.723c-.951.555-2.005.959-3.127 1.184a4.92 4.92 0 00-8.384 4.482C7.69 8.095 4.067 6.13 1.64 3.162a4.822 4.822 0 00-.666 2.475c0 1.71.87 3.213 2.188 4.096a4.904 4.904 0 01-2.228-.616v.06a4.923 4.923 0 003.946 4.827 4.996 4.996 0 01-2.212.085 4.936 4.936 0 004.604 3.417 9.867 9.867 0 01-6.102 2.105c-.39 0-.779-.023-1.17-.067a13.995 13.995 0 007.557 2.209c9.053 0 13.998-7.496 13.998-13.985 0-.21 0-.42-.015-.63A9.935 9.935 0 0024 4.59z" />
                      </svg>
                    </a>
                  ) : (
                    <div className={`w-8 h-8 rounded-full border-2 flex items-center justify-center cursor-not-allowed ${theme === 'dark'
                        ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-40'
                        : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-60'
                      }`} title="Twitter">
                      <svg className={`w-4 h-4 ${theme === 'dark' ? 'fill-[#9ca3af]' : 'fill-[#6b7280]'}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M23.953 4.57a10 10 0 01-2.825.775 4.958 4.958 0 002.163-2.723c-.951.555-2.005.959-3.127 1.184a4.92 4.92 0 00-8.384 4.482C7.69 8.095 4.067 6.13 1.64 3.162a4.822 4.822 0 00-.666 2.475c0 1.71.87 3.213 2.188 4.096a4.904 4.904 0 01-2.228-.616v.06a4.923 4.923 0 003.946 4.827 4.996 4.996 0 01-2.212.085 4.936 4.936 0 004.604 3.417 9.867 9.867 0 01-6.102 2.105c-.39 0-.779-.023-1.17-.067a13.995 13.995 0 007.557 2.209c9.053 0 13.998-7.496 13.998-13.985 0-.21 0-.42-.015-.63A9.935 9.935 0 0024 4.59z" />
                      </svg>
                    </div>
                  )}

                  {/* Discord */}
                  {profileData?.discord ? (
                    <a
                      href={`https://discord.com/users/${profileData.discord.replace(/^@/, '')}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                      title="Discord"
                    >
                      <svg className="w-4 h-4" fill="#c9983a" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.958a.076.076 0 0 0-.041-.039 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.039c.36.663.772 1.33 1.225 1.958a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                      </svg>
                    </a>
                  ) : (
                    <div className={`w-8 h-8 rounded-full border-2 flex items-center justify-center cursor-not-allowed ${theme === 'dark'
                        ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-40'
                        : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-60'
                      }`} title="Discord">
                      <svg className={`w-4 h-4 ${theme === 'dark' ? 'fill-[#9ca3af]' : 'fill-[#6b7280]'}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.958a.076.076 0 0 0-.041-.039 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.039c.36.663.772 1.33 1.225 1.958a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                      </svg>
                    </div>
                  )}
                  {/* Farcaster */}
                  {profileData?.farcaster ? (
                    <a
                      href={`https://warpcast.com/${profileData.farcaster.replace(/^@/, '')}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center hover:scale-110 hover:shadow-[0_4px_12px_rgba(201,152,58,0.4)] transition-all duration-300"
                      title="Farcaster"
                    >
                      <svg
                        className="w-4 h-4"
                        fill="#c9983a"
                        viewBox="0 0 256 256"
                        xmlns="http://www.w3.org/2000/svg"
                      >
                        <path d="M52 32h152v192h-32V96h-88v128H52V32zm32 32v32h88V64H84z" />
                      </svg>
                    </a>
                  ) : (  <div
                      className={`w-8 h-8 rounded-full border-2 flex items-center justify-center cursor-not-allowed ${
                        theme === 'dark'
                          ? 'bg-gradient-to-br from-gray-400/20 to-gray-500/10 border-gray-400/30 opacity-40'
                          : 'bg-gradient-to-br from-gray-300/40 to-gray-400/30 border-gray-400/50 opacity-60'
                      }`}
                      title="Farcaster"
                    >
                      <svg
                        className={`w-4 h-4 ${
                          theme === 'dark'
                            ? 'fill-[#9ca3af]'
                            : 'fill-[#6b7280]'
                        }`}
                        viewBox="0 0 256 256"
                        xmlns="http://www.w3.org/2000/svg"
                      >
                        <path d="M52 32h152v192h-32V96h-88v128H52V32zm32 32v32h88V64H84z" />
                      </svg>
                    </div>
                  )}
                </div>
              )}

              {/* Stats Grid - Inline Premium Style */}
              <div className="space-y-4 mb-6">
                {/* Row 1 - Contributions & Rewards */}
                <div className="flex items-center gap-8">
                  <div className="flex items-center gap-3 group/stat">
                    <div className="w-12 h-12 rounded-[14px] bg-gradient-to-br from-[#c9983a]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-2 border-[#c9983a]/50 flex items-center justify-center shadow-[0_4px_16px_rgba(201,152,58,0.25),inset_0_1px_2px_rgba(255,255,255,0.2)] group-hover/stat:scale-110 group-hover/stat:shadow-[0_6px_24px_rgba(201,152,58,0.4)] transition-all duration-300">
                      <GitPullRequest className="w-6 h-6 text-[#c9983a] drop-shadow-sm" />
                    </div>
                    <div>
                      {isLoadingProfile ? (
                        <>
                          <SkeletonLoader variant="text" width="60px" height="28px" className="mb-1" />
                          <SkeletonLoader variant="text" width="100px" height="12px" />
                        </>
                      ) : (
                        <>
                          <div className={`text-[28px] font-black leading-none mb-1 drop-shadow-sm transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                            }`}>
                            {profileData?.contributions_count || 0}
                          </div>
                          <div className={`text-[12px] font-bold uppercase tracking-wider transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                            }`}>Contributions</div>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-3 group/stat">
                    <div className="w-12 h-12 rounded-[14px] bg-gradient-to-br from-[#c9983a]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-2 border-[#c9983a]/50 flex items-center justify-center shadow-[0_4px_16px_rgba(201,152,58,0.25),inset_0_1px_2px_rgba(255,255,255,0.2)] group-hover/stat:scale-110 group-hover/stat:shadow-[0_6px_24px_rgba(201,152,58,0.4)] transition-all duration-300">
                      <Trophy className="w-6 h-6 text-[#c9983a] drop-shadow-sm" />
                    </div>
                    <div>
                      {isLoadingProfile ? (
                        <>
                          <SkeletonLoader variant="text" width="40px" height="28px" className="mb-1" />
                          <SkeletonLoader variant="text" width="80px" height="12px" />
                        </>
                      ) : (
                        <>
                          <div className={`text-[28px] font-black leading-none mb-1 drop-shadow-sm transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                            }`}>{profileData?.rewards_count || 0}</div>
                          <div className={`text-[12px] font-bold uppercase tracking-wider transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                            }`}>Rewards</div>
                        </>
                      )}
                    </div>
                  </div>
                </div>

                {/* Row 2 - Projects Stats (clickable to open project list modals) */}
                <div className="flex items-center gap-8">
                  <div className="relative flex items-center gap-3 group/stat">
                    <div className="w-9 h-9 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center shadow-[0_3px_12px_rgba(201,152,58,0.25),inset_0_1px_2px_rgba(255,255,255,0.2)] group-hover/stat:scale-110 group-hover/stat:shadow-[0_5px_20px_rgba(201,152,58,0.4)] transition-all duration-300">
                      <Users className="w-5 h-5 text-[#c9983a] drop-shadow-sm" />
                    </div>
                    {isLoadingProfile ? (
                      <SkeletonLoader variant="text" width="180px" height="15px" />
                    ) : (
                      <button
                        type="button"
                        onClick={() => (profileData?.projects_contributed_to_count ?? 0) > 0 && setContributorModalOpen(true)}
                        disabled={(profileData?.projects_contributed_to_count ?? 0) === 0}
                        className={`text-[15px] font-medium transition-colors text-left hover:opacity-90 disabled:opacity-60 disabled:cursor-default ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          } ${(profileData?.projects_contributed_to_count ?? 0) > 0 ? 'cursor-pointer' : ''}`}
                      >
                        Contributor on <span className={`font-black text-[16px] transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                          }`}>{profileData?.projects_contributed_to_count || 0}</span> projects
                      </button>
                    )}
                    {/* Small popover for contributed projects */}
                    {contributorModalOpen && projects.length > 0 && (
                      <div
                        className={`absolute z-[200] top-full mt-2 left-0 w-[260px] rounded-[18px] border shadow-lg ${
                          theme === 'dark'
                            ? 'bg-[#3a3228] border-white/15'
                            : 'bg-[#e4d4c0] border-white/40'
                        }`}
                      >
                        <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
                          <div className="flex items-center gap-2">
                            <Users className="w-4 h-4 text-[#c9983a]" />
                            <span className={`text-[13px] font-semibold ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                              Projects contributed to
                            </span>
                          </div>
                          <button
                            type="button"
                            onClick={() => setContributorModalOpen(false)}
                            className="text-[13px] text-[#c9983a] hover:text-[#a67c2e]"
                          >
                            ×
                          </button>
                        </div>
                        <div className="max-h-[260px] overflow-y-auto p-2 space-y-1">
                          {projects.map((project) => {
                            const name = project.github_full_name.split('/')[1] || project.github_full_name;
                            return (
                              <button
                                key={project.id}
                                type="button"
                                onClick={() => {
                                  onProjectClick?.(project.id);
                                  setContributorModalOpen(false);
                                }}
                                className={`w-full flex items-center gap-3 px-3 py-2 rounded-[12px] text-left transition-all hover:bg-white/10 ${
                                  theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                }`}
                              >
                                {project.owner_avatar_url ? (
                                  <img
                                    src={project.owner_avatar_url}
                                    alt=""
                                    className="w-8 h-8 rounded-[10px] object-cover flex-shrink-0"
                                  />
                                ) : project.language ? (
                                  <LanguageIcon language={project.language} className="w-8 h-8 flex-shrink-0" />
                                ) : (
                                  <FolderGit2 className="w-6 h-6 flex-shrink-0 text-[#c9983a]" />
                                )}
                                <span className="text-[13px] font-medium truncate">{name}</span>
                              </button>
                            );
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-8">
                  <div className="relative flex items-center gap-3 group/stat">
                    <div className="w-9 h-9 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center shadow-[0_3px_12px_rgba(201,152,58,0.25),inset_0_1px_2px_rgba(255,255,255,0.2)] group-hover/stat:scale-110 transition-all duration-300">
                      <Star className="w-5 h-5 text-[#c9983a] fill-[#c9983a] drop-shadow-sm" />
                    </div>
                    {isLoadingProfile ? (
                      <SkeletonLoader variant="text" width="150px" height="15px" />
                    ) : (
                      <button
                        type="button"
                        onClick={() => (profileData?.projects_led_count ?? 0) > 0 && setLeadModalOpen(true)}
                        disabled={(profileData?.projects_led_count ?? 0) === 0}
                        className={`text-[15px] font-medium transition-colors text-left hover:opacity-90 disabled:opacity-60 disabled:cursor-default ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          } ${(profileData?.projects_led_count ?? 0) > 0 ? 'cursor-pointer' : ''}`}
                      >
                        Lead <span className={`font-black text-[16px] transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                          }`}>{profileData?.projects_led_count || 0}</span> projects
                      </button>
                    )}
                    {/* Small popover for led projects */}
                    {leadModalOpen && projectsLed.length > 0 && (
                      <div
                        className={`absolute z-[200] top-full mt-2 left-0 w-[260px] rounded-[18px] border shadow-lg ${
                          theme === 'dark'
                            ? 'bg-[#3a3228] border-white/15'
                            : 'bg-[#e4d4c0] border-white/40'
                        }`}
                      >
                        <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
                          <div className="flex items-center gap-2">
                            <Star className="w-4 h-4 text-[#c9983a] fill-[#c9983a]" />
                            <span className={`text-[13px] font-semibold ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                              Projects led
                            </span>
                          </div>
                          <button
                            type="button"
                            onClick={() => setLeadModalOpen(false)}
                            className="text-[13px] text-[#c9983a] hover:text-[#a67c2e]"
                          >
                            ×
                          </button>
                        </div>
                        <div className="max-h-[260px] overflow-y-auto p-2 space-y-1">
                          {projectsLed.map((project) => {
                            const name = project.github_full_name.split('/')[1] || project.github_full_name;
                            return (
                              <button
                                key={project.id}
                                type="button"
                                onClick={() => {
                                  onProjectClick?.(project.id);
                                  setLeadModalOpen(false);
                                }}
                                className={`w-full flex items-center gap-3 px-3 py-2 rounded-[12px] text-left transition-all hover:bg-white/10 ${
                                  theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                }`}
                              >
                                {project.owner_avatar_url ? (
                                  <img
                                    src={project.owner_avatar_url}
                                    alt=""
                                    className="w-8 h-8 rounded-[10px] object-cover flex-shrink-0"
                                  />
                                ) : project.language ? (
                                  <LanguageIcon language={project.language} className="w-8 h-8 flex-shrink-0" />
                                ) : (
                                  <FolderGit2 className="w-6 h-6 flex-shrink-0 text-[#c9983a]" />
                                )}
                                <span className="text-[13px] font-medium truncate">{name}</span>
                              </button>
                            );
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Right Section - Epic Rank Badge */}
          <div className="relative group/rank flex-shrink-0">
            {/* Outer Glow - Multi-layer */}
            <div className="absolute inset-0 rounded-[28px] blur-2xl group-hover/rank:blur-3xl transition-all duration-700 opacity-80 bg-gradient-to-br from-[#c9983a]/50 via-[#d4af37]/35 to-transparent" />
            <div className="absolute inset-0 rounded-[28px] blur-xl group-hover/rank:scale-110 transition-transform duration-700 bg-gradient-to-br from-[#ffd700]/30 to-transparent" />

            {/* Main Badge */}
            <div className="relative backdrop-blur-[40px] rounded-[28px] border-[3.5px] border-white/50 shadow-[0_15px_60px_rgba(201,152,58,0.5),inset_0_2px_4px_rgba(255,255,255,0.5),0_0_60px_rgba(255,215,0,0.2)] p-10 min-w-[200px] text-center group-hover/rank:scale-105 group-hover/rank:shadow-[0_20px_80px_rgba(201,152,58,0.6),inset_0_2px_4px_rgba(255,255,255,0.6)] transition-all duration-500 bg-gradient-to-br from-[#c9983a]/40 via-[#d4af37]/30 to-[#c9983a]/25">
              {/* Decorative Elements */}
              <div className="absolute top-4 left-4 w-4 h-4 rounded-full bg-white/50 shadow-[0_0_12px_rgba(255,255,255,0.8)] animate-pulse" />
              <div className="absolute top-4 right-4 w-3 h-3 rounded-full bg-[#c9983a]/70 shadow-[0_0_10px_rgba(201,152,58,0.9)]" />
              <div className="absolute bottom-4 left-1/2 -translate-x-1/2 w-3 h-3 rounded-full bg-white/40" />

              {/* Rank Number */}
              <div className="relative mb-3">
                {isLoadingProfile ? (
                  <SkeletonLoader variant="text" width="120px" height="64px" className="mx-auto" />
                ) : profileData?.rank?.position ? (
                  <div className="text-[64px] font-black bg-gradient-to-b from-[#1a1410] via-[#2d2820] to-[#c9983a] bg-clip-text text-transparent leading-none drop-shadow-[0_4px_12px_rgba(0,0,0,0.2)]" style={{ letterSpacing: '-0.02em' }}>
                    {profileData.rank.position}
                    <span className="text-[36px] align-super">
                      {profileData.rank.position === 1 ? 'st' :
                        profileData.rank.position === 2 ? 'nd' :
                          profileData.rank.position === 3 ? 'rd' : 'th'}
                    </span>
                  </div>
                ) : (
                  <div className="text-[48px] font-black text-gray-400 leading-none">
                    Unranked
                  </div>
                )}
              </div>

              {/* Divider */}
              {!isLoadingProfile && (
                <div className="h-[3px] w-20 mx-auto bg-gradient-to-r from-transparent via-[#c9983a]/80 to-transparent mb-4 rounded-full shadow-[0_2px_8px_rgba(201,152,58,0.4)]" />
              )}

              {/* Badge Label */}
              {isLoadingProfile ? (
                <SkeletonLoader variant="default" width="140px" height="36px" className="mx-auto rounded-[10px]" />
              ) : (
                <div className="inline-flex items-center gap-2 px-4 py-2 rounded-[10px] bg-white/[0.3] border-2 border-[#c9983a]/50 shadow-[0_3px_12px_rgba(201,152,58,0.3),inset_0_1px_2px_rgba(255,255,255,0.4)]">
                  {getRankIcon(profileData?.rank?.tier_name || 'Bronze')}
                  <span className="text-[13px] font-black text-[#c9983a] uppercase tracking-[0.15em]">
                    {profileData?.rank?.tier_name || 'Bronze'}
                  </span>
                </div>
              )}

              {/* Shine Effect */}
              <div className="absolute inset-0 bg-gradient-to-tr from-transparent via-white/15 to-transparent rounded-[28px] opacity-0 group-hover/rank:opacity-100 transition-opacity duration-700" />

              {/* Rotating Glow */}
              <div className="absolute inset-0 bg-gradient-to-r from-transparent via-[#ffd700]/10 to-transparent rounded-[28px] animate-pulse" />
            </div>
          </div>
        </div>
      </div>

      {/* Referral Link Section */}
      {!viewingUserId && !viewingUserLogin && <ReferralLink />}

      {/* Projects Led / Most */}
      <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 relative overflow-hidden group/projects">
        {/* Animated Background Glow */}
        <div className="absolute bottom-0 left-0 w-[400px] h-[400px] bg-gradient-to-tr from-[#c9983a]/8 to-transparent rounded-full blur-3xl pointer-events-none group-hover/projects:scale-125 transition-transform duration-1000" />

        <div className="relative flex items-center justify-between mb-6">
          <h2 className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Projects led / Most</h2>
          <button
            type="button"
            onClick={() => setShowAllProjects((prev) => !prev)}
            className="text-[13px] text-[#c9983a] hover:text-[#a67c2e] cursor-pointer font-medium transition-all hover:scale-105 hover:translate-x-1 duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={projects.length <= 3}
          >
            {showAllProjects ? 'Show less' : 'See all →'}
          </button>
        </div>

        <div className={`relative grid gap-5 ${showAllProjects ? 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4' : 'grid-cols-1 md:grid-cols-3'}`}>
          {isLoadingProjects ? (
            // Skeleton loaders for projects
            Array.from({ length: 3 }).map((_, idx) => (
              <div
                key={idx}
                className={`backdrop-blur-[20px] rounded-[16px] border p-5 ${theme === 'dark'
                    ? 'bg-white/[0.08] border-white/10'
                    : 'bg-white/[0.15] border-white/25'
                  }`}
              >
                <div className="flex items-center gap-3 mb-4">
                  <SkeletonLoader variant="default" width="48px" height="48px" className="rounded-[12px]" />
                  <SkeletonLoader variant="text" width="60%" height="16px" />
                </div>
                <div className="flex items-center gap-3 mb-4">
                  <SkeletonLoader variant="text" width="40px" height="13px" />
                  <SkeletonLoader variant="text" width="40px" height="13px" />
                  <SkeletonLoader variant="text" width="40px" height="13px" />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <SkeletonLoader variant="default" width="100%" height="60px" className="rounded-[10px]" />
                  <SkeletonLoader variant="default" width="100%" height="60px" className="rounded-[10px]" />
                </div>
              </div>
            ))
          ) : (showAllProjects ? projects : projects.slice(0, 3)).length > 0 ? (
            (showAllProjects ? projects : projects.slice(0, 3)).map((project, idx) => {
              const projectName = project.github_full_name.split('/')[1] || project.github_full_name;
              return (
                <div
                  key={project.id}
                  role="button"
                  tabIndex={0}
                  onClick={() => onProjectClick?.(project.id)}
                  onKeyDown={(e) => e.key === 'Enter' && onProjectClick?.(project.id)}
                  className={`backdrop-blur-[20px] rounded-[16px] border p-5 hover:scale-105 hover:shadow-[0_12px_36px_rgba(0,0,0,0.12)] transition-all duration-300 cursor-pointer group/project ${theme === 'dark'
                      ? 'bg-white/[0.08] border-white/10 hover:bg-white/[0.12] hover:border-white/15'
                      : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:border-white/40'
                    }`}
                  style={{
                    animationDelay: `${idx * 100}ms`,
                  }}
                >
                  {/* Project Header */}
                  <div className="flex items-center gap-3 mb-4">
                    <div className="w-12 h-12 rounded-[12px] bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center shadow-md overflow-hidden group-hover/project:scale-110 group-hover/project:rotate-6 transition-all duration-300">
                      {project.owner_avatar_url ? (
                        <img
                          src={project.owner_avatar_url}
                          alt={projectName}
                          className="w-full h-full object-cover"
                        />
                      ) : project.language ? (
                        <LanguageIcon language={project.language} className="w-8 h-8" />
                      ) : (
                        <FolderGit2 className="w-6 h-6 text-white" />
                      )}
                    </div>
                    <div className="flex-1">
                      <h3 className={`text-[16px] font-bold group-hover/project:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>{projectName}</h3>
                    </div>
                  </div>

                  {/* Project Metrics with Icons */}
                  <div className="flex items-center gap-3 mb-4 text-[13px]">
                    <div className={`flex items-center gap-1.5 group-hover/project:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`}>
                      <Star className="w-5 h-5" />
                      <span>{(project.stars_count || 0).toLocaleString()}</span>
                    </div>
                    <div className={`flex items-center gap-1.5 group-hover/project:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`}>
                      <Users className="w-5 h-5" />
                      <span>{(project.contributors_count || 0).toLocaleString()}</span>
                    </div>
                    <div className={`flex items-center gap-1.5 group-hover/project:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`}>
                      <GitFork className="w-5 h-5" />
                      <span>{(project.forks_count || 0).toLocaleString()}</span>
                    </div>
                  </div>

                  {/* Bottom Stats - Rewards and Merged PRs */}
                  <div className="grid grid-cols-2 gap-3">
                    <div className={`backdrop-blur-[15px] rounded-[10px] border p-3 group-hover/project:bg-white/[0.15] transition-all ${theme === 'dark' ? 'bg-white/[0.06] border-white/8' : 'bg-white/[0.1] border-white/20'
                      }`}>
                      <div className="flex items-center gap-2 mb-2">
                        <div className="w-7 h-7 rounded-full bg-[#c9983a]/20 flex items-center justify-center group-hover/project:scale-110 transition-transform">
                          <DollarSign className="w-4 h-4 text-[#c9983a]" />
                        </div>
                        <span className={`text-[10px] font-medium transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          }`}>Rewards</span>
                      </div>
                      <div className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>{project.rewards_amount || 0}</div>
                    </div>
                    <div className={`backdrop-blur-[15px] rounded-[10px] border p-3 group-hover/project:bg-white/[0.15] transition-all ${theme === 'dark' ? 'bg-white/[0.06] border-white/8' : 'bg-white/[0.1] border-white/20'
                      }`}>
                      <div className="flex items-center gap-2 mb-2">
                        <div className="w-7 h-7 rounded-full bg-[#c9983a]/20 flex items-center justify-center group-hover/project:scale-110 transition-transform">
                          <GitMerge className="w-4 h-4 text-[#c9983a]" />
                        </div>
                        <span className={`text-[10px] font-medium transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          }`}>Merged PRs</span>
                      </div>
                      <div className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>{project.merged_prs_count || 0}</div>
                    </div>
                  </div>
                </div>
              );
            })
          ) : (
            <div className={`col-span-3 text-center py-8 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
              <div className="text-center py-8">
                <FolderGit2 className={`w-12 h-12 mx-auto mb-3 ${theme === 'dark' ? 'text-gray-500' : 'text-gray-400'}`} />
                <p className={`text-[14px] font-medium mb-1 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>
                  No projects contributed yet
                </p>
                <p className={`text-[12px] transition-colors ${theme === 'dark' ? 'text-gray-500' : 'text-gray-400'
                  }`}>
                  Start contributing to projects to see them here
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Most active languages & ecosystems - Combined */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Most active languages */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6">
          <div className="flex items-center gap-2 mb-5">
            <Code className="w-5 h-5 text-[#c9983a]" />
            <h2 className={`text-[16px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>Most active languages</h2>
          </div>

          <div className="space-y-3">
            {isLoadingProfile ? (
              // Skeleton loaders for languages
              Array.from({ length: 3 }).map((_, idx) => (
                <div
                  key={idx}
                  className="backdrop-blur-[20px] bg-white/[0.15] rounded-[12px] border border-white/25 p-4"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <SkeletonLoader variant="circle" width="24px" height="24px" />
                      <SkeletonLoader variant="text" width="80px" height="15px" />
                    </div>
                    <div className="flex items-center gap-1.5">
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                    </div>
                  </div>
                </div>
              ))
            ) : activeLanguages.length > 0 ? (
              activeLanguages.map((language) => (
                <div
                  key={language.name}
                  className="backdrop-blur-[20px] bg-white/[0.15] rounded-[12px] border border-white/25 p-4 hover:bg-white/[0.2] transition-all group cursor-pointer"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <LanguageIcon language={language.name} className="w-6 h-6" />
                      <span className={`text-[15px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>{language.name}</span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      {Array.from({ length: 3 }).map((_, idx) => (
                        <div
                          key={idx}
                          className={`w-2.5 h-2.5 rounded-full transition-all ${idx < language.activityLevel
                              ? 'bg-[#c9983a] shadow-[0_0_8px_rgba(201,152,58,0.6)] group-hover:scale-125'
                              : 'bg-white/20'
                            }`}
                        />
                      ))}
                    </div>
                  </div>
                </div>
              ))
            ) : (
              <div className={`text-center py-4 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                No languages found
              </div>
            )}
          </div>
        </div>

        {/* Most active ecosystems */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6">
          <div className="flex items-center gap-2 mb-5">
            <Globe className="w-5 h-5 text-[#c9983a]" />
            <h2 className={`text-[16px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>Most active ecosystems</h2>
          </div>

          <div className="space-y-3">
            {isLoadingProfile ? (
              // Skeleton loaders for ecosystems
              Array.from({ length: 2 }).map((_, idx) => (
                <div
                  key={idx}
                  className="backdrop-blur-[20px] bg-white/[0.15] rounded-[12px] border border-white/25 p-4"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <SkeletonLoader variant="circle" width="24px" height="24px" />
                      <SkeletonLoader variant="text" width="100px" height="15px" />
                    </div>
                    <div className="flex items-center gap-1.5">
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                    </div>
                  </div>
                </div>
              ))
            ) : activeEcosystems.length > 0 ? (
              activeEcosystems.map((ecosystem) => (
                <div
                  key={ecosystem.name}
                  className="backdrop-blur-[20px] bg-white/[0.15] rounded-[12px] border border-white/25 p-4 hover:bg-white/[0.2] transition-all group cursor-pointer"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <Globe className="w-6 h-6 text-[#c9983a]" />
                      <span className={`text-[15px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>{ecosystem.name}</span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      {Array.from({ length: 3 }).map((_, idx) => (
                        <div
                          key={idx}
                          className={`w-2.5 h-2.5 rounded-full transition-all ${idx < ecosystem.activityLevel
                              ? 'bg-[#c9983a] shadow-[0_0_8px_rgba(201,152,58,0.6)] group-hover:scale-125'
                              : 'bg-white/20'
                            }`}
                        />
                      ))}
                    </div>
                  </div>
                </div>
              ))
            ) : (
              <div className={`text-center py-4 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                No ecosystems found
              </div>
            )}
          </div>
        </div>
      </div>

      {/* NFT Badge Gallery */}
      <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 relative overflow-hidden group/badges">

      {/* Background Glow */}
      <div className="absolute top-0 right-0 w-[300px] h-[300px] bg-gradient-to-bl from-[#c9983a]/10 to-transparent rounded-full blur-3xl pointer-events-none group-hover/badges:scale-125 transition-transform duration-1000" />

        <div className="relative flex items-center justify-between mb-6">
          <div>
            <h2
              className={`text-[20px] font-bold ${
                theme === 'dark'
                  ? 'text-[#f5f5f5]'
                  : 'text-[#2d2820]'
              }`}
            >
              NFT Badge Gallery
            </h2>

            <p
              className={`text-[13px] mt-1 ${
                theme === 'dark'
                  ? 'text-[#a1a1aa]'
                  : 'text-[#7a6b5a]'
              }`}
            >
              Minted achievements earned across ecosystems
            </p>
          </div>

          <div className="flex items-center gap-4">
            <div
              className="
              px-3 py-1
              rounded-full
              bg-[#c9983a]/15
              border border-[#c9983a]/30
              text-[#c9983a]
              text-[12px]
              font-semibold
            "
            >
              {mockBadges.length} Badges
            </div>

            <button
              type="button"
              onClick={() => setShowAllBadges((prev) => !prev)}
              disabled={mockBadges.length <= 3}
              className="
                text-[13px]
                text-[#c9983a]
                hover:text-[#a67c2e]
                cursor-pointer
                font-medium
                transition-all
                hover:scale-105
                hover:translate-x-1
                duration-200
                disabled:opacity-50
                disabled:cursor-not-allowed
              "
            >
              {showAllBadges ? 'Show less' : 'See all →'}
            </button>
          </div>
        </div>


        
        <div className="space-y-3">
            {isLoadingProfile ? (
              // Skeleton loaders for ecosystems
              Array.from({ length: 2 }).map((_, idx) => (
                <div
                  key={idx}
                  className="backdrop-blur-[20px] bg-white/[0.15] rounded-[12px] border border-white/25 p-4"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <SkeletonLoader variant="circle" width="24px" height="24px" />
                      <SkeletonLoader variant="text" width="100px" height="15px" />
                    </div>
                    <div className="flex items-center gap-1.5">
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                      <SkeletonLoader variant="circle" width="10px" height="10px" />
                    </div>
                  </div>
                </div>
              ))
            ) : mockBadges.length > 0 ? (
                <div
          className="
          grid
          grid-cols-1
          sm:grid-cols-2
          xl:grid-cols-3
          gap-5
        "
        >
          {(showAllBadges ? mockBadges : mockBadges.slice(0, 3)).map((badge) => (
            <div
              key={badge.id}
              className={`
              backdrop-blur-[20px]
              rounded-[18px]
              border
              overflow-hidden
              cursor-pointer
              transition-all
              duration-300
              hover:scale-[1.03]
              hover:-translate-y-1
              hover:shadow-[0_12px_36px_rgba(0,0,0,0.15)]
              ${
                theme === 'dark'
                  ? 'bg-white/[0.08] border-white/10'
                  : 'bg-white/[0.15] border-white/20'
              }
            `}
            >
              {/* NFT Image */}
              <div className="relative h-[180px] overflow-hidden">
                <img
                  src={badge.image}
                  alt={badge.name}
                  className="w-full h-full object-cover transition-transform duration-700 hover:scale-110"
                />

                <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />

                <div
                  className="
                  absolute
                  top-3
                  right-3
                  px-2
                  py-1
                  rounded-full
                  text-[11px]
                  font-semibold
                  bg-[#c9983a]/90
                  text-white
                "
                >
                  {badge.rarity}
                </div>
              </div>

              {/* Badge Details */}
              <div className="p-4">
                <h3
                  className={`font-bold text-[16px] mb-1 ${
                    theme === 'dark'
                      ? 'text-[#f5f5f5]'
                      : 'text-[#2d2820]'
                  }`}
                >
                  {badge.name}
                </h3>

                <p
                  className={`text-[13px] mb-4 ${
                    theme === 'dark'
                      ? 'text-[#d4d4d4]'
                      : 'text-[#7a6b5a]'
                  }`}
                >
                  {badge.description}
                </p>

                <div className="flex items-center justify-between">
                  <span
                    className={`text-[12px] ${
                      theme === 'dark'
                        ? 'text-[#a1a1aa]'
                        : 'text-[#8b7355]'
                    }`}
                  >
                    Minted
                  </span>

                  <span
                    className={`text-[12px] font-medium ${
                      theme === 'dark'
                        ? 'text-[#f5f5f5]'
                        : 'text-[#2d2820]'
                    }`}
                  >
                    {badge.mintedAt}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
            ) : (
              <div className={`text-center py-4 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                No badges found
              </div>
            )}
          </div>







      
      </div>

      {/* Rewards Distribution - New Responsive Component */}
      <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6 sm:p-8 relative overflow-hidden group/rewards">
        {/* Animated Background Glow */}
        <div className="absolute top-0 right-0 w-[300px] h-[300px] bg-gradient-to-br from-[#c9983a]/10 to-transparent rounded-full blur-3xl pointer-events-none group-hover/rewards:scale-125 transition-transform duration-1000" />
        
        <div className="relative">
          <RewardsChart 
            data={rewardsData} 
            totalRewards={totalRewards}
            isLoading={false}
          />
        </div>
      </div>

      {/* Contribution Heatmap - New Responsive Component */}
      <div className="backdrop-blur-[40px] bg-white/[0.18] rounded-xl sm:rounded-2xl border border-white/30 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-4 sm:p-6 lg:p-8">
        <ContributionHeatmap
          data={contributionCalendar}
          totalContributions={contributionCalendar.reduce((sum, day) => sum + day.count, 0)}
          isLoading={isLoadingCalendar}
        />
      </div>

      {/* Contributions Activity */}
      <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8">
        <h2 className={`text-[20px] font-bold mb-6 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Contributions Activity</h2>

        {/* Search and Filter */}
        <div className="flex items-center gap-3 mb-6">
          <div className="relative flex-1">
            <Search className={`absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`} />
            <input
              type="text"
              placeholder="Search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className={`w-full pl-12 pr-4 py-3 rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/40 transition-all text-[13px] ${theme === 'dark' ? 'text-[#f5f5f5] placeholder-[#d4d4d4]' : 'text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
            />
          </div>
        </div>

        {/* Activity List */}
        {isLoadingActivity ? (
          <div className="space-y-2">
            {Array.from({ length: 3 }).map((_, idx) => (
              <div key={idx} className="backdrop-blur-[20px] bg-white/[0.08] rounded-[12px] border border-white/20 p-5">
                <SkeletonLoader variant="text" width="150px" height="20px" className="mb-3" />
                <div className="space-y-2">
                  {Array.from({ length: 2 }).map((_, itemIdx) => (
                    <div key={itemIdx} className="flex items-center gap-4">
                      <SkeletonLoader variant="circle" width="32px" height="32px" />
                      <SkeletonLoader variant="text" width="60%" height="16px" />
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        ) : Object.keys(contributionsByMonth).length === 0 ? (
          <div className={`text-center py-12 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            <Calendar className="w-16 h-16 mx-auto mb-4 opacity-50" />
            <p className="text-[16px] font-medium">No contributions yet</p>
            <p className="text-[14px] mt-2">Start contributing to verified projects to see your activity here!</p>
          </div>
        ) : (
          <div className="space-y-2">
            {Object.entries(contributionsByMonth).map(([month, items]) => (
              <div key={month} className="backdrop-blur-[20px] bg-white/[0.08] rounded-[12px] border border-white/20 overflow-hidden">
                {/* Month Header with Calendar Icon */}
                <button
                  onClick={() => toggleMonth(month)}
                  className="w-full flex items-center gap-3 px-5 py-3.5 hover:bg-white/[0.05] transition-all group"
                >
                  <Calendar className={`w-4 h-4 group-hover:text-[#c9983a] transition-colors flex-shrink-0 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`} />
                  <span className={`text-[14px] font-semibold flex-1 text-left transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>{month}</span>
                  <ChevronRight
                    className={`w-4 h-4 transition-all duration-200 ${expandedMonths[month] ? 'rotate-90' : ''
                      } ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}
                  />
                </button>

                {/* Horizontal Divider */}
                {expandedMonths[month] && (
                  <div className="border-t border-white/15" />
                )}

                {/* Month Items */}
                {expandedMonths[month] && (
                  <div className="px-5 py-2">
                    {items.map((item, idx) => {
                      // Determine icon and styling based on type
                      let IconComponent: LucideIcon | null = Circle;
                      let iconBgColor = 'bg-[#c9983a]/50';
                      let labelPrefix = '';

                      if (item.type === 'pr') {
                        IconComponent = GitPullRequest;
                        iconBgColor = 'bg-[#d4af37]/50';
                        labelPrefix = '';
                      } else if (item.type === 'review') {
                        IconComponent = null; // No icon for reviews
                        iconBgColor = '';
                        labelPrefix = 'Review: ';
                      } else if (item.type === 'issue') {
                        IconComponent = Circle;
                        iconBgColor = 'bg-[#c9983a]/50';
                        labelPrefix = '';
                      }

                      return (
                        <div key={item.id} className="relative">
                          {/* Vertical Line on Left */}
                          {idx < items.length - 1 && (
                            <div className="absolute left-[20px] top-[36px] bottom-[-8px] w-[2px] bg-gradient-to-b from-white/25 to-white/8" />
                          )}

                          <div
                            onClick={() => {
                              if (item.type === 'issue' && onIssueClick) {
                                onIssueClick(item.id, item.project_id);
                              }
                            }}
                            className={`flex items-center gap-4 py-2.5 hover:bg-white/[0.08] -mx-2 px-2 rounded-lg transition-all group/item ${item.type === 'issue' ? 'cursor-pointer' : 'cursor-default'
                              }`}
                          >
                            {/* Icon + Number Badge (only for issues and PRs) */}
                            {item.type !== 'review' && IconComponent && (
                              <div className="relative z-10 flex items-center gap-2.5 flex-shrink-0">
                                {/* Icon Circle */}
                                <div className={`w-10 h-10 rounded-full ${iconBgColor} shadow-[0_4px_16px_rgba(0,0,0,0.3)] flex items-center justify-center group-hover/item:scale-110 group-hover/item:shadow-[0_5px_20px_rgba(0,0,0,0.4)] transition-all duration-200`}>
                                  <IconComponent
                                    className="w-5 h-5 text-white group-hover/item:scale-110 transition-transform"
                                    fill={item.type === 'issue' ? 'white' : 'none'}
                                    strokeWidth={item.type === 'issue' ? 0 : 3}
                                  />
                                </div>

                                {/* Number Badge */}
                                <div className={`px-3.5 py-1.5 rounded-[6px] ${iconBgColor} shadow-[0_3px_10px_rgba(0,0,0,0.25)]`}>
                                  <span className="text-[14px] font-bold text-white">
                                    {item.number}
                                  </span>
                                </div>
                              </div>
                            )}

                            {/* Review label without icon */}
                            {item.type === 'review' && (
                              <div className="relative z-10 flex-shrink-0 w-[120px]">
                                <span className={`text-[15px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                                  }`}>
                                  Review:
                                </span>
                              </div>
                            )}

                            {/* Content */}
                            <div className="flex-1 min-w-0">
                              <h4 className={`text-[15px] font-medium transition-colors ${theme === 'dark' ? 'text-[#f5f5f5] group-hover/item:text-[#d4d4d4]' : 'text-[#2d2820] group-hover/item:text-[#4a3f2f]'
                                }`}>
                                {labelPrefix}{item.title}
                              </h4>
                            </div>

                            {/* Date */}
                            <span className={`text-[13px] font-medium whitespace-nowrap flex-shrink-0 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                              }`}>
                              {item.date}
                            </span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
