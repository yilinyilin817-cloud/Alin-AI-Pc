<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ChatDotRound,
  User,
  Collection,
  MagicStick,
  Cpu,
  Setting,
  Plus,
  ChatLineRound,
  Bell,
  Moon,
  Sunny,
  HomeFilled,
} from "@element-plus/icons-vue";
import { usePersonaStore } from "@/stores/persona";
import { useChatStore } from "@/stores/chat";
import { useSettingsStore } from "@/stores/settings";
import { useNotificationStore } from "@/stores/notification";
import SessionList from "@/components/SessionList.vue";
import EmotionIndicator from "@/components/EmotionIndicator.vue";
import NotificationCenter from "@/components/NotificationCenter.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import BaseCard from "@/components/base/BaseCard.vue";
import CommandPalette from "@/components/CommandPalette.vue";
import { useEmotion } from "@/composables/useEmotion";
import { useWeChatStore } from "@/stores/wechat";
import { useTheme } from "@/composables/useTheme";
import { isTauri } from "@/api/env";
import type { WeChatMessageEvent, WeChatAccountEvent } from "@/types";

const route = useRoute();
const router = useRouter();
const personaStore = usePersonaStore();
const chatStore = useChatStore();
const settingsStore = useSettingsStore();
const wechatStore = useWeChatStore();
const notificationStore = useNotificationStore();
const { currentEmotion } = useEmotion();
const { effectiveTheme, toggleTheme, applyThemeClass } = useTheme();

const themeIcon = computed(() => (effectiveTheme.value === "dark" ? Moon : Sunny));

const notificationsVisible = ref(false);
const commandPaletteVisible = ref(false);

let unlistenMessage: (() => void) | null = null;
let unlistenAccount: (() => void) | null = null;
let unlistenNewChat: (() => void) | null = null;
let unlistenToggleTheme: (() => void) | null = null;
let lastNotifiedWechatError = "";

const navItems = [
  { path: "/dashboard", icon: HomeFilled, label: "首页" },
  { path: "/chat", icon: ChatDotRound, label: "对话" },
  { path: "/wechat", icon: ChatLineRound, label: "微信" },
  { path: "/persona", icon: User, label: "角色" },
  { path: "/knowledge", icon: Collection, label: "知识库" },
  { path: "/skills", icon: MagicStick, label: "技能" },
  { path: "/models", icon: Cpu, label: "模型" },
  { path: "/settings", icon: Setting, label: "设置" },
];

function getBaseRoutePath(path: string): string | undefined {
  const base = "/" + (path.split("/")[1] || "");
  if (base === "/") return undefined;
  // 只持久化导航中存在的已知基础路由，避免临时/特殊路由被恢复
  const known = navItems.some((item) => item.path === base);
  return known ? base : undefined;
}

// 路由变化时记录最后访问的基础路径（剥离动态 id）
watch(
  () => route.path,
  (path) => {
    const base = getBaseRoutePath(path);
    if (base) {
      settingsStore.setLastRoute(base);
    }
  },
  { immediate: true },
);

const activeMenu = computed(() => {
  const base = "/" + (route.path.split("/")[1] || "dashboard");
  return base;
});

const showSessionList = computed(() => route.path.startsWith("/chat"));

const topbarTitle = computed(() => {
  if (route.path.startsWith("/dashboard")) return "首页";
  return chatStore.currentSession?.title ?? personaStore.currentPersona?.name ?? "AI 伴侣";
});

// 路由切换后重新应用主题类，防止部分组件在页面多次切换后出现文字过淡
watch(
  () => route.path,
  () => applyThemeClass(),
);

/** 初始化应用状态 */
onMounted(async () => {
  // 初始化聊天（加载会话、设置监听）
  await chatStore.init();
  // 初始化微信 iLink 通道
  wechatStore.init().catch((e) => console.warn("wechat init:", e));
  // 若在 /chat 但没有 sessionId，自动选择第一个会话或创建新会话
  if (route.path.startsWith("/chat") && !route.params.sessionId) {
    autoSelectSession();
  }

  // 系统欢迎通知
  notificationStore.add({
    type: "info",
    title: "欢迎使用 AI 伴侣",
    source: "system",
  });

  setupNotificationListeners();
  setupTrayListeners();

  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("open-command-palette", onOpenCommandPaletteEvent);
});

onUnmounted(() => {
  unlistenMessage?.();
  unlistenAccount?.();
  unlistenNewChat?.();
  unlistenToggleTheme?.();
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("open-command-palette", onOpenCommandPaletteEvent);
});

async function setupTrayListeners() {
  if (!isTauri()) return;

  const { listen } = await import("@tauri-apps/api/event");
  unlistenNewChat = await listen("tray-new-chat", () => {
    handleNewSession();
  });
  unlistenToggleTheme = await listen("toggle-theme", () => {
    settingsStore.toggleTheme();
  });
}

async function setupNotificationListeners() {
  if (!isTauri()) return;

  const { listen } = await import("@tauri-apps/api/event");
  ensureNotificationPermission();

  unlistenMessage = await listen<WeChatMessageEvent>(
    "wechat-message",
    (e) => {
      const { sessionId, message } = e.payload;
      if (message.direction !== "inbound") return;

      const currentSessionId = route.params.sessionId;
      if (
        route.path.startsWith("/wechat") &&
        currentSessionId === sessionId
      ) {
        return;
      }

      const session = wechatStore.sessions.find((s) => s.id === sessionId);
      const title = message.senderName || session?.peerName || "微信消息";
      const body =
        message.msgType === "text"
          ? (message.content ?? "")
          : `[${message.msgType}]`;

      notificationStore.add({
        type: "message",
        source: "wechat",
        title,
        body,
        metadata: { sessionId, messageId: message.id },
        action: {
          label: "查看",
          handler: () => {
            router.push(`/wechat/${sessionId}`);
          },
        },
      });

      sendNativeNotification(title, body);
    },
  );

  unlistenAccount = await listen<WeChatAccountEvent>(
    "wechat-account",
    (e) => {
      if (e.payload.accountId !== wechatStore.account?.id) return;

      if (e.payload.status === "error" || e.payload.lastError) {
        const err = e.payload.lastError || "微信账号同步异常";
        if (err === lastNotifiedWechatError) return;
        lastNotifiedWechatError = err;

        notificationStore.add({
          type: "error",
          source: "wechat",
          title: "微信同步错误",
          body: err,
          action: {
            label: "重试",
            handler: () => wechatStore.reconnect(),
          },
        });
      } else if (e.payload.status === "online") {
        lastNotifiedWechatError = "";
      }
    },
  );
}

async function ensureNotificationPermission() {
  if (!("Notification" in window)) return;
  if (Notification.permission === "default") {
    try {
      await Notification.requestPermission();
    } catch {
      // ignore
    }
  }
}

function sendNativeNotification(title: string, body?: string) {
  if (!("Notification" in window)) return;
  if (Notification.permission !== "granted") return;
  if (!document.hidden) return;
  try {
    new Notification(title, { body });
  } catch {
    // ignore
  }
}

function autoSelectSession() {
  if (chatStore.sessions.length > 0) {
    const firstId = chatStore.sessions[0].id;
    chatStore.selectSession(firstId);
    router.replace(`/chat/${firstId}`).catch((e) => console.warn("autoSelectSession:", e));
  } else if (personaStore.currentPersonaId) {
    handleNewSession();
  }
}

async function handleNewSession() {
  const pid = personaStore.currentPersonaId ?? personaStore.personas[0]?.id;
  if (!pid) return;
  const session = await chatStore.createSession(pid);
  router.push(`/chat/${session.id}`).catch((e) => console.warn("handleNewSession:", e));
}

function selectAndNavigate(personaId: string) {
  personaStore.selectPersona(personaId);
  // 如果在角色编辑器页面，同步更新路由
  if (route.path.startsWith("/persona")) {
    router.replace(`/persona/${personaId}`).catch(() => {});
  }
}

function openCommandPalette() {
  commandPaletteVisible.value = true;
}

function onKeyDown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    openCommandPalette();
  }
}

function onOpenCommandPaletteEvent() {
  openCommandPalette();
}

</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-brand">
        <span class="brand-icon">✦</span>
        <span class="brand-name">AI 伴侣</span>
      </div>

      <div class="sidebar-middle">
        <div class="sidebar-nav">
          <el-menu
            :default-active="activeMenu"
            mode="vertical"
            :collapse="false"
            :collapse-transition="false"
            router
            class="sidebar-menu"
          >
            <el-menu-item v-for="item in navItems" :key="item.path" :index="item.path">
              <el-icon><component :is="item.icon" /></el-icon>
              <span>{{ item.label }}</span>
              <el-badge
                v-if="item.path === '/wechat' && wechatStore.totalUnread > 0"
                :value="wechatStore.totalUnread"
                :max="99"
                style="margin-left: auto"
              />
            </el-menu-item>
          </el-menu>
        </div>

        <div v-if="showSessionList" class="session-section">
          <div class="session-header">
            <span>会话</span>
            <el-button :icon="Plus" circle size="small" @click="handleNewSession" />
          </div>
          <SessionList />
        </div>
      </div>

      <div class="sidebar-personas">
        <div class="persona-label">当前角色</div>
        <div class="persona-list">
          <div
            v-for="p in personaStore.personas"
            :key="p.id"
            class="persona-item"
            :class="{ active: p.id === personaStore.currentPersonaId }"
            @click="selectAndNavigate(p.id)"
          >
            <el-avatar :size="28">{{ p.name[0] }}</el-avatar>
            <span class="persona-name">{{ p.name }}</span>
          </div>
        </div>
      </div>
    </aside>

    <div class="main-area">
      <header class="topbar">
        <div class="topbar-left">
          <el-avatar :size="32">
            {{ personaStore.currentPersona?.name[0] }}
          </el-avatar>
          <span class="topbar-title">{{ topbarTitle }}</span>
        </div>
        <div class="topbar-right">
          <el-badge
            :value="notificationStore.unreadCount"
            :max="99"
            :hidden="notificationStore.unreadCount === 0"
          >
            <el-button circle text @click="notificationsVisible = true">
              <el-icon :size="20"><Bell /></el-icon>
            </el-button>
          </el-badge>
          <EmotionIndicator :emotion="currentEmotion" size="small" />
          <BaseCard class="topbar-actions" :hoverable="false">
            <BaseButton variant="ghost" :icon="themeIcon" circle @click="toggleTheme" />
          </BaseCard>
        </div>
      </header>

      <NotificationCenter v-model="notificationsVisible" />
      <CommandPalette v-model="commandPaletteVisible" />

      <main class="main-content">
        <router-view />
      </main>
    </div>
  </div>
</template>

<style scoped lang="scss">
.app-layout {
  display: flex;
  width: 100%;
  height: 100%;
  min-height: 100%;
  overflow: hidden;
  background: var(--bg-gradient-base);
}

.sidebar {
  width: var(--sidebar-width, 240px);
  min-width: var(--sidebar-width, 240px);
  height: 100%;
  max-height: 100%;
  background: var(--glass-bg, rgba(36, 37, 58, 0.6));
  backdrop-filter: blur(var(--glass-blur, 16px));
  -webkit-backdrop-filter: blur(var(--glass-blur, 16px));
  border-right: 1px solid var(--glass-border, rgba(255, 255, 255, 0.08));
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  position: relative;
  z-index: 100;
  box-shadow: var(--glass-shadow, 0 8px 32px rgba(0, 0, 0, 0.25));
  overflow: hidden;
}

.sidebar-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 20px;
  font-size: 18px;
  font-weight: 600;
  color: var(--color-primary);
  background: var(--glass-gradient);
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.sidebar-middle {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;

  &::-webkit-scrollbar {
    width: 4px;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: var(--glass-border, rgba(255, 255, 255, 0.15));
    border-radius: 2px;
  }
  &::-webkit-scrollbar-thumb:hover {
    background: var(--glass-border-hover, rgba(255, 255, 255, 0.25));
  }
}

.sidebar-nav {
  width: 100%;
  min-width: 0;
  padding: 8px 0;
  flex-shrink: 0;
}

.brand-icon {
  font-size: 22px;
  text-shadow: var(--shadow-brand-icon);
}

.sidebar-menu {
  border-right: none;
  background: transparent;
  width: 100% !important;
  --el-menu-bg-color: transparent;
  --el-menu-text-color: var(--color-text-secondary);
  --el-menu-hover-bg-color: var(--glass-hover-bg);
  --el-menu-active-color: var(--color-primary);

  :deep(.el-menu-item) {
    margin: 4px 8px;
    border-radius: var(--radius-md);
    color: var(--el-menu-text-color);
    opacity: 1;
    transition: background-color var(--transition), box-shadow var(--transition), transform var(--transition-fast);

    &:hover {
      background: var(--glass-hover-bg);
      box-shadow: var(--glass-glow);
    }

    &.is-active {
      background: var(--glass-active-bg);
      color: var(--el-menu-active-color);
      box-shadow: var(--glass-glow), inset 0 0 0 1px var(--glass-border-hover);
    }
  }

  :deep(.el-sub-menu__title) {
    border-radius: var(--radius-md);
  }
}

.session-section {
  display: flex;
  flex-direction: column;
  padding: 0 8px;
  flex-shrink: 0;
}

.session-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  font-size: 12px;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.sidebar-personas {
  padding: 12px;
  border-top: 1px solid var(--glass-border);
  background: var(--glass-bg-light);
  flex-shrink: 0;
}

.persona-label {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-bottom: 8px;
  flex-shrink: 0;
}

.persona-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 200px;
  overflow-y: auto;
  overflow-x: hidden;

  &::-webkit-scrollbar {
    width: 3px;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: var(--glass-border, rgba(255, 255, 255, 0.15));
    border-radius: 2px;
  }
}

.persona-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-secondary);
  opacity: 1;
  transition: background-color var(--transition), border-color var(--transition), color var(--transition), box-shadow var(--transition);
  background: transparent;
  border: 1px solid transparent;
  flex-shrink: 0;

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border);
  }

  &.active {
    background: var(--glass-active-bg);
    border-color: var(--glass-border-hover);
    color: var(--color-primary);
    box-shadow: var(--glass-glow);
  }
}

.persona-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.topbar {
  height: var(--topbar-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  flex-shrink: 0;
  box-shadow: var(--shadow-topbar);
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.topbar-actions {
  :deep(.base-card__body) {
    padding: 4px;
  }
}

.topbar-title {
  font-size: 15px;
  font-weight: 500;
  color: var(--color-text-primary);
}

.main-content {
  flex: 1;
  overflow: hidden;
  background: var(--bg-gradient-base);
}
</style>
