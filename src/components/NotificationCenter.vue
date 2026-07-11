<script setup lang="ts">
import { computed } from "vue";
import { useNotificationStore, type AppNotification, type NotificationType } from "@/stores/notification";
import {
  ChatDotRound,
  CircleCloseFilled,
  WarningFilled,
  InfoFilled,
  Delete,
  Check,
} from "@element-plus/icons-vue";

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

const store = useNotificationStore();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const iconMap: Record<NotificationType, typeof ChatDotRound> = {
  message: ChatDotRound,
  error: CircleCloseFilled,
  warning: WarningFilled,
  info: InfoFilled,
};

const colorMap: Record<NotificationType, string> = {
  message: "var(--color-primary, #7c6bf0)",
  error: "var(--color-danger, #f87171)",
  warning: "var(--color-warning, #fbbf24)",
  info: "var(--color-info, #6b7280)",
};

function formatTime(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return d.toLocaleDateString();
}

function handleItemClick(n: AppNotification) {
  store.markRead(n.id);
}

function handleAction(n: AppNotification, event: MouseEvent) {
  event.stopPropagation();
  n.action?.handler();
  store.markRead(n.id);
}

function handleRemove(n: AppNotification, event: MouseEvent) {
  event.stopPropagation();
  store.remove(n.id);
}

function handleMarkAll() {
  store.markAllRead();
}

function handleClearAll() {
  store.clearAll();
}
</script>

<template>
  <el-drawer
    v-model="visible"
    title="通知中心"
    size="380px"
    direction="rtl"
    :with-header="false"
    class="notification-drawer"
  >
    <div class="notification-panel">
      <div class="notification-header">
        <div class="header-title">
          <span class="title-text">通知中心</span>
          <el-badge
            v-if="store.unreadCount > 0"
            :value="store.unreadCount"
            :max="99"
            type="danger"
          />
        </div>
        <div class="header-actions">
          <el-button
            text
            size="small"
            :icon="Check"
            :disabled="store.unreadCount === 0"
            @click="handleMarkAll"
          >
            全部已读
          </el-button>
          <el-button
            text
            size="small"
            type="danger"
            :icon="Delete"
            :disabled="store.notifications.length === 0"
            @click="handleClearAll"
          >
            清空
          </el-button>
        </div>
      </div>

      <div class="notification-list">
        <el-empty
          v-if="store.notifications.length === 0"
          description="暂无通知"
          :image-size="80"
        />

        <div
          v-for="n in store.notifications"
          :key="n.id"
          class="notification-item"
          :class="{ unread: !n.read }"
          @click="handleItemClick(n)"
        >
          <div class="item-icon">
            <el-icon :size="20" :color="colorMap[n.type]">
              <component :is="iconMap[n.type]" />
            </el-icon>
          </div>

          <div class="item-body">
            <div class="item-title">
              <span class="title-label">{{ n.title }}</span>
              <span class="item-time">{{ formatTime(n.createdAt) }}</span>
            </div>
            <div v-if="n.body" class="item-content">{{ n.body }}</div>
            <div class="item-meta">
              <el-tag v-if="n.source" size="small" effect="plain" class="source-tag">
                {{ n.source }}
              </el-tag>
              <el-button
                v-if="n.action"
                size="small"
                type="primary"
                link
                @click="handleAction(n, $event)"
              >
                {{ n.action.label }}
              </el-button>
            </div>
          </div>

          <div class="item-actions">
            <el-button
              text
              circle
              size="small"
              :icon="Delete"
              @click="handleRemove(n, $event)"
            />
          </div>
        </div>
      </div>
    </div>
  </el-drawer>
</template>

<style scoped lang="scss">
.notification-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.notification-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--glass-border);
  margin-bottom: 8px;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-text {
  font-size: 16px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.notification-list {
  flex: 1;
  overflow-y: auto;
  padding-right: 4px;
}

.notification-item {
  display: flex;
  gap: 10px;
  padding: 10px 8px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background 0.15s ease;
  position: relative;

  &:hover {
    background: var(--glass-hover-bg);
  }

  &.unread::before {
    content: "";
    position: absolute;
    left: 0;
    top: 14px;
    bottom: 14px;
    width: 3px;
    border-radius: 0 2px 2px 0;
    background: var(--color-primary);
  }
}

.item-icon {
  flex-shrink: 0;
  padding-top: 2px;
}

.item-body {
  flex: 1;
  min-width: 0;
}

.item-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.title-label {
  font-size: 14px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-time {
  font-size: 11px;
  color: var(--color-text-muted);
  flex-shrink: 0;
}

.item-content {
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  margin-bottom: 6px;
}

.item-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.source-tag {
  text-transform: capitalize;
}

.item-actions {
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.15s ease;

  .notification-item:hover & {
    opacity: 1;
  }
}
</style>
