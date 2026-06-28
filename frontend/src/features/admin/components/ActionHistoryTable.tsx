import { useMemo, useState } from "react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { Clock, AlertTriangle, PauseCircle, Ban, CheckCircle, Search, Shield } from "lucide-react";
import { Table, TableHeader, TableBody, TableHead, TableRow, TableCell } from "../../../app/components/ui/table";
import type { AuditActionType, AuditEntry } from "./types";

interface ActionHistoryTableProps {
  entries: AuditEntry[];
}

const actionConfig: Record<AuditActionType, { icon: typeof AlertTriangle; label: string; light: string; dark: string }> = {
  warn: { icon: AlertTriangle, label: "Warning", light: "text-amber-700 bg-amber-500/10", dark: "text-amber-300 bg-amber-500/20" },
  pause: { icon: PauseCircle, label: "Paused", light: "text-orange-700 bg-orange-500/10", dark: "text-orange-300 bg-orange-500/20" },
  terminate: { icon: Ban, label: "Terminated", light: "text-red-700 bg-red-500/10", dark: "text-red-300 bg-red-500/20" },
  resolve: { icon: CheckCircle, label: "Resolved", light: "text-green-700 bg-green-500/10", dark: "text-green-300 bg-green-500/20" },
};

export function ActionHistoryTable({ entries }: ActionHistoryTableProps) {
  const { theme } = useTheme();
  const isDark = theme === "dark";
  const [search, setSearch] = useState("");
  const [actionFilter, setActionFilter] = useState<AuditActionType | "all">("all");

  const filtered = useMemo(() => {
    let result = [...entries];
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((e) => e.programName.toLowerCase().includes(q) || e.performedBy.toLowerCase().includes(q) || e.reason.toLowerCase().includes(q));
    }
    if (actionFilter !== "all") {
      result = result.filter((e) => e.action === actionFilter);
    }
    result.sort((a, b) => new Date(b.performedAt).getTime() - new Date(a.performedAt).getTime());
    return result;
  }, [entries, search, actionFilter]);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-xs">
          <Search className={`absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
          <input
            type="text"
            placeholder="Search audit log..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            aria-label="Search audit log"
            className={`w-full rounded-[10px] border py-2 pl-10 pr-3 text-[13px] outline-none transition-colors ${
              isDark
                ? "border-white/10 bg-white/[0.06] text-[#e8dfd0] placeholder-[#b8a898] focus:border-[#c9983a]/50"
                : "border-white/20 bg-white/40 text-[#2d2820] placeholder-[#7a6b5a] focus:border-[#c9983a]/30"
            }`}
          />
        </div>
        <div className="flex items-center gap-1.5">
          {(["all", "warn", "pause", "terminate", "resolve"] as const).map((a) => (
            <button
              key={a}
              onClick={() => setActionFilter(a)}
              className={`rounded-[8px] px-2.5 py-1.5 text-[11px] font-medium transition-all ${
                actionFilter === a
                  ? isDark
                    ? "bg-[#c9983a]/20 text-[#e8c77f] border border-[#c9983a]/30"
                    : "bg-[#c9983a]/15 text-[#a67c2e] border border-[#c9983a]/20"
                  : isDark
                    ? "text-[#b8a898] hover:bg-white/[0.06] border border-transparent"
                    : "text-[#7a6b5a] hover:bg-white/30 border border-transparent"
              }`}
            >
              {a === "all" ? "All" : a.charAt(0).toUpperCase() + a.slice(1)}
            </button>
          ))}
        </div>
      </div>

      <div className={`rounded-[14px] border overflow-hidden ${isDark ? "border-white/10" : "border-white/20"}`}>
        <Table>
          <TableHeader>
            <TableRow className={isDark ? "border-white/10" : "border-white/20"}>
              <TableHead className={`text-[12px] font-semibold ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Action</TableHead>
              <TableHead className={`text-[12px] font-semibold ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Program</TableHead>
              <TableHead className={`text-[12px] font-semibold ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Reason</TableHead>
              <TableHead className={`text-[12px] font-semibold ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Performed By</TableHead>
              <TableHead className={`text-[12px] font-semibold ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Date</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((entry) => {
              const config = actionConfig[entry.action];
              const Icon = config.icon;
              const styles = isDark ? config.dark : config.light;
              return (
                <TableRow
                  key={entry.id}
                  className={`${isDark ? "border-white/5 hover:bg-white/[0.04]" : "border-white/10 hover:bg-white/20"}`}
                >
                  <TableCell>
                    <span className={`inline-flex items-center gap-1.5 rounded-[6px] px-2 py-1 text-[12px] font-medium ${styles}`}>
                      <Icon className="h-3.5 w-3.5" />
                      {config.label}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className={`text-[13px] font-medium ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>{entry.programName}</span>
                  </TableCell>
                  <TableCell>
                    <span className={`text-[13px] ${isDark ? "text-[#d4d4d4]" : "text-[#57534e]"}`}>{entry.reason}</span>
                  </TableCell>
                  <TableCell>
                    <span className={`text-[13px] ${isDark ? "text-[#d4d4d4]" : "text-[#57534e]"}`}>{entry.performedBy}</span>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1.5">
                      <Clock className={`h-3.5 w-3.5 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                      <span className={`text-[13px] ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                        {new Date(entry.performedAt).toLocaleDateString()}
                      </span>
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
            {filtered.length === 0 && (
              <TableRow>
                <TableCell colSpan={5} className="text-center py-12">
                  <div className={`flex flex-col items-center ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                    <Shield className="h-10 w-10 mb-3 opacity-30" />
                    <p className="text-[14px] font-medium">No audit entries found</p>
                    <p className="text-[12px] mt-1">No moderation actions have been recorded yet</p>
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
