export type Severity = "low" | "medium" | "high" | "critical";

export type ActionType = "warn" | "pause" | "terminate";

export type ProgramStatus = "pending" | "warned" | "paused" | "terminated";

export interface FlagEntry {
  id: string;
  reason: string;
  reportedBy: string;
  reportedAt: string;
  severity: Severity;
  evidenceUrl?: string;
  automated: boolean;
}

export interface FlaggedProgram {
  id: string;
  name: string;
  description: string;
  severity: Severity;
  flagCount: number;
  reason: string;
  reportedBy: string;
  reportedAt: string;
  riskScore: number;
  status: ProgramStatus;
  programUrl: string;
  owner: string;
  createdAt: string;
  flagHistory: FlagEntry[];
}

export type AuditActionType = ActionType | "resolve";

export interface AuditEntry {
  id: string;
  programName: string;
  action: AuditActionType;
  performedBy: string;
  performedAt: string;
  reason: string;
  programId: string;
}
