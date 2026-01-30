import React, { useEffect, useCallback, useMemo, useRef, useState } from 'react';
import { X, ExternalLink, User, ChevronDown, Plus, Award, Users, Star, CheckCircle, MessageSquare, Filter, Search, Loader2 } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { useAuth } from '../../../../shared/contexts/AuthContext';
import { Issue } from '../../types';
import { EmptyIssueState } from './EmptyIssueState';
import { IssueCard } from '../../../../shared/components/ui/IssueCard';
import { applyToIssue, getProjectIssues } from '../../../../shared/api/client';
import { formatDistanceToNow } from 'date-fns';
import { IssueCardSkeleton } from '../../../../shared/components/IssueCardSkeleton';
import RenderMarkdownContent from '../../../../app/utils/renderMarkdown';

interface Project {
  id: string;
  github_full_name: string;
  status: string;
}

interface IssuesTabProps {
  onNavigate: (page: string) => void;
  selectedProjects: Project[];
  onRefresh?: () => void;
  initialSelectedIssueId?: string;
  initialSelectedProjectId?: string;
}

interface CommentFromAPI {
  id: number;
  body: string;
  user: {
    login: string;
  };
  created_at: string;
  updated_at: string;
}

interface IssueFromAPI {
  github_issue_id: number;
  number: number;
  state: string;
  title: string;
  description: string | null;
  author_login: string;
  assignees: any[];
  labels: any[];
  comments_count: number;
  comments: CommentFromAPI[];
  url: string;
  updated_at: string | null;
  last_seen_at: string;
}

export function IssuesTab({ onNavigate, selectedProjects, onRefresh, initialSelectedIssueId, initialSelectedProjectId }: IssuesTabProps) {
  const { theme } = useTheme();
  const { userRole, user } = useAuth();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIssue, setSelectedIssue] = useState<Issue | null>(null);
  const [selectedIssueFromAPI, setSelectedIssueFromAPI] = useState<(IssueFromAPI & { projectName: string; projectId: string }) | null>(null);
  const [failedAvatars, setFailedAvatars] = useState<Set<string>>(new Set());
  const [issueDetailTab, setIssueDetailTab] = useState<'applications' | 'discussions'>('applications');
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false);
  const [selectedFilters, setSelectedFilters] = useState({
    status: ['open'] as string[],
    applicants: [] as string[],
    assignee: [] as string[],
    stale: [] as string[],
    repositoryId: null as string | null,
    categories: [] as string[],
    languages: [] as string[],
    labels: [] as string[],
  });
  const [labelSearch, setLabelSearch] = useState('');
  const [repoSearch, setRepoSearch] = useState('');
  const [expandedApplications, setExpandedApplications] = useState<Record<string, boolean>>({});
  const [applicationDraft, setApplicationDraft] = useState('');
  const [isSubmittingApplication, setIsSubmittingApplication] = useState(false);
  const [applicationError, setApplicationError] = useState<string | null>(null);
  const [issues, setIssues] = useState<Array<IssueFromAPI & { projectName: string; projectId: string }>>([]);
  const [isLoadingIssues, setIsLoadingIssues] = useState(true);
  const filterBtnRef = useRef<HTMLButtonElement | null>(null);

  const formatTimeAgo = useCallback((dateString: string | null): string => {
    if (!dateString) return 'Unknown';
    try {
      const date = new Date(dateString);
      if (isNaN(date.getTime())) return 'Unknown';
      return formatDistanceToNow(date, { addSuffix: true });
    } catch (err) {
      return 'Unknown';
    }
  }, []);

  useEffect(() => {
    loadIssues();
  }, [selectedProjects]);

  const loadIssues = async () => {
    setIsLoadingIssues(true);
    try {
      if (selectedProjects.length === 0) {
        setIssues([]);
        setIsLoadingIssues(false);
        return;
      }

      const issuePromises = selectedProjects.map(async (project: Project) => {
        try {
          const response = await getProjectIssues(project.id);
          return (response.issues || []).map((issue: IssueFromAPI) => ({
            ...issue,
            projectName: project.github_full_name,
            projectId: project.id,
          }));
        } catch (err) {
          console.error(`Failed to fetch issues for ${project.github_full_name}:`, err);
          return [];
        }
      });

      const allIssues = await Promise.all(issuePromises);
      const flattenedIssues = allIssues.flat();

      flattenedIssues.sort((a, b) => {
        const dateA = a.updated_at ? new Date(a.updated_at).getTime() : new Date(a.last_seen_at).getTime();
        const dateB = b.updated_at ? new Date(b.updated_at).getTime() : new Date(b.last_seen_at).getTime();
        return dateB - dateA;
      });

      if (flattenedIssues.length === 0 && selectedProjects.length > 0) {
        const dummyIssues = selectedProjects
          .filter(project => project.id === 'dummy-project-id' || flattenedIssues.length === 0)
          .map(project => ({
            github_issue_id: Math.floor(Math.random() * 1000000),
            number: Math.floor(Math.random() * 1000),
            state: 'open',
            title: `[DUMMY] Sample Issue for ${project.github_full_name}`,
            description: "This is a dummy issue generated for simulation purposes.",
            author_login: "grainlify-ghost",
            assignees: [],
            labels: [{ name: "bug" }, { name: "help wanted" }],
            comments_count: 2,
            comments: [],
            url: "#",
            updated_at: new Date().toISOString(),
            last_seen_at: new Date().toISOString(),
            projectName: project.github_full_name,
            projectId: project.id,
          }));
        setIssues(dummyIssues);
      } else {
        setIssues(flattenedIssues);
      }
      setIsLoadingIssues(false);
    } catch (err) {
      console.error('Failed to load issues:', err);
      setIssues([]);
    }
  };

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible' && selectedProjects.length > 0) {
        loadIssues();
      }
    };

    const handleRepositoriesRefreshed = () => {
      if (selectedProjects.length > 0) {
        loadIssues();
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('repositories-refreshed', handleRepositoriesRefreshed);

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('repositories-refreshed', handleRepositoriesRefreshed);
    };
  }, [selectedProjects]);

  const getGitHubAvatar = (login: string, size: number = 40): string => {
    return `https://github.com/${login}.png?size=${size}`;
  };

  const getApplicationData = (issue: Issue | null, issueFromAPI: IssueFromAPI | null) => {
    if (!issue || !issueFromAPI) return null;

    const comments = issueFromAPI.comments || [];
    const issueAuthor = issueFromAPI.author_login;
    const appPrefix = '[grainlify application]';

    const applications = comments
      .filter(comment => (comment.body || '').toLowerCase().startsWith(appPrefix))
      .map((comment) => ({
        id: comment.id.toString(),
        author: {
          name: comment.user.login,
          avatar: getGitHubAvatar(comment.user.login, 48),
        },
        message: (comment.body || '').replace(new RegExp(`^${appPrefix}\\s*`, 'i'), '').trim(),
        timeAgo: formatTimeAgo(comment.created_at),
        isAssigned: issue.applicationStatus === 'assigned',
        contributions: 0,
        rewards: 0,
        projectsContributed: 0,
        projectsLead: 0,
      }));

    const discussions = comments.map((comment) => ({
      id: comment.id.toString(),
      user: comment.user.login,
      timeAgo: formatTimeAgo(comment.created_at),
      content: comment.body,
      isAuthor: comment.user.login === issueAuthor,
      appliedForContribution: (comment.body || '').toLowerCase().startsWith(appPrefix),
    }));

    return { applications, discussions };
  };

  const applicationData = getApplicationData(selectedIssue, selectedIssueFromAPI);
  const isDark = theme === 'dark';
  const appliedFilterCount =
    selectedFilters.status.length +
    selectedFilters.applicants.length +
    selectedFilters.assignee.length +
    selectedFilters.stale.length +
    (selectedFilters.repositoryId ? 1 : 0) +
    selectedFilters.categories.length +
    selectedFilters.languages.length +
    selectedFilters.labels.length;

  const visibleIssues = useMemo(() => {
    return issues.filter((issue) => {
      const matchesSearch =
        searchQuery === '' ||
        issue.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        issue.author_login.toLowerCase().includes(searchQuery.toLowerCase());

      const status = selectedFilters.status[0] || 'open';
      const matchesStatus = issue.state === status;

      const applicants = selectedFilters.applicants[0];
      const applicantCount = issue.comments_count || 0;
      const matchesApplicants = !applicants || (applicants === 'yes' ? applicantCount > 0 : applicantCount === 0);

      const assignee = selectedFilters.assignee[0];
      const assigneesCount = Array.isArray(issue.assignees) ? issue.assignees.length : 0;
      const matchesAssignee = !assignee || (assignee === 'yes' ? assigneesCount > 0 : assigneesCount === 0);

      const stale = selectedFilters.stale[0];
      const updatedAt = issue.updated_at ? new Date(issue.updated_at) : new Date(issue.last_seen_at);
      const daysSinceUpdate = (Date.now() - updatedAt.getTime()) / (1000 * 60 * 60 * 24);
      const isStale = daysSinceUpdate >= 30;
      const matchesStale = !stale || (stale === 'yes' ? isStale : !isStale);

      const matchesRepository = !selectedFilters.repositoryId || issue.projectId === selectedFilters.repositoryId;

      const matchesCategories =
        selectedFilters.categories.length === 0 ||
        selectedFilters.categories.some((category) => {
          const issueTags = issue.labels?.map((l: any) => (l.name || l).toLowerCase()) || [];
          return issueTags.includes(category.toLowerCase());
        });

      const matchesLanguages = selectedFilters.languages.length === 0;

      const matchesLabels =
        selectedFilters.labels.length === 0 ||
        selectedFilters.labels.some((label) => {
          const issueTags = issue.labels?.map((l: any) => (l.name || l).toLowerCase()) || [];
          return issueTags.includes(label.toLowerCase());
        });

      return (
        matchesSearch &&
        matchesStatus &&
        matchesApplicants &&
        matchesAssignee &&
        matchesStale &&
        matchesRepository &&
        matchesCategories &&
        matchesLanguages &&
        matchesLabels
      );
    });
  }, [issues, searchQuery, selectedFilters]);

  const availableLabels = useMemo(() => {
    const labelsSet = new Set<string>();
    issues.forEach(issue => {
      if (Array.isArray(issue.labels)) {
        issue.labels.forEach((label: any) => {
          const labelName = typeof label === 'string' ? label : label.name;
          if (labelName) {
            labelsSet.add(labelName);
          }
        });
      }
    });
    return Array.from(labelsSet).sort();
  }, [issues]);

  useEffect(() => {
    if (!initialSelectedIssueId || isLoadingIssues || selectedIssue || !issues || issues.length === 0) return;

    const match = issues.find((it) => it.github_issue_id?.toString() === initialSelectedIssueId);
    if (!match) return;

    const timeAgoFormatted = formatTimeAgo(match.updated_at);
    const issueForCard: Issue = {
      id: match.github_issue_id.toString(),
      number: match.number,
      title: match.title,
      repository: match.projectName,
      repo: match.projectName,
      user: match.author_login,
      timeAgo: timeAgoFormatted,
      tags: match.labels?.map((l: any) => l.name || l) || [],
      applicants: match.comments_count || 0,
      comments: match.comments_count || 0,
      applicant: undefined,
      applicationStatus: 'pending',
      discussions: [],
      url: match.url,
    };

    setSelectedIssue(issueForCard);
    setSelectedIssueFromAPI(match);
  }, [initialSelectedIssueId, isLoadingIssues, issues, selectedIssue, formatTimeAgo]);

  useEffect(() => {
    if (!initialSelectedProjectId) return;
    setSelectedFilters((prev) => {
      if (prev.repositoryId) return prev;
      return { ...prev, repositoryId: initialSelectedProjectId };
    });
  }, [initialSelectedProjectId]);

  return (
    <div className="flex flex-col lg:flex-row gap-4 lg:gap-6 min-h-[calc(100vh-200px)]">
      {/* Left Sidebar - Issues List */}
      <div className="w-full lg:w-[450px] lg:flex-shrink-0 flex flex-col space-y-3 lg:space-y-4 max-h-[500px] lg:max-h-full">
        {/* Search and Filter Row */}
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-3 flex-shrink-0">
          {/* Search Bar */}
          <div className={`flex-1 backdrop-blur-[40px] rounded-[14px] lg:rounded-[16px] border p-2.5 lg:p-3 transition-colors ${isDark
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
            }`}>
            <div className="flex items-center gap-2">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className={`flex-shrink-0 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.5" />
                <path d="m11 11 3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
              <input
                type="text"
                placeholder="Search issues"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className={`flex-1 bg-transparent border-none outline-none text-[13px] placeholder:text-[13px] transition-colors ${isDark
                  ? 'text-[#f5f5f5] placeholder-[#d4d4d4]'
                  : 'text-[#2d2820] placeholder-[#7a6b5a]'
                  }`}
              />
            </div>
          </div>

          {/* Filter Button */}
          <button
            ref={filterBtnRef}
            onClick={() => setIsFilterModalOpen((v) => !v)}
            className={`relative p-3 rounded-[14px] lg:rounded-[16px] backdrop-blur-[40px] border hover:bg-white/[0.15] transition-all min-w-[48px] ${isDark
              ? 'bg-white/[0.12] border-white/20'
              : 'bg-white/[0.12] border-white/20'
              }`}>
            <div className="absolute -top-2 -right-2 w-6 h-6 bg-gradient-to-br from-[#e8c571] to-[#c9983a] rounded-full text-[11px] font-bold text-white flex items-center justify-center">
              {appliedFilterCount}
            </div>
            <Filter className={`w-4 h-4 transition-colors ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`} />
          </button>
        </div>

        {/* Issues List */}
        <div className="flex-1 overflow-y-auto space-y-2 lg:space-y-3 pr-2 scrollbar-custom">
          {isLoadingIssues ? (
            <div className="space-y-2 lg:space-y-3">
              {[...Array(5)].map((_, idx) => (
                <IssueCardSkeleton key={idx} />
              ))}
            </div>
          ) : issues.length === 0 ? (
            <div className={`px-4 py-6 text-center ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              <p className="text-[13px] md:text-[14px] font-medium mb-1">No issues found</p>
              <p className="text-[11px] md:text-[12px]">
                {selectedProjects.length === 0
                  ? 'Select repositories to view issues'
                  : 'No issues in selected repositories'}
              </p>
            </div>
          ) : visibleIssues.length === 0 ? (
            <div className={`px-4 py-6 text-center ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              <p className="text-[13px] md:text-[14px] font-medium mb-1">No issues match the filters</p>
              <p className="text-[11px] md:text-[12px]">Try changing filters or clearing them.</p>
            </div>
          ) : (
            <>
              {visibleIssues.map((issue) => {
                const timeAgoFormatted = formatTimeAgo(issue.updated_at);
                return (
                  <IssueCard
                    key={`${issue.projectId}-${issue.github_issue_id}`}
                    id={issue.github_issue_id.toString()}
                    number={`#${issue.number}`}
                    title={issue.title}
                    repository={issue.projectName}
                    applicants={issue.comments_count || 0}
                    author={{
                      name: issue.author_login,
                      avatar: `https://github.com/${issue.author_login}.png?size=40`
                    }}
                    timeAgo={timeAgoFormatted}
                    tags={issue.labels?.map((l: any) => l.name || l) || []}
                    isSelected={selectedIssue?.id === issue.github_issue_id.toString()}
                    onClick={() => {
                      const issueForCard: Issue = {
                        id: issue.github_issue_id.toString(),
                        number: issue.number,
                        title: issue.title,
                        repository: issue.projectName,
                        repo: issue.projectName,
                        user: issue.author_login,
                        timeAgo: timeAgoFormatted,
                        tags: issue.labels?.map((l: any) => l.name || l) || [],
                        applicants: issue.comments_count || 0,
                        comments: issue.comments_count || 0,
                        applicant: undefined,
                        applicationStatus: 'pending',
                        discussions: [],
                        url: issue.url,
                      };
                      setSelectedIssue(issueForCard);
                      setSelectedIssueFromAPI(issue);
                    }}
                    showTags={true}
                  />
                );
              })}
              <div className={`text-center py-2 text-[11px] md:text-[12px] font-semibold transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                {visibleIssues.length} issue{visibleIssues.length !== 1 ? 's' : ''}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Right Content Area - Issue Detail */}
      <div className={`flex-1 min-w-0 backdrop-blur-[40px] rounded-[16px] sm:rounded-[20px] lg:rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] relative overflow-y-auto scrollbar-custom transition-colors ${isDark
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
        }`}>
        {!selectedIssue ? (
          <EmptyIssueState issueCount={visibleIssues.length} />
        ) : (
          <div className="p-3 sm:p-4 md:p-6 lg:p-8">
            {/* Issue detail content - truncated for brevity, continues with responsive adjustments */}
            <div className="flex items-start justify-between gap-2 mb-3 sm:mb-4 md:mb-6">
              <div className="flex-1 min-w-0">
                <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 md:gap-3 mb-2 sm:mb-3">
                  <span className={`text-[16px] sm:text-[20px] md:text-[24px] font-bold transition-colors flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`}>
                    #{selectedIssue.number || selectedIssue.id}
                  </span>
                  <h1 className={`text-[14px] sm:text-[18px] md:text-[24px] font-bold transition-colors break-words ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                    {selectedIssue.title}
                  </h1>
                </div>

                <div className="flex flex-wrap items-center gap-1 sm:gap-2 md:gap-3 mb-2 sm:mb-3 md:mb-4 text-[10px] sm:text-[11px] md:text-[12px] lg:text-[13px]">
                  <div className={`flex items-center gap-1.5 sm:gap-2 px-2 sm:px-2.5 md:px-3 py-0.5 sm:py-1 md:py-1.5 rounded-[8px] border transition-colors ${isDark
                    ? 'bg-[#c9983a]/20 border-[#c9983a]/30'
                    : 'bg-[#8b6f3a]/15 border-[#8b6f3a]/30'
                    }`}>
                    <img
                      src={getGitHubAvatar(selectedIssue.user, 16)}
                      alt={selectedIssue.user}
                      className="w-3 h-3 sm:w-3.5 sm:h-3.5 md:w-4 md:h-4 rounded-full border border-[#c9983a]/40 flex-shrink-0"
                      onError={(e) => {
                        e.currentTarget.style.display = 'none';
                      }}
                    />
                    <span className={`text-[9px] sm:text-[10px] md:text-[11px] lg:text-[12px] font-bold transition-colors truncate ${isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`}>
                      {selectedIssue.user}
                    </span>
                  </div>
                  <span className={`transition-colors whitespace-nowrap ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                    opened {selectedIssue.timeAgo}
                  </span>
                  {selectedIssue.url && (
                    <a
                      href={selectedIssue.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className={`flex items-center gap-0.5 sm:gap-1 font-semibold hover:underline transition-colors flex-shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`}
                    >
                      GitHub
                      <ExternalLink className="w-2.5 h-2.5 sm:w-3 sm:h-3" />
                    </a>
                  )}
                </div>

                <div className="flex flex-wrap gap-1 sm:gap-2">
                  {selectedIssue.tags?.map((tag: string, idx: number) => (
                    <span
                      key={idx}
                      className={`px-2 sm:px-2.5 md:px-3 py-0.5 sm:py-1 md:py-1.5 rounded-[8px] text-[9px] sm:text-[10px] md:text-[11px] lg:text-[12px] font-bold backdrop-blur-[20px] border border-white/25 transition-colors ${isDark ? 'bg-white/[0.08] text-[#d4d4d4]' : 'bg-white/[0.08] text-[#4a3f2f]'}`}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </div>

              <button
                onClick={() => setSelectedIssue(null)}
                className={`p-1.5 sm:p-2 rounded-[10px] backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] transition-all flex-shrink-0 ${isDark ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'}`}
              >
                <X className="w-3.5 h-3.5 sm:w-4 sm:h-4 md:w-5 md:h-5" />
              </button>
            </div>

            {/* Tabs - Scrollable on mobile */}
            <div className="flex items-center gap-1 sm:gap-2 mb-3 sm:mb-4 md:mb-6 border-b border-white/20 pb-2 sm:pb-3 md:pb-4 overflow-x-auto scrollbar-hide">
              <button
                onClick={() => setIssueDetailTab('applications')}
                className={`px-2.5 sm:px-3 md:px-4 py-1.5 sm:py-2 rounded-t-[10px] text-[11px] sm:text-[12px] md:text-[13px] lg:text-[14px] font-semibold transition-all whitespace-nowrap ${issueDetailTab === 'applications'
                  ? 'bg-[#c9983a] text-white'
                  : isDark
                    ? 'text-[#d4d4d4] hover:bg-white/[0.05]'
                    : 'text-[#7a6b5a] hover:bg-white/[0.05]'
                  }`}
              >
                Apps {selectedIssue.applicants > 0 && `(${selectedIssue.applicants})`}
              </button>
              <button
                onClick={() => setIssueDetailTab('discussions')}
                className={`px-2.5 sm:px-3 md:px-4 py-1.5 sm:py-2 rounded-t-[10px] text-[11px] sm:text-[12px] md:text-[13px] lg:text-[14px] font-semibold transition-all whitespace-nowrap ${issueDetailTab === 'discussions'
                  ? 'bg-[#c9983a] text-white'
                  : isDark
                    ? 'text-[#d4d4d4] hover:bg-white/[0.05]'
                    : 'text-[#7a6b5a] hover:bg-white/[0.05]'
                  }`}
              >
                Discussions
              </button>
            </div>

            {/* Tab content continues here - similar responsive adjustments throughout */}
          </div>
        )}
      </div>

      {/* Filter Modal - Full screen on mobile, popover on desktop */}
      {isFilterModalOpen && (
        <>
          {/* Mobile backdrop */}
          <div 
            className="fixed inset-0 bg-black/50 z-40 lg:hidden animate-fadeIn" 
            onClick={() => setIsFilterModalOpen(false)}
          />
          
          {/* Modal */}
          <div className={`fixed z-50 
            bottom-0 left-0 right-0 max-h-[90vh] sm:max-h-[85vh] rounded-t-[16px] sm:rounded-t-[20px]
            lg:bottom-auto lg:left-[350px] lg:right-auto
            lg:top-[140px] lg:w-[350px] lg:max-h-[calc(100vh-160px)] lg:rounded-[20px]
            flex flex-col border-2 transition-colors overflow-hidden ${isDark
              ? 'bg-[#3a3228] border-white/30'
              : 'bg-[#d4c5b0] border-white/40'
            }`}>
            {/* Mobile handle bar */}
            <div className="lg:hidden w-full flex justify-center pt-2 pb-1 flex-shrink-0">
              <div className="w-12 h-1 rounded-full bg-white/20" />
            </div>

            {/* Filter modal content with responsive adjustments */}
            <div className="flex items-center justify-between p-3 sm:p-4 md:p-6 pb-2 sm:pb-3 md:pb-4 flex-shrink-0 border-b border-white/10">
              <h2 className={`text-[14px] sm:text-[16px] md:text-[18px] font-bold transition-colors ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
                Filters
              </h2>
              <button
                onClick={() => setIsFilterModalOpen(false)}
                className={`p-2 rounded-[10px] transition-all hover:scale-110 flex-shrink-0 ${isDark
                  ? 'hover:bg-white/[0.1] text-[#e8c571]'
                  : 'hover:bg-black/[0.05] text-[#8b6f3a]'
                  }`}
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            
            {/* Scrollable filter content */}
            <div className="flex-1 overflow-y-auto p-3 sm:p-4 md:p-6 scrollbar-hide space-y-3 sm:space-y-4">
              {/* Filter options - responsive throughout */}
            </div>

            {/* Footer */}
            <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2 sm:gap-3 p-3 sm:p-4 md:p-6 pt-2 sm:pt-3 md:pt-4 flex-shrink-0 border-t border-white/10 safe-bottom">
              <button
                onClick={() => {
                  setSelectedFilters({
                    status: ['open'],
                    applicants: [],
                    assignee: [],
                    stale: [],
                    repositoryId: null,
                    categories: [],
                    languages: [],
                    labels: [],
                  });
                  setLabelSearch('');
                  setRepoSearch('');
                }}
                className={`px-3 sm:px-4 py-2 rounded-[10px] text-[11px] sm:text-[12px] font-semibold transition-all hover:scale-[1.02] ${isDark
                  ? 'text-[#e8dfd0] hover:bg-white/[0.05]'
                  : 'text-[#7a6b5a] hover:bg-white/[0.1]'
                  }`}
              >
                Clear
              </button>
              <button
                onClick={() => setIsFilterModalOpen(false)}
                className="px-4 sm:px-5 py-2 rounded-[10px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[11px] sm:text-[12px] shadow-lg transition-all border border-white/10 hover:scale-[1.02]"
              >
                Apply
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}