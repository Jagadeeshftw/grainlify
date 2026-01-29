import { X, Filter, SlidersHorizontal } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { useState, useEffect } from "react";
import { Dropdown } from "../../../shared/components/ui/Dropdown";
import { ProjectCard, Project } from "../components/ProjectCard";
import { ProjectCardSkeleton } from "../components/ProjectCardSkeleton";
import { getPublicProjects, getEcosystems } from "../../../shared/api/client";
import {
  isValidProject,
  getRepoName,
} from "../../../shared/utils/projectFilter";

import { useOptimisticData } from "../../../shared/hooks/useOptimisticData";

interface BrowsePageProps {
  onProjectClick?: (id: string) => void;
}

// Helper function to format numbers (e.g., 1234 -> "1.2K", 1234567 -> "1.2M")
const formatNumber = (num: number): string => {
  if (num >= 1000000) {
    return `${(num / 1000000).toFixed(1)}M`;
  }
  if (num >= 1000) {
    return `${(num / 1000).toFixed(1)}K`;
  }
  return num.toString();
};

// Helper function to get project icon/avatar
const getProjectIcon = (githubFullName: string): string => {
  const [owner] = githubFullName.split("/");
  return `https://github.com/${owner}.png?size=40`;
};

// Helper function to get gradient color based on project name
const getProjectColor = (name: string): string => {
  const colors = [
    "from-blue-500 to-cyan-500",
    "from-purple-500 to-pink-500",
    "from-green-500 to-emerald-500",
    "from-red-500 to-pink-500",
    "from-orange-500 to-red-500",
    "from-gray-600 to-gray-800",
    "from-green-600 to-green-800",
    "from-cyan-500 to-blue-600",
  ];
  const hash = name
    .split("")
    .reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
};

// Helper function to truncate description to first line or first 80 characters
const truncateDescription = (
  description: string | undefined | null,
  maxLength: number = 80,
): string => {
  if (!description || description.trim() === "") {
    return "";
  }

  // Get first line
  const firstLine = description.split("\n")[0].trim();

  // If first line is longer than maxLength, truncate it
  if (firstLine.length > maxLength) {
    return firstLine.substring(0, maxLength).trim() + "...";
  }

  return firstLine;
};

export function BrowsePage({ onProjectClick }: BrowsePageProps) {
  const { theme } = useTheme();
  const [openDropdown, setOpenDropdown] = useState<string | null>(null);
  const [isMobileFiltersOpen, setIsMobileFiltersOpen] = useState(false);
  const [searchTerms, setSearchTerms] = useState<{ [key: string]: string }>({
    languages: "",
    ecosystems: "",
    categories: "",
    tags: "",
  });
  const [selectedFilters, setSelectedFilters] = useState<{
    [key: string]: string[];
  }>({
    languages: [],
    ecosystems: [],
    categories: [],
    tags: [],
  });

  // Use optimistic data hook for projects with 30-second cache
  const {
    data: projects,
    isLoading,
    hasError,
    fetchData: fetchProjects,
  } = useOptimisticData<Project[]>([], { cacheDuration: 30000 });

  const [ecosystems, setEcosystems] = useState<Array<{ name: string }>>([]);
  const [isLoadingEcosystems, setIsLoadingEcosystems] = useState(true);

  // Filter options data
  const filterOptions = {
    languages: [
      { name: "TypeScript" },
      { name: "JavaScript" },
      { name: "Python" },
      { name: "Go" },
      { name: "Rust" },
      { name: "Java" },
    ],
    ecosystems: ecosystems,
    categories: [
      { name: "Frontend" },
      { name: "Backend" },
      { name: "Full Stack" },
      { name: "DevOps" },
      { name: "Mobile" },
    ],
    tags: [
      { name: "Good first issues" },
      { name: "Open issues" },
      { name: "Help wanted" },
      { name: "Bug" },
      { name: "Feature" },
      { name: "Documentation" },
    ],
  };

  // Fetch ecosystems from API
  useEffect(() => {
    const fetchEcosystems = async () => {
      setIsLoadingEcosystems(true);
      try {
        const response = await getEcosystems();
        // Handle different response structures
        let ecosystemsArray: any[] = [];

        if (response && Array.isArray(response)) {
          ecosystemsArray = response;
        } else if (
          response &&
          response.ecosystems &&
          Array.isArray(response.ecosystems)
        ) {
          ecosystemsArray = response.ecosystems;
        } else if (response && typeof response === "object") {
          // Try to find any array property
          const keys = Object.keys(response);
          for (const key of keys) {
            if (Array.isArray((response as any)[key])) {
              ecosystemsArray = (response as any)[key];
              break;
            }
          }
        }

        // Filter only active ecosystems and map to expected format
        const activeEcosystems = ecosystemsArray
          .filter((eco: any) => eco.status === "active")
          .map((eco: any) => ({ name: eco.name }));

        setEcosystems(activeEcosystems);
      } catch (err) {
        console.error("BrowsePage: Failed to fetch ecosystems:", err);
        // Fallback to empty array on error
        setEcosystems([]);
      } finally {
        setIsLoadingEcosystems(false);
      }
    };

    fetchEcosystems();
  }, []);

  const toggleFilter = (filterType: string, value: string) => {
    setSelectedFilters((prev) => ({
      ...prev,
      [filterType]: prev[filterType].includes(value)
        ? prev[filterType].filter((v) => v !== value)
        : [...prev[filterType], value],
    }));
  };

  const clearFilter = (filterType: string, value: string) => {
    setSelectedFilters((prev) => ({
      ...prev,
      [filterType]: prev[filterType].filter((v) => v !== value),
    }));
  };

  const clearAllFilters = () => {
    setSelectedFilters({
      languages: [],
      ecosystems: [],
      categories: [],
      tags: [],
    });
  };

  const getFilteredOptions = (filterType: string) => {
    const searchTerm = searchTerms[filterType].toLowerCase();
    return filterOptions[filterType as keyof typeof filterOptions].filter(
      (option: any) => option.name.toLowerCase().includes(searchTerm),
    );
  };

  // Fetch projects from API
  useEffect(() => {
    const loadProjects = async () => {
      await fetchProjects(async () => {
        try {
          const params: {
            language?: string;
            ecosystem?: string;
            category?: string;
            tags?: string;
          } = {};

          // Apply filters
          if (selectedFilters.languages.length > 0) {
            params.language = selectedFilters.languages[0]; // API supports single language
          }
          if (selectedFilters.ecosystems.length > 0) {
            params.ecosystem = selectedFilters.ecosystems[0]; // API supports single ecosystem
          }
          if (selectedFilters.categories.length > 0) {
            params.category = selectedFilters.categories[0]; // API supports single category
          }
          if (selectedFilters.tags.length > 0) {
            params.tags = selectedFilters.tags.join(','); // API supports comma-separated tags
          }

          const response = await getPublicProjects(params);

          console.log('BrowsePage: API response received', { response });

          // Handle response - check if it's valid
          let projectsArray: any[] = [];
          if (response && response.projects && Array.isArray(response.projects)) {
            projectsArray = response.projects;
          } else if (Array.isArray(response)) {
            // Handle case where API returns array directly
            projectsArray = response;
          } else {
            console.warn('BrowsePage: Unexpected response format', response);
            projectsArray = [];
          }

          // Map API response to Project interface
          const mappedProjects: Project[] = projectsArray
            .filter(isValidProject)
            .map((p) => {
              const repoName = getRepoName(p.github_full_name);
              return {
                id: p.id || `project-${Date.now()}-${Math.random()}`, // Fallback ID if missing
                name: repoName,
                icon: getProjectIcon(p.github_full_name),
                stars: formatNumber(p.stars_count || 0),
                forks: formatNumber(p.forks_count || 0),
                contributors: p.contributors_count || 0,
                openIssues: p.open_issues_count || 0,
                prs: p.open_prs_count || 0,
                description: truncateDescription(p.description) || `${p.language || 'Project'} repository${p.category ? ` - ${p.category}` : ''}`,
                tags: Array.isArray(p.tags) ? p.tags : [],
                color: getProjectColor(repoName),
              };
            });

          console.log('BrowsePage: Mapped projects', { count: mappedProjects.length });
          return mappedProjects;
        } catch (err) {
          console.error('BrowsePage: Failed to fetch projects:', err);
          throw err; // Re-throw to let the hook handle the error
        }
      });
    };

    loadProjects();
  }, [selectedFilters, fetchProjects]);

  const totalActiveFilters = Object.values(selectedFilters).reduce(
    (acc, arr) => acc + arr.length,
    0,
  );

  return (
    <div className="space-y-4 sm:space-y-6">
      {/* Active Filters Display */}
      {Object.values(selectedFilters).some((arr) => arr.length > 0) && (
        <div className="flex flex-wrap gap-2">
          {Object.entries(selectedFilters).map(([filterType, values]) =>
            values.map((value) => (
              <span
                key={`${filterType}-${value}`}
                className={`px-3 py-1.5 sm:px-3.5 sm:py-2 rounded-[10px] text-xs sm:text-[13px] font-semibold border-[1.5px] flex items-center gap-1.5 sm:gap-2 transition-all hover:scale-105 shadow-lg ${
                  theme === "dark"
                    ? "bg-[#a17932] border-[#c9983a] text-white"
                    : "bg-[#b8872f] border-[#a17932] text-white"
                }`}
              >
                {value}
                <button
                  onClick={() => clearFilter(filterType, value)}
                  className="hover:text-red-200 transition-colors p-0.5"
                  aria-label={`Remove ${value} filter`}
                >
                  <X className="w-3 h-3" />
                </button>
              </span>
            )),
          )}
        </div>
      )}

      {/* Mobile Filter Button */}
      <div className="lg:hidden">
        <button
          onClick={() => setIsMobileFiltersOpen(true)}
          className={`w-full py-2.5 px-4 rounded-[12px] border-[1.5px] text-[14px] font-semibold flex items-center justify-center gap-2 transition-all ${
            totalActiveFilters > 0
              ? "bg-[#a17932] border-[#c9983a] text-white"
              : theme === "dark"
                ? "bg-white/[0.08] border-white/15 text-[#d4d4d4] hover:bg-white/[0.12]"
                : "bg-white/[0.15] border-white/25 text-[#6b5d4d] hover:bg-white/[0.2]"
          }`}
        >
          <SlidersHorizontal className="w-4 h-4" />
          Filters
          {totalActiveFilters > 0 && (
            <span className="bg-white/20 px-1.5 py-0.5 rounded-full text-xs">
              {totalActiveFilters}
            </span>
          )}
        </button>
      </div>

      {/* Desktop Filters */}
      <div className="hidden lg:flex items-center flex-wrap gap-3">
        {["languages", "ecosystems", "categories", "tags"].map((filterType) => (
          <Dropdown
            key={filterType}
            filterType={filterType}
            options={filterOptions[filterType as keyof typeof filterOptions]}
            selectedValues={selectedFilters[filterType]}
            onToggle={(value) => toggleFilter(filterType, value)}
            searchValue={searchTerms[filterType]}
            onSearchChange={(value) =>
              setSearchTerms((prev) => ({ ...prev, [filterType]: value }))
            }
            isOpen={openDropdown === filterType}
            onToggleOpen={() =>
              setOpenDropdown(openDropdown === filterType ? null : filterType)
            }
            onClose={() => setOpenDropdown(null)}
          />
        ))}
      </div>

      {/* Mobile Filter Modal */}
      {isMobileFiltersOpen && (
        <div className="fixed inset-0 z-50 lg:hidden">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={() => setIsMobileFiltersOpen(false)}
          />
          
          {/* Modal Content */}
          <div className="absolute inset-x-0 bottom-0 bg-[#2d2820]/[0.98] rounded-t-[24px] border-t border-[#c9983a]/30 animate-in slide-in-from-bottom duration-300 max-h-[85vh] overflow-y-auto">
            {/* Header */}
            <div className={`sticky top-0 px-5 py-4 flex items-center justify-between border-b border-white/15 backdrop-blur-[20px] ${
              theme === "dark" ? "bg-[#2d2820]/[0.95]" : "bg-[#d4c5b0]/[0.95]"
            }`}>
              <div className="flex items-center gap-3">
                <Filter className="w-5 h-5 text-[#c9983a]" />
                <h2 className="text-[18px] font-bold text-white">Filters</h2>
                {totalActiveFilters > 0 && (
                  <span className="bg-[#c9983a] text-white px-2 py-0.5 rounded-full text-xs">
                    {totalActiveFilters}
                  </span>
                )}
              </div>
              <button
                onClick={() => setIsMobileFiltersOpen(false)}
                className="p-2 rounded-lg hover:bg-white/10 transition-colors"
                aria-label="Close filters"
              >
                <X className="w-5 h-5 text-white" />
              </button>
            </div>

            {/* Filter Options */}
            <div className="p-5 space-y-6">
              {["languages", "ecosystems", "categories", "tags"].map((filterType) => (
                <div key={filterType}>
                  <h3 className="text-[15px] font-semibold text-white mb-3 capitalize">
                    {filterType}
                  </h3>
                  
                  {/* Search Input */}
                  <div className="relative mb-3">
                    <Search 
                      className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#9a8a7a]" 
                    />
                    <input
                      type="text"
                      placeholder={`Search ${filterType}...`}
                      value={searchTerms[filterType]}
                      onChange={(e) =>
                        setSearchTerms((prev) => ({ ...prev, [filterType]: e.target.value }))
                      }
                      className="w-full pl-10 pr-3 py-2.5 rounded-[11px] bg-white/[0.1] border border-white/20 text-white placeholder-[#9a8a7a] text-[14px] focus:outline-none focus:border-[#c9983a] focus:shadow-[0_0_0_3px_rgba(201,152,58,0.15)]"
                    />
                  </div>

                  {/* Options */}
                  <div className="space-y-2 max-h-[200px] overflow-y-auto">
                    {getFilteredOptions(filterType).map((option) => {
                      const isSelected = selectedFilters[filterType].includes(option.name);
                      
                      return (
                        <button
                          key={option.name}
                          onClick={() => toggleFilter(filterType, option.name)}
                          className={`w-full px-4 py-3 rounded-[12px] transition-all flex items-center gap-3 ${
                            isSelected
                              ? "bg-[#c9983a]/20 border border-[#c9983a]/40"
                              : "bg-white/[0.05] border border-transparent hover:bg-white/[0.1]"
                          }`}
                        >
                          <div className={`w-5 h-5 rounded-[5px] border-2 flex items-center justify-center transition-all ${
                            isSelected
                              ? "bg-[#c9983a] border-[#c9983a]"
                              : "border-white/30"
                          }`}>
                            {isSelected && (
                              <svg className="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="3">
                                <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                              </svg>
                            )}
                          </div>
                          <span className={`text-[14px] font-medium ${
                            isSelected ? "text-[#f5c563]" : "text-white"
                          }`}>
                            {option.name}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>

            {/* Footer Actions */}
            <div className="sticky bottom-0 px-5 py-4 border-t border-white/15 bg-[#2d2820]/[0.95] backdrop-blur-[20px]">
              <div className="flex gap-3">
                {totalActiveFilters > 0 && (
                  <button
                    onClick={clearAllFilters}
                    className="flex-1 py-3 px-4 rounded-[12px] border border-white/20 text-white font-semibold text-[14px] hover:bg-white/10 transition-all"
                  >
                    Clear All
                  </button>
                )}
                <button
                  onClick={() => setIsMobileFiltersOpen(false)}
                  className="flex-1 py-3 px-4 rounded-[12px] bg-[#c9983a] text-white font-semibold text-[14px] hover:bg-[#b8872f] transition-all"
                >
                  Apply Filters
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Projects Grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 sm:gap-5">
          {[...Array(8)].map((_, idx) => (
            <ProjectCardSkeleton key={idx} />
          ))}
        </div>
      ) : projects.length === 0 ? (
        <div
          className={`p-6 sm:p-8 rounded-[16px] border text-center ${
            theme === "dark"
              ? "bg-white/[0.08] border-white/15 text-[#d4d4d4]"
              : "bg-white/[0.15] border-white/25 text-[#7a6b5a]"
          }`}
        >
          <p className="text-[15px] sm:text-[16px] font-semibold">No projects found</p>
          <p className="text-[13px] sm:text-[14px] mt-2">
            Try adjusting your filters or check back later.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 sm:gap-5">
          {projects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              onClick={onProjectClick}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// Search icon component for inline use
function Search({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
      />
    </svg>
  );
}
