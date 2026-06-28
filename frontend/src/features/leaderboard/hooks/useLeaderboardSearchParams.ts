import { useCallback, useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import type {
  URLFilterState,
  TimePeriod,
  RoleFilter,
  LeaderboardType,
  FilterType,
  LeaderboardSort,
} from "../types";

const DEFAULT_PERIOD: TimePeriod = "all-time";
const DEFAULT_ECOSYSTEM = "all";
const DEFAULT_ROLE: RoleFilter = "all";
const DEFAULT_SORT: LeaderboardSort = "earnings";
const DEFAULT_TYPE: LeaderboardType = "contributors";
const DEFAULT_FILTER: FilterType = "overall";

const VALID_PERIODS = new Set<string>(["weekly", "monthly", "all-time"]);
const VALID_ROLES = new Set<string>(["all", "core", "contributor", "first-timer"]);
const VALID_SORTS = new Set<string>(["earnings", "contributions", "reputation"]);
const VALID_TYPES = new Set<string>(["contributors", "projects"]);
const VALID_FILTERS = new Set<string>(["overall", "rewards", "contributions", "ecosystems"]);

function parseParam<T>(value: string | null, valid: Set<string>, fallback: T): T {
  if (value && valid.has(value)) return value as unknown as T;
  if (value) console.warn(`[leaderboard] Unknown filter value "${value}" — using default`);
  return fallback;
}

function validateOrFallback<T>(value: T | undefined, valid: Set<string>, fallback: T): T {
  if (value === undefined) return fallback;
  return valid.has(value as string) ? value : fallback;
}

export interface UseLeaderboardSearchParamsReturn {
  filters: URLFilterState;
  setFilters: (partial: Partial<URLFilterState>) => void;
  resetFilters: () => void;
  shareURL: string;
  isDefault: boolean;
  copyShareLink: () => Promise<boolean>;
}

export function useLeaderboardSearchParams(): UseLeaderboardSearchParamsReturn {
  const [searchParams, setSearchParams] = useSearchParams();

  const filters = useMemo<URLFilterState>(() => ({
    period: parseParam<TimePeriod>(searchParams.get("period"), VALID_PERIODS, DEFAULT_PERIOD),
    ecosystem: searchParams.get("ecosystem") ?? DEFAULT_ECOSYSTEM,
    role: parseParam<RoleFilter>(searchParams.get("role"), VALID_ROLES, DEFAULT_ROLE),
    sort: parseParam<LeaderboardSort>(searchParams.get("sort"), VALID_SORTS, DEFAULT_SORT),
    type: parseParam<LeaderboardType>(searchParams.get("type"), VALID_TYPES, DEFAULT_TYPE),
    filter: parseParam<FilterType>(searchParams.get("filter"), VALID_FILTERS, DEFAULT_FILTER),
  }), [searchParams]);

  const setFilters = useCallback((partial: Partial<URLFilterState>) => {
    setSearchParams((prev) => {
      const next = new URLSearchParams(prev);

      const setIfNonDefault = (key: string, value: string | undefined, defaultValue: string) => {
        if (value !== undefined && value !== defaultValue) {
          next.set(key, value);
        } else if (value !== undefined) {
          next.delete(key);
        }
      };

      if ("period" in partial) {
        const v = validateOrFallback(partial.period, VALID_PERIODS, DEFAULT_PERIOD);
        setIfNonDefault("period", v, DEFAULT_PERIOD);
      }
      if ("ecosystem" in partial) {
        setIfNonDefault("ecosystem", partial.ecosystem ?? DEFAULT_ECOSYSTEM, DEFAULT_ECOSYSTEM);
      }
      if ("role" in partial) {
        const v = validateOrFallback(partial.role, VALID_ROLES, DEFAULT_ROLE);
        setIfNonDefault("role", v, DEFAULT_ROLE);
      }
      if ("sort" in partial) {
        const v = validateOrFallback(partial.sort, VALID_SORTS, DEFAULT_SORT);
        setIfNonDefault("sort", v, DEFAULT_SORT);
      }
      if ("type" in partial) {
        const v = validateOrFallback(partial.type, VALID_TYPES, DEFAULT_TYPE);
        setIfNonDefault("type", v, DEFAULT_TYPE);
      }
      if ("filter" in partial) {
        const v = validateOrFallback(partial.filter, VALID_FILTERS, DEFAULT_FILTER);
        setIfNonDefault("filter", v, DEFAULT_FILTER);
      }

      return next;
    }, { replace: true });
  }, [setSearchParams]);

  const resetFilters = useCallback(() => {
    setSearchParams((prev) => {
      const next = new URLSearchParams(prev);
      const keys = ["period", "ecosystem", "role", "sort", "type", "filter"];
      for (const k of keys) next.delete(k);
      return next;
    }, { replace: true });
  }, [setSearchParams]);

  const shareURL = useMemo(() => {
    const base = `${window.location.origin}${window.location.pathname}`;
    const qs = searchParams.toString();
    return qs ? `${base}?${qs}` : base;
  }, [searchParams]);

  const isDefault = useMemo(() => {
    const f = filters;
    return (
      (f.period ?? DEFAULT_PERIOD) === DEFAULT_PERIOD &&
      (f.ecosystem ?? DEFAULT_ECOSYSTEM) === DEFAULT_ECOSYSTEM &&
      (f.role ?? DEFAULT_ROLE) === DEFAULT_ROLE &&
      (f.sort ?? DEFAULT_SORT) === DEFAULT_SORT &&
      (f.type ?? DEFAULT_TYPE) === DEFAULT_TYPE &&
      (f.filter ?? DEFAULT_FILTER) === DEFAULT_FILTER
    );
  }, [filters]);

  const copyShareLink = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(shareURL);
      return true;
    } catch {
      try {
        const textarea = document.createElement("textarea");
        textarea.value = shareURL;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
        return true;
      } catch {
        return false;
      }
    }
  }, [shareURL]);

  return {
    filters,
    setFilters,
    resetFilters,
    shareURL,
    isDefault,
    copyShareLink,
  };
}
