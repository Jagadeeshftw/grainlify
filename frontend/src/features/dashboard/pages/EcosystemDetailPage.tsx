import React, { useState, useEffect } from 'react';
import { ChevronRight, ExternalLink, Users, FolderGit2, AlertCircle, GitPullRequest } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { ProjectCard, Project } from '../components/ProjectCard';
import { SearchWithFilter } from '../components/SearchWithFilter';
import { getPublicProjects, getEcosystemDetail, type EcosystemDetail } from '../../../shared/api/client';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';

const formatNumber = (num: number): string => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
};

const getProjectIcon = (githubFullName: string): string => {
  const [owner] = githubFullName.split('/');
  // Use higher‑resolution owner avatar so cards look crisp
  return `https://github.com/${owner}.png?size=200`;
};

const getProjectColor = (name: string): string => {
  const colors = [
    'from-blue-500 to-cyan-500',
    'from-purple-500 to-pink-500',
    'from-green-500 to-emerald-500',
    'from-red-500 to-pink-500',
    'from-orange-500 to-red-500',
    'from-gray-600 to-gray-800',
    'from-green-600 to-green-800',
    'from-cyan-500 to-blue-600',
  ];
  const hash = name.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
};

interface EcosystemDetailPageProps {
  ecosystemId: string;
  ecosystemName: string;
  /** From list page (GET /ecosystems) so detail page shows same icon/description when detail fetch is pending or empty */
  initialDescription?: string | null;
  initialLogoUrl?: string | null;
  onBack: () => void;
  onProjectClick?: (id: string) => void;
}

export function EcosystemDetailPage({ ecosystemId, ecosystemName, initialDescription, initialLogoUrl, onBack, onProjectClick }: EcosystemDetailPageProps) {
  const { theme } = useTheme();
  const [activeTab, setActiveTab] = useState<'overview' | 'projects' | 'community'>('overview');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedLanguages, setSelectedLanguages] = useState<string[]>([]);
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);
  const [ecosystemProjects, setEcosystemProjects] = useState<Project[]>([]);
  const [isLoadingProjects, setIsLoadingProjects] = useState(true);
  const [ecosystemDetail, setEcosystemDetail] = useState<EcosystemDetail | null>(null);
  const [isLoadingDetail, setIsLoadingDetail] = useState(true);

  const fetchEcosystemDetail = React.useCallback(async () => {
    if (!ecosystemId) {
      setEcosystemDetail(null);
      setIsLoadingDetail(false);
      return;
    }
    setIsLoadingDetail(true);
    try {
      const detail = await getEcosystemDetail(ecosystemId);
      setEcosystemDetail(detail);
    } catch {
      setEcosystemDetail(null);
    } finally {
      setIsLoadingDetail(false);
    }
  }, [ecosystemId]);

  useEffect(() => {
    fetchEcosystemDetail();
  }, [fetchEcosystemDetail]);

  // Refetch when user returns to this tab (e.g. after editing in admin) so logo/description stay in sync
  useEffect(() => {
    const handleVisibility = () => {
      if (document.visibilityState === 'visible' && ecosystemId) {
        fetchEcosystemDetail();
      }
    };
    document.addEventListener('visibilitychange', handleVisibility);
    return () => document.removeEventListener('visibilitychange', handleVisibility);
  }, [ecosystemId, fetchEcosystemDetail]);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      setIsLoadingProjects(true);
      try {
        const res = await getPublicProjects({ ecosystem: ecosystemName, limit: 100 });
        if (cancelled || !res?.projects) return;
        const mapped: Project[] = (res.projects as Array<{
          id: string;
          github_full_name: string;
          language: string | null;
          tags: string[];
          category: string | null;
          stars_count: number;
          forks_count: number;
          contributors_count: number;
          open_issues_count: number;
          open_prs_count: number;
          ecosystem_name: string | null;
          description?: string | null;
        }>).filter((p) => (p.github_full_name.split('/')[1] || '') !== '.github').map((p) => {
          const repoName = p.github_full_name.split('/')[1] || p.github_full_name;
          return {
            id: p.id,
            name: repoName,
            icon: getProjectIcon(p.github_full_name),
            stars: formatNumber(p.stars_count || 0),
            forks: formatNumber(p.forks_count || 0),
            contributors: p.contributors_count || 0,
            openIssues: p.open_issues_count || 0,
            prs: p.open_prs_count || 0,
            description: (p.description && p.description.trim()) || `${repoName} project`,
            tags: Array.isArray(p.tags) ? p.tags.slice(0, 4) : [],
            color: getProjectColor(repoName),
          };
        });
        setEcosystemProjects(mapped);
      } catch {
        if (!cancelled) setEcosystemProjects([]);
      } finally {
        if (!cancelled) setIsLoadingProjects(false);
      }
    };
    load();
    return () => { cancelled = true; };
  }, [ecosystemName]);

  // Prefer API detail when loaded; use hardcoded fallbacks only when detail is null (loading/error)
  const detail = ecosystemDetail;
  const hasDetail = detail != null;

  const normalizeLinks = (raw: unknown): Array<{ label: string; url: string }> => {
    if (!Array.isArray(raw) || raw.length === 0) return [];
    return raw.map((l) => ({
      label: (l && typeof l === 'object' && 'label' in l && typeof (l as { label?: unknown }).label === 'string') ? (l as { label: string }).label : '',
      url: (l && typeof l === 'object' && 'url' in l && typeof (l as { url?: unknown }).url === 'string') ? (l as { url: string }).url : '',
    })).filter((l) => l.label.trim() || l.url.trim());
  };
  const normalizeKeyAreas = (raw: unknown): Array<{ title: string; description: string }> => {
    if (!Array.isArray(raw) || raw.length === 0) return [];
    return raw.map((k) => ({
      title: (k && typeof k === 'object' && 'title' in k && typeof (k as { title?: unknown }).title === 'string') ? (k as { title: string }).title : '',
      description: (k && typeof k === 'object' && 'description' in k && typeof (k as { description?: unknown }).description === 'string') ? (k as { description: string }).description : '',
    })).filter((k) => k.title.trim() || k.description.trim());
  };
  const normalizeTechnologies = (raw: unknown): string[] => {
    if (!Array.isArray(raw) || raw.length === 0) return [];
    return raw.map((t) => (typeof t === 'string' ? t.trim() : '')).filter(Boolean);
  };

  const apiLinks = normalizeLinks(detail?.links);
  const apiKeyAreas = normalizeKeyAreas(detail?.key_areas);
  const apiTechnologies = normalizeTechnologies(detail?.technologies);

  // Prefer API (detail fetch) for logo/description; fall back to list data (initialDescription/initialLogoUrl) so same icon/description as list page
  const apiDescription = hasDetail && detail?.description != null ? String(detail.description).trim() : '';
  const apiLogoUrl = hasDetail && detail?.logo_url != null && String(detail.logo_url).trim() !== '' ? String(detail.logo_url).trim() : null;
  const displayDescription = apiDescription || (initialDescription != null && initialDescription !== '' ? String(initialDescription).trim() : '') || (hasDetail ? '' : 'Projects building decentralized protocols, tooling, and infrastructure.');
  const displayLogoUrl = apiLogoUrl || (initialLogoUrl != null && initialLogoUrl !== '' ? String(initialLogoUrl).trim() : null);

  const ecosystemData = {
    name: (hasDetail && detail?.name) ? detail.name : ecosystemName,
    logo: displayLogoUrl ? undefined : ecosystemName.charAt(0).toUpperCase(),
    logoUrl: displayLogoUrl,
    description: displayDescription,
    languages: [] as { name: string; percentage: number }[],
    links: hasDetail && apiLinks.length > 0
      ? apiLinks
      : hasDetail
        ? []
        : [
            { label: 'Official Website', url: detail?.website_url || 'web3.ecosystem.example', icon: 'website' },
            { label: 'Discord Community', url: 'discord.gg', icon: 'discord' },
            { label: 'Twitter', url: 'twitter.com', icon: 'twitter' },
          ].map(({ label, url }) => ({ label, url })),
    // Use detail API counts when available; when detail returns 0 but we have projects (from Projects tab), use aggregated counts so Overview matches what user sees
    stats: (() => {
      const fromDetail = hasDetail && detail != null;
      const detailProjects = fromDetail ? Number(detail?.project_count) || 0 : 0;
      const detailContributors = fromDetail ? Number(detail?.contributors_count) || 0 : 0;
      const detailIssues = fromDetail ? Number(detail?.open_issues_count) || 0 : 0;
      const detailPrs = fromDetail ? Number(detail?.open_prs_count) || 0 : 0;
      const fromProjects = {
        projects: ecosystemProjects.length,
        contributors: ecosystemProjects.reduce((s, p) => s + (p.contributors || 0), 0),
        issues: ecosystemProjects.reduce((s, p) => s + p.openIssues, 0),
        prs: ecosystemProjects.reduce((s, p) => s + (p.prs || 0), 0),
      };
      const useProjectsFallback = fromProjects.projects > 0 && detailProjects === 0;
      return {
        activeContributors: { value: useProjectsFallback ? fromProjects.contributors : (fromDetail ? detailContributors : fromProjects.contributors), change: '' },
        activeProjects: { value: useProjectsFallback ? fromProjects.projects : (fromDetail ? detailProjects : fromProjects.projects), change: '' },
        availableIssues: { value: useProjectsFallback ? fromProjects.issues : (fromDetail ? detailIssues : fromProjects.issues), change: '' },
        mergedPullRequests: { value: useProjectsFallback ? fromProjects.prs : (fromDetail ? detailPrs : fromProjects.prs), change: '' },
      };
    })(),
    about: hasDetail
      ? (detail?.about?.trim() || '')
      : `The ${ecosystemName} ecosystem represents a paradigm shift towards decentralized applications, protocols, and infrastructure.`,
    keyAreas: hasDetail && apiKeyAreas.length > 0
      ? apiKeyAreas
      : hasDetail
        ? []
        : [
            { title: 'Blockchain Protocols', description: 'Core blockchain technologies and consensus mechanisms' },
            { title: 'DeFi (Decentralized Finance)', description: 'Financial applications built on blockchain' },
            { title: 'NFTs & Digital Assets', description: 'Tokenization and digital ownership' },
            { title: `${ecosystemName} Infrastructure`, description: 'Wallets, nodes, and developer tools' },
            { title: 'DAOs', description: 'Decentralized autonomous organizations' },
          ],
    technologies: hasDetail && apiTechnologies.length > 0
      ? apiTechnologies
      : hasDetail
        ? []
        : ['TypeScript for smart contract development and tooling', 'Rust for high-performance blockchain infrastructure', 'Solidity for Ethereum smart contracts', 'JavaScript/TypeScript for dApp frontends'],
  };

  const filteredProjects = ecosystemProjects.filter(
    (project) =>
      project.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      project.description.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const isDark = theme === 'dark';

  return (
    <div className="h-full overflow-y-auto px-4 md:px-0">
      {/* Breadcrumb Navigation */}
      <div className="mb-4 md:mb-6 flex items-center gap-1.5 md:gap-2 ml-0 md:ml-12 overflow-x-auto scrollbar-hide">
        <button
          onClick={onBack}
          className={`text-[12px] md:text-[14px] font-semibold transition-colors touch-manipulation whitespace-nowrap flex-shrink-0 ${
            isDark 
              ? 'text-[#d4d4d4] hover:text-[#c9983a] active:text-[#c9983a]' 
              : 'text-[#7a6b5a] hover:text-[#a67c2a] active:text-[#a67c2a]'
          }`}
        >
          Ecosystems
        </button>
        <ChevronRight className={`w-3 h-3 md:w-4 md:h-4 flex-shrink-0 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`} />
        <span className={`text-[12px] md:text-[14px] font-bold transition-colors whitespace-nowrap flex-shrink-0 ${
          isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
        }`}>
          {ecosystemName}
        </span>
        <ChevronRight className={`w-3 h-3 md:w-4 md:h-4 flex-shrink-0 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`} />
        <span className={`text-[12px] md:text-[14px] font-semibold transition-colors whitespace-nowrap flex-shrink-0 ${
          isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'
        }`}>
          Overview
        </span>
      </div>

      <div className="flex flex-col lg:flex-row gap-4 md:gap-6">
        {/* Left Sidebar - Ecosystem Info */}
        <div className="flex-[1] flex-shrink-0 space-y-4 md:space-y-6">
          {/* Ecosystem Header */}
          <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
            <div className="flex items-center gap-3 md:gap-4 mb-3 md:mb-4">
              <div className={`w-12 h-12 md:w-16 md:h-16 rounded-full flex items-center justify-center flex-shrink-0 overflow-hidden ${ecosystemData.logoUrl ? 'bg-white' : 'bg-gradient-to-br from-[#c9983a] to-[#d4af37]'}`}>
                {ecosystemData.logoUrl ? (
                  <img src={ecosystemData.logoUrl} alt="" className="w-full h-full object-contain p-0.5" />
                ) : (
                  <span className="text-[18px] md:text-[24px] font-bold text-white">{ecosystemData.logo}</span>
                )}
              </div>
              <div className="flex-1 min-w-0">
                <h1 className={`text-[16px] md:text-[20px] font-bold transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  {ecosystemData.name} Ecosystem
                </h1>
                <div className="flex items-center gap-3 md:gap-4 mt-1">
                  <div className="flex items-center gap-1.5 md:gap-2">
                    <Users className={`w-3 h-3 md:w-3.5 md:h-3.5 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`} />
                    <span className={`text-[11px] md:text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`}>
                      {isLoadingDetail ? '—' : ecosystemData.stats.activeContributors.value}
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5 md:gap-2">
                    <FolderGit2 className={`w-3 h-3 md:w-3.5 md:h-3.5 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`} />
                    <span className={`text-[11px] md:text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`}>
                      {isLoadingDetail ? '—' : ecosystemData.stats.activeProjects.value}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Description */}
          <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
            <h2 className={`text-[14px] md:text-[16px] font-bold mb-2 md:mb-3 transition-colors ${
              isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Description
            </h2>
            <p className={`text-[12px] md:text-[13px] leading-relaxed transition-colors ${
              isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              {ecosystemData.description}
            </p>
          </div>

          {/* Languages - only show when configured (optional for future) */}
          {ecosystemData.languages.length > 0 && (
            <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
              <h2 className={`text-[14px] md:text-[16px] font-bold mb-2 md:mb-3 transition-colors ${
                isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>
                Languages
              </h2>
              <div className="flex flex-wrap gap-2 md:gap-3">
                {ecosystemData.languages.map((lang, idx) => (
                  <div
                    key={idx}
                    className="px-2.5 md:px-3 py-1 md:py-1.5 rounded-[6px] md:rounded-[8px] backdrop-blur-[20px] border border-white/25 bg-white/[0.08]"
                  >
                    <span className="text-[11px] md:text-[12px] font-semibold text-[#c9983a]">{lang.name}</span>
                    <span className={`ml-1.5 md:ml-2 text-[10px] md:text-[11px] ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                      {lang.percentage}%
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Links */}
          <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
            <h2 className={`text-[14px] md:text-[16px] font-bold mb-3 md:mb-4 transition-colors ${
              isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Links
            </h2>
            <div className="space-y-2 md:space-y-3">
              {ecosystemData.links.map((link, idx) => (
                <a
                  key={idx}
                  href={/^https?:\/\//i.test(link.url) ? link.url : `https://${link.url}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center justify-between p-2.5 md:p-3 rounded-[10px] md:rounded-[12px] backdrop-blur-[20px] border border-white/25 bg-white/[0.08] hover:bg-white/[0.15] active:bg-white/[0.2] transition-all group touch-manipulation min-h-[44px]"
                >
                  <div className="flex-1 min-w-0">
                    <div className={`text-[12px] md:text-[13px] font-semibold transition-colors truncate ${
                      isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>
                      {link.label}
                    </div>
                    <div className={`text-[10px] md:text-[11px] transition-colors truncate ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      {link.url}
                    </div>
                  </div>
                  <ExternalLink className="w-4 h-4 text-[#c9983a] opacity-0 group-hover:opacity-100 group-active:opacity-100 transition-opacity flex-shrink-0 ml-2" />
                </a>
              ))}
            </div>
          </div>
        </div>

        {/* Main Content Area */}
        <div className="flex-[3] min-w-0">
          {/* Tabs */}
          <div className="flex items-center gap-2 md:gap-3 mb-4 md:mb-6 overflow-x-auto scrollbar-hide">
            <button
              onClick={() => setActiveTab('overview')}
              className={`px-4 md:px-5 py-2 md:py-2.5 rounded-[10px] md:rounded-[12px] text-[12px] md:text-[14px] font-semibold transition-all touch-manipulation whitespace-nowrap flex-shrink-0 min-h-[44px] ${
                activeTab === 'overview'
                  ? 'bg-[#c9983a] text-white shadow-lg'
                  : isDark
                    ? 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#d4d4d4] hover:bg-white/[0.15] active:bg-white/[0.18]'
                    : 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#7a6b5a] hover:bg-white/[0.15] active:bg-white/[0.2]'
              }`}
            >
              Overview
            </button>
            <button
              onClick={() => setActiveTab('projects')}
              className={`px-4 md:px-5 py-2 md:py-2.5 rounded-[10px] md:rounded-[12px] text-[12px] md:text-[14px] font-semibold transition-all touch-manipulation whitespace-nowrap flex-shrink-0 min-h-[44px] ${
                activeTab === 'projects'
                  ? 'bg-[#c9983a] text-white shadow-lg'
                  : isDark
                    ? 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#d4d4d4] hover:bg-white/[0.15] active:bg-white/[0.18]'
                    : 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#7a6b5a] hover:bg-white/[0.15] active:bg-white/[0.2]'
              }`}
            >
              Projects
            </button>
            <button
              onClick={() => setActiveTab('community')}
              className={`px-4 md:px-5 py-2 md:py-2.5 rounded-[10px] md:rounded-[12px] text-[12px] md:text-[14px] font-semibold transition-all touch-manipulation whitespace-nowrap flex-shrink-0 min-h-[44px] ${
                activeTab === 'community'
                  ? 'bg-[#c9983a] text-white shadow-lg'
                  : isDark
                    ? 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#d4d4d4] hover:bg-white/[0.15] active:bg-white/[0.18]'
                    : 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#7a6b5a] hover:bg-white/[0.15] active:bg-white/[0.2]'
              }`}
            >
              Community
            </button>
          </div>

          {activeTab === 'overview' && (
            <div className="space-y-4 md:space-y-6">
              {/* Stats Grid */}
              <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 md:gap-4">
                <div className="backdrop-blur-[40px] rounded-[12px] md:rounded-[16px] border bg-white/[0.12] border-white/20 p-3 md:p-5">
                  <div className="flex flex-col md:flex-row md:items-center gap-1.5 md:gap-2 mb-1.5 md:mb-2">
                    <Users className={`w-3.5 h-3.5 md:w-4 md:h-4 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[9px] md:text-[11px] font-bold uppercase tracking-wide leading-tight ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Active Contributors
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[20px] md:text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {isLoadingDetail ? '—' : ecosystemData.stats.activeContributors.value}
                    </span>
                  </div>
                </div>

                <div className="backdrop-blur-[40px] rounded-[12px] md:rounded-[16px] border bg-white/[0.12] border-white/20 p-3 md:p-5">
                  <div className="flex flex-col md:flex-row md:items-center gap-1.5 md:gap-2 mb-1.5 md:mb-2">
                    <FolderGit2 className={`w-3.5 h-3.5 md:w-4 md:h-4 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[9px] md:text-[11px] font-bold uppercase tracking-wide leading-tight ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Active Projects
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[20px] md:text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {isLoadingDetail ? '—' : ecosystemData.stats.activeProjects.value}
                    </span>
                  </div>
                </div>

                <div className="backdrop-blur-[40px] rounded-[12px] md:rounded-[16px] border bg-white/[0.12] border-white/20 p-3 md:p-5">
                  <div className="flex flex-col md:flex-row md:items-center gap-1.5 md:gap-2 mb-1.5 md:mb-2">
                    <AlertCircle className={`w-3.5 h-3.5 md:w-4 md:h-4 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[9px] md:text-[11px] font-bold uppercase tracking-wide leading-tight ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Available Issues
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[20px] md:text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {isLoadingDetail ? '—' : ecosystemData.stats.availableIssues.value}
                    </span>
                  </div>
                </div>

                <div className="backdrop-blur-[40px] rounded-[12px] md:rounded-[16px] border bg-white/[0.12] border-white/20 p-3 md:p-5">
                  <div className="flex flex-col md:flex-row md:items-center gap-1.5 md:gap-2 mb-1.5 md:mb-2">
                    <GitPullRequest className={`w-3.5 h-3.5 md:w-4 md:h-4 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[9px] md:text-[11px] font-bold uppercase tracking-wide leading-tight ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Open PRs
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[20px] md:text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {isLoadingDetail ? '—' : ecosystemData.stats.mergedPullRequests.value}
                    </span>
                  </div>
                </div>
              </div>

              {/* About Section */}
              <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
                <h2 className={`text-[16px] md:text-[18px] font-bold mb-3 md:mb-4 transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  About {ecosystemName}
                </h2>
                <p className={`text-[12px] md:text-[14px] leading-relaxed transition-colors ${
                  isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                  {ecosystemData.about}
                </p>
              </div>

              {/* Key Areas */}
              <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
                <h2 className={`text-[16px] md:text-[18px] font-bold mb-3 md:mb-4 transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  Key Areas
                </h2>
                <ul className="space-y-2 md:space-y-3">
                  {ecosystemData.keyAreas.map((area, idx) => (
                    <li key={idx} className="flex gap-2 md:gap-3">
                      <span className={`mt-0.5 md:mt-1 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>•</span>
                      <div className="flex-1 min-w-0">
                        <span className={`font-bold text-[12px] md:text-[14px] ${
                          isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>
                          {area.title}:
                        </span>{' '}
                        <span className={`text-[12px] md:text-[14px] ${
                          isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                        }`}>
                          {area.description}
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
              </div>

              {/* Technologies */}
              <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-4 md:p-6">
                <h2 className={`text-[16px] md:text-[18px] font-bold mb-3 md:mb-4 transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  Technologies
                </h2>
                <p className={`text-[11px] md:text-[13px] mb-2 md:mb-3 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                  Supported technologies for ecosystem projects:
                </p>
                <ul className="space-y-1.5 md:space-y-2">
                  {ecosystemData.technologies.map((tech, idx) => (
                    <li key={idx} className="flex gap-2 md:gap-3">
                      <span className={`mt-0.5 md:mt-1 flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>•</span>
                      <span className={`text-[12px] md:text-[14px] ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                        {tech}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )}

          {activeTab === 'projects' && (
            <div className="space-y-4 md:space-y-6">
              {/* Search and Filter */}
              <SearchWithFilter
                searchPlaceholder="Search"
                searchValue={searchQuery}
                onSearchChange={setSearchQuery}
                filterSections={[
                  {
                    title: 'Languages',
                    hasSearch: true,
                    options: [
                      { label: 'TypeScript', value: 'typescript' },
                      { label: 'JavaScript', value: 'javascript' },
                      { label: 'Python', value: 'python' },
                      { label: 'Rust', value: 'rust' },
                    ],
                    selectedValues: selectedLanguages,
                    onToggle: (value) => {
                      setSelectedLanguages(prev =>
                        prev.includes(value)
                          ? prev.filter(v => v !== value)
                          : [...prev, value]
                      );
                    },
                  },
                  {
                    title: 'Categories',
                    hasSearch: false,
                    options: [
                      { label: 'Frontend', value: 'frontend' },
                      { label: 'Backend', value: 'backend' },
                      { label: 'Full Stack', value: 'fullstack' },
                      { label: 'DevOps', value: 'devops' },
                    ],
                    selectedValues: selectedCategories,
                    onToggle: (value) => {
                      setSelectedCategories(prev =>
                        prev.includes(value)
                          ? prev.filter(v => v !== value)
                          : [...prev, value]
                      );
                    },
                  },
                ]}
                onReset={() => {
                  setSearchQuery('');
                  setSelectedLanguages([]);
                  setSelectedCategories([]);
                }}
              />

              {/* Projects Grid */}
              {isLoadingProjects ? (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-5">
                  {[1, 2, 3, 4].map((i) => (
                    <div key={i} className={`rounded-[18px] border p-5 ${isDark ? 'bg-white/[0.08] border-white/15' : 'bg-white/[0.15] border-white/25'}`}>
                      <SkeletonLoader className="h-11 w-11 rounded-[12px] mb-4" />
                      <SkeletonLoader className="h-4 w-3/4 mb-2" />
                      <SkeletonLoader className="h-3 w-full mb-1" />
                      <SkeletonLoader className="h-3 w-5/6 mb-4" />
                      <div className="flex gap-2 mb-4">
                        <SkeletonLoader className="h-4 w-12" />
                        <SkeletonLoader className="h-4 w-12" />
                      </div>
                      <div className="flex gap-2">
                        <SkeletonLoader className="h-7 w-16 rounded-[8px]" />
                        <SkeletonLoader className="h-7 w-20 rounded-[8px]" />
                      </div>
                    </div>
                  ))}
                </div>
              ) : filteredProjects.length === 0 ? (
                <div className={`rounded-[16px] border p-8 text-center ${isDark ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]' : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'}`}>
                  <FolderGit2 className={`w-10 h-10 mx-auto mb-3 ${isDark ? 'text-[#b8a898]' : 'text-[#8a7b6a]'}`} />
                  <p className="text-[14px] font-semibold">No projects in {ecosystemName} yet</p>
                  <p className="text-[12px] mt-1">Projects added under this ecosystem will appear here.</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-5">
                  {filteredProjects.map((project) => (
                    <ProjectCard key={project.id} project={project} onClick={onProjectClick} />
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === 'community' && (
            <div className="backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border bg-white/[0.12] border-white/20 p-6 md:p-8 text-center">
              <Users className={`w-10 h-10 md:w-12 md:h-12 mx-auto mb-3 md:mb-4 ${
                isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`} />
              <p className={`text-[12px] md:text-[14px] ${
                isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
                Community view coming soon
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
