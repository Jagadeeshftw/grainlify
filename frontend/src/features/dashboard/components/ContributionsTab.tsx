import { useState } from "react";
import {
  Filter,
  Clock,
  GitFork,
  Star,
  Search,
  X,
  Circle,
  Check,
  ChevronDown,
} from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";

export function ContributionsTab() {
  const { theme } = useTheme();
  const [isFilterOpen, setIsFilterOpen] = useState(false);
  const [selectedProject, setSelectedProject] = useState("");
  const [selectedRewards, setSelectedRewards] = useState<string[]>([
    "Rewarded",
    "Unrewarded",
  ]);
  const [isProjectSectionOpen, setIsProjectSectionOpen] = useState(true);
  const [isRewardedSectionOpen, setIsRewardedSectionOpen] = useState(true);

  const contributions = {
    applied: [
      {
        id: 1,
        title: "Add dark mode support to the dashboard",
        badge: "2001",
        time: "1 day ago",
        contributor: "contributor1",
        tag: "enhancement",
        tagType: "enhancement" as const,
      },
      {
        id: 2,
        title: "Improve accessibility in form components",
        badge: "1002",
        time: "1 day ago",
        contributor: "contributor1",
        tag: "good first issue",
        tagType: "enhancement" as const,
      },
      {
        id: 3,
        title: "Refactor authentication logic",
        badge: "2001",
        time: "2 days ago",
        contributor: "contributor1",
        tag: "refactor",
        tagType: "enhancement" as const,
      },
    ],
    assigned: [
      {
        id: 1,
        title: "Implement user profile page",
        badge: "2001",
        time: "1 day ago",
        project: "React",
        tag: "feature",
        tagType: "feature" as const,
      },
      {
        id: 2,
        title: "Add unit tests for API endpoints",
        badge: "2002",
        time: "2 days ago",
        project: "Next.js",
        tag: "testing",
        tagType: "enhancement" as const,
      },
    ],
    pending: [
      {
        id: 1,
        title: "Optimize database queries",
        badge: "3001",
        time: "1 day ago",
        project: "React",
        tag: "performance",
        tagType: "enhancement" as const,
      },
      {
        id: 2,
        title: "Add error boundary component",
        badge: "3002",
        time: "1 day ago",
        project: "Next.js",
        tag: "enhancement",
        tagType: "enhancement" as const,
      },
      {
        id: 3,
        title: "Refactor state management",
        badge: "3003",
        time: "1 day ago",
        project: "Modus",
        tag: "refactor",
        tagType: "enhancement" as const,
      },
    ],
    complete: [
      {
        id: 1,
        title: "Fix memory leak in event listeners",
        time: "20 days ago",
        project: "React",
        tag: "bug",
        tagType: "bug" as const,
      },
      {
        id: 2,
        title: "Implement caching layer",
        time: "10 days ago",
        project: "Next.js",
        tag: "enhancement",
        tagType: "enhancement" as const,
      },
      {
        id: 3,
        title: "Code review API reflections",
        time: "12 days ago",
        project: "Modus",
        tag: "enhancement",
        tagType: "enhancement" as const,
      },
    ],
  };

  return (
    <>
      <div className="space-y-4">
        {/* Filter and Search Bar */}
        <div className="flex items-center gap-2 sm:gap-3">
          {/* Filter Button */}
          <button
            onClick={() => setIsFilterOpen(true)}
            className={`h-10 sm:h-12 flex-shrink-0 w-10 sm:w-12 flex items-center justify-center rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] hover:border-[#c9983a]/40 transition-all ${
              theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
            }`}
          >
            <Filter className="w-5 h-5" />
          </button>

          {/* Search Bar */}
          <div className="relative flex-1">
            <Search
              className={`absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 z-10 transition-colors ${
                theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
              }`}
            />
            <input
              type="text"
              placeholder="Search"
              className={`w-full pl-12 pr-4 py-2.5 sm:py-3 rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/40 transition-all text-[13px] ${
                theme === "dark"
                  ? "text-[#f5efe5] placeholder-[#b8a898]"
                  : "text-[#2d2820] placeholder-[#7a6b5a]"
              }`}
            />
          </div>
        </div>

        {/* Desktop Kanban Board - Hidden on Mobile */}
        <div className="hidden md:block overflow-x-auto scrollbar-hide">
          <div className="flex gap-5 min-w-max pb-4">
            {/* Applied Column */}
            <div className="w-[320px] flex-shrink-0">
              <div
                className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-colors ${
                  theme === "dark"
                    ? "bg-white/[0.08] border-white/10"
                    : "bg-white/[0.15] border-white/25"
                }`}
              >
                <div className="flex items-center justify-between mb-4">
                  <h3
                    className={`text-[16px] font-bold transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    Applied{" "}
                    <span
                      className={
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }
                    >
                      3
                    </span>
                  </h3>
                </div>
                <div className="space-y-3">
                  {contributions.applied.map((item) => (
                    <div
                      key={item.id}
                      className={`rounded-[12px] border p-4 transition-all ${
                        theme === "dark"
                          ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                          : "bg-white/[0.2] border-white/30 hover:border-white/40"
                      }`}
                    >
                      <div className="flex items-start justify-between mb-3">
                        <h4
                          className={`text-[14px] font-semibold flex-1 pr-2 transition-colors ${
                            theme === "dark"
                              ? "text-[#f5f5f5]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {item.title}
                        </h4>
                        <span className="px-2 py-0.5 bg-green-500/20 border border-green-600/30 rounded-[6px] text-[10px] font-semibold text-green-800">
                          {item.badge}
                        </span>
                      </div>
                      <div
                        className={`flex items-center space-x-2 mb-3 text-[12px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        <Clock className="w-3.5 h-3.5" />
                        <span>{item.time}</span>
                      </div>
                      <div className="flex items-center space-x-2 mb-3">
                        <div className="w-5 h-5 rounded-full bg-gradient-to-br from-purple-500 to-pink-500" />
                        <span
                          className={`text-[12px] transition-colors ${
                            theme === "dark"
                              ? "text-[#d4d4d4]"
                              : "text-[#7a6b5a]"
                          }`}
                        >
                          {item.contributor}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span
                          className={`px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors ${
                            theme === "dark"
                              ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                              : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                          }`}
                        >
                          {item.tag}
                        </span>
                        <button className="text-[12px] text-[#c9983a] hover:text-[#a67c2e] font-medium flex items-center space-x-1">
                          <span>See application</span>
                          <GitFork className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Assigned Issue Column */}
            <div className="w-[320px] flex-shrink-0">
              <div
                className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-colors ${
                  theme === "dark"
                    ? "bg-white/[0.08] border-white/10"
                    : "bg-white/[0.15] border-white/25"
                }`}
              >
                <div className="flex items-center justify-between mb-4">
                  <h3
                    className={`text-[16px] font-bold transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    Assigned issue{" "}
                    <span
                      className={
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }
                    >
                      2
                    </span>
                  </h3>
                </div>
                <div className="space-y-3">
                  {contributions.assigned.map((item) => (
                    <div
                      key={item.id}
                      className={`rounded-[12px] border p-4 transition-all ${
                        theme === "dark"
                          ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                          : "bg-white/[0.2] border-white/30 hover:border-white/40"
                      }`}
                    >
                      <div className="flex items-start justify-between mb-3">
                        <h4
                          className={`text-[14px] font-semibold flex-1 pr-2 transition-colors ${
                            theme === "dark"
                              ? "text-[#f5f5f5]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {item.title}
                        </h4>
                        <span className="px-2 py-0.5 bg-green-500/20 border border-green-600/30 rounded-[6px] text-[10px] font-semibold text-green-800">
                          {item.badge}
                        </span>
                      </div>
                      <div
                        className={`flex items-center space-x-2 mb-3 text-[12px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        <Clock className="w-3.5 h-3.5" />
                        <span>{item.time}</span>
                      </div>
                      <div className="flex items-center space-x-2 mb-3">
                        <div className="w-5 h-5 rounded-full bg-gradient-to-br from-blue-500 to-cyan-500" />
                        <span
                          className={`text-[12px] transition-colors ${
                            theme === "dark"
                              ? "text-[#d4d4d4]"
                              : "text-[#7a6b5a]"
                          }`}
                        >
                          {item.project}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span
                          className={`px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors ${
                            theme === "dark"
                              ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                              : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                          }`}
                        >
                          {item.tag}
                        </span>
                        <button className="text-[12px] text-[#c9983a] hover:text-[#a67c2e] font-medium flex items-center space-x-1">
                          <span>See application</span>
                          <GitFork className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Pending Review Column */}
            <div className="w-[320px] flex-shrink-0">
              <div
                className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-colors ${
                  theme === "dark"
                    ? "bg-white/[0.08] border-white/10"
                    : "bg-white/[0.15] border-white/25"
                }`}
              >
                <div className="flex items-center justify-between mb-4">
                  <h3
                    className={`text-[16px] font-bold transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    Pending review{" "}
                    <span
                      className={
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }
                    >
                      3
                    </span>
                  </h3>
                </div>
                <div className="space-y-3">
                  {contributions.pending.map((item) => (
                    <div
                      key={item.id}
                      className={`rounded-[12px] border p-4 transition-all ${
                        theme === "dark"
                          ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                          : "bg-white/[0.2] border-white/30 hover:border-white/40"
                      }`}
                    >
                      <div className="flex items-start justify-between mb-3">
                        <h4
                          className={`text-[14px] font-semibold flex-1 pr-2 transition-colors ${
                            theme === "dark"
                              ? "text-[#f5f5f5]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {item.title}
                        </h4>
                        <span className="px-2 py-0.5 bg-purple-500/20 border border-purple-600/30 rounded-[6px] text-[10px] font-semibold text-purple-800">
                          {item.badge}
                        </span>
                      </div>
                      <div
                        className={`flex items-center space-x-2 mb-3 text-[12px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        <Clock className="w-3.5 h-3.5" />
                        <span>{item.time}</span>
                      </div>
                      <div className="flex items-center space-x-2 mb-3">
                        <div className="w-5 h-5 rounded-full bg-gradient-to-br from-green-500 to-emerald-500" />
                        <span
                          className={`text-[12px] transition-colors ${
                            theme === "dark"
                              ? "text-[#d4d4d4]"
                              : "text-[#7a6b5a]"
                          }`}
                        >
                          {item.project}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span
                          className={`px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors ${
                            theme === "dark"
                              ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                              : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                          }`}
                        >
                          {item.tag}
                        </span>
                        <button className="text-[12px] text-[#c9983a] hover:text-[#a67c2e] font-medium flex items-center space-x-1">
                          <span>See detail</span>
                          <Star className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Complete Column */}
            <div className="w-[320px] flex-shrink-0">
              <div
                className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-colors ${
                  theme === "dark"
                    ? "bg-white/[0.08] border-white/10"
                    : "bg-white/[0.15] border-white/25"
                }`}
              >
                <div className="flex items-center justify-between mb-4">
                  <h3
                    className={`text-[16px] font-bold transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    Complete{" "}
                    <span
                      className={
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }
                    >
                      3
                    </span>
                  </h3>
                </div>
                <div className="space-y-3">
                  {contributions.complete.map((item) => (
                    <div
                      key={item.id}
                      className={`rounded-[12px] border p-4 transition-all ${
                        theme === "dark"
                          ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                          : "bg-white/[0.2] border-white/30 hover:border-white/40"
                      }`}
                    >
                      <div className="flex items-start justify-between mb-3">
                        <h4
                          className={`text-[14px] font-semibold flex-1 pr-2 transition-colors ${
                            theme === "dark"
                              ? "text-[#f5f5f5]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {item.title}
                        </h4>
                      </div>
                      <div
                        className={`flex items-center space-x-2 mb-3 text-[12px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        <Clock className="w-3.5 h-3.5" />
                        <span>{item.time}</span>
                      </div>
                      <div className="flex items-center space-x-2 mb-3">
                        <div className="w-5 h-5 rounded-full bg-gradient-to-br from-orange-500 to-red-500" />
                        <span
                          className={`text-[12px] transition-colors ${
                            theme === "dark"
                              ? "text-[#d4d4d4]"
                              : "text-[#7a6b5a]"
                          }`}
                        >
                          {item.project}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span
                          className={`px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors ${
                            item.tagType === "bug"
                              ? "bg-red-500/15 border border-red-600/25 text-red-800"
                              : theme === "dark"
                                ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                                : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                          }`}
                        >
                          {item.tag}
                        </span>
                        <button className="text-[12px] text-[#c9983a] hover:text-[#a67c2e] font-medium flex items-center space-x-1">
                          <span>See detail</span>
                          <Star className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Mobile Vertical Layout - Hidden on Desktop */}
        <div className="md:hidden space-y-4">
          {/* Applied Section */}
          <div
            className={`backdrop-blur-[30px] rounded-[16px] border p-4 transition-colors ${
              theme === "dark"
                ? "bg-white/[0.08] border-white/10"
                : "bg-white/[0.15] border-white/25"
            }`}
          >
            <h3
              className={`text-[15px] font-bold mb-3 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Applied{" "}
              <span
                className={
                  theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                }
              >
                ({contributions.applied.length})
              </span>
            </h3>
            <div className="space-y-2">
              {contributions.applied.map((item) => (
                <div
                  key={item.id}
                  className={`rounded-[10px] border p-3 transition-all ${
                    theme === "dark"
                      ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                      : "bg-white/[0.2] border-white/30 hover:border-white/40"
                  }`}
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <h4
                      className={`text-[13px] font-semibold flex-1 transition-colors ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}
                    >
                      {item.title}
                    </h4>
                    <span className="px-1.5 py-0.5 bg-green-500/20 border border-green-600/30 rounded-[4px] text-[9px] font-semibold text-green-800 flex-shrink-0">
                      {item.badge}
                    </span>
                  </div>
                  <div
                    className={`flex items-center space-x-1.5 mb-2 text-[11px] transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    <Clock className="w-3 h-3" />
                    <span>{item.time}</span>
                  </div>
                  <div className="flex items-center gap-1.5 mb-2">
                    <span
                      className={`px-1.5 py-0.5 rounded-[4px] text-[10px] font-medium transition-colors ${
                        theme === "dark"
                          ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                          : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                      }`}
                    >
                      {item.tag}
                    </span>
                  </div>
                  <button className="text-[11px] text-[#c9983a] hover:text-[#a67c2e] font-medium w-full py-1.5">
                    See application
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Assigned Issue Section */}
          <div
            className={`backdrop-blur-[30px] rounded-[16px] border p-4 transition-colors ${
              theme === "dark"
                ? "bg-white/[0.08] border-white/10"
                : "bg-white/[0.15] border-white/25"
            }`}
          >
            <h3
              className={`text-[15px] font-bold mb-3 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Assigned issue{" "}
              <span
                className={
                  theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                }
              >
                ({contributions.assigned.length})
              </span>
            </h3>
            <div className="space-y-2">
              {contributions.assigned.map((item) => (
                <div
                  key={item.id}
                  className={`rounded-[10px] border p-3 transition-all ${
                    theme === "dark"
                      ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                      : "bg-white/[0.2] border-white/30 hover:border-white/40"
                  }`}
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <h4
                      className={`text-[13px] font-semibold flex-1 transition-colors ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}
                    >
                      {item.title}
                    </h4>
                    <span className="px-1.5 py-0.5 bg-green-500/20 border border-green-600/30 rounded-[4px] text-[9px] font-semibold text-green-800 flex-shrink-0">
                      {item.badge}
                    </span>
                  </div>
                  <div
                    className={`flex items-center space-x-1.5 mb-2 text-[11px] transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    <Clock className="w-3 h-3" />
                    <span>{item.time}</span>
                  </div>
                  <div className="flex items-center gap-1.5 mb-2">
                    <span
                      className={`px-1.5 py-0.5 rounded-[4px] text-[10px] font-medium transition-colors ${
                        theme === "dark"
                          ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                          : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                      }`}
                    >
                      {item.tag}
                    </span>
                  </div>
                  <button className="text-[11px] text-[#c9983a] hover:text-[#a67c2e] font-medium w-full py-1.5">
                    See application
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Pending Review Section */}
          <div
            className={`backdrop-blur-[30px] rounded-[16px] border p-4 transition-colors ${
              theme === "dark"
                ? "bg-white/[0.08] border-white/10"
                : "bg-white/[0.15] border-white/25"
            }`}
          >
            <h3
              className={`text-[15px] font-bold mb-3 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Pending review{" "}
              <span
                className={
                  theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                }
              >
                ({contributions.pending.length})
              </span>
            </h3>
            <div className="space-y-2">
              {contributions.pending.map((item) => (
                <div
                  key={item.id}
                  className={`rounded-[10px] border p-3 transition-all ${
                    theme === "dark"
                      ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                      : "bg-white/[0.2] border-white/30 hover:border-white/40"
                  }`}
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <h4
                      className={`text-[13px] font-semibold flex-1 transition-colors ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}
                    >
                      {item.title}
                    </h4>
                    <span className="px-1.5 py-0.5 bg-purple-500/20 border border-purple-600/30 rounded-[4px] text-[9px] font-semibold text-purple-800 flex-shrink-0">
                      {item.badge}
                    </span>
                  </div>
                  <div
                    className={`flex items-center space-x-1.5 mb-2 text-[11px] transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    <Clock className="w-3 h-3" />
                    <span>{item.time}</span>
                  </div>
                  <div className="flex items-center gap-1.5 mb-2">
                    <span
                      className={`px-1.5 py-0.5 rounded-[4px] text-[10px] font-medium transition-colors ${
                        theme === "dark"
                          ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                          : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                      }`}
                    >
                      {item.tag}
                    </span>
                  </div>
                  <button className="text-[11px] text-[#c9983a] hover:text-[#a67c2e] font-medium w-full py-1.5">
                    See detail
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Complete Section */}
          <div
            className={`backdrop-blur-[30px] rounded-[16px] border p-4 transition-colors ${
              theme === "dark"
                ? "bg-white/[0.08] border-white/10"
                : "bg-white/[0.15] border-white/25"
            }`}
          >
            <h3
              className={`text-[15px] font-bold mb-3 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Complete{" "}
              <span
                className={
                  theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                }
              >
                ({contributions.complete.length})
              </span>
            </h3>
            <div className="space-y-2">
              {contributions.complete.map((item) => (
                <div
                  key={item.id}
                  className={`rounded-[10px] border p-3 transition-all ${
                    theme === "dark"
                      ? "bg-white/[0.08] border-white/15 hover:border-white/25"
                      : "bg-white/[0.2] border-white/30 hover:border-white/40"
                  }`}
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <h4
                      className={`text-[13px] font-semibold flex-1 transition-colors ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}
                    >
                      {item.title}
                    </h4>
                    <Check className="w-4 h-4 text-green-600 flex-shrink-0" />
                  </div>
                  <div
                    className={`flex items-center space-x-1.5 mb-2 text-[11px] transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    <Clock className="w-3 h-3" />
                    <span>{item.time}</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <span
                      className={`px-1.5 py-0.5 rounded-[4px] text-[10px] font-medium transition-colors ${
                        item.tagType === "bug"
                          ? "bg-red-500/15 border border-red-600/25 text-red-800"
                          : theme === "dark"
                            ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                            : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                      }`}
                    >
                      {item.tag}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Filter Modal */}
      {isFilterOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 bg-black/40 backdrop-blur-sm z-[50] transition-opacity"
            onClick={() => setIsFilterOpen(false)}
          />

          {/* Filter Panel */}
          <div
            className={`fixed top-0 right-0 h-full w-[400px] backdrop-blur-[40px] border-l z-[60] shadow-[0_0_40px_rgba(0,0,0,0.15)] p-6 flex flex-col animate-slide-in-right transition-colors ${
              theme === "dark"
                ? "bg-[#2d2820]/95 border-white/10 text-[#f5efe5]"
                : "bg-[#e5ddd1]/95 border-white/30 text-[#2d2820]"
            }`}
          >
            {/* Header */}
            <div className="flex items-center justify-between mb-6">
              <h2
                className={`text-[18px] font-bold ${
                  theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"
                }`}
              >
                Filter contributions
              </h2>
              <button
                onClick={() => setIsFilterOpen(false)}
                className={`w-8 h-8 flex items-center justify-center rounded-lg transition-all ${
                  theme === "dark"
                    ? "hover:bg-white/[0.1]"
                    : "hover:bg-white/[0.3]"
                }`}
              >
                <X
                  className={`w-6 h-6 stroke-[2.5] ${
                    theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"
                  }`}
                />
              </button>
            </div>

            {/* Filter Content */}
            <div className="flex-1 space-y-5 overflow-y-auto scrollbar-hide">
              {/* Projects Section */}
              <div>
                <button
                  onClick={() => setIsProjectSectionOpen(!isProjectSectionOpen)}
                  className="w-full flex items-center justify-between mb-3"
                >
                  <div className="flex items-center gap-3">
                    <Circle
                      className={`w-5 h-5 ${theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
                    />
                    <span
                      className={`text-[14px] font-semibold ${theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"}`}
                    >
                      Projects
                    </span>
                  </div>
                  <ChevronDown
                    className={`w-5 h-5 transition-transform ${isProjectSectionOpen ? "rotate-180" : ""} ${
                      theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"
                    }`}
                  />
                </button>

                {isProjectSectionOpen && (
                  <div className="space-y-3">
                    {/* Search Field */}
                    <div className="relative">
                      <Search
                        className={`absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 ${
                          theme === "dark" ? "text-[#b8a898]" : "text-[#2d2820]"
                        }`}
                      />
                      <input
                        type="text"
                        placeholder="Search"
                        className={`w-full pl-10 pr-4 py-3 rounded-[12px] backdrop-blur-[30px] border transition-all text-[13px] focus:outline-none focus:border-[#c9983a]/40 ${
                          theme === "dark"
                            ? "bg-white/[0.05] border-white/10 text-[#f5efe5] placeholder-[#b8a898] focus:bg-white/[0.1]"
                            : "bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2]"
                        }`}
                      />
                    </div>

                    {/* Results Section */}
                    <div
                      className={`backdrop-blur-[20px] rounded-[12px] border p-4 ${
                        theme === "dark"
                          ? "bg-white/[0.05] border-white/10"
                          : "bg-white/[0.1] border-white/20"
                      }`}
                    >
                      <p
                        className={`text-[12px] text-center ${theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
                      >
                        No items found
                      </p>
                    </div>
                  </div>
                )}
              </div>

              {/* Rewarded Section */}
              <div>
                <button
                  onClick={() =>
                    setIsRewardedSectionOpen(!isRewardedSectionOpen)
                  }
                  className="w-full flex items-center justify-between mb-3"
                >
                  <div className="flex items-center gap-3">
                    <Circle
                      className={`w-5 h-5 ${theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
                    />
                    <span
                      className={`text-[14px] font-semibold ${theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"}`}
                    >
                      Rewarded
                    </span>
                    <span className="px-2 py-0.5 bg-[#c9983a] text-white text-[11px] font-semibold rounded-full">
                      {selectedRewards.length}
                    </span>
                  </div>
                  <ChevronDown
                    className={`w-5 h-5 transition-transform ${isRewardedSectionOpen ? "rotate-180" : ""} ${
                      theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"
                    }`}
                  />
                </button>

                {isRewardedSectionOpen && (
                  <div className="space-y-2">
                    {/* Main Dropdown showing current selection */}
                    <div
                      className={`px-4 py-3 rounded-[12px] backdrop-blur-[30px] border ${
                        theme === "dark"
                          ? "bg-white/[0.05] border-white/10"
                          : "bg-white/[0.15] border-white/25"
                      }`}
                    >
                      <span
                        className={`text-[13px] ${theme === "dark" ? "text-[#f5efe5]" : "text-[#2d2820]"}`}
                      >
                        {selectedRewards.join(", ")}
                      </span>
                    </div>

                    {/* Options List */}
                    <div className="space-y-1">
                      {["Rewarded", "Unrewarded"].map((reward) => (
                        <button
                          key={reward}
                          onClick={() => {
                            if (selectedRewards.includes(reward)) {
                              setSelectedRewards(
                                selectedRewards.filter((r) => r !== reward),
                              );
                            } else {
                              setSelectedRewards([...selectedRewards, reward]);
                            }
                          }}
                          className={`w-full px-4 py-3 rounded-[12px] text-left text-[13px] font-medium transition-all flex items-center justify-between ${
                            selectedRewards.includes(reward)
                              ? "bg-[#c9983a] text-white shadow-[0_4px_12px_rgba(201,152,58,0.3)]"
                              : theme === "dark"
                                ? "backdrop-blur-[20px] bg-white/[0.05] border border-white/10 text-[#f5efe5] hover:bg-white/[0.1]"
                                : "backdrop-blur-[20px] bg-white/[0.1] border border-white/20 text-[#2d2820] hover:bg-white/[0.15]"
                          }`}
                        >
                          <span>{reward}</span>
                          {selectedRewards.includes(reward) && (
                            <Check className="w-4 h-4" />
                          )}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Footer Buttons */}
            <div
              className={`flex items-center gap-3 mt-6 pt-6 border-t ${
                theme === "dark" ? "border-white/10" : "border-white/20"
              }`}
            >
              <button
                onClick={() => {
                  setSelectedProject("");
                  setSelectedRewards(["Rewarded", "Unrewarded"]);
                }}
                className={`flex-1 px-4 py-3 rounded-[12px] backdrop-blur-[30px] border text-[13px] font-semibold transition-all ${
                  theme === "dark"
                    ? "bg-white/[0.05] border-white/10 text-[#b8a898] hover:bg-white/[0.1]"
                    : "bg-white/[0.15] border-white/25 text-[#6b5d4d] hover:bg-white/[0.2]"
                }`}
              >
                Reset
              </button>
              <button
                onClick={() => setIsFilterOpen(false)}
                className="flex-1 px-4 py-3 rounded-[12px] bg-[#c9983a] text-white text-[13px] font-semibold shadow-[0_4px_12px_rgba(201,152,58,0.3)] hover:shadow-[0_6px_16px_rgba(201,152,58,0.4)] transition-all"
              >
                Save
              </button>
            </div>
          </div>
        </>
      )}
    </>
  );
}
