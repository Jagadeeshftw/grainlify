import { useState, useEffect } from 'react';
import { Search, Loader2, AlertCircle } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { PRFilterType } from '../../types';
import { PRRow } from './PRRow';
import { PRFilterDropdown } from './PRFilterDropdown';
import { getMyProjects, getProjectPRs } from '../../../../shared/api/client';
import { PRRowSkeleton } from '../../../../shared/components/PRRowSkeleton';

interface PRFromAPI {
  github_pr_id: number;
  number: number;
  state: string;
  title: string;
  author_login: string;
  url: string;
  merged: boolean;
  created_at: string | null;
  updated_at: string | null;
  closed_at: string | null;
  merged_at: string | null;
  last_seen_at: string;
}

interface Project {
  id: string;
  github_full_name: string;
  status: string;
}

interface PullRequestsTabProps {
  selectedProjects: Project[];
  onRefresh?: () => void;
}

export function PullRequestsTab({ selectedProjects, onRefresh }: PullRequestsTabProps) {
  const { theme } = useTheme();
  const [searchQuery, setSearchQuery] = useState('');
  const [filter, setFilter] = useState<PRFilterType>('All states');
  const [isFilterDropdownOpen, setIsFilterDropdownOpen] = useState(false);
  const [prs, setPrs] = useState<Array<PRFromAPI & { projectName: string }>>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch PRs from selected projects
  useEffect(() => {
    loadPRs();
  }, [selectedProjects]);

  const loadPRs = async () => {
    setIsLoading(true);
    setError(null);
    try {
      if (selectedProjects.length === 0) {
        setPrs([]);
        setIsLoading(false);
        return;
      }

      // Fetch PRs from all selected projects in parallel
      const prPromises = selectedProjects.map(async (project: Project) => {
        try {
          const response = await getProjectPRs(project.id);
          return (response.prs || []).map((pr: PRFromAPI) => ({
            ...pr,
            projectName: project.github_full_name,
          }));
        } catch (err) {
          console.error(`Failed to fetch PRs for ${project.github_full_name}:`, err);
          return [];
        }
      });

      const allPRs = await Promise.all(prPromises);
      const flattenedPRs = allPRs.flat();
      
      // Sort by updated_at (most recent first)
      flattenedPRs.sort((a, b) => {
        const dateA = a.updated_at ? new Date(a.updated_at).getTime() : new Date(a.last_seen_at).getTime();
        const dateB = b.updated_at ? new Date(b.updated_at).getTime() : new Date(b.last_seen_at).getTime();
        return dateB - dateA;
      });

      setPrs(flattenedPRs);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load pull requests';
      setError(errorMessage);
      setPrs([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Refresh PRs when selectedProjects change
  // Also refresh when page becomes visible (user switches back to tab)
  // And when repositories are refreshed (new repo added)
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible' && selectedProjects.length > 0) {
        loadPRs();
      }
    };

    const handleRepositoriesRefreshed = () => {
      // Refresh PRs when repositories are added/updated
      if (selectedProjects.length > 0) {
        loadPRs();
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('repositories-refreshed', handleRepositoriesRefreshed);
    
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('repositories-refreshed', handleRepositoriesRefreshed);
    };
  }, [selectedProjects]);

  // Filter PRs based on search and filter
  const filteredPRs = prs.filter(pr => {
    // Search filter
    const matchesSearch = searchQuery === '' || 
      pr.title.toLowerCase().includes(searchQuery.toLowerCase()) || 
      pr.author_login.toLowerCase().includes(searchQuery.toLowerCase());

    // Status filter - map API state to filter type
    let matchesStatus = true;
    if (filter !== 'All states') {
      if (filter === 'Merged') {
        matchesStatus = pr.merged === true;
      } else if (filter === 'Open') {
        matchesStatus = pr.state === 'open';
      } else if (filter === 'Closed') {
        matchesStatus = pr.state === 'closed' && pr.merged === false;
      } else if (filter === 'Draft') {
        // GitHub API doesn't have draft state in the response we're getting
        // We'll skip draft filter for now or check title/body
        matchesStatus = false; // No draft info in current API response
      }
    }

    return matchesSearch && matchesStatus;
  });

  const handleClearFilters = () => {
    setSearchQuery('');
    setFilter('All states');
  };

  return (
    <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 transition-colors ${
      theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
    }`}>
      {/* Header */}
      <div className="mb-6">
        <h2 className={`text-[28px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
        }`}>Pull Requests</h2>
        <p className={`text-[14px] transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>Review and manage pull requests with quality indicators and contributor insights.</p>
      </div>

      {/* Search and Filters (responsive) */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-3 mb-6">
        {/* Search Bar */}
        <div className="relative flex-1">
          <Search className={`absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`} />
          <input
            type="text"
            placeholder="Search pull request by title or author name..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={`w-full pl-10 pr-4 py-2 sm:py-3 rounded-[12px] sm:rounded-[14px] backdrop-blur-[25px] border text-[13px] sm:text-[14px] focus:outline-none transition-all ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/20 text-[#e8dfd0] placeholder:text-[#8a7b6a] focus:bg-white/[0.12] focus:border-[#c9983a]/40'
                : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder:text-[#9a8b7a] focus:bg-white/[0.2] focus:border-[#c9983a]/40'
            }`}
          />
        </div>

        {/* Filter Dropdown (stacked on mobile) */}
        <div className="mt-2 sm:mt-0 sm:ml-2 sm:flex-shrink-0">
          <PRFilterDropdown
            value={filter}
            onChange={setFilter}
            isOpen={isFilterDropdownOpen}
            onToggle={() => setIsFilterDropdownOpen(!isFilterDropdownOpen)}
            onClose={() => setIsFilterDropdownOpen(false)}
          />
        </div>

        {/* Clear Filters Button (full width on mobile) */}
        <button 
          className={`mt-2 sm:mt-0 w-full sm:w-auto px-4 sm:px-5 py-2 sm:py-3 rounded-[12px] backdrop-blur-[25px] border transition-all text-[13px] font-semibold ${
            theme === 'dark'
              ? 'bg-white/[0.08] border-white/20 hover:bg-white/[0.12] hover:border-[#c9983a]/30 text-[#b8a898]'
              : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:border-[#c9983a]/30 text-[#7a6b5a]'
          }`}
          onClick={handleClearFilters}
        >
          Clear filters
        </button>
      </div>

      {/* Pull Requests Table */}
      <div className="space-y-4">
        {/* Table Header (responsive) */}
        <div className={`grid grid-cols-1 sm:grid-cols-[2fr_1.5fr_1fr_0.5fr] gap-6 px-4 sm:px-6 py-3 border-b-2 transition-colors ${
          theme === 'dark' ? 'border-white/20' : 'border-white/20'
        }`}>
          <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
            theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
          }`}>Pull Request</div>
          <div className={`hidden sm:block text-[12px] font-bold uppercase tracking-wide transition-colors ${
            theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
          }`}>Author</div>
          <div className={`hidden sm:block text-[12px] font-bold uppercase tracking-wide transition-colors ${
            theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
          }`}>Repository</div>
          <div className={`hidden sm:block text-[12px] font-bold uppercase tracking-wide transition-colors ${
            theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
          }`}>Indicators</div>
        </div>

        {/* Pull Request Rows */}
        {isLoading ? (
          <div className="space-y-4">
            {[...Array(8)].map((_, idx) => (
              <PRRowSkeleton key={idx} />
            ))}
          </div>
        ) : error ? (
          <div className={`flex items-center gap-3 px-6 py-4 mx-4 rounded-[12px] ${
            theme === 'dark'
              ? 'bg-red-500/10 border border-red-500/30 text-red-400'
              : 'bg-red-100 border border-red-300 text-red-700'
          }`}>
            <AlertCircle className="w-5 h-5 flex-shrink-0" />
            <span className="text-[14px] font-medium">{error}</span>
          </div>
        ) : filteredPRs.length > 0 ? (
          filteredPRs.map((pr) => {
            // Determine status: merged takes priority, then state
            const status: 'merged' | 'draft' | 'open' | 'closed' = pr.merged 
              ? 'merged' 
              : (pr.state === 'open' ? 'open' : 'closed');
            
            // Determine which date to use for status detail
            let statusDate: string;
            let statusAction: string;
            if (pr.merged && pr.merged_at) {
              statusDate = pr.merged_at;
              statusAction = 'merged';
            } else if (pr.state === 'closed' && pr.closed_at) {
              statusDate = pr.closed_at;
              statusAction = 'closed';
            } else if (pr.state === 'open' && pr.created_at) {
              statusDate = pr.created_at;
              statusAction = 'opened';
            } else {
              statusDate = pr.updated_at || pr.last_seen_at;
              statusAction = pr.state;
            }

            // Convert API PR to component format
            const prForComponent = {
              id: pr.github_pr_id,
              number: pr.number,
              title: pr.title,
              status: status,
              statusDetail: `${statusAction} ${formatTimeAgo(statusDate)}`,
              url: pr.url, // Add URL for clicking
              author: {
                name: pr.author_login,
                avatar: '',
                badges: [],
              },
              repo: pr.projectName.split('/')[1] || pr.projectName,
              org: pr.projectName.split('/')[0] || '',
              indicators: [] as ('check' | 'x' | 'trophy' | 'eye' | 'code')[],
            };
            return <PRRow key={`${pr.github_pr_id}-${pr.projectName}`} pr={prForComponent} />;
          })
        ) : (
          <div className="text-center py-12">
            <p className={`text-[14px] font-medium mb-1 transition-colors ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              {selectedProjects.length === 0 
                ? 'Select repositories to view pull requests' 
                : 'No pull requests found in selected repositories'}
            </p>
            {selectedProjects.length === 0 && (
              <p className={`text-[12px] transition-colors ${
                theme === 'dark' ? 'text-[#8a7b6a]' : 'text-[#9a8b7a]'
              }`}>
                Use the repository selector above to choose which repositories to view
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// Helper function to format time ago
function formatTimeAgo(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);
  const diffMonths = Math.floor(diffDays / 30);
  const diffYears = Math.floor(diffDays / 365);

  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins} ${diffMins === 1 ? 'minute' : 'minutes'} ago`;
  if (diffHours < 24) return `${diffHours} ${diffHours === 1 ? 'hour' : 'hours'} ago`;
  if (diffDays < 30) return `${diffDays} ${diffDays === 1 ? 'day' : 'days'} ago`;
  if (diffMonths < 12) return `${diffMonths} ${diffMonths === 1 ? 'month' : 'months'} ago`;
  return `${diffYears} ${diffYears === 1 ? 'year' : 'years'} ago`;
}