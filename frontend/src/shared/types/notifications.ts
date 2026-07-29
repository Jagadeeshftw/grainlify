export type NotificationType =
  | 'bounty_awarded'
  | 'submission_received'
  | 'pr_reviewed'
  | 'payout_confirmed'
  | 'system_alert';

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  body: string;
  read: boolean;
  createdAt: string;
  actionUrl?: string;
  actor?: {
    name: string;
    avatarUrl: string;
  };
}

export interface NotificationsResponse {
  notifications: Notification[];
  unreadCount: number;
  total: number;
  limit: number;
  offset: number;
}

export type NotificationFilterMode = 'all' | 'unread';

export type NotificationGroupLabel = 'Today' | 'Yesterday' | 'This Week' | 'Earlier';

export interface NotificationGroup {
  label: NotificationGroupLabel;
  items: Notification[];
}
