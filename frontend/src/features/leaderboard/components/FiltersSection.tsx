import { useState, useEffect, useRef } from "react";
import { ChevronDown, Users, Code, Sparkles } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { getEcosystems } from "../../../shared/api/client";
import type { FilterType, TimePeriod, RoleFilter, EcosystemOption } from "../types";

interface FiltersSectionProps {
  activeFilter: FilterType;
  onFilterChange: (filter: FilterType) => void;
  selectedEcosystem: EcosystemOption;
  onEcosystemChange: (ecosystem: EcosystemOption) => void;
  showDropdown: boolean;
  onToggleDropdown: () => void;
  isLoaded: boolean;
  timePeriod: TimePeriod;
  onTimePeriodChange: (period: TimePeriod) => void;
  roleFilter: RoleFilter;
  onRoleFilterChange: (role: RoleFilter) => void;
}

interface FilterOption {
  label: string;
  value: FilterType;
}

const roleOptions: { label: string; value: RoleFilter }[] = [
  { label: "All Roles", value: "all" },
  { label: "Core", value: "core" },
  { label: "Contributor", value: "contributor" },
  { label: "First Timer", value: "first-timer" },
];

const timePeriodOptions: { label: string; value: TimePeriod }[] = [
  { label: "Weekly", value: "weekly" },
  { label: "Monthly", value: "monthly" },
  { label: "All Time", value: "all-time" },
];

export function FiltersSection({
  activeFilter,
  onFilterChange,
  selectedEcosystem,
  onEcosystemChange,
  showDropdown,
  onToggleDropdown,
  isLoaded,
  timePeriod,
  onTimePeriodChange,
  roleFilter,
  onRoleFilterChange,
}: FiltersSectionProps) {
  const { theme } = useTheme();

  const [ecosystemOptions, setEcosystemOptions] = useState<EcosystemOption[]>([
    { label: "All Ecosystems", value: "all" },
  ]);
  const [loading, setLoading] = useState(false);
  const [showFilterDropdown, setShowFilterDropdown] = useState(false);
  const [showRoleDropdown, setShowRoleDropdown] = useState(false);
  const filterRef = useRef<HTMLDivElement>(null);
  const roleRef = useRef<HTMLDivElement>(null);
  const ecosystemRef = useRef<HTMLDivElement>(null);

  const filterOptions: FilterOption[] = [
    { label: "Overall Leaderboard", value: "overall" },
    { label: "Total Rewards", value: "rewards" },
    { label: "Total Contributions", value: "contributions" },
  ];

  const getActiveFilterLabel = () => {
    const activeOption = filterOptions.find(
      (option) => option.value === activeFilter
    );
    return activeOption?.label || "Overall Leaderboard";
  };

  useEffect(() => {
    const fetchEcosystems = async () => {
      try {
        setLoading(true);
        const data = await getEcosystems();
        const activeEcosystems = data.ecosystems
          .filter((e: { status: string }) => e.status === "active")
          .map((e: { name: string; slug: string }) => ({
            label: e.name,
            value: e.slug,
          }));
        setEcosystemOptions([
          { label: "All Ecosystems", value: "all" },
          ...activeEcosystems,
        ]);
      } catch (err) {
        console.error("Failed to fetch ecosystems:", err);
      } finally {
        setLoading(false);
      }
    };
    fetchEcosystems();
  }, []);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (filterRef.current && !filterRef.current.contains(e.target as Node)) {
        setShowFilterDropdown(false);
      }
      if (roleRef.current && !roleRef.current.contains(e.target as Node)) {
        setShowRoleDropdown(false);
      }
      if (ecosystemRef.current && !ecosystemRef.current.contains(e.target as Node)) {
        if (showDropdown) onToggleDropdown();
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showDropdown, onToggleDropdown]);

  const btnBase = `flex items-center gap-2 px-4 py-2.5 rounded-[12px] backdrop-blur-[30px] border transition-all duration-300 outline-2 outline-offset-2 outline-transparent focus-visible:outline-[#c9983a] focus-visible:outline focus-visible:border-[#c9983a]/60`;
  const btnTheme = theme === "dark"
    ? "bg-white/[0.08] border-white/15 hover:bg-white/[0.12]"
    : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2]";

  return (
    <div
      className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[20px] border border-white/20 shadow-[0_4px_16px_rgba(0,0,0,0.06)] p-5 transition-all duration-700 delay-900 relative z-50 ${
        isLoaded ? "opacity-100 translate-y-0" : "opacity-0 translate-y-8"
      }`}
      role="region"
      aria-label="Leaderboard filters"
    >
      {/* Time Period Tabs */}
      <div className="flex items-center justify-between flex-wrap gap-3 mb-4 pb-4 border-b border-white/10" role="tablist" aria-label="Time period">
        <div className="flex gap-1.5 bg-white/[0.08] rounded-[12px] p-1">
          {timePeriodOptions.map((opt) => (
            <button
              key={opt.value}
              role="tab"
              aria-selected={timePeriod === opt.value}
              onClick={() => onTimePeriodChange(opt.value)}
              className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all duration-300 outline-2 outline-offset-2 outline-transparent focus-visible:outline-[#c9983a] focus-visible:outline ${
                timePeriod === opt.value
                  ? "bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_2px_8px_rgba(162,121,44,0.3)]"
                  : theme === "dark"
                    ? "text-[#d4d4d4] hover:text-[#f5f5f5] hover:bg-white/[0.08]"
                    : "text-[#7a6b5a] hover:text-[#2d2820] hover:bg-white/[0.12]"
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-center justify-end flex-wrap gap-3">
        {/* Sort Filter Dropdown */}
        <div className="relative z-[100]" ref={filterRef}>
          <button
            onClick={() => {
              setShowFilterDropdown(!showFilterDropdown);
              setShowRoleDropdown(false);
              if (showDropdown) onToggleDropdown();
            }}
            className={`${btnBase} ${btnTheme}`}
            aria-haspopup="listbox"
            aria-expanded={showFilterDropdown}
            aria-label={`Sort by: ${getActiveFilterLabel()}`}
          >
            <span
              className={`text-[13px] font-semibold transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              {getActiveFilterLabel()}
            </span>
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 ${
                showFilterDropdown ? "rotate-180" : ""
              } ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
            />
          </button>
          {showFilterDropdown && (
            <div
              className={`absolute right-0 mt-2 w-[220px] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-[100] animate-dropdown-in ${
                theme === "dark" ? "bg-[#2d2820]/95" : "bg-white/95"
              }`}
              role="listbox"
              aria-label="Sort options"
            >
              {filterOptions.map((option) => (
                <button
                  key={option.value}
                  role="option"
                  aria-selected={activeFilter === option.value}
                  onClick={() => {
                    onFilterChange(option.value);
                    setShowFilterDropdown(false);
                  }}
                  className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${
                    activeFilter === option.value
                      ? `${theme === "dark" ? "bg-white/[0.08]" : "bg-white/[0.1]"} font-bold ${theme === "dark" ? "hover:bg-white/[0.12]" : "hover:bg-white/[0.15]"}`
                      : `${theme === "dark" ? "hover:bg-white/[0.08]" : "hover:bg-white/[0.1]"}`
                  } ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                >
                  {option.label}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Role Filter Dropdown */}
        <div className="relative z-[100]" ref={roleRef}>
          <button
            onClick={() => {
              setShowRoleDropdown(!showRoleDropdown);
              setShowFilterDropdown(false);
              if (showDropdown) onToggleDropdown();
            }}
            className={`${btnBase} ${btnTheme}`}
            aria-haspopup="listbox"
            aria-expanded={showRoleDropdown}
            aria-label={`Role filter: ${roleOptions.find(o => o.value === roleFilter)?.label}`}
          >
            {roleFilter === "core" ? (
              <Sparkles className="w-4 h-4 text-[#c9983a]" />
            ) : roleFilter === "first-timer" ? (
              <Code className="w-4 h-4 text-[#c9983a]" />
            ) : (
              <Users className="w-4 h-4 text-[#c9983a]" />
            )}
            <span
              className={`text-[13px] font-semibold transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              {roleOptions.find(o => o.value === roleFilter)?.label}
            </span>
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 ${
                showRoleDropdown ? "rotate-180" : ""
              } ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
            />
          </button>
          {showRoleDropdown && (
            <div
              className={`absolute right-0 mt-2 w-[180px] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-[100] animate-dropdown-in ${
                theme === "dark" ? "bg-[#2d2820]/95" : "bg-white/95"
              }`}
              role="listbox"
              aria-label="Role options"
            >
              {roleOptions.map((option) => (
                <button
                  key={option.value}
                  role="option"
                  aria-selected={roleFilter === option.value}
                  onClick={() => {
                    onRoleFilterChange(option.value);
                    setShowRoleDropdown(false);
                  }}
                  className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all flex items-center gap-2 ${
                    roleFilter === option.value
                      ? `${theme === "dark" ? "bg-white/[0.08]" : "bg-white/[0.1]"} font-bold`
                      : ""
                  } ${theme === "dark" ? "text-[#f5f5f5] hover:bg-white/[0.08]" : "text-[#2d2820] hover:bg-white/[0.1]"}`}
                >
                  {option.label}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Ecosystem Dropdown */}
        <div className="relative z-[100]" ref={ecosystemRef}>
          <button
            onClick={() => {
              onToggleDropdown();
              setShowFilterDropdown(false);
              setShowRoleDropdown(false);
            }}
            className={`${btnBase} ${btnTheme}`}
            aria-haspopup="listbox"
            aria-expanded={showDropdown}
            aria-label={`Ecosystem: ${selectedEcosystem.label}`}
          >
            <span
              className={`text-[13px] font-semibold transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              {selectedEcosystem.label}
            </span>
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 ${
                showDropdown ? "rotate-180" : ""
              } ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
            />
          </button>
          {showDropdown && (
            <div
              className={`absolute right-0 mt-2 w-[220px] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-[100] animate-dropdown-in ${
                theme === "dark" ? "bg-[#2d2820]/95" : "bg-white/95"
              }`}
              role="listbox"
              aria-label="Ecosystem options"
            >
              {loading ? (
                <div className="px-4 py-3 flex justify-center">
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" role="status" aria-label="Loading ecosystems" />
                </div>
              ) : (
                ecosystemOptions.map((eco) => (
                  <button
                    key={eco.value}
                    role="option"
                    aria-selected={selectedEcosystem.value === eco.value}
                    onClick={() => {
                      onEcosystemChange({ label: eco.label, value: eco.value });
                      onToggleDropdown();
                    }}
                    className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${
                      selectedEcosystem.value === eco.value
                        ? `font-bold ${theme === "dark" ? "bg-white/[0.08]" : "bg-white/[0.1]"}`
                        : ""
                    } ${theme === "dark" ? "text-[#f5f5f5] hover:bg-white/[0.08]" : "text-[#2d2820] hover:bg-white/[0.1]"}`}
                  >
                    {eco.label}
                  </button>
                ))
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
