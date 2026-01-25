import { useState, useEffect } from "react";
import { ChevronDown } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { getEcosystems } from "../../../shared/api/client";
import { FilterType } from "../types";

interface FiltersSectionProps {
  activeFilter: FilterType;
  onFilterChange: (filter: FilterType) => void;
  selectedEcosystem: EcosystemOption;
  onEcosystemChange: (ecosystem: EcosystemOption) => void;
  showDropdown: boolean;
  onToggleDropdown: () => void;
  isLoaded: boolean;
}

interface EcosystemOption {
  label: string;
  value: string;
}

export function FiltersSection({
  activeFilter,
  onFilterChange,
  selectedEcosystem,
  onEcosystemChange,
  showDropdown,
  onToggleDropdown,
  isLoaded,
  // ecosystems,
  // isLoadingEcosystems = false,
}: FiltersSectionProps) {
  const { theme } = useTheme();

  const [ecosystemOptions, setEcosystemOptions] = useState<EcosystemOption[]>([
    { label: "All Ecosystems", value: "all" },
  ]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const fetchEcosystems = async () => {
      try {
        setLoading(true);
        const data = await getEcosystems();

        const activeEcosystems = data.ecosystems
          .filter((e) => e.status === "active")
          .map((e) => ({
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

  return (
    <div
      className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[20px] border border-white/20 shadow-[0_4px_16px_rgba(0,0,0,0.06)] p-5 transition-all duration-700 delay-900 relative z-50 ${isLoaded ? "opacity-100 translate-y-0" : "opacity-0 translate-y-8"
        }`}
    >
      <div className="flex items-center justify-between flex-wrap gap-4">
        {(["overall", "rewards", "contributions"] as FilterType[]).map(
          (filter) => (
            <button
              key={filter}
              onClick={() => onFilterChange(filter)}
              className={`px-5 py-2.5 rounded-[12px] font-semibold text-[14px] transition-all duration-300 hover:scale-105 ${activeFilter === filter
                ? "bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_4px_16px_rgba(201,152,58,0.35)] border border-white/10 animate-pulse-subtle"
                : `backdrop-blur-[30px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#6b5d4d]"
                }`
                }`}
            >
              {filter === "overall"
                ? "Overall Leaderboard"
                : filter === "rewards"
                  ? "Total Rewards"
                  : "Total Contributions"}
            </button>
          ),
        )}

        <div className="relative z-[100]">
          <button
            onClick={onToggleDropdown}
            className="flex items-center gap-2 px-4 py-2.5 rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] hover:scale-105 transition-all duration-300"
          >
            <span
              className={`text-[13px] font-semibold transition-colors ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                }`}
            >
              {selectedEcosystem.label}
            </span>
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 ${showDropdown ? "rotate-180" : ""} ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                }`}
            />
          </button>
          {showDropdown && (
            <div className="absolute right-0 mt-2 w-[200px] backdrop-blur-[40px] bg-white/[0.18] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-[100] animate-dropdown-in">
              {loading ? (
                <div className="px-4 py-3 flex justify-center">
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                </div>
              ) : (
                ecosystemOptions.map((eco, index) => (
                  <button
                    key={eco.value}
                    onClick={() => {
                      onEcosystemChange({ label: eco.label, value: eco.value });
                      onToggleDropdown();
                    }}
                    className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${index === 0
                      ? `bg-white/[0.15] font-bold hover:bg-white/[0.25]`
                      : "hover:bg-white/[0.2]"
                      } ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
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
