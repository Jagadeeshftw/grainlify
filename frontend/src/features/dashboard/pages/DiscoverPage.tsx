import { useTheme } from "../../../shared/contexts/ThemeContext";
import { Heart, Star, GitFork, ArrowUpRight, Target, Zap } from "lucide-react";
import { IssueCard } from "../../../shared/components/ui/IssueCard";
import { useState, useEffect } from "react";
import { IssueDetailPage } from "./IssueDetailPage";
import { ProjectDetailPage } from "./ProjectDetailPage";
import {
  getRecommendedProjects,
  getPublicProjectIssues,
} from "../../../shared/api/client";
import { SkeletonLoader } from "../../../shared/components/SkeletonLoader";
import { useOptimisticData } from "../../../shared/hooks/useOptimisticData";

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

// Helper function to clean and truncate issue description
const cleanIssueDescription = (
  description: string | null | undefined,
  maxLines: number = 2,
  maxLength: number = 150,
): string => {
  if (!description || description.trim() === "") {
    return "";
  }

  // Remove markdown headers and formatting
  let cleaned = description
    // Remove markdown headers (##, ###, etc.)
    .replace(/^#{1,6}\s+/gm, "")
    // Remove bold/italic markdown (**text**, *text*)
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    // Remove common prefixes like "Description:", "**Description:**", etc.
    .replace(/^(Description|DESCRIPTION|description):\s*/i, "")
    .replace(/^\*\*Description:\*\*\s*/i, "")
    .replace(/^\*\*DESCRIPTION:\*\*\s*/i, "")
    // Remove leading/trailing whitespace
    .trim();

  // Split into lines and take first maxLines
  const lines = cleaned.split("\n").filter((line) => line.trim() !== "");
  const selectedLines = lines.slice(0, maxLines).join(" ").trim();

  // Truncate if too long
  if (selectedLines.length > maxLength) {
    return selectedLines.substring(0, maxLength).trim() + "...";
  }

  return selectedLines;
};

// Helper function to calculate days left (mock for now, can be enhanced with actual dates)
const getDaysLeft = (): string => {
  const days = Math.floor(Math.random() * 10) + 1;
  return `${days} days left`;
};

// Helper function to get primary tag from issue labels
const getPrimaryTag = (labels: any[]): string | undefined => {
  if (!Array.isArray(labels) || labels.length === 0) return undefined;

  // Check for common tags
  const tagMap: Record<string, string> = {
    "good first issue": "good first issue",
    "good-first-issue": "good first issue",
    bug: "bug",
    enhancement: "enhancement",
    feature: "feature",
    performance: "performance",
    a11y: "a11y",
    accessibility: "a11y",
  };

  for (const label of labels) {
    const labelName =
      typeof label === "string"
        ? label.toLowerCase()
        : (label?.name || "").toLowerCase();
    if (tagMap[labelName]) {
      return tagMap[labelName];
    }
  }

  return undefined;
};

type ProjectType = {
  id: string;
  name: string;
  icon: string;
  stars: string;
  forks: string;
  issues: number;
  description: string;
  tags: string[];
  color: string;
};

type IssueType = {
  id: string;
  title: string;
  description: string;
  language: string;
  daysLeft: string;
  primaryTag?: string;
  projectId: string;
};

interface DiscoverPageProps {
  onGoToBilling?: () => void;
  onGoToOpenSourceWeek?: () => void;
}

export function DiscoverPage({
  onGoToBilling,
  onGoToOpenSourceWeek,
}: DiscoverPageProps) {
  const { theme } = useTheme();
  const [selectedIssue, setSelectedIssue] = useState<{
    issueId: string;
    projectId?: string;
  } | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    null,
  );

  // Use optimistic data hook for projects with 30-second cache
  const {
    data: projects,
    isLoading: isLoadingProjects,
    fetchData: fetchProjects,
  } = useOptimisticData<ProjectType[]>([], { cacheDuration: 30000 });

  // Use optimistic data hook for issues with 30-second cache
  const {
    data: recommendedIssues,
    isLoading: isLoadingIssues,
    fetchData: fetchIssues,
  } = useOptimisticData<IssueType[]>([], { cacheDuration: 30000 });

  // Fetch recommended projects
  useEffect(() => {
    const loadRecommendedProjects = async () => {
      await fetchProjects(async () => {
        const response = await getRecommendedProjects(8);
        console.log("DiscoverPage: Recommended projects response", response);

        // Handle response - check if it exists and has projects array
        if (!response) {
          console.warn("DiscoverPage: No response received");
          return [];
        }

        // Handle both { projects: [...] } and direct array response
        const projectsArray =
          response.projects || (Array.isArray(response) ? response : []);

        if (!Array.isArray(projectsArray)) {
          console.error(
            "DiscoverPage: Invalid response format - projects is not an array",
            response,
          );
          return [];
        }

        const filteredProjects = projectsArray.filter((p) => {
          if (!p || !p.id || !p.github_full_name) return false;
          const repoName =
            p.github_full_name.split("/")[1] || p.github_full_name;
          return repoName !== ".github";
        });

        const mappedProjects = filteredProjects.map((p) => {
          const repoName = p.github_full_name.split('/')[1] || p.github_full_name;
          return {
            id: p.id,
            name: repoName,
            icon: getProjectIcon(p.github_full_name),
            stars: formatNumber(p.stars_count || 0),
            forks: formatNumber(p.forks_count || 0),
            issues: p.open_issues_count || 0,
            description: truncateDescription(p.description) || "",
            tags: Array.isArray(p.tags) ? p.tags.slice(0, 2) : [],
            color: getProjectColor(repoName),
          };
        });

        console.log("DiscoverPage: Mapped projects", mappedProjects);
        return mappedProjects;
      });
    };

    loadRecommendedProjects();
  }, [fetchProjects]);

  // Fetch recommended issues from top projects
  useEffect(() => {
    const loadRecommendedIssues = async () => {
      if (projects.length === 0) return;

      setIsLoadingIssues(true);
      const issues: Array<{
        id: string;
        title: string;
        description: string;
        language: string;
        daysLeft: string;
        primaryTag?: string;
        projectId: string;
      }> = [];

      // Try to get issues from projects, moving to next if a project has no issues
      for (const project of projects) {
        if (issues.length >= 6) break; // We only need 6 issues

        try {
          const issuesResponse = await getPublicProjectIssues(project.id);
          if (issuesResponse?.issues && Array.isArray(issuesResponse.issues) && issuesResponse.issues.length > 0) {
            // Take up to 2 issues from this project
            const projectIssues = issuesResponse.issues.slice(0, 2);
            for (const issue of projectIssues) {
              if (issues.length >= 6) break;

              // Get project language for the issue
              const projectData = projects.find(p => p.id === project.id);
              const language = projectData?.tags.find(t => ['TypeScript', 'JavaScript', 'Python', 'Rust', 'Go', 'CSS', 'HTML'].includes(t)) || projectData?.tags[0] || 'TypeScript';

              issues.push({
                id: String(issue.github_issue_id),
                title: issue.title || 'Untitled Issue',
                description: cleanIssueDescription(issue.description),
                language: language,
                daysLeft: getDaysLeft(),
                primaryTag: getPrimaryTag(issue.labels || []),
                projectId: project.id,
              });
            }
          }
        } catch (err) {
          // If fetching issues fails, continue to next project
          console.warn(`Failed to fetch issues for project ${project.id}:`, err);
          continue;
        }
      }

      setRecommendedIssues(issues);
      setIsLoadingIssues(false);
    };

    loadRecommendedIssues();
  }, [projects, isLoadingProjects, fetchIssues]);

  // If an issue is selected, show the detail page instead
  if (selectedIssue) {
    return (
      <IssueDetailPage
        issueId={selectedIssue.issueId}
        projectId={selectedIssue.projectId}
        onClose={() => setSelectedIssue(null)}
      />
    );
  }

  // If a project is selected, show the detail page instead
  if (selectedProjectId) {
    return (
      <ProjectDetailPage
        projectId={selectedProjectId}
        onClose={() => setSelectedProjectId(null)}
      />
    );
  }

  return (
    <div className="space-y-4 md:space-y-6 px-4 md:px-0 pb-8">
      {/* Hero Section */}
      <div className={`backdrop-blur-[40px] rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6 md:p-12 text-center transition-colors ${theme === 'dark'
        ? 'bg-gradient-to-br from-white/[0.08] to-white/[0.04] border-white/10'
        : 'bg-gradient-to-br from-white/[0.15] to-white/[0.08] border-white/20'
        }`}>
        <h1 className={`text-2xl md:text-[36px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>
          Get matched to your next
        </h1>
        <h2 className="text-3xl md:text-[42px] font-bold bg-gradient-to-r from-[#c9983a] via-[#a67c2e] to-[#8b7355] bg-clip-text text-transparent mb-4 md:mb-6">
          Open source contributions!
        </h2>
        <p className={`text-sm md:text-[16px] mb-6 md:mb-8 max-w-2xl mx-auto transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>
          Get matched automatically once you add your billing profile and verify your KYC so we can route rewards on-chain.
        </p>
        <button
          onClick={onGoToBilling}
          disabled={!onGoToBilling}
          className={`w-full md:w-auto px-6 py-3 md:px-8 md:py-4 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-sm md:text-[16px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all inline-flex items-center justify-center space-x-2 border border-white/10 ${!onGoToBilling ? 'opacity-70 cursor-default' : ''
            }`}
        >
          <span>Add billing profile & verify KYC (1/3)</span>
          <ArrowUpRight className="w-5 h-5 flex-shrink-0" />
        </button>
      </div>

      {/* Embark on GrainHack */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
          ? 'bg-gradient-to-br from-white/[0.1] to-white/[0.06] border-white/15'
          : 'bg-gradient-to-br from-white/[0.18] to-white/[0.12] border-white/25'
        }`}>
        <div className="flex items-center justify-between">
          <div className="flex-1">
            <h3 className={`text-[28px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>
              Join the <span className="text-[#c9983a]">GrainHack</span>
            </h3>
            <p className={`text-[16px] mb-6 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
              Join our GrainHack week and track your Open Source Week progress directly from your dashboard.
            </p>
            <button
              onClick={onGoToOpenSourceWeek}
              disabled={!onGoToOpenSourceWeek}
              className={`px-6 py-3 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.4)] transition-all border border-white/10 ${!onGoToOpenSourceWeek ? 'opacity-70 cursor-default' : ''}`}
            >
              Let's go
            </button>
          </div>
          <div className="w-16 h-16 md:w-24 md:h-24 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15 flex-shrink-0">
            <Target className="w-8 h-8 md:w-12 md:h-12 text-white" />
          </div>
        </div>
      </div>

      {/* Recommended Projects */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.12] border-white/20'
        }`}>
        <div className="flex items-center space-x-3 mb-2">
          <Zap className="w-6 h-6 text-[#c9983a] drop-shadow-sm" />
          <h3 className={`text-[24px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
            Recommended Projects ({projects.length})
          </h3>
        </div>
        <p className={`text-[14px] mb-6 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
          Finding best suited your interests and expertise
        </p>

        {isLoadingProjects ? (
          <div className="flex flex-col md:flex-row gap-4 md:gap-6 overflow-x-auto pb-2">
            {[...Array(4)].map((_, idx) => (
              <div key={idx} className={`flex-shrink-0 w-full md:w-[320px] rounded-[20px] border p-6 ${theme === 'dark' ? 'bg-white/[0.08] border-white/15' : 'bg-white/[0.15] border-white/25'}`}>
                {/* Icon and Heart button */}
                <div className="flex items-start justify-between mb-4">
                  <SkeletonLoader
                    variant="default"
                    className="w-12 h-12 rounded-[14px]"
                  />
                  <SkeletonLoader
                    variant="default"
                    className="w-5 h-5 rounded-full"
                  />
                </div>

                {/* Title */}
                <SkeletonLoader className="h-5 w-3/4 mb-2" />

                {/* Description */}
                <SkeletonLoader className="h-3 w-full mb-1" />
                <SkeletonLoader className="h-3 w-5/6 mb-4" />

                {/* Stars and Forks */}
                <div className="flex items-center space-x-4 mb-4">
                  <SkeletonLoader className="h-4 w-16" />
                  <SkeletonLoader className="h-4 w-16" />
                </div>

                {/* Tags */}
                <div className="flex flex-wrap gap-2">
                  <SkeletonLoader className="h-7 w-20 rounded-[10px]" />
                  <SkeletonLoader className="h-7 w-24 rounded-[10px]" />
                </div>
              </div>
            ))}
          </div>
        ) : projects.length === 0 ? (
          <div className={`p-8 rounded-[16px] border text-center ${theme === 'dark'
            ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]'
            : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
            }`}>
            <p className="text-[16px] font-semibold">No recommended projects found</p>
          </div>
        ) : (
          <div className="flex flex-col md:flex-row gap-4 md:gap-6 md:overflow-x-auto pb-2 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
            {projects.map((project) => (
              <div
                key={project.id}
                onClick={() => setSelectedProjectId(String(project.id))}
                className={`backdrop-blur-[30px] rounded-[20px] border p-6 transition-all cursor-pointer flex-shrink-0 w-full md:w-[320px] ${theme === 'dark'
                  ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12] hover:shadow-[0_8px_24px_rgba(201,152,58,0.15)]'
                  : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.08)]'
                  }`}
              >
                <div className="flex items-start justify-between mb-4">
                  {project.icon.startsWith("http") ? (
                    <img
                      src={project.icon}
                      alt={project.name}
                      className="w-12 h-12 rounded-[14px] border border-white/20 flex-shrink-0"
                      onError={(e) => {
                        (e.target as HTMLImageElement).src =
                          `https://github.com/github.png?size=40`;
                      }}
                    />
                  ) : (
                    <div
                      className={`w-12 h-12 rounded-[14px] bg-gradient-to-br ${project.color} flex items-center justify-center shadow-md text-2xl`}
                    >
                      {project.icon}
                    </div>
                  )}
                  <button className="text-[#c9983a] hover:text-[#a67c2e] transition-colors">
                    <Heart className="w-5 h-5" />
                  </button>
                </div>

                <h4
                  className={`text-[18px] font-bold mb-2 transition-colors ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.name}
                </h4>
                <p
                  className={`text-[13px] mb-4 line-clamp-2 transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  {project.description}
                </p>

                <div
                  className={`flex items-center space-x-4 text-[13px] mb-4 transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  <div className="flex items-center space-x-1">
                    <Star className="w-3.5 h-3.5 text-[#c9983a]" />
                    <span>{project.stars}</span>
                  </div>
                  <div className="flex items-center space-x-1">
                    <GitFork className="w-3.5 h-3.5 text-[#c9983a]" />
                    <span>{project.forks}</span>
                  </div>
                </div>

                <div className="flex flex-wrap gap-2">
                  {project.tags.map((tag, idx) => (
                    <span
                      key={idx}
                      className={`px-3 py-1.5 rounded-[10px] border text-[12px] font-semibold shadow-[0_2px_8px_rgba(201,152,58,0.15)] ${theme === 'dark'
                        ? 'bg-[#c9983a]/15 border-[#c9983a]/30 text-[#f5c563]'
                        : 'bg-[#c9983a]/20 border-[#c9983a]/35 text-[#8b6f3a]'
                        }`}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Recommended Issues */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.12] border-white/20'
        }`}>
        <h3 className={`text-[24px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
          Recommended Issues
        </h3>
        <p className={`text-[14px] mb-6 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
          Issues that match your interests and expertise
        </p>

        {isLoadingIssues ? (
          <div className="flex flex-col md:flex-row gap-4 md:overflow-x-auto pb-2">
            {[...Array(3)].map((_, idx) => (
              <div key={idx} className={`flex-shrink-0 w-full md:w-[480px] rounded-[16px] border p-6 ${theme === 'dark' ? 'bg-white/[0.08] border-white/15' : 'bg-white/[0.15] border-white/25'}`}>
                {/* Title with status indicator */}
                <div className="flex items-start gap-3 mb-3">
                  <SkeletonLoader
                    variant="circle"
                    className="w-5 h-5 flex-shrink-0"
                  />
                  <SkeletonLoader className="h-5 w-3/4" />
                </div>

                {/* Description */}
                <div className="ml-8 mb-4">
                  <SkeletonLoader className="h-4 w-full mb-1" />
                  <SkeletonLoader className="h-4 w-5/6" />
                </div>

                {/* Bottom row: Language, Days Left, Tag */}
                <div className="flex items-center justify-between ml-8">
                  <div className="flex items-center gap-3">
                    <SkeletonLoader variant="circle" className="w-4 h-4" />
                    <SkeletonLoader className="h-4 w-20" />
                    <SkeletonLoader className="h-4 w-16" />
                  </div>
                  <SkeletonLoader className="h-6 w-24 rounded-[6px]" />
                </div>
              </div>
            ))}
          </div>
        ) : recommendedIssues.length === 0 ? (
          <div className={`p-8 rounded-[16px] border text-center ${theme === 'dark'
            ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]'
            : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
            }`}>
            <p className="text-[16px] font-semibold">No recommended issues found</p>
            <p className="text-[14px] mt-2">Try checking back later or explore projects manually.</p>
          </div>
        ) : (
          <div className="flex flex-col md:flex-row gap-4 md:overflow-x-auto pb-2 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
            {recommendedIssues.map((issue) => (
              <div key={issue.id} className="flex-shrink-0 w-full md:w-[480px]">
                <IssueCard
                  id={issue.id}
                  title={issue.title}
                  description={issue.description}
                  language={issue.language}
                  daysLeft={issue.daysLeft}
                  variant="recommended"
                  primaryTag={issue.primaryTag}
                  onClick={() =>
                    setSelectedIssue({
                      issueId: issue.id,
                      projectId: issue.projectId,
                    })
                  }
                />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
