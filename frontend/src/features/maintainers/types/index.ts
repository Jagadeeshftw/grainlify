import { LucideIcon } from "lucide-react";

// Tab 1: Dashboard types
export interface StatCard {
  id: number;
  title: string;
  subtitle: string;
  value: number;
  change: number;
  icon: LucideIcon;
}

export interface Activity {
  id: number;
  type: "pr" | "issue";
  number: number;
  title: string;
  label: string | null;
  timeAgo: string;
  timestamp?: string | Date | number;
  createdAt?: string | Date | number;
  projectId?: string;
}

export interface ChartDataPoint {
  month: string;
  applications: number;
  merged: number;
}

// Tab 2: Issues types (will add later)
export interface Applicant {
  name: string;
  appliedDate: string;
  badge?: string;
  stats?: ApplicantStat[];
  profileStats?: {
    contributions: number;
    rewards: number;
    contributorProjects: number;
    leadProjects: number;
  };
  message?: string;
}

export interface ApplicantStat {
  label: string;
  value: string;
  color: "golden" | "green" | "orange" | "red";
}

export interface Discussion {
  id: number;
  user: string;
  timeAgo: string;
  timestamp?: string | Date | number;
  createdAt?: string | Date | number;
  isAuthor?: boolean;
  appliedForContribution?: boolean;
  content: string;
}

export interface Issue {
  id: string | number; // Can be string (github_issue_id) or number (issue number)
  number?: number; // GitHub issue number (e.g., #1, #2)
  title: string;
  repo: string;
  repository?: string; // Alternative field name
  comments: number;
  applicants: number;
  tags: string[];
  user: string;
  timeAgo: string;
  icon?: "rocket" | "users" | "user";
  applicationStatus: "none" | "assigned" | "pending";
  applicant?: Applicant;
  discussions?: Discussion[];
  url?: string; // GitHub URL
}

export interface FilterState {
  status: string;
  applicants: string;
  assignee: string;
  stale: string;
  categories: string[];
  languages: string[];
  labels: string[];
}

// Tab 3: Pull Requests types
export interface PullRequest {
  id: number;
  number: number;
  title: string;
  status: "merged" | "draft" | "open" | "closed";
  statusDetail: string;
  url?: string; // GitHub URL for opening PR
  closes?: string;
  author: {
    name: string;
    avatar: string;
    badges: string[];
  };
  repo: string;
  org: string;
  indicators: ("check" | "x" | "trophy" | "eye" | "code")[];
}

export type PRFilterType = "All states" | "Open" | "Merged" | "Closed" | "Draft";

/**
 * A pull request that is linked to an issue, used by the PR-linking badge
 * on IssueCard components.
 */
export interface LinkedPR {
  id: number;
  number: number;
  title: string;
  status: 'open' | 'merged' | 'closed' | 'draft';
  /** Human-readable detail, e.g. "merged 2 days ago by JagadeeshFtw" */
  statusDetail: string;
  author: {
    name: string;
    /** GitHub avatar URL. Falls back to initials if absent or fails to load. */
    avatar?: string;
  };
  /** Full GitHub PR URL for the "Open on GitHub" link. */
  url?: string;
}

// Remove Waves from TabType
export type TabType = "Dashboard" | "Issues" | "Pull Requests" | "Analytics";

/** Analytics date-range filter periods */
export type AnalyticsPeriod = "7d" | "30d" | "90d" | "all";

/** One stage of the bounty conversion funnel */
export interface FunnelStage {
  name: string;
  value: number;
  fill: string;
}

/** Payout status values */
export type PayoutStatus = "paid" | "pending" | "processing" | "failed";

/** One row in the payout history table */
export interface PayoutRecord {
  id: string;
  date: string;           // ISO 8601
  contributor: string;    // GitHub username
  avatarUrl?: string;
  repository: string;     // "org/repo"
  amount: number;         // in XLM
  status: PayoutStatus;
}

/** One entry in the top-contributors module */
export interface TopContributor {
  rank: number;
  username: string;
  avatarUrl?: string;
  totalEarned: number;    // in XLM
  trend: "up" | "down" | "same";
  trendValue: number;     // absolute rank delta
}

// Shared types
export interface Repository {
  id: number;
  org: string;
  label?: string;
  repos: string[];
}

// ─── Program Creation Wizard types ───────────────────────────────────────────

// The four recipe types supported by the wizard.
export type RecipeType = "hackathon" | "bounty" | "grant" | "ongoing";

// A single payout bracket (e.g. "1st Place" = 50%).
export interface PayoutBracket {
  label: string;
  percentage: number;
}

// A milestone-gated release schedule entry.
export interface ScheduleEntry {
  milestone: string;
  releasePercentage: number;
  unlockAfterDays: number;
}

// Full wizard form state, passed between steps
export interface WizardFormState {
  // Step 1 — Recipe & details
  recipe: RecipeType | null;
  programName: string;
  ecosystemName: string;
  description: string;

  // Step 2 — Funding
  fundingAmount: string;
  fundingToken: "XLM" | "USDC" | "EURC";
  minBounty: string;
  maxBounty: string;

  // Step 3 — Schedule / brackets
  brackets: PayoutBracket[];
  scheduleEntries: ScheduleEntry[];
  useSchedule: boolean;
}

// Validation error state for a single wizard step.
export interface WizardStepError {
  step: 1 | 2 | 3 | 4;
  message: string;
}
