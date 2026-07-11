import { defineStore } from "pinia";
import { ref, computed } from "vue";

export type NotificationType = "message" | "error" | "warning" | "info";

export interface NotificationAction {
  label: string;
  handler: () => void;
}

export interface AppNotification {
  id: string;
  type: NotificationType;
  title: string;
  body?: string;
  source?: string;
  action?: NotificationAction;
  read: boolean;
  createdAt: number;
  metadata?: Record<string, unknown>;
}

export type NotificationInput = Omit<AppNotification, "id" | "read" | "createdAt">;

export const useNotificationStore = defineStore("notification", () => {
  const rawNotifications = ref<AppNotification[]>([]);

  const notifications = computed<AppNotification[]>(() =>
    [...rawNotifications.value].sort((a, b) => b.createdAt - a.createdAt),
  );

  const unreadCount = computed(
    () => rawNotifications.value.filter((n) => !n.read).length,
  );

  function add(input: NotificationInput): AppNotification {
    const notification: AppNotification = {
      ...input,
      id: `${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      read: false,
      createdAt: Date.now(),
    };
    rawNotifications.value.unshift(notification);
    return notification;
  }

  function markRead(id: string) {
    const n = rawNotifications.value.find((item) => item.id === id);
    if (n) n.read = true;
  }

  function markAllRead() {
    rawNotifications.value.forEach((n) => {
      n.read = true;
    });
  }

  function remove(id: string) {
    const idx = rawNotifications.value.findIndex((item) => item.id === id);
    if (idx >= 0) rawNotifications.value.splice(idx, 1);
  }

  function clearAll() {
    rawNotifications.value = [];
  }

  return {
    notifications,
    unreadCount,
    add,
    markRead,
    markAllRead,
    remove,
    clearAll,
  };
});
