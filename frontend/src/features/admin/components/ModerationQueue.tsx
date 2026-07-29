import { useState, useMemo } from "react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { Shield, Search, Filter, Clock, GitFork, User, ChevronRight } from "lucide-react";
import { Checkbox } from "../../../app/components/ui/checkbox";
import { SeverityBadge } from "./SeverityBadge";
import type { Severity, FlaggedProgram } from "./types";

interface ModerationQueueProps {
  programs: FlaggedProgram[];
  onSelect: (id: string) => void;
  onSelectMultiple: (ids: string[]) => void;
  selectedIds: string[];
}

const severityRank: Record<Severity, number> = { critical: 4, high: 3, medium: 2, low: 1 };

export function ModerationQueue({ programs, onSelect, onSelectMultiple, selectedIds }: ModerationQueueProps) {
  const { theme } = useTheme();
  const isDark = theme === "dark";
  const [search, setSearch] = useState("");
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [sortBy, setSortBy] = useState<"severity" | "date" | "riskScore">("severity");

  const filtered = useMemo(() => {
    let result = [...programs];
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((p) => p.name.toLowerCase().includes(q) || p.reason.toLowerCase().includes(q));
    }
    if (severityFilter !== "all") {
      result = result.filter((p) => p.severity === severityFilter);
    }
    result.sort((a, b) => {
      if (sortBy === "severity") return severityRank[b.severity] - severityRank[a.severity];
      if (sortBy === "riskScore") return b.riskScore - a.riskScore;
      return new Date(b.reportedAt).getTime() - new Date(a.reportedAt).getTime();
    });
    return result;
  }, [programs, search, severityFilter, sortBy]);

  const allSelected = filtered.length > 0 && selectedIds.length === filtered.length;
  const toggleAll = () => {
    if (allSelected) {
      onSelectMultiple([]);
    } else {
      onSelectMultiple(filtered.map((p) => p.id));
    }
  };

  return (
    <div className="space-y-4">
      <div className={`backdrop-blur-[25px] rounded-[16px] border p-1 ${isDark ? "bg-white/[0.06] border-white/10" : "bg-white/[0.12] border-white/20"}`}>
        <div className="flex flex-col gap-3 p-3 sm:flex-row sm:items-center">
          <div className="relative flex-1">
            <Search className={`absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
            <input
              type="text"
              placeholder="Search flagged programs..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              aria-label="Search flagged programs"
              className={`w-full rounded-[10px] border py-2 pl-10 pr-3 text-[13px] outline-none transition-colors ${
                isDark
                  ? "border-white/10 bg-white/[0.06] text-[#e8dfd0] placeholder-[#b8a898] focus:border-[#c9983a]/50"
                  : "border-white/20 bg-white/40 text-[#2d2820] placeholder-[#7a6b5a] focus:border-[#c9983a]/30"
              }`}
            />
          </div>
          <div className="flex items-center gap-2">
            <Filter className={`h-4 w-4 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
            {(["all", "low", "medium", "high", "critical"] as const).map((s) => (
              <button
                key={s}
                onClick={() => setSeverityFilter(s)}
                className={`rounded-[8px] px-2.5 py-1.5 text-[12px] font-medium transition-all ${
                  severityFilter === s
                    ? isDark
                      ? "bg-[#c9983a]/20 text-[#e8c77f] border border-[#c9983a]/30"
                      : "bg-[#c9983a]/15 text-[#a67c2e] border border-[#c9983a]/20"
                    : isDark
                      ? "text-[#b8a898] hover:bg-white/[0.06] border border-transparent"
                      : "text-[#7a6b5a] hover:bg-white/30 border border-transparent"
                }`}
              >
                {s === "all" ? "All" : s.charAt(0).toUpperCase() + s.slice(1)}
              </button>
            ))}
          </div>
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
            aria-label="Sort by"
            className={`rounded-[8px] border px-2.5 py-1.5 text-[12px] font-medium outline-none ${
              isDark
                ? "bg-white/[0.06] border-white/10 text-[#e8dfd0]"
                : "bg-white/40 border-white/20 text-[#2d2820]"
            }`}
          >
            <option value="severity">Sort: Severity</option>
            <option value="riskScore">Sort: Risk Score</option>
            <option value="date">Sort: Date</option>
          </select>
        </div>
      </div>

      <div className="space-y-2" role="list" aria-label="Flagged programs list">
        {filtered.map((program) => {
          const isSelected = selectedIds.includes(program.id);
          return (
            <div
              key={program.id}
              role="listitem"
              className={`group backdrop-blur-[25px] rounded-[14px] border p-4 transition-all hover:scale-[1.01] cursor-pointer ${
                isDark
                  ? `${isSelected ? "bg-[#c9983a]/10 border-[#c9983a]/30" : "bg-white/[0.06] border-white/10"} hover:bg-white/[0.10]`
                  : `${isSelected ? "bg-[#c9983a]/10 border-[#c9983a]/30" : "bg-white/[0.12] border-white/20"} hover:bg-white/[0.18]`
              }`}
              onClick={() => onSelect(program.id)}
              onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(program.id); } }}
              tabIndex={0}
            >
              <div className="flex items-start gap-4">
                <div onClick={(e) => e.stopPropagation()} onKeyDown={(e) => e.stopPropagation()}>
                  <Checkbox
                    checked={isSelected}
                    onCheckedChange={() => {
                      if (isSelected) onSelectMultiple(selectedIds.filter((id) => id !== program.id));
                      else onSelectMultiple([...selectedIds, program.id]);
                    }}
                    aria-label={`Select ${program.name}`}
                  />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                    <div className="flex items-center gap-3 min-w-0">
                      <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-[10px] ${
                        isDark ? "bg-white/[0.08]" : "bg-white/40"
                      }`}>
                        <GitFork className={`h-5 w-5 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                      </div>
                      <div className="min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className={`text-[15px] font-bold truncate max-w-[200px] sm:max-w-[300px] ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>
                            {program.name}
                          </span>
                          <SeverityBadge severity={program.severity} count={program.flagCount} />
                        </div>
                        <p className={`mt-0.5 text-[13px] truncate max-w-md ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                          {program.reason}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-4 shrink-0">
                      <div className="hidden sm:flex items-center gap-1.5">
                        <div
                          className={`h-8 w-8 rounded-full flex items-center justify-center text-[11px] font-bold ${
                            program.riskScore >= 80
                              ? "bg-red-500/20 text-red-400"
                              : program.riskScore >= 50
                                ? "bg-orange-500/20 text-orange-400"
                                : "bg-amber-500/20 text-amber-400"
                          }`}
                        >
                          {program.riskScore}
                        </div>
                        <span className={`text-[11px] ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Risk</span>
                      </div>
                      <div className="flex items-center gap-2 text-[12px]">
                        <User className={`h-3.5 w-3.5 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                        <span className={`truncate max-w-[100px] ${isDark ? "text-[#d4d4d4]" : "text-[#57534e]"}`}>
                          {program.reportedBy}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 text-[12px]">
                        <Clock className={`h-3.5 w-3.5 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                        <span className={isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}>
                          {new Date(program.reportedAt).toLocaleDateString()}
                        </span>
                      </div>
                      <ChevronRight className={`h-4 w-4 opacity-0 group-hover:opacity-100 transition-opacity ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          );
        })}
        {filtered.length === 0 && (
          <div className={`flex flex-col items-center justify-center py-16 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
            <Shield className="h-12 w-12 mb-4 opacity-40" />
            <p className="text-[16px] font-medium">No flagged programs found</p>
            <p className="text-[13px] mt-1">Try adjusting your filters or search terms</p>
          </div>
        )}
      </div>
    </div>
  );
}
