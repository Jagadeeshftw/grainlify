import { useState, useEffect, useMemo } from 'react';
import { ChevronDown, ChevronRight, Plus, Settings as SettingsIcon, AlertCircle, Package } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';
import { DashboardTab } from '../components/dashboard/DashboardTab';
import { IssuesTab } from '../components/issues/IssuesTab';
import { PullRequestsTab } from '../components/pull-requests/PullRequestsTab';
import { TabType } from '../types';
import { getMyProjects } from '../../../shared/api/client';
import { InstallGitHubAppModal } from '../components/InstallGitHubAppModal';

interface MaintainersPageProps {
  onNavigate: (page: string) => void;
}

interface Project {
  id: string;
  github_full_name: string;
  status: string;
  ecosystem_name: string;
  language: string | null;
  tags: string[];
  category: string | null;
}

interface GroupedRepository {
  org: string;
  repos: Array<{
    id: string;
    name: string;
    fullName: string;
    status: string;
  }>;
}

export function MaintainersPage({ onNavigate }: MaintainersPageProps) {
  const { theme } = useTheme();
  const [activeTab, setActiveTab] = useState<TabType>('Dashboard');
  const [isRepoDropdownOpen, setIsRepoDropdownOpen] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [expandedOrgs, setExpandedOrgs] = useState<Set<string>>(new Set());
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedRepoIds, setSelectedRepoIds] = useState<Set<string>>(new Set());
  const [failedAvatars, setFailedAvatars] = useState<Set<string>>(new Set());
  const [targetIssueId, setTargetIssueId] = useState<string | undefined>(undefined);
  const [targetProjectId, setTargetProjectId] = useState<string | undefined>(undefined);

  useEffect(() => {
  if (projects && projects.length > 0) {
    // Extraemos los IDs de todos los proyectos cargados
    const allIds = projects.map(project => project.id);
    // Inicializamos el Set con todos los IDs seleccionados
    setSelectedRepoIds(new Set(allIds));
  }
}, [projects]);


  // Helper function to get GitHub repository avatar (owner's avatar)
  const getRepoAvatar = (githubFullName: string, size: number = 20): string => {
    const [owner] = githubFullName.split('/');
    return `https://github.com/${owner}.png?size=${size}`;
  };

  const tabs: TabType[] = ['Dashboard', 'Issues', 'Pull Requests'];

  // Fetch projects from API
  useEffect(() => {
    loadProjects();

    // Check if we're returning from GitHub App installation
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('github_app_installed') === 'true') {
      // Refresh projects after a short delay to allow backend to sync
      setTimeout(() => {
        loadProjects();
      }, 3000); // Increased delay to allow backend sync to complete
      // Clean up URL
      window.history.replaceState({}, '', window.location.pathname);
    }
  }, []);

  // Expose refresh function for child components
  const refreshAll = () => {
    loadProjects();
    // Trigger a custom event that child components can listen to
    window.dispatchEvent(new CustomEvent('repositories-refreshed'));
  };

 const loadProjects = async () => {
  setIsLoading(true);
  setError(null);

  try {
    const data = await getMyProjects();

    // Ensure data is an array
    const projectsArray = Array.isArray(data) ? data : [];

    setProjects(projectsArray);
    setError(null);
  } catch (err) {
    const errorMessage =
      err instanceof Error ? err.message : 'Failed to load repositories';

    if (
      errorMessage.includes('Authentication failed') ||
      errorMessage.includes('401')
    ) {
      setError('Please sign in to view your repositories');
    } else if (errorMessage.includes('Network error')) {
      setError(
        'Unable to connect to the server. Please check your connection and try again.'
      );
    } else {
      setError(errorMessage);
    }

    // Clear projects on error so UI shows error state
    setProjects([]);
  } finally {
    setIsLoading(false);
  }
};

  // Group repositories by organization (owner)
  const groupedRepositories = useMemo<GroupedRepository[]>(() => {
    const grouped = new Map<string, GroupedRepository>();

    projects.forEach((project) => {
      const [org, repoName] = project.github_full_name.split('/');
      if (!org || !repoName) return;

      if (!grouped.has(org)) {
        grouped.set(org, { org, repos: [] });
      }

      const group = grouped.get(org)!;
      group.repos.push({
        id: project.id,
        name: repoName,
        fullName: project.github_full_name,
        status: project.status,
      });
    });

    return Array.from(grouped.values()).sort((a, b) => a.org.localeCompare(b.org));
  }, [projects]);

  // Toggle organization expansion
  const toggleOrgExpansion = (org: string) => {
    setExpandedOrgs(prev => {
      const next = new Set(prev);
      if (next.has(org)) {
        next.delete(org);
      } else {
        next.add(org);
      }
      return next;
    });
  };

  // Toggle repository selection
  const toggleRepoSelection = (repoId: string) => {
    setSelectedRepoIds(prev => {
      const next = new Set(prev);
      if (next.has(repoId)) {
        next.delete(repoId);
      } else {
        next.add(repoId);
      }
      return next;
    });
  };

  // Get selected projects
  const selectedProjects = useMemo(() => {
    if (selectedRepoIds.size === 0) {
      // If no repos selected, return all verified projects
      return projects.filter(p => p.status === 'verified');
    }
    return projects.filter(p => selectedRepoIds.has(p.id) && p.status === 'verified');
  }, [projects, selectedRepoIds]);

  const handleNavigateToIssue = (issueId: string, projectId: string) => {
    setTargetIssueId(issueId);
    setTargetProjectId(projectId);
    setActiveTab('Issues');
  };

  return (
    <div className="space-y-6">
      {/* Top Navigation Bar */}
      <div className={`backdrop-blur-[40px] rounded-[20px] border p-2 relative z-10 transition-colors ${theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/25'
        }`}>
        <div className="flex items-center gap-4">
          {/* Repository Selector */}
          <div className="relative z-50">
            <button
              className={`flex items-center gap-3 px-5 py-3 rounded-[14px] backdrop-blur-[30px] border transition-all group ${theme === 'dark'
                ? 'bg-white/[0.08] border-white/20 hover:bg-white/[0.12] hover:border-[#c9983a]/40'
                : 'bg-white/[0.15] border-white/30 hover:bg-white/[0.2] hover:border-[#c9983a]/30'
                }`}
              onClick={() => setIsRepoDropdownOpen(!isRepoDropdownOpen)}
            >
              <span className={`text-[14px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                }`}>Select repositories</span>
              <ChevronDown className={`w-4 h-4 transition-all ${isRepoDropdownOpen ? 'rotate-180' : ''} ${theme === 'dark' ? 'text-[#b8a898] group-hover:text-[#e8c77f]' : 'text-[#7a6b5a] group-hover:text-[#c9983a]'
                }`} />
            </button>

            {/* Dropdown Menu */}
            {isRepoDropdownOpen && (
              <div className={`absolute top-full left-0 mt-2 w-[380px] rounded-[20px] border-2 z-50 overflow-hidden transition-colors ${theme === 'dark'
                ? 'bg-[#3a3228] border-white/30'
                : 'bg-[#d4c5b0] border-white/40'
                }`}>
                {/* Header */}
                <div className={`px-6 py-5 border-b-2 bg-gradient-to-b from-white/10 to-transparent transition-colors ${theme === 'dark' ? 'border-white/20' : 'border-white/30'
                  }`}>
                  <h3 className={`text-[17px] font-bold transition-colors ${theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                    }`}>Select repositories</h3>
                </div>

                {/* Repository List */}
                <div className="py-3 max-h-[420px] overflow-y-auto">
                  {isLoading ? (
                    <div className="px-6 space-y-2">
                      {[...Array(5)].map((_, idx) => (
                        <div key={idx} className="flex items-center gap-3 py-3">
                          <SkeletonLoader variant="circle" className="w-5 h-5 flex-shrink-0" />
                          <SkeletonLoader className="h-4 w-32" />
                          <SkeletonLoader className="h-4 w-20 ml-auto" />
                        </div>
                      ))}
                    </div>
                  ) : error ? (
                    <div className={`flex items-center gap-3 px-6 py-4 mx-4 rounded-[12px] ${theme === 'dark'
                      ? 'bg-red-500/10 border border-red-500/30 text-red-400'
                      : 'bg-red-100 border border-red-300 text-red-700'
                      }`}>
                      <AlertCircle className="w-5 h-5 flex-shrink-0" />
                      <span className="text-[14px] font-medium">{error}</span>
                    </div>
                  ) : groupedRepositories.length === 0 ? (
                    <div className={`px-6 py-8 text-center ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                      }`}>
                      <p className="text-[14px] font-medium mb-1">No repositories found</p>
                      <p className="text-[12px]">Add your first repository to get started</p>
                    </div>
                  ) : (
                    groupedRepositories.map((group) => {
                      const isExpanded = expandedOrgs.has(group.org);

                      return (
                        <div key={group.org}>
                          {/* Organization/Project */}
                          <button
                            className={`w-full px-6 py-3.5 flex items-center justify-between transition-all group/repo ${theme === 'dark'
                              ? 'hover:bg-[#4a3e30]'
                              : 'hover:bg-[#c9b8a0]'
                              }`}
                            onClick={() => group.repos.length > 0 && toggleOrgExpansion(group.org)}
                          >
                            <div className="flex items-center gap-3 flex-1">
                              <span className={`text-[15px] font-bold group-hover/repo:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                }`}>
                                {group.org}
                              </span>
                              {group.repos.length === 0 && (
                                <span className={`text-[12px] italic font-medium transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#8a7b6a]'
                                  }`}>
                                  No synced repos
                                </span>
                              )}
                            </div>
                            {group.repos.length > 0 && (
                              <ChevronRight
                                className={`w-4 h-4 group-hover/repo:text-[#c9983a] transition-all ${isExpanded ? 'rotate-90' : ''} ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                  }`}
                              />
                            )}
                          </button>

                          {/* Sub-repositories (if expanded) */}
                          {group.repos.length > 0 && isExpanded && (
                            <div className={`py-2 space-y-1 transition-colors ${theme === 'dark' ? 'bg-[#2d2820]/30' : 'bg-[#c9b8a0]/30'
                              }`}>
                              {group.repos.map((repo) => (
                                <label
                                  key={repo.id}
                                  className={`flex items-center gap-3 px-6 py-2.5 rounded-[10px] mx-4 cursor-pointer group/subrepo transition-all ${theme === 'dark'
                                    ? 'hover:bg-[#4a3e30]'
                                    : 'hover:bg-[#c9b8a0]'
                                    }`}
                                >
                                  <input
                                    type="checkbox"
                                    checked={selectedRepoIds.has(repo.id)}
                                    onChange={() => toggleRepoSelection(repo.id)}
                                    className={`w-[18px] h-[18px] rounded-[4px] border-2 checked:bg-[#c9983a] checked:border-[#c9983a] focus:ring-2 focus:ring-[#c9983a]/40 transition-all cursor-pointer appearance-none checked:after:content-['✓'] checked:after:text-white checked:after:text-[12px] checked:after:flex checked:after:items-center checked:after:justify-center checked:after:font-bold ${theme === 'dark'
                                      ? 'border-[#b8a898]/50 bg-[#2d2820]'
                                      : 'border-[#7a6b5a]/50 bg-[#e8dfd0]'
                                      }`}
                                    style={{
                                      display: 'flex',
                                      alignItems: 'center',
                                      justifyContent: 'center'
                                    }}
                                  />
                                  <div className="flex items-center gap-2 flex-1">
                                    {failedAvatars.has(getRepoAvatar(repo.fullName, 20)) ? (
                                      <div className="w-5 h-5 rounded-md bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center flex-shrink-0">
                                        <Package className="w-3 h-3 text-white" />
                                      </div>
                                    ) : (
                                      <img
                                        src={getRepoAvatar(repo.fullName, 20)}
                                        alt={repo.name}
                                        className="w-5 h-5 rounded-md border border-[#c9983a]/40 flex-shrink-0"
                                        onError={() => setFailedAvatars(prev => new Set(prev).add(getRepoAvatar(repo.fullName, 20)))}
                                      />
                                    )}
                                    <span className={`text-[14px] font-semibold group-hover/subrepo:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                      }`}>
                                      {repo.name}
                                    </span>
                                    {repo.status === 'pending_verification' && (
                                      <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${theme === 'dark'
                                        ? 'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30'
                                        : 'bg-yellow-100 text-yellow-700 border border-yellow-300'
                                        }`}>
                                        Pending
                                      </span>
                                    )}
                                  
                                    {repo.status === 'rejected' && (
                                      <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${theme === 'dark'
                                        ? 'bg-red-500/20 text-red-400 border border-red-500/30'
                                        : 'bg-red-100 text-red-700 border border-red-300'
                                        }`}>
                                        Rejected
                                      </span>
                                    )}
                                  </div>
                                </label>
                              ))}
                            </div>
                          )}
                        </div>
                      );
                    })
                  )}
                </div>

                {/* Footer: Add Repository */}
                <div className={`px-6 py-5 border-t-2 bg-gradient-to-t from-white/10 to-transparent transition-colors ${theme === 'dark' ? 'border-white/20' : 'border-white/30'
                  }`}>
                  <button
                    onClick={() => setIsAddModalOpen(true)}
                    className={`w-full flex items-center justify-between px-5 py-3.5 rounded-[14px] border-2 transition-all group/add ${theme === 'dark'
                      ? 'bg-white/10 border-white/25 hover:bg-white/20 hover:border-[#c9983a]/40'
                      : 'bg-white/40 border-white/50 hover:bg-white/60 hover:border-[#c9983a]/40'
                      } hover:shadow-[0_6px_20px_rgba(201,152,58,0.3)]`}
                  >
                    <div className="flex items-center gap-3">
                      <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 flex items-center justify-center border border-[#c9983a]/40">
                        <Plus className="w-4 h-4 text-[#c9983a] group-hover/add:text-[#b8872e] transition-colors" strokeWidth={2.5} />
                      </div>
                      <span className={`text-[14px] font-bold group-hover/add:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                        }`}>
                        Add a repository
                      </span>
                    </div>
                    <SettingsIcon className={`w-4 h-4 group-hover/add:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                      }`} />
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* Tab Navigation */}
          <div className="flex items-center gap-2 flex-1">
            {tabs.map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-5 py-3 rounded-[14px] text-[14px] font-semibold transition-all ${activeTab === tab
                  ? theme === 'dark'
                    ? 'bg-gradient-to-br from-[#c9983a]/40 via-[#d4af37]/35 to-[#c9983a]/30 border-2 border-[#c9983a]/70 text-[#fef5e7]'
                    : 'bg-gradient-to-br from-[#c9983a]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-2 border-[#c9983a]/50 text-[#2d2820]'
                  : theme === 'dark'
                    ? 'text-white bg-white/[0.15] border-2 border-white/25 hover:bg-white/[0.25] hover:border-white/40'
                    : 'text-[#7a6b5a] hover:text-[#2d2820] hover:bg-white/[0.1] border-2 border-transparent'
                  }`}
              >
                {tab}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Tab Content */}
      {activeTab === 'Dashboard' && (
        <DashboardTab
          selectedProjects={selectedProjects}
          onRefresh={refreshAll}
          onNavigateToIssue={handleNavigateToIssue}
        />
      )}

      {activeTab === 'Issues' && (
        <IssuesTab
          onNavigate={onNavigate}
          selectedProjects={selectedProjects}
          onRefresh={refreshAll}
          initialSelectedIssueId={targetIssueId}
          initialSelectedProjectId={targetProjectId}
        />
      )}

      {activeTab === 'Pull Requests' && <PullRequestsTab selectedProjects={selectedProjects} onRefresh={refreshAll} />}

      {/* Install GitHub App Modal */}
      <InstallGitHubAppModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        onSuccess={refreshAll}
      />
    </div>
  );
}