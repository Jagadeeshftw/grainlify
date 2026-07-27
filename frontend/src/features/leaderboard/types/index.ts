export type FilterType = 'overall' | 'rewards' | 'contributions' | 'ecosystems';
export type LeaderboardType = 'contributors' | 'projects';
export type TimePeriod = 'weekly' | 'monthly' | 'all-time';
export type RoleFilter = 'all' | 'core' | 'contributor' | 'first-timer';
export type RankDirection = 'up' | 'down' | 'same';

export interface DeltaIndicator {
  direction: RankDirection;
  value: number; // positive for up, negative for down, 0 for same
}

export interface LeaderData {
  rank: number;
  rank_tier?: string;
  rank_tier_name?: string;
  username: string;
  avatar: string;
  user_id?: string;
  score: number;
  trend: RankDirection;
  trendValue: number;
  contributions?: number;
  ecosystems?: string[];
  previousRank?: number; // for rank-change animation
  role?: RoleFilter;
}

export interface ProjectData {
  rank: number;
  name: string;
  logo: string;
  score: number;
  trend: RankDirection;
  trendValue: number;
  contributors?: number;
  ecosystems?: string[];
  activity?: string;
  previousRank?: number;
}

/** Sort dimension for the leaderboard data view */
export type LeaderboardSort = 'earnings' | 'contributions' | 'reputation';

/** Adapter type for label-value dropdown options used with GlassDropdown */
export interface DropdownOption<T extends string = string> {
  label: string;
  value: T;
}

/**
 * Serializable filter state that maps 1:1 to URL search params.
 * Only non-default values are serialised.
 */
export interface URLFilterState {
  period?: TimePeriod;
  ecosystem?: string;
  role?: RoleFilter;
  sort?: LeaderboardSort;
  type?: LeaderboardType;
  filter?: FilterType;
}

export interface Petal {
  id: number;
  left: number;
  delay: number;
  duration: number;
  rotation: number;
  size: number;
}

export interface FilterState {
  timePeriod: TimePeriod;
  ecosystem: string;
  role: RoleFilter;
}

export interface EcosystemOption {
  label: string;
  value: string;
}
