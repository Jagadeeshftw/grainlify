import { LucideIcon } from 'lucide-react';

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
  type: 'pr' | 'issue';
  number: number;
  title: string;
  label: string | null;
  timeAgo: string;
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
  color: 'golden' | 'green' | 'orange' | 'red';
}

export interface Discussion {
  id: number;
  user: string;
  timeAgo: string;
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
  icon?: 'rocket' | 'users' | 'user';
  applicationStatus: 'none' | 'assigned' | 'pending';
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
  status: 'merged' | 'draft' | 'open' | 'closed';
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
  indicators: ('check' | 'x' | 'trophy' | 'eye' | 'code')[];
}

export type PRFilterType = 'All states' | 'Open' | 'Merged' | 'Closed' | 'Draft';

// Remove Waves from TabType
export type TabType = 'Dashboard' | 'Issues' | 'Pull Requests';

// Shared types
export interface Repository {
  id: number;
  org: string;
  label?: string;
  repos: string[];
}