import { useState } from "react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { AlertTriangle, Ban, PauseCircle, CheckSquare, Square } from "lucide-react";
import {
  AlertDialog,
  AlertDialogTrigger,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogAction,
  AlertDialogCancel,
} from "../../../app/components/ui/alert-dialog";
import { Button } from "../../../app/components/ui/button";
import { Checkbox } from "../../../app/components/ui/checkbox";

type BulkAction = "warn" | "pause" | "terminate";

interface BulkActionToolbarProps {
  selectedCount: number;
  totalCount: number;
  onSelectAll: () => void;
  onClearSelection: () => void;
  onBulkAction: (action: BulkAction) => void;
}

export function BulkActionToolbar({ selectedCount, totalCount, onSelectAll, onClearSelection, onBulkAction }: BulkActionToolbarProps) {
  const { theme } = useTheme();
  const isDark = theme === "dark";
  const [action, setAction] = useState<BulkAction | null>(null);

  if (selectedCount === 0) return null;

  const bulkActionConfig: Record<BulkAction, { label: string; icon: typeof AlertTriangle; destructive: boolean; description: string }> = {
    warn: {
      label: "Warn Selected",
      icon: AlertTriangle,
      destructive: false,
      description: `Send warnings to ${selectedCount} selected program(s). The program owners will be notified with a formal warning.`,
    },
    pause: {
      label: "Pause Selected",
      icon: PauseCircle,
      destructive: false,
      description: `Pause ${selectedCount} selected program(s). All contributions will be temporarily halted.`,
    },
    terminate: {
      label: "Terminate Selected",
      icon: Ban,
      destructive: true,
      description: `Permanently terminate ${selectedCount} selected program(s). This action cannot be undone. All associated data will be removed.`,
    },
  };

  return (
    <div
      className={`backdrop-blur-[25px] rounded-[14px] border p-3 flex items-center justify-between transition-all ${
        isDark ? "bg-white/[0.08] border-white/15" : "bg-white/[0.15] border-white/25"
      }`}
      role="toolbar"
      aria-label="Bulk actions toolbar"
    >
      <div className="flex items-center gap-3">
        <button
          onClick={() => {
            if (selectedCount === totalCount) onClearSelection();
            else onSelectAll();
          }}
          className={`flex items-center gap-2 rounded-[8px] px-3 py-1.5 text-[12px] font-medium transition-colors ${
            isDark ? "hover:bg-white/[0.08] text-[#d4d4d4]" : "hover:bg-white/30 text-[#57534e]"
          }`}
        >
          {selectedCount === totalCount ? <Square className="h-4 w-4" /> : <CheckSquare className="h-4 w-4" />}
          {selectedCount === totalCount ? "Deselect all" : "Select all"}
        </button>
        <span className={`text-[13px] font-medium ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
          {selectedCount} of {totalCount} selected
        </span>
        <button
          onClick={onClearSelection}
          className={`text-[12px] underline underline-offset-2 ${
            isDark ? "text-[#b8a898] hover:text-[#d4d4d4]" : "text-[#7a6b5a] hover:text-[#57534e]"
          }`}
        >
          Clear
        </button>
      </div>

      <div className="flex items-center gap-2">
        {(["warn", "pause", "terminate"] as const).map((a) => {
          const config = bulkActionConfig[a];
          return (
            <AlertDialog key={a}>
              <AlertDialogTrigger asChild>
                <Button
                  variant={config.destructive ? "destructive" : "outline"}
                  size="sm"
                  onClick={() => setAction(a)}
                  className={`rounded-[8px] text-[12px] gap-1.5 ${
                    config.destructive
                      ? ""
                      : isDark
                        ? "border-white/15 text-[#d4d4d4] hover:bg-white/[0.08]"
                        : "border-white/30 text-[#57534e] hover:bg-white/40"
                  }`}
                >
                  <config.icon className="h-3.5 w-3.5" />
                  {config.label}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent className={`${isDark ? "bg-[#1a1714] border-white/15" : "bg-white border-white/30"} max-w-[440px] rounded-[16px]`}>
                <AlertDialogHeader>
                  <div className={`mx-auto flex h-12 w-12 items-center justify-center rounded-full ${
                    config.destructive ? "bg-red-500/20" : isDark ? "bg-white/[0.08]" : "bg-black/10"
                  }`}>
                    <config.icon className={`h-6 w-6 ${config.destructive ? "text-red-400" : isDark ? "text-[#c9983a]" : "text-[#a67c2e]"}`} />
                  </div>
                  <AlertDialogTitle className={`text-center text-[16px] ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>
                    {config.label}
                  </AlertDialogTitle>
                  <AlertDialogDescription className="text-center text-[13px] leading-relaxed">
                    {config.description}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter className="sm:justify-center gap-2">
                  <AlertDialogCancel className={`rounded-[10px] text-[13px] ${isDark ? "border-white/15 text-[#d4d4d4]" : "border-white/30"}`}>
                    Cancel
                  </AlertDialogCancel>
                  <AlertDialogAction
                    onClick={() => onBulkAction(a)}
                    className={`rounded-[10px] text-[13px] font-semibold ${
                      config.destructive
                        ? "bg-red-600 hover:bg-red-700 text-white"
                        : "bg-[#c9983a] hover:bg-[#b8892e] text-white"
                    }`}
                  >
                    Yes, {config.destructive ? "Terminate" : "Continue"}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          );
        })}
      </div>
    </div>
  );
}
