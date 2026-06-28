import { useState } from "react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { X, ExternalLink, Flag, AlertTriangle, PauseCircle, Ban, User, Clock, Shield, FileText } from "lucide-react";
import { Button } from "../../../app/components/ui/button";
import { Separator } from "../../../app/components/ui/separator";
import { ScrollArea } from "../../../app/components/ui/scroll-area";
import { SeverityBadge } from "./SeverityBadge";
import type { ActionType, FlaggedProgram } from "./types";

interface ProgramModerationDrawerProps {
  program: FlaggedProgram | null;
  open: boolean;
  onClose: () => void;
  onAction: (action: ActionType, programId: string) => void;
}

export function ProgramModerationDrawer({ program, open, onClose, onAction }: ProgramModerationDrawerProps) {
  const { theme } = useTheme();
  const isDark = theme === "dark";
  const [confirmAction, setConfirmAction] = useState<ActionType | null>(null);

  if (!open || !program) return null;

  const actionConfig: Record<ActionType, { label: string; icon: typeof AlertTriangle; variant: "outline" | "destructive"; description: string }> = {
    warn: {
      label: "Send Warning",
      icon: AlertTriangle,
      variant: "outline",
      description: "This will notify the program owner with a formal warning. No immediate impact on the program.",
    },
    pause: {
      label: "Pause Program",
      icon: PauseCircle,
      variant: "outline",
      description: "This will temporarily disable the program. Contributions will be halted until re-enabled by an admin.",
    },
    terminate: {
      label: "Terminate Program",
      icon: Ban,
      variant: "destructive",
      description: "This will permanently remove the program from the platform. This action cannot be undone.",
    },
  };

  return (
    <div
      className={`fixed inset-y-0 right-0 z-50 w-full max-w-[540px] border-l shadow-2xl transition-transform duration-300 ${
        open ? "translate-x-0" : "translate-x-full"
      } ${isDark ? "bg-[#1a1714] border-white/10" : "bg-white border-white/20"}`}
      role="dialog"
      aria-modal="true"
      aria-label={`Moderation details for ${program.name}`}
    >
      <div className="flex h-full flex-col">
        <div className={`flex items-center justify-between px-6 py-4 border-b ${isDark ? "border-white/10" : "border-white/20"}`}>
          <div className="flex items-center gap-3">
            <Shield className={`h-5 w-5 ${isDark ? "text-[#c9983a]" : "text-[#a67c2e]"}`} />
            <h2 className={`text-[16px] font-bold ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>Moderation Review</h2>
          </div>
          <button
            onClick={onClose}
            className={`rounded-[8px] p-2 transition-colors ${isDark ? "hover:bg-white/[0.08] text-[#b8a898]" : "hover:bg-white/40 text-[#7a6b5a]"}`}
            aria-label="Close drawer"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <ScrollArea className="flex-1">
          <div className="px-6 py-5 space-y-6">
            <div className="space-y-3">
              <div className="flex items-start justify-between">
                <div>
                  <h3 className={`text-[22px] font-bold ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>{program.name}</h3>
                  <p className={`mt-1 text-[14px] ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>{program.description}</p>
                </div>
                <SeverityBadge severity={program.severity} count={program.flagCount} />
              </div>

              <div className="flex items-center gap-4 text-[13px]">
                <div className="flex items-center gap-1.5">
                  <User className={`h-4 w-4 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                  <span className={isDark ? "text-[#d4d4d4]" : "text-[#57534e]"}>by {program.owner}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <Clock className={`h-4 w-4 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
                  <span className={isDark ? "text-[#d4d4d4]" : "text-[#57534e]"}>Created {new Date(program.createdAt).toLocaleDateString()}</span>
                </div>
              </div>

              <a
                href={program.programUrl}
                target="_blank"
                rel="noopener noreferrer"
                className={`inline-flex items-center gap-1.5 text-[13px] font-medium ${
                  isDark ? "text-[#c9983a] hover:text-[#e8c77f]" : "text-[#a67c2e] hover:text-[#c9983a]"
                }`}
              >
                <ExternalLink className="h-4 w-4" />
                View Program
              </a>
            </div>

            <Separator className={isDark ? "bg-white/10" : "bg-white/30"} />

            <div>
              <div className="flex items-center gap-2 mb-3">
                <Flag className={`h-4 w-4 ${isDark ? "text-[#c9983a]" : "text-[#a67c2e]"}`} />
                <h4 className={`text-[14px] font-bold ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>
                  Flag History ({program.flagHistory.length})
                </h4>
              </div>
              <div className="space-y-2">
                {program.flagHistory.map((flag) => (
                  <div
                    key={flag.id}
                    className={`rounded-[12px] border p-3 ${
                      isDark ? "bg-white/[0.04] border-white/10" : "bg-white/30 border-white/20"
                    }`}
                  >
                    <div className="flex items-start justify-between mb-1.5">
                      <div className="flex items-center gap-2">
                        <SeverityBadge severity={flag.severity} />
                        {flag.automated && (
                          <span className={`text-[11px] px-1.5 py-0.5 rounded-[4px] ${
                            isDark ? "bg-purple-500/20 text-purple-300" : "bg-purple-500/10 text-purple-700"
                          }`}>Automated</span>
                        )}
                      </div>
                      <span className={`text-[11px] ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                        {new Date(flag.reportedAt).toLocaleDateString()}
                      </span>
                    </div>
                    <p className={`text-[13px] ${isDark ? "text-[#d4d4d4]" : "text-[#57534e]"}`}>{flag.reason}</p>
                    <div className="flex items-center gap-3 mt-1.5 text-[12px]">
                      <span className={isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}>
                        Reported by: {flag.reportedBy}
                      </span>
                      {flag.evidenceUrl && (
                        <a
                          href={flag.evidenceUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className={`inline-flex items-center gap-1 font-medium ${
                            isDark ? "text-[#c9983a] hover:text-[#e8c77f]" : "text-[#a67c2e] hover:text-[#c9983a]"
                          }`}
                        >
                          <FileText className="h-3.5 w-3.5" />
                          Evidence
                          <ExternalLink className="h-3 w-3" />
                        </a>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <Separator className={isDark ? "bg-white/10" : "bg-white/30"} />

            <div>
              <h4 className={`text-[14px] font-bold mb-3 ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}>Actions</h4>
              <div className="space-y-2">
                {(["warn", "pause", "terminate"] as const).map((action) => {
                  const config = actionConfig[action];
                  const Icon = config.icon;
                  return (
                    <div key={action}>
                      <Button
                        variant={config.variant}
                        onClick={() => setConfirmAction(action)}
                        className={`w-full justify-start gap-2.5 rounded-[10px] text-[13px] h-auto px-4 py-3 ${
                          action === "terminate"
                            ? isDark
                              ? "hover:bg-red-500/20 border-red-500/30 text-red-300"
                              : "hover:bg-red-500/10 border-red-500/30 text-red-700"
                            : ""
                        }`}
                      >
                        <Icon className="h-4 w-4" />
                        <span className="font-semibold">{config.label}</span>
                      </Button>
                      {confirmAction === action && (
                        <div className={`mt-2 rounded-[10px] border p-3 ${
                          isDark ? "bg-white/[0.06] border-white/10" : "bg-white/40 border-white/20"
                        }`}>
                          <p className={`text-[12px] mb-2 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                            {config.description}
                          </p>
                          <div className="flex gap-2">
                            <Button
                              variant={action === "terminate" ? "destructive" : "default"}
                              size="sm"
                              onClick={() => { onAction(action, program.id); setConfirmAction(null); }}
                              className="rounded-[8px] text-[12px]"
                            >
                              Confirm {config.label}
                            </Button>
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => setConfirmAction(null)}
                              className="rounded-[8px] text-[12px]"
                            >
                              Cancel
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
