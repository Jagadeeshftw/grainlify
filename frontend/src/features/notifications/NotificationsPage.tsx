import { useState, useEffect, useCallback, useRef } from "react";
import {
  Bell,
  Award,
  GitPullRequest,
  GitMerge,
  Wallet,
  AlertTriangle,
  CheckCheck,
  Eye,
  EyeOff,
  ArrowRight,
  Loader2,
} from "lucide-react";
import { useTheme } from "../../shared/contexts/ThemeContext";
import { getNotifications, markNotificationRead, markAllNotificationsRead } from "../../shared/api/client";
import { groupNotificationsByDate, formatTimeAgo } from "../../shared/utils/notifications";
import type { Notification, NotificationType, NotificationFilterMode } from "../../shared/types/notifications";

const TYPE_ICON: Record<NotificationType, typeof Award> = {
  bounty_awarded: Award,
  submission_received: GitPullRequest,
  pr_reviewed: GitMerge,
  payout_confirmed: Wallet,
  system_alert: AlertTriangle,
};

const TYPE_COLOR: Record<NotificationType, string> = {
  bounty_awarded: "text-[#f1b400] bg-[#f1b400]/10",
  submission_received: "text-[#22c55e] bg-[#22c55e]/10",
  pr_reviewed: "text-[#c9983a] bg-[#c9983a]/10",
  payout_confirmed: "text-[#16a34a] bg-[#16a34a]/10",
  system_alert: "text-[#f59e0b] bg-[#f59e0b]/10",
};

const TYPE_LABEL: Record<NotificationType, string> = {
  bounty_awarded: "Bounty Awarded",
  submission_received: "Submission Received",
  pr_reviewed: "PR Reviewed",
  payout_confirmed: "Payout Confirmed",
  system_alert: "System Alert",
};

type FilterType = "all" | NotificationType;

const FILTER_CHIPS: { id: FilterType; label: string }[] = [
  { id: "all", label: "All" },
  { id: "bounty_awarded", label: "Bounty" },
  { id: "submission_received", label: "Submission" },
  { id: "pr_reviewed", label: "PR Review" },
  { id: "payout_confirmed", label: "Payout" },
  { id: "system_alert", label: "System" },
];

const PAGE_SIZE = 20;

function SkeletonRow() {
  const { theme } = useTheme();
  const darkTheme = theme === "dark";
  return (
    <div className="flex items-start gap-4 p-4">
      <div className={`w-10 h-10 rounded-full flex-shrink-0 ${darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"}`} />
      <div className="flex-1 space-y-2">
        <div className={`h-4 rounded w-2/3 ${darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"} animate-shimmer`} />
        <div className={`h-3 rounded w-1/2 ${darkTheme ? "bg-white/[0.06]" : "bg-white/[0.1]"} animate-shimmer`} />
        <div className={`h-3 rounded w-1/4 ${darkTheme ? "bg-white/[0.04]" : "bg-white/[0.08]"} animate-shimmer`} />
      </div>
    </div>
  );
}

export function NotificationsPage() {
  const { theme } = useTheme();
  const darkTheme = theme === "dark";

  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [typeFilter, setTypeFilter] = useState<FilterType>("all");
  const [readFilter, setReadFilter] = useState<NotificationFilterMode>("all");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [markingAllRead, setMarkingAllRead] = useState(false);

  const sentinelRef = useRef<HTMLDivElement>(null);
  const offsetRef = useRef(0);
  const hasMoreRef = useRef(true);

  const fetchPage = useCallback(async (offset: number, append: boolean) => {
    if (append) {
      setLoadingMore(true);
    } else {
      setLoading(true);
    }
    setError(null);
    try {
      const params: { type?: string; read?: boolean; limit: number; offset: number } = {
        limit: PAGE_SIZE,
        offset,
      };
      if (typeFilter !== "all") params.type = typeFilter;
      if (readFilter === "unread") params.read = false;

      const data = await getNotifications(params);
      const mapped: Notification[] = (data.notifications || []).map((n) => ({
        id: n.id,
        type: n.type as NotificationType,
        title: n.title,
        body: n.body,
        read: n.read,
        createdAt: n.created_at,
        actionUrl: n.action_url,
        actor: n.actor ? { name: n.actor.name, avatarUrl: n.actor.avatar_url } : undefined,
      }));

      if (append) {
        setNotifications((prev) => [...prev, ...mapped]);
      } else {
        setNotifications(mapped);
      }
      setTotal(data.total ?? 0);
      hasMoreRef.current = offset + PAGE_SIZE < (data.total ?? 0);
      offsetRef.current = offset;
    } catch {
      setError("Failed to load notifications");
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, [typeFilter, readFilter]);

  useEffect(() => {
    offsetRef.current = 0;
    hasMoreRef.current = true;
    setSelectedIds(new Set());
    fetchPage(0, false);
  }, [fetchPage]);

  // Infinite scroll
  useEffect(() => {
    if (!sentinelRef.current || loading || loadingMore || !hasMoreRef.current) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMoreRef.current && !loadingMore) {
          fetchPage(offsetRef.current + PAGE_SIZE, true);
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(sentinelRef.current);
    return () => observer.disconnect();
  }, [loading, loadingMore, fetchPage]);

  const handleMarkAllRead = async () => {
    setMarkingAllRead(true);
    try {
      await markAllNotificationsRead();
      setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
      setSelectedIds(new Set());
    } catch {
      // handled by next fetch
    } finally {
      setMarkingAllRead(false);
    }
  };

  const handleToggleRead = async (notification: Notification) => {
    if (notification.read) return;
    try {
      await markNotificationRead(notification.id);
      setNotifications((prev) =>
        prev.map((n) => (n.id === notification.id ? { ...n, read: true } : n)),
      );
    } catch {
      // handled by next fetch on re-open
    }
  };

  const handleToggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleBulkMarkRead = async () => {
    if (selectedIds.size === 0) return;
    try {
      await Promise.all(Array.from(selectedIds).map((id) => markNotificationRead(id)));
      setNotifications((prev) =>
        prev.map((n) => (selectedIds.has(n.id) ? { ...n, read: true } : n)),
      );
      setSelectedIds(new Set());
    } catch {
      // handled by next fetch
    }
  };

  const unreadCount = notifications.filter((n) => !n.read).length;
  const groups = groupNotificationsByDate(notifications);

  const glassCard = `backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-colors ${
    darkTheme
      ? "bg-[#2d2820]/[0.4] border-white/10"
      : "bg-white/[0.12] border-white/20"
  }`;

  return (
    <div className="space-y-6 max-w-3xl mx-auto px-4 sm:px-6 lg:px-8">
      {/* Header */}
      <div className={`${glassCard} p-6 sm:p-8`}>
        <div className="flex items-start justify-between flex-wrap gap-4">
          <div>
            <h1
              tabIndex={-1}
              className={`text-[28px] font-bold mb-1 ${
                darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
              }`}
            >
              Notifications Center
            </h1>
            <p className={`text-[14px] ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
              {total} notification{total !== 1 ? "s" : ""}
              {unreadCount > 0 && ` · ${unreadCount} unread`}
            </p>
          </div>
          {unreadCount > 0 && (
            <button
              onClick={handleMarkAllRead}
              disabled={markingAllRead}
              className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] transition-all flex items-center gap-2 ${
                markingAllRead ? "opacity-40 pointer-events-none" : "hover:bg-white/[0.25]"
              } ${
                darkTheme
                  ? "bg-[#3d342c]/[0.5] border-white/20 text-[#d4c5b0]"
                  : "bg-white/[0.2] border-white/30 text-[#2d2820]"
              }`}
            >
              <CheckCheck className="w-4 h-4" />
              Mark all as read
            </button>
          )}
        </div>
      </div>

      {/* Type Filter Chips */}
      <div className="flex gap-2 overflow-x-auto scrollbar-hide pb-2">
        {FILTER_CHIPS.map((chip) => (
          <button
            key={chip.id}
            onClick={() => setTypeFilter(chip.id)}
            className={`whitespace-nowrap rounded-[999px] px-4 py-2 text-[13px] font-medium backdrop-blur-[25px] border transition-all focus-visible:ring-2 focus-visible:ring-[#c9983a] ${
              typeFilter === chip.id
                ? "bg-[#c9983a] text-white border-[#c9983a]"
                : darkTheme
                  ? "bg-white/[0.06] border-white/10 text-[#d4c5b0] hover:bg-white/[0.12]"
                  : "bg-white/[0.12] border-white/25 text-[#6b5d4d] hover:bg-white/[0.2]"
            }`}
          >
            {chip.label}
          </button>
        ))}
      </div>

      {/* Read/Unread Filter + Sort */}
      <div className="flex items-center justify-between">
        <div className="flex rounded-[12px] overflow-hidden border border-white/10">
          {(["all", "unread"] as NotificationFilterMode[]).map((mode) => (
            <button
              key={mode}
              onClick={() => setReadFilter(mode)}
              className={`px-4 py-2 text-[13px] font-medium transition-all focus-visible:ring-2 focus-visible:ring-[#c9983a] ${
                readFilter === mode
                  ? darkTheme
                    ? "bg-[#c9983a]/20 text-[#f5efe5]"
                    : "bg-[#c9983a]/15 text-[#2d2820]"
                  : darkTheme
                    ? "bg-transparent text-[#b8a898]"
                    : "bg-transparent text-[#7a6b5a]"
              }`}
            >
              {mode === "all" ? "All" : "Unread Only"}
            </button>
          ))}
        </div>
        <p className={`text-[12px] ${darkTheme ? "text-[#8a7e70]" : "text-[#9f8b74]"}`}>
          Newest first
        </p>
      </div>

      {/* Loading State */}
      {loading && (
        <div className={glassCard}>
          {Array.from({ length: 5 }).map((_, i) => (
            <SkeletonRow key={i} />
          ))}
        </div>
      )}

      {/* Error State */}
      {error && !loading && (
        <div className={`${glassCard} p-12 flex flex-col items-center justify-center text-center`}>
          <AlertTriangle className="w-12 h-12 text-[#ef4444] mb-4" />
          <h3 className={`text-lg font-semibold mb-2 ${darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"}`}>
            Failed to load
          </h3>
          <p className={`text-[14px] mb-4 ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
            {error}
          </p>
          <button
            onClick={() => fetchPage(0, false)}
            className="px-6 py-2.5 rounded-[12px] bg-[#c9983a] text-white font-medium text-[14px] hover:bg-[#a67c2e] transition-colors"
          >
            Try again
          </button>
        </div>
      )}

      {/* Empty State */}
      {!loading && !error && notifications.length === 0 && (
        <div className={`${glassCard} p-12 flex flex-col items-center justify-center text-center`}>
          <div
            className={`w-20 h-20 rounded-full flex items-center justify-center mb-4 ${
              darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"
            }`}
          >
            <Bell className={`w-10 h-10 ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
          </div>
          <h3 className={`text-lg font-semibold mb-1 ${darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"}`}>
            No notifications yet
          </h3>
          <p className={`text-[14px] max-w-sm ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
            {typeFilter !== "all"
              ? "No notifications match this filter. Try selecting a different type."
              : "You'll see updates about your contributions, rewards, and project activity here."}
          </p>
          {typeFilter !== "all" && (
            <button
              onClick={() => setTypeFilter("all")}
              className="mt-4 text-[13px] font-medium text-[#c9983a] hover:text-[#a67c2e] transition-colors"
            >
              Clear filters
            </button>
          )}
        </div>
      )}

      {/* Notification List */}
      {!loading && !error && notifications.length > 0 && (
        <div className={glassCard}>
          <div role="list" aria-label="Notifications">
            {groups.map((group) => (
              <div key={group.label}>
                <div
                  className={`px-4 sm:px-6 py-3 border-b ${
                    darkTheme ? "border-white/[0.06] bg-white/[0.02]" : "border-white/[0.12] bg-white/[0.04]"
                  }`}
                >
                  <p className="text-[11px] font-semibold uppercase tracking-wider text-[#8a7e70]">
                    {group.label}
                  </p>
                </div>
                {group.items.map((notification) => {
                  const isSelected = selectedIds.has(notification.id);
                  return (
                    <div
                      key={notification.id}
                      role="listitem"
                      className={`flex items-start gap-3 sm:gap-4 p-4 sm:p-6 border-b transition-colors ${
                        darkTheme
                          ? "border-white/[0.06] hover:bg-white/[0.04]"
                          : "border-white/[0.1] hover:bg-white/[0.06]"
                      } ${isSelected ? (darkTheme ? "bg-[#c9983a]/[0.06]" : "bg-[#c9983a]/[0.04]") : ""} ${
                        !notification.read ? (darkTheme ? "bg-white/[0.02]" : "bg-white/[0.04]") : ""
                      }`}
                    >
                      {/* Select checkbox */}
                      <button
                        onClick={() => handleToggleSelect(notification.id)}
                        aria-label={isSelected ? "Deselect notification" : "Select notification"}
                        className={`w-5 h-5 rounded border-2 flex-shrink-0 mt-2.5 transition-colors ${
                          isSelected
                            ? "bg-[#c9983a] border-[#c9983a]"
                            : darkTheme
                              ? "border-white/20 hover:border-white/40"
                              : "border-white/30 hover:border-[#c9983a]/50"
                        }`}
                      >
                        {isSelected && (
                          <svg viewBox="0 0 24 24" fill="none" className="w-3 h-3 mx-auto text-white">
                            <path d="M5 13l4 4L19 7" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
                          </svg>
                        )}
                      </button>

                      {/* Icon */}
                      <div
                        className={`w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0 ${
                          TYPE_COLOR[notification.type]?.split(" ")[1] || (darkTheme ? "bg-white/[0.1]" : "bg-white/[0.15]")
                        }`}
                      >
                        {(() => {
                          const Icon = TYPE_ICON[notification.type] || Bell;
                          return (
                            <Icon
                              className={`w-5 h-5 ${TYPE_COLOR[notification.type]?.split(" ")[0] || "text-[#c9983a]"}`}
                            />
                          );
                        })()}
                      </div>

                      {/* Content */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-start justify-between gap-2">
                          <p
                            className={`text-[14px] leading-tight truncate ${
                              notification.read
                                ? `font-medium ${darkTheme ? "text-[#d4c5b0]" : "text-[#2d2820]"}`
                                : `font-semibold ${darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"}`
                            }`}
                          >
                            {notification.title}
                          </p>
                          <div className="flex items-center gap-2 flex-shrink-0">
                            <span className={`text-[11px] whitespace-nowrap ${darkTheme ? "text-[#8a7e70]" : "text-[#9f8b74]"}`}>
                              {formatTimeAgo(notification.createdAt)}
                            </span>
                            {!notification.read && (
                              <span className="w-2 h-2 bg-[#c9983a] rounded-full flex-shrink-0" aria-label="Unread" />
                            )}
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                handleToggleRead(notification);
                              }}
                              aria-label={notification.read ? "Mark as unread" : "Mark as read"}
                              className={`p-1 rounded transition-colors ${
                                darkTheme
                                  ? "hover:bg-white/[0.1] text-[#8a7e70] hover:text-[#e8dfd0]"
                                  : "hover:bg-white/[0.15] text-[#9f8b74] hover:text-[#2d2820]"
                              }`}
                            >
                              {notification.read ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
                            </button>
                          </div>
                        </div>
                        <p className={`text-[13px] leading-relaxed mt-1 line-clamp-2 ${
                          darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
                        }`}>
                          {notification.body}
                        </p>
                        {notification.actionUrl && (
                          <a
                            href={notification.actionUrl}
                            className={`inline-flex items-center gap-1 mt-2 text-[12px] font-medium transition-colors ${
                              darkTheme ? "text-[#c9983a] hover:text-[#a67c2e]" : "text-[#c9983a] hover:text-[#a67c2e]"
                            }`}
                          >
                            View details
                            <ArrowRight className="w-3 h-3" />
                          </a>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>

          {/* Infinite scroll sentinel */}
          <div ref={sentinelRef} className="h-4" />

          {/* Loading more */}
          {loadingMore && (
            <div className="py-4">
              <SkeletonRow />
            </div>
          )}

          {/* End of list */}
          {!hasMoreRef.current && notifications.length > 0 && (
            <p className={`text-center text-[13px] py-6 ${darkTheme ? "text-[#8a7e70]" : "text-[#9f8b74]"}`}>
              You've reached the end
            </p>
          )}
        </div>
      )}

      {/* Bulk Action Bar */}
      {selectedIds.size > 0 && (
        <div
          className={`fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex items-center gap-4 px-5 py-3 rounded-[16px] backdrop-blur-[40px] border shadow-[0_8px_32px_rgba(0,0,0,0.2)] ${
            darkTheme
              ? "bg-[#2d2820]/[0.95] border-white/15"
              : "bg-white/[0.9] border-white/30"
          }`}
        >
          <span className={`text-[13px] font-medium ${darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
            {selectedIds.size} selected
          </span>
          <div className="flex items-center gap-2">
            <button
              onClick={handleBulkMarkRead}
              className="px-4 py-1.5 rounded-[10px] bg-[#c9983a] text-white text-[13px] font-medium hover:bg-[#a67c2e] transition-colors"
            >
              Mark as read
            </button>
            <button
              onClick={() => setSelectedIds(new Set())}
              className={`px-4 py-1.5 rounded-[10px] text-[13px] font-medium transition-colors ${
                darkTheme
                  ? "bg-white/[0.1] text-[#d4c5b0] hover:bg-white/[0.15]"
                  : "bg-white/[0.2] text-[#6b5d4d] hover:bg-white/[0.3]"
              }`}
            >
              Clear selection
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
