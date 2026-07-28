import type { Notification, NotificationGroup, NotificationGroupLabel } from '../types/notifications';

export function formatTimeAgo(dateString: string): string {
  const now = new Date();
  const date = new Date(dateString);
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 2) return '1m ago';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 2) return '1h ago';
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 2) return 'Yesterday';
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

export function groupNotificationsByDate(items: Notification[]): NotificationGroup[] {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86400000);
  const weekAgo = new Date(today.getTime() - 6 * 86400000);

  const groups: Map<NotificationGroupLabel, Notification[]> = new Map([
    ['Today', []],
    ['Yesterday', []],
    ['This Week', []],
    ['Earlier', []],
  ]);

  for (const item of items) {
    const itemDate = new Date(item.createdAt);
    const itemDay = new Date(itemDate.getFullYear(), itemDate.getMonth(), itemDate.getDate());

    if (itemDay.getTime() === today.getTime()) {
      groups.get('Today')!.push(item);
    } else if (itemDay.getTime() === yesterday.getTime()) {
      groups.get('Yesterday')!.push(item);
    } else if (itemDay >= weekAgo) {
      groups.get('This Week')!.push(item);
    } else {
      groups.get('Earlier')!.push(item);
    }
  }

  return Array.from(groups.entries())
    .filter(([_, items]) => items.length > 0)
    .map(([label, items]) => ({ label, items }));
}
