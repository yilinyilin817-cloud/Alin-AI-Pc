<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { Search, ChatDotRound, User, Cpu, Collection, Setting, Plus, Close, SetUp, Grid } from "@element-plus/icons-vue";
import BaseModal from "@/components/base/BaseModal.vue";
import { useChatStore } from "@/stores/chat";
import { usePersonaStore } from "@/stores/persona";
import { usePluginStore } from "@/stores/plugin";
import { useModelStore } from "@/stores/model";
import { useSettingsStore } from "@/stores/settings";
import { useTheme } from "@/composables/useTheme";
import { isTauri } from "@/api/env";
import type { ModelConfig } from "@/types";

interface PaletteItem {
  id: string;
  type: "session" | "persona" | "model" | "knowledge" | "settings" | "command" | "workflow" | "plugin";
  title: string;
  subtitle: string;
  icon?: typeof Search;
  action: () => void | Promise<void>;
}

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

const router = useRouter();
const chatStore = useChatStore();
const personaStore = usePersonaStore();
const pluginStore = usePluginStore();
const modelStore = useModelStore();
const settingsStore = useSettingsStore();
const { toggleTheme } = useTheme();

const query = ref("");
const selectedIndex = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);
const models = ref<ModelConfig[]>([]);

const categoryLabels: Record<PaletteItem["type"], string> = {
  session: "会话",
  persona: "角色",
  workflow: "工作流",
  plugin: "插件",
  model: "模型",
  knowledge: "知识库",
  settings: "设置",
  command: "命令",
};

const categoryIcons: Record<PaletteItem["type"], typeof Search> = {
  session: ChatDotRound,
  persona: User,
  workflow: SetUp,
  plugin: Grid,
  model: Cpu,
  knowledge: Collection,
  settings: Setting,
  command: Plus,
};

function close() {
  emit("update:modelValue", false);
  query.value = "";
}

async function handleNewChat() {
  const pid = personaStore.currentPersonaId ?? personaStore.personas[0]?.id;
  if (!pid) return;
  const session = await chatStore.createSession(pid);
  close();
  router.push(`/chat/${session.id}`).catch((e) => console.warn("command new chat:", e));
}

function handleToggleTheme() {
  toggleTheme();
  close();
}

function handleOpenSettings() {
  close();
  router.push("/settings").catch((e) => console.warn("command settings:", e));
}

async function handleExit() {
  if (isTauri()) {
    try {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      await getCurrentWebviewWindow().close();
    } catch (e) {
      console.warn("Tauri close window failed, fallback to window.close", e);
      window.close();
    }
  } else {
    window.close();
  }
}

const staticItems = computed<PaletteItem[]>(() => {
  const fontSizeMap: Record<string, string> = { small: "小", default: "默认", large: "大" };
  const densityMap: Record<string, string> = { compact: "紧凑", default: "默认", cozy: "宽松" };
  const themeMap: Record<string, string> = { dark: "深色", light: "浅色", system: "跟随系统" };

  return [
    {
      id: "knowledge-all",
      type: "knowledge",
      title: "知识库",
      subtitle: "浏览与管理知识库",
      action: () => {
        close();
        router.push("/knowledge").catch((e) => console.warn("command knowledge:", e));
      },
    },
    {
      id: "settings-theme",
      type: "settings",
      title: "切换主题",
      subtitle: `当前主题：${themeMap[settingsStore.theme] ?? settingsStore.theme}`,
      action: handleToggleTheme,
    },
    {
      id: "settings-font",
      type: "settings",
      title: "字体大小",
      subtitle: `当前：${fontSizeMap[settingsStore.fontSize] ?? settingsStore.fontSize}`,
      action: handleOpenSettings,
    },
    {
      id: "settings-density",
      type: "settings",
      title: "消息密度",
      subtitle: `当前：${densityMap[settingsStore.messageDensity] ?? settingsStore.messageDensity}`,
      action: handleOpenSettings,
    },
    {
      id: "cmd-new-chat",
      type: "command",
      title: "新建聊天",
      subtitle: "创建一个与当前角色的新会话",
      action: handleNewChat,
    },
    {
      id: "cmd-toggle-theme",
      type: "command",
      title: "切换主题",
      subtitle: "在深色与浅色模式之间切换",
      action: handleToggleTheme,
    },
    {
      id: "cmd-open-settings",
      type: "command",
      title: "打开设置",
      subtitle: "进入设置页面",
      action: handleOpenSettings,
    },
    {
      id: "cmd-exit",
      type: "command",
      title: "退出应用",
      subtitle: "关闭 AI 伴侣",
      action: handleExit,
    },
  ];
});

const allItems = computed<PaletteItem[]>(() => {
  const sessions: PaletteItem[] = chatStore.sessions.map((s) => ({
    id: `session-${s.id}`,
    type: "session",
    title: s.title,
    subtitle: new Date(s.updatedAt).toLocaleString("zh-CN"),
    action: () => {
      chatStore.selectSession(s.id);
      close();
      router.push(`/chat/${s.id}`).catch((e) => console.warn("command session:", e));
    },
  }));

  const personas: PaletteItem[] = personaStore.personas.map((p) => ({
    id: `persona-${p.id}`,
    type: "persona",
    title: p.name,
    subtitle: p.greeting,
    action: async () => {
      personaStore.selectPersona(p.id);
      const session = await chatStore.createSession(p.id);
      close();
      router.push(`/chat/${session.id}`).catch((e) => console.warn("command persona:", e));
    },
  }));

  const workflowItems: PaletteItem[] = personaStore.personas.flatMap((p) =>
    (p.workflows ?? []).map((w) => ({
      id: `workflow-${w.id}`,
      type: "workflow" as const,
      title: w.name || "未命名工作流",
      subtitle: `${p.name} · ${w.trigger.type === "message" ? "收到消息" : w.trigger.type === "scheduled" ? "定时" : "事件"}`,
      action: () => {
        close();
        router.push(`/persona/${p.id}?tab=workflows`).catch((e) => console.warn("command workflow:", e));
      },
    })),
  );

  const pluginItems: PaletteItem[] = pluginStore.plugins.map((plugin) => ({
    id: `plugin-${plugin.id}`,
    type: "plugin" as const,
    title: plugin.manifest.name,
    subtitle: plugin.manifest.description || (plugin.enabled ? "已启用" : "已禁用"),
    action: () => {
      close();
      router.push("/plugins").catch((e) => console.warn("command plugin:", e));
    },
  }));

  const modelItems: PaletteItem[] = models.value.map((m) => ({
    id: `model-${m.id}`,
    type: "model",
    title: m.name,
    subtitle: `${m.modelType} · ${m.isActive ? "使用中" : m.status}`,
    action: () => {
      close();
      router.push("/models").catch((e) => console.warn("command model:", e));
    },
  }));

  return [...sessions, ...personas, ...workflowItems, ...pluginItems, ...modelItems, ...staticItems.value];
});

const filteredItems = computed<PaletteItem[]>(() => {
  const q = query.value.trim().toLowerCase();

  // Slash commands
  if (q.startsWith("/")) {
    switch (q) {
      case "/new":
        return allItems.value.filter((i) => i.id === "cmd-new-chat");
      case "/theme":
        return allItems.value.filter((i) => i.id === "cmd-toggle-theme" || i.id === "settings-theme");
      case "/settings":
        return allItems.value.filter((i) => i.id === "cmd-open-settings");
      default:
        return [];
    }
  }

  if (!q) return allItems.value;
  return allItems.value.filter(
    (i) => i.title.toLowerCase().includes(q) || i.subtitle.toLowerCase().includes(q),
  );
});

const groupedItems = computed(() => {
  const groups: Record<string, PaletteItem[]> = {};
  const order: PaletteItem["type"][] = ["session", "persona", "workflow", "plugin", "model", "knowledge", "settings", "command"];
  for (const item of filteredItems.value) {
    if (!groups[item.type]) groups[item.type] = [];
    groups[item.type].push(item);
  }
  return order
    .filter((type) => groups[type]?.length)
    .map((type) => ({ type, label: categoryLabels[type], icon: categoryIcons[type], items: groups[type] }));
});

const flatItems = computed(() => groupedItems.value.flatMap((g) => g.items));

function execute(item: PaletteItem) {
  item.action();
}

function executeSelected() {
  const item = flatItems.value[selectedIndex.value];
  if (item) execute(item);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, flatItems.value.length - 1);
    scrollSelectedIntoView();
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
    scrollSelectedIntoView();
  } else if (event.key === "Enter") {
    event.preventDefault();
    executeSelected();
  } else if (event.key === "Escape") {
    event.preventDefault();
    close();
  }
}

function scrollSelectedIntoView() {
  nextTick(() => {
    const el = document.querySelector(".palette-item.is-selected");
    el?.scrollIntoView({ block: "nearest" });
  });
}

watch(
  () => props.modelValue,
  (visible) => {
    if (visible) {
      query.value = "";
      selectedIndex.value = 0;
      // 加载模型、角色与插件列表
      if (!personaStore.initialized) {
        personaStore.loadPersonas().catch((e) => console.warn("CommandPalette: load personas failed", e));
      }
      if (!pluginStore.initialized) {
        pluginStore.loadPlugins().catch((e) => console.warn("CommandPalette: load plugins failed", e));
      }
      modelStore.loadModels().then((list) => {
        models.value = list;
      }).catch((e) => console.warn("CommandPalette: load models failed", e));
      nextTick(() => {
        inputRef.value?.focus();
      });
    }
  },
);

watch(query, () => {
  selectedIndex.value = 0;
});
</script>

<template>
  <BaseModal
    :model-value="modelValue"
    width="640px"
    :show-close="false"
    :close-on-click-modal="true"
    :close-on-press-escape="true"
    class="command-palette-modal"
    @update:model-value="(v) => emit('update:modelValue', v)"
  >
    <div class="command-palette" @keydown="onKeydown">
      <div class="palette-input-wrap">
        <el-icon class="palette-search-icon"><Search /></el-icon>
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          class="palette-input"
          placeholder="搜索会话、角色、模型、命令… 试试 /new /theme /settings"
          autocomplete="off"
          spellcheck="false"
        />
        <el-button
          v-if="query"
          class="palette-clear"
          text
          circle
          size="small"
          :icon="Close"
          @click="query = ''"
        />
      </div>

      <div class="palette-results">
        <template v-if="flatItems.length">
          <div
            v-for="group in groupedItems"
            :key="group.type"
            class="palette-group"
          >
            <div class="palette-group-label">
              <el-icon><component :is="group.icon" /></el-icon>
              <span>{{ group.label }}</span>
            </div>
            <div
              v-for="item in group.items"
              :key="item.id"
              class="palette-item"
              :class="{ 'is-selected': flatItems[selectedIndex]?.id === item.id }"
              @click="execute(item)"
              @mouseenter="selectedIndex = flatItems.findIndex((i) => i.id === item.id)"
            >
              <div class="item-main">
                <div class="item-title">{{ item.title }}</div>
                <div class="item-subtitle">{{ item.subtitle }}</div>
              </div>
            </div>
          </div>
        </template>
        <div v-else class="palette-empty">
          未找到匹配项
        </div>
      </div>

      <div class="palette-footer">
        <span><kbd>↑</kbd><kbd>↓</kbd> 选择</span>
        <span><kbd>Enter</kbd> 执行</span>
        <span><kbd>Esc</kbd> 关闭</span>
      </div>
    </div>
  </BaseModal>
</template>

<style scoped lang="scss">
.command-palette-modal {
  :deep(.el-dialog) {
    border-radius: var(--radius-xl);
    overflow: hidden;
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    border: 1px solid var(--glass-border);
    box-shadow: var(--shadow-lg);
  }

  :deep(.el-dialog__header) {
    display: none;
  }

  :deep(.el-dialog__body) {
    padding: 0;
  }
}

.command-palette {
  display: flex;
  flex-direction: column;
  max-height: 70vh;
}

.palette-input-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--glass-border);
}

.palette-search-icon {
  color: var(--color-text-muted);
  font-size: 18px;
}

.palette-input {
  flex: 1;
  border: none;
  background: transparent;
  color: var(--color-text-primary);
  font-size: 16px;
  outline: none;

  &::placeholder {
    color: var(--color-text-muted);
  }
}

.palette-clear {
  color: var(--color-text-muted);
}

.palette-results {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.palette-group {
  margin-bottom: 8px;
}

.palette-group-label {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.palette-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);

  &:hover,
  &.is-selected {
    background: var(--glass-active-bg);
    box-shadow: var(--glass-glow);
  }
}

.item-main {
  flex: 1;
  min-width: 0;
}

.item-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-subtitle {
  font-size: 12px;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}

.palette-empty {
  padding: 32px 0;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 14px;
}

.palette-footer {
  display: flex;
  gap: 16px;
  padding: 10px 18px;
  border-top: 1px solid var(--glass-border);
  font-size: 12px;
  color: var(--color-text-muted);

  kbd {
    display: inline-block;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background: var(--glass-active-bg);
    border: 1px solid var(--glass-border);
    font-family: inherit;
    margin: 0 2px;
  }
}
</style>
