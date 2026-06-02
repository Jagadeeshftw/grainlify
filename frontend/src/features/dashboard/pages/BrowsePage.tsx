import { X, SlidersHorizontal, Search, ChevronDown, Check } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { useState, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
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
  // Use higher‑resolution owner avatar so cards look crisp
  return `https://github.com/${owner}.png?size=200`;
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
  const [isFilterDrawerOpen, setIsFilterDrawerOpen] = useState(false);
  const [drawerOpenSections, setDrawerOpenSections] = useState<Record<string, boolean>>({});
  const drawerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

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

  const activeFilterCount = Object.values(selectedFilters).reduce(
    (sum, arr) => sum + arr.length, 0
  );

  const toggleDrawerSection = (section: string) => {
    setDrawerOpenSections(prev => ({
      ...prev,
      [section]: !prev[section],
    }));
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

  // Focus trap for filter drawer
  useEffect(() => {
    if (!isFilterDrawerOpen) return;

    const drawer = drawerRef.current;
    if (!drawer) return;

    const focusableElements = drawer.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const first = focusableElements[0];
    const last = focusableElements[focusableElements.length - 1];

    const handleTab = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsFilterDrawerOpen(false);
        triggerRef.current?.focus();
        return;
      }
      if (e.key !== "Tab") return;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first?.focus();
      }
    };

    document.addEventListener("keydown", handleTab);
    first?.focus();

    return () => document.removeEventListener("keydown", handleTab);
  }, [isFilterDrawerOpen]);

  const filterTypes = ["languages", "ecosystems", "categories", "tags"] as const;

  const renderFilterDrawer = () => {
    if (!isFilterDrawerOpen) return null;

    return createPortal(
      <>
        <div
          className="fixed inset-0 bg-black/50 backdrop-blur-sm z-40 transition-opacity"
          onClick={() => {
            setIsFilterDrawerOpen(false);
            triggerRef.current?.focus();
          }}
          aria-hidden="true"
        />
        <div
          ref={drawerRef}
          id="filter-drawer"
          role="dialog"
          aria-modal="true"
          aria-label="Filters"
          className={`fixed top-0 right-0 h-full w-[85vw] max-w-[400px] backdrop-blur-[40px] border-l z-50 shadow-[0_0_40px_rgba(0,0,0,0.15)] flex flex-col animate-slide-in-right ${
            theme === "dark"
              ? "bg-[#2d2820]/95 border-white/30"
              : "bg-[#e5ddd1]/95 border-white/30"
          }`}
        >
          {/* Drawer Header */}
          <div className="flex items-center justify-between px-6 py-5 border-b border-white/15">
            <h2 className={`text-[18px] font-bold ${
              theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
            }`}>
              Filters
              {activeFilterCount > 0 && (
                <span className="ml-2 px-2 py-0.5 bg-[#c9983a] text-white text-[11px] font-semibold rounded-full">
                  {activeFilterCount}
                </span>
              )}
            </h2>
            <button
              onClick={() => {
                setIsFilterDrawerOpen(false);
                triggerRef.current?.focus();
              }}
              aria-label="Close filters"
              className={`w-8 h-8 flex items-center justify-center rounded-lg transition-all ${
                theme === "dark"
                  ? "hover:bg-white/[0.1] text-[#f5f5f5]"
                  : "hover:bg-white/[0.3] text-[#2d2820]"
              }`}
            >
              <X className="w-5 h-5 stroke-[2.5]" />
            </button>
          </div>

          {/* Drawer Content — Accordion Sections */}
          <div className="flex-1 overflow-y-auto scrollbar-hide px-6 py-4 space-y-4">
            {filterTypes.map((filterType) => {
              const isOpen = drawerOpenSections[filterType] ?? (filterType === "languages");
              const options = filterOptions[filterType];
              const selected = selectedFilters[filterType];
              const searchTerm = searchTerms[filterType];

              return (
                <div key={filterType} className="border-b border-white/10 pb-4">
                  <button
                    onClick={() => toggleDrawerSection(filterType)}
                    aria-expanded={isOpen}
                    className="w-full flex items-center justify-between py-2 group"
                  >
                    <div className="flex items-center gap-2">
                      <span className={`text-[14px] font-semibold capitalize ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}>
                        {filterType}
                      </span>
                      {selected.length > 0 && (
                        <span className="px-2 py-0.5 bg-[#c9983a] text-white text-[10px] font-semibold rounded-full">
                          {selected.length}
                        </span>
                      )}
                    </div>
                    <ChevronDown className={`w-4 h-4 transition-transform duration-200 ${
                      isOpen ? "rotate-180" : ""
                    } ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`} />
                  </button>

                  {isOpen && (
                    <div className="pt-2 space-y-1">
                      {/* Search within filter section */}
                      <div className="relative mb-2">
                        <Search className={`absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 ${
                          theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                        }`} />
                        <input
                          type="text"
                          placeholder={`Search ${filterType}...`}
                          value={searchTerm}
                          onChange={(e) => setSearchTerms(prev => ({ ...prev, [filterType]: e.target.value }))}
                          className={`w-full pl-10 pr-3 py-2.5 rounded-[11px] border-[1.5px] focus:outline-none transition-all text-[13px] ${
                            theme === "dark"
                              ? "bg-[#1a1512] border-white/[0.2] text-[#f5f5f5] placeholder-[#9a8a7a] focus:border-[#c9983a]"
                              : "bg-white/[0.3] backdrop-blur-[20px] border-white/[0.4] text-[#2d2820] placeholder-[#8a7a6a] focus:border-[#c9983a]"
                          }`}
                        />
                      </div>

                      {/* Options */}
                      {options.filter(o => o.name.toLowerCase().includes((searchTerm || "").toLowerCase())).length > 0 ? (
                        options
                          .filter(o => o.name.toLowerCase().includes((searchTerm || "").toLowerCase()))
                          .map((option) => {
                            const isSelected = selected.includes(option.name);
                            return (
                              <button
                                key={option.name}
                                onClick={() => toggleFilter(filterType, option.name)}
                                role="option"
                                aria-selected={isSelected}
                                className={`w-full px-4 py-3 rounded-[12px] text-left text-[13px] font-medium transition-all flex items-center justify-between ${
                                  isSelected
                                    ? "bg-[#c9983a] text-white shadow-[0_4px_12px_rgba(201,152,58,0.3)]"
                                    : theme === "dark"
                                      ? "backdrop-blur-[20px] bg-white/[0.1] border border-white/20 text-[#f5f5f5] hover:bg-white/[0.15]"
                                      : "backdrop-blur-[20px] bg-white/[0.1] border border-white/20 text-[#2d2820] hover:bg-white/[0.15]"
                                }`}
                              >
                                <span className="truncate">{option.name}</span>
                                {isSelected && (
                                  <Check className="w-4 h-4 flex-shrink-0 ml-2" />
                                )}
                              </button>
                            );
                          })
                      ) : (
                        <div className={`px-4 py-6 text-center rounded-[12px] ${
                          theme === "dark" ? "bg-white/[0.05] text-[#b8a898]" : "bg-white/[0.1] text-[#7a6b5a]"
                        }`}>
                          <p className="text-[12px]">No options found</p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* Drawer Footer — Reset */}
          <div className="px-6 py-4 border-t border-white/15">
            <button
              onClick={() => {
                setSelectedFilters({
                  languages: [],
                  ecosystems: [],
                  categories: [],
                  tags: [],
                });
                setSearchTerms({
                  languages: "",
                  ecosystems: "",
                  categories: "",
                  tags: "",
                });
              }}
              disabled={activeFilterCount === 0}
              className={`w-full px-4 py-3 rounded-[12px] text-[13px] font-semibold transition-all ${
                activeFilterCount > 0
                  ? theme === "dark"
                    ? "bg-white/[0.15] border border-white/25 text-[#f5f5f5] hover:bg-white/[0.2]"
                    : "bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                  : "opacity-40 cursor-not-allowed"
              }`}
            >
              Reset all filters
            </button>
          </div>
        </div>
      </>,
      document.body
    );
  };

  return (
    <div className="space-y-6">
      {/* Active Filters Display — all breakpoints */}
      {activeFilterCount > 0 && (
        <div className="flex flex-wrap gap-2" role="status" aria-label="Active filters">
          {Object.entries(selectedFilters).map(([filterType, values]) =>
            values.map((value) => (
              <span
                key={`${filterType}-${value}`}
                className={`px-3.5 py-2 rounded-[10px] text-[13px] font-semibold border-[1.5px] flex items-center gap-2 transition-all hover:scale-105 shadow-lg ${
                  theme === "dark"
                    ? "bg-[#a17932] border-[#c9983a] text-white"
                    : "bg-[#b8872f] border-[#a17932] text-white"
                }`}
              >
                {value}
                <button
                  onClick={() => clearFilter(filterType, value)}
                  aria-label={`Remove ${value}`}
                  className="hover:text-red-200 transition-colors"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </span>
            )),
          )}
        </div>
      )}

      {/* Filters — desktop: inline dropdowns (lg+), mobile: hidden (drawer used instead) */}
      <div className="hidden lg:flex items-center flex-wrap gap-3">
        {filterTypes.map((filterType) => (
          <Dropdown
            key={filterType}
            filterType={filterType}
            options={filterOptions[filterType]}
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

      {/* Filter FAB — visible below lg */}
      <div className="fixed bottom-6 right-6 z-40 lg:hidden">
        <button
          ref={triggerRef}
          onClick={() => setIsFilterDrawerOpen(true)}
          aria-label="Open filters"
          aria-expanded={isFilterDrawerOpen}
          aria-controls="filter-drawer"
          className="w-14 h-14 rounded-full bg-gradient-to-br from-[#c9983a] to-[#b8872f] shadow-xl hover:shadow-2xl flex items-center justify-center transition-all hover:scale-105 active:scale-95"
        >
          <SlidersHorizontal className="w-6 h-6 text-white" />
          {activeFilterCount > 0 && (
            <span className="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-red-500 text-white text-[10px] font-bold flex items-center justify-center shadow-lg">
              {activeFilterCount > 9 ? "9+" : activeFilterCount}
            </span>
          )}
        </button>
      </div>

      {/* Filter Drawer Portal */}
      {renderFilterDrawer()}

      {/* Projects Grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 xl:grid-cols-5 gap-4 md:gap-5" aria-busy="true">
          {[...Array(8)].map((_, idx) => (
            <ProjectCardSkeleton key={idx} />
          ))}
        </div>
      ) : projects.length === 0 ? (
        <div
          role="status"
          className={`p-8 rounded-[16px] border text-center ${
            theme === "dark"
              ? "bg-white/[0.08] border-white/15 text-[#d4d4d4]"
              : "bg-white/[0.15] border-white/25 text-[#7a6b5a]"
          }`}
        >
          <p className="text-[16px] font-semibold">No projects found</p>
          <p className="text-[14px] mt-2">
            Try adjusting your filters or check back later.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 xl:grid-cols-5 gap-4 md:gap-5">
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
