import { AlertTriangle, AlertCircle, Info, Skull, type LucideIcon } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";

type Severity = "low" | "medium" | "high" | "critical";

interface SeverityBadgeProps {
  severity: Severity;
  count?: number;
  className?: string;
}

interface SeverityConfig {
  icon: LucideIcon;
  label: string;
  light: { bg: string; text: string; border: string };
  dark: { bg: string; text: string; border: string };
}

const severityMap: Record<Severity, SeverityConfig> = {
  low: {
    icon: Info,
    label: "Low",
    light: { bg: "bg-blue-500/10", text: "text-blue-700", border: "border-blue-500/20" },
    dark: { bg: "bg-blue-500/20", text: "text-blue-300", border: "border-blue-500/30" },
  },
  medium: {
    icon: AlertCircle,
    label: "Medium",
    light: { bg: "bg-amber-500/10", text: "text-amber-700", border: "border-amber-500/20" },
    dark: { bg: "bg-amber-500/20", text: "text-amber-300", border: "border-amber-500/30" },
  },
  high: {
    icon: AlertTriangle,
    label: "High",
    light: { bg: "bg-orange-500/10", text: "text-orange-700", border: "border-orange-500/20" },
    dark: { bg: "bg-orange-500/20", text: "text-orange-300", border: "border-orange-500/30" },
  },
  critical: {
    icon: Skull,
    label: "Critical",
    light: { bg: "bg-red-500/10", text: "text-red-700", border: "border-red-500/20" },
    dark: { bg: "bg-red-500/20", text: "text-red-300", border: "border-red-500/30" },
  },
};

export function SeverityBadge({ severity, count, className = "" }: SeverityBadgeProps) {
  const { theme } = useTheme();
  const isDark = theme === "dark";
  const config = severityMap[severity];
  const Icon = config.icon;
  const styles = isDark ? config.dark : config.light;

  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-[8px] border px-2.5 py-1 text-[12px] font-semibold leading-none ${styles.bg} ${styles.text} ${styles.border} ${className}`}
      role="status"
      aria-label={`Severity: ${config.label}${count !== undefined ? `, ${count} flags` : ""}`}
    >
      <Icon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
      <span>{config.label}</span>
      {count !== undefined && count > 0 && (
        <span className={`ml-0.5 inline-flex h-4 min-w-[16px] items-center justify-center rounded-[4px] px-1 text-[10px] font-bold ${isDark ? "bg-white/15 text-white" : "bg-black/10 text-black"}`}>
          {count}
        </span>
      )}
    </span>
  );
}
