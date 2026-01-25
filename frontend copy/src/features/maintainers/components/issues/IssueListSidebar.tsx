import { Search, Filter } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { Issue } from '../../types';
import { IssueCard } from './IssueCard';
import { IssueFilterDropdown } from './IssueFilterDropdown';

interface IssueListSidebarProps {
  issues: Issue[];
  issueFilter: string;
  setIssueFilter: (filter: string) => void;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  isFilterDropdownOpen: boolean;
  setIsFilterDropdownOpen: (open: boolean) => void;
  appliedFilterCount: number;
  onFilterClick: () => void;
  onIssueSelect: (issue: Issue) => void;
}

export function IssueListSidebar({
  issues,
  issueFilter,
  setIssueFilter,
  searchQuery,
  setSearchQuery,
  isFilterDropdownOpen,
  setIsFilterDropdownOpen,
  appliedFilterCount,
  onFilterClick,
  onIssueSelect,
}: IssueListSidebarProps) {
  const { theme } = useTheme();
  return (
    <div className={`w-[380px] backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] overflow-hidden flex flex-col transition-colors ${
      theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
    }`}>
      {/* Fixed Header Section */}
      <div className="p-6 flex-shrink-0">
        {/* All Dropdown + Filters */}
        <div className="flex items-center gap-3 mb-6">
          <IssueFilterDropdown
            value={issueFilter}
            onChange={setIssueFilter}
            isOpen={isFilterDropdownOpen}
            onToggle={() => setIsFilterDropdownOpen(!isFilterDropdownOpen)}
            onClose={() => setIsFilterDropdownOpen(false)}
          />

          <button 
            className="relative px-4 py-3 rounded-[14px] backdrop-blur-[25px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] hover:border-[#c9983a]/30 transition-all group" 
            onClick={onFilterClick}
          >
            <Filter className="w-4 h-4 text-[#7a6b5a] group-hover:text-[#c9983a] transition-colors" />
            <span className="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] border-2 border-[#d4c5b0] flex items-center justify-center text-[11px] font-bold text-white shadow-lg">
              {appliedFilterCount}
            </span>
          </button>
        </div>

        {/* Search Bar */}
        <div className="relative">
          <Search className={`absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`} />
          <input
            type="text"
            placeholder="Search"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={`w-full pl-11 pr-4 py-3 rounded-[14px] backdrop-blur-[25px] border text-[14px] focus:outline-none focus:border-[#c9983a]/40 focus:ring-2 focus:ring-[#c9983a]/20 transition-all ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] placeholder:text-[#8a7b6a] focus:bg-white/[0.12]'
                : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder:text-[#9a8b7a] focus:bg-white/[0.2]'
            }`}
          />
        </div>
      </div>

      {/* Scrollable Issues List */}
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        <div className="space-y-3">
          {issues.map((issue, idx) => (
            <IssueCard
              key={issue.id}
              issue={issue}
              index={idx}
              onClick={() => onIssueSelect(issue)}
            />
          ))}
        </div>
      </div>

      {/* Fixed Footer: Total Issues Count */}
      <div className={`px-6 py-5 border-t text-center flex-shrink-0 bg-gradient-to-t from-white/5 to-transparent transition-colors ${
        theme === 'dark' ? 'border-white/10' : 'border-white/20'
      }`}>
        <span className={`text-[13px] font-bold transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>{issues.length} issues</span>
      </div>
    </div>
  );
}