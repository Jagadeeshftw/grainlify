import { X } from "lucide-react";
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
        setProjects(mappedProjects);
        setIsLoading(false);
        setHasError(false);
        return mappedProjects;
      });
    };

    loadProjects();
  }, [selectedFilters, fetchProjects]);

  return (
    <div className="space-y-6">
      {/* Active Filters Display */}
      {Object.values(selectedFilters).some((arr) => arr.length > 0) && (
        <div className="flex flex-wrap gap-2">
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
                  className="hover:text-red-200 transition-colors"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </span>
            )),
          )}
        </div>
      )}

      {/* Filters */}
      <div className="flex items-center flex-wrap gap-3">
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

      {/* Projects Grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
          {[...Array(8)].map((_, idx) => (
            <ProjectCardSkeleton key={idx} />
          ))}
        </div>
      ) : projects.length === 0 ? (
        <div
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
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
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
