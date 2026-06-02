import { Bell, AlertTriangle, CheckCheck, Award, GitPullRequest, GitMerge, Wallet } from "lucide-react";
import { useTheme } from "../contexts/ThemeContext";
import { useState, useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { getNotifications, getNotificationCount, markAllNotificationsRead } from "../api/client";
import { groupNotificationsByDate, formatTimeAgo } from "../utils/notifications";
import type { Notification, NotificationType } from "../types/notifications";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuTrigger,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "../../app/components/ui/dropdown-menu";

interface NotificationsDropdownProp {
  showMobileNav: boolean;
  closeMobileNav: () => void;
}

const TYPE_ICON: Record<NotificationType, typeof Award> = {
  bounty_awarded: Award,
  submission_received: GitPullRequest,
  pr_reviewed: GitMerge,
  payout_confirmed: Wallet,
  system_alert: AlertTriangle,
};

const TYPE_COLOR: Record<NotificationType, string> = {
  bounty_awarded: "text-[#f1b400]",
  submission_received: "text-[#22c55e]",
  pr_reviewed: "text-[#c9983a]",
  payout_confirmed: "text-[#16a34a]",
  system_alert: "text-[#f59e0b]",
};

function NotificationTypeIcon({ type }: { type: NotificationType }) {
  const Icon = TYPE_ICON[type] || Bell;
  return <Icon className={`w-4 h-4 ${TYPE_COLOR[type] || "text-[#c9983a]"}`} />;
}

function SkeletonItem() {
  const { theme } = useTheme();
  const darkTheme = theme === "dark";
  return (
    <div className="flex items-start gap-3 px-4 py-3">
      <div className={`w-8 h-8 rounded-full flex-shrink-0 ${darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"}`} />
      <div className="flex-1 space-y-2">
        <div className={`h-3 rounded w-3/4 ${darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"} animate-shimmer`} />
        <div className={`h-2.5 rounded w-1/2 ${darkTheme ? "bg-white/[0.06]" : "bg-white/[0.1]"} animate-shimmer`} />
        <div className={`h-2 rounded w-1/4 ${darkTheme ? "bg-white/[0.04]" : "bg-white/[0.08]"} animate-shimmer`} />
      </div>
    </div>
  );
}

export function NotificationsDropdown({ showMobileNav, closeMobileNav }: NotificationsDropdownProp) {
  const { theme } = useTheme();
  const navigate = useNavigate();
  const darkTheme = theme === "dark";
  const [open, setOpen] = useState(false);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [markingAllRead, setMarkingAllRead] = useState(false);

  const fetchNotifications = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [listData, countData] = await Promise.all([
        getNotifications({ limit: 10 }),
        getNotificationCount(),
      ]);
      const mapped: Notification[] = (listData.notifications || []).map((n) => ({
        id: n.id,
        type: n.type as NotificationType,
        title: n.title,
        body: n.body,
        read: n.read,
        createdAt: n.created_at,
        actionUrl: n.action_url,
        actor: n.actor ? { name: n.actor.name, avatarUrl: n.actor.avatar_url } : undefined,
      }));
      setNotifications(mapped);
      setUnreadCount(countData.count ?? listData.unread_count ?? 0);
    } catch {
      setError("Failed to load notifications");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      fetchNotifications();
    }
  }, [open, fetchNotifications]);

  // Fetch count on mount for badge, independent of dropdown
  useEffect(() => {
    getNotificationCount()
      .then((data) => setUnreadCount(data.count ?? 0))
      .catch(() => {});
  }, []);

  const handleMarkAllRead = async () => {
    setMarkingAllRead(true);
    try {
      await markAllNotificationsRead();
      setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
      setUnreadCount(0);
    } catch {
      // restore optimistic update handled by next fetch
    } finally {
      setMarkingAllRead(false);
    }
  };

  const handleViewAll = () => {
    setOpen(false);
    closeMobileNav();
    navigate("/dashboard?page=notifications");
  };

  const handleNotificationClick = (notification: Notification) => {
    setOpen(false);
    closeMobileNav();
    if (notification.actionUrl) {
      navigate(notification.actionUrl);
    }
  };

  const formatCount = (count: number): string => {
    return count > 99 ? "99+" : count.toString();
  };

  const groups = groupNotificationsByDate(notifications);

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <button
          aria-label={`Notifications${unreadCount > 0 ? ` (${unreadCount} unread)` : ""}`}
          aria-haspopup="true"
          aria-expanded={open}
          className={`h-[46px] w-[46px] rounded-full relative items-center justify-center backdrop-blur-[40px] transition-all hover:scale-105 focus-visible:ring-2 focus-visible:ring-[#c9983a] shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] ${
            darkTheme ? "bg-[#2d2820]" : "bg-[#d4c5b0]"
          }
          ${showMobileNav ? "flex w-[80%] max-w-[800px] rounded-sm" : "hidden lg:flex"}`}
        >
          <div
            className={`absolute inset-0 pointer-events-none ${showMobileNav ? "rounded-sm" : "rounded-full"} ${
              darkTheme
                ? "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.5),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.11)]"
                : "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.15),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.35)]"
            }`}
          />
          <Bell
            className={`w-4 h-4 relative z-10 transition-colors ${
              darkTheme
                ? "text-[rgba(255,255,255,0.69)]"
                : "text-[rgba(45,40,32,0.75)]"
            }`}
          />
          {showMobileNav && (
            <span className={`ml-2 ${darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
              Notification
            </span>
          )}
          {unreadCount > 0 && (
            <span
              role="status"
              aria-live="polite"
              aria-label={`${unreadCount} unread notification${unreadCount !== 1 ? "s" : ""}`}
              className="absolute -top-0.5 -right-0.5 lg:-top-1 lg:-right-1 min-w-[18px] h-[18px] px-1 rounded-full z-20 border-2 flex items-center justify-center bg-[#ef4444] text-white animate-badge-in"
            >
              <span className="text-[10px] font-bold leading-none tabular-nums">
                {formatCount(unreadCount)}
              </span>
            </span>
          )}
          <span aria-live="polite" aria-atomic="true" className="sr-only">
            {unreadCount > 0
              ? `You have ${unreadCount} unread notifications.`
              : "No unread notifications."}
          </span>
        </button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        align="end"
        sideOffset={8}
        className={`w-80 sm:w-[calc(100vw-32px)] max-sm:w-[calc(100vw-32px)] rounded-[18px] backdrop-blur-[40px] border shadow-[0_8px_32px_rgba(0,0,0,0.12),0_0_20px_rgba(201,152,58,0.15)] overflow-hidden p-0 max-h-[60vh] ${
          darkTheme
            ? "bg-white/[0.08] border-white/15"
            : "bg-white/[0.15] border-white/25"
        }`}
      >
        <div role="dialog" aria-label="Notifications">
          {/* Header */}
          <DropdownMenuLabel
            className={`px-4 py-4 border-b flex items-center justify-between ${
              darkTheme ? "border-white/10" : "border-white/20"
            }`}
          >
            <div className="flex items-center space-x-3">
              <div
                className={`w-10 h-10 rounded-full flex items-center justify-center ${
                  darkTheme ? "bg-white/[0.12]" : "bg-white/[0.2]"
                }`}
              >
                <Bell className="w-5 h-5 text-[#c9983a]" />
              </div>
              <div>
                <p
                  className={`font-semibold text-sm ${
                    darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
                  }`}
                >
                  Notifications
                </p>
                {unreadCount > 0 && (
                  <p className="text-[11px] text-[#c9983a] font-medium">
                    {unreadCount} unread
                  </p>
                )}
              </div>
            </div>
            {unreadCount > 0 && (
              <button
                onClick={handleMarkAllRead}
                disabled={markingAllRead}
                aria-label={`Mark all ${unreadCount} notifications as read`}
                className={`text-[13px] font-medium flex items-center gap-1.5 transition-colors ${
                  markingAllRead
                    ? "opacity-40 pointer-events-none"
                    : darkTheme
                      ? "text-[#c9983a] hover:text-[#a67c2e]"
                      : "text-[#c9983a] hover:text-[#a67c2e]"
                }`}
              >
                <CheckCheck className="w-3.5 h-3.5" />
                Mark all read
              </button>
            )}
          </DropdownMenuLabel>

          {/* Loading State */}
          {loading && (
            <div className="py-2">
              <SkeletonItem />
              <SkeletonItem />
              <SkeletonItem />
            </div>
          )}

          {/* Error State */}
          {error && !loading && (
            <div className="px-4 py-8 flex flex-col items-center justify-center text-center">
              <AlertTriangle className="w-8 h-8 text-[#ef4444] mb-3" />
              <p className={`text-sm font-medium mb-2 ${
                darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
              }`}>
                {error}
              </p>
              <button
                onClick={fetchNotifications}
                className="text-[13px] font-medium text-[#c9983a] hover:text-[#a67c2e] transition-colors"
              >
                Try again
              </button>
            </div>
          )}

          {/* Empty State */}
          {!loading && !error && notifications.length === 0 && (
            <div
              className={`px-4 py-12 flex flex-col items-center justify-center ${
                darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
              }`}
            >
              <div
                className={`w-16 h-16 rounded-full flex items-center justify-center mb-4 ${
                  darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"
                }`}
              >
                <Bell
                  className={`w-8 h-8 ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
                />
              </div>
              <p
                className={`text-sm font-medium mb-1 ${
                  darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
                }`}
              >
                No notifications yet
              </p>
              <p className="text-xs text-center max-w-[200px]">
                You'll see updates about your contributions, rewards, and project activity here.
              </p>
            </div>
          )}

          {/* Notification List */}
          {!loading && !error && notifications.length > 0 && (
            <div className="custom-scrollbar overflow-y-auto max-h-[50vh]">
              <div role="list" aria-label="Notification list">
                {groups.map((group) => (
                  <div key={group.label}>
                    <p
                      className={`text-[11px] font-semibold uppercase tracking-wider px-4 py-2 ${
                        darkTheme ? "text-[#8a7e70]" : "text-[#8a7e70]"
                      }`}
                    >
                      {group.label}
                    </p>
                    {group.items.map((notification) => {
                      const Icon = TYPE_ICON[notification.type] || Bell;
                      return (
                        <DropdownMenuItem
                          key={notification.id}
                          role="listitem"
                          onClick={() => handleNotificationClick(notification)}
                          className={`flex items-start gap-3 px-4 py-3 cursor-pointer animate-notify-slide-in ${
                            darkTheme
                              ? "hover:bg-white/[0.12] focus:bg-white/[0.12] focus:text-[#e8dfd0]"
                              : "hover:bg-white/[0.2] focus:bg-white/[0.2]"
                          } ${!notification.read ? "bg-white/[0.04]" : ""}`}
                        >
                          <div
                            className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
                              darkTheme ? "bg-white/[0.1]" : "bg-white/[0.15]"
                            }`}
                          >
                            <Icon className={`w-4 h-4 ${TYPE_COLOR[notification.type] || "text-[#c9983a]"}`} />
                          </div>
                          <div className="flex-1 min-w-0">
                            <p
                              className={`text-[13px] leading-tight truncate ${
                                notification.read
                                  ? `font-medium ${darkTheme ? "text-[#d4c5b0]" : "text-[#2d2820]"}`
                                  : `font-semibold ${darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"}`
                              }`}
                            >
                              {notification.title}
                            </p>
                            <p
                              className={`text-[12px] leading-tight truncate mt-0.5 ${
                                darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
                              }`}
                            >
                              {notification.body}
                            </p>
                            <p
                              className={`text-[11px] mt-0.5 ${
                                darkTheme ? "text-[#8a7e70]" : "text-[#9f8b74]"
                              }`}
                            >
                              {formatTimeAgo(notification.createdAt)}
                            </p>
                          </div>
                          {!notification.read && (
                            <span
                              aria-label="Unread notification"
                              className="w-2 h-2 bg-[#c9983a] rounded-full flex-shrink-0 mt-1.5"
                            />
                          )}
                        </DropdownMenuItem>
                      );
                    })}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Footer */}
          {!loading && !error && (
            <>
              <DropdownMenuSeparator className={darkTheme ? "bg-white/10" : "bg-white/20"} />
              <button
                onClick={handleViewAll}
                className={`w-full px-4 py-3 text-[13px] font-medium text-[#c9983a] hover:text-[#a67c2e] transition-colors text-center ${
                  darkTheme ? "hover:bg-white/[0.06]" : "hover:bg-white/[0.1]"
                }`}
              >
                View all notifications &rarr;
              </button>
            </>
          )}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
