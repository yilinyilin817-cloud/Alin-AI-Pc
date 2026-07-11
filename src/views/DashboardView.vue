<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  ChatDotRound,
  User,
  Cpu,
  ChatLineRound,
  Plus,
  Setting,
  Moon,
  Sunny,
  Search,
  ArrowRight,
  Microphone,
  Grid,
} from "@element-plus/icons-vue";
import { usePersonaStore } from "@/stores/persona";
import { useChatStore } from "@/stores/chat";
import { useWeChatStore } from "@/stores/wechat";
import { useModelStore } from "@/stores/model";
import { useTheme } from "@/composables/useTheme";
import BaseCard from "@/components/base/BaseCard.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import type { ModelConfig } from "@/types";

const router = useRouter();
const personaStore = usePersonaStore();
const chatStore = useChatStore();
const wechatStore = useWeChatStore();
const modelStore = useModelStore();
const { effectiveTheme, toggleTheme } = useTheme();

const models = ref<ModelConfig[]>([]);
const modelLoading = ref(false);

const themeIcon = computed(() => (effectiveTheme.value === "dark" ? Moon : Sunny));

const today = computed(() => {
  const d = new Date();
  const weekdays = ["星期日", "星期一", "星期二", "星期三", "星期四", "星期五", "星期六"];
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日 ${weekdays[d.getDay()]}`;
});

const recentSessions = computed(() => chatStore.sessions.slice(0, 6));

const activeModels = computed(() => models.value.filter((m) => m.isActive || m.status === "active"));
const onlineModelCount = computed(() => activeModels.value.length);
const modelStatusText = computed(() => {
  if (modelLoading.value) return "检测中…";
  if (onlineModelCount.value === 0) return "暂无在线模型";
  return `${onlineModelCount.value} 个模型在线`;
});

const latestWechatPreview = computed(() => {
  const sessions = wechatStore.sessions;
  if (!sessions.length) return "暂无新消息";
  const sorted = [...sessions].sort(
    (a, b) => new Date(b.lastMsgAt ?? 0).getTime() - new Date(a.lastMsgAt ?? 0).getTime(),
  );
  const latest = sorted[0];
  const sender = latest.peerName ?? "微信";
  const content = latest.lastMsgPreview ?? "";
  return content ? `${sender}: ${content}` : `${sender} 发来新消息`;
});

onMounted(async () => {
  modelLoading.value = true;
  try {
    models.value = await modelStore.loadModels();
  } catch (e) {
    console.warn("Dashboard: load models failed", e);
  } finally {
    modelLoading.value = false;
  }
});

function navigateToSession(sessionId: string) {
  chatStore.selectSession(sessionId);
  router.push(`/chat/${sessionId}`).catch((e) => console.warn("navigateToSession:", e));
}

async function startPersonaChat(personaId: string) {
  personaStore.selectPersona(personaId);
  const session = await chatStore.createSession(personaId);
  router.push(`/chat/${session.id}`).catch((e) => console.warn("startPersonaChat:", e));
}

function handleNewChat() {
  const pid = personaStore.currentPersonaId ?? personaStore.personas[0]?.id;
  if (!pid) return;
  startPersonaChat(pid);
}

function openCommandPalette() {
  // 通过全局自定义事件触发 AppLayout 中的命令面板
  window.dispatchEvent(new CustomEvent("open-command-palette"));
}

async function startVoiceInput() {
  const pid = personaStore.currentPersonaId ?? personaStore.personas[0]?.id;
  if (!pid) return;
  personaStore.selectPersona(pid);
  const session = await chatStore.createSession(pid);
  router.push(`/chat/${session.id}?voice=1`).catch((e) => console.warn("startVoiceInput:", e));
}

function openPluginMarket() {
  router.push("/skills").catch((e) => console.warn("openPluginMarket:", e));
}
</script>

<template>
  <div class="dashboard-view">
    <div class="dashboard-content">
      <header class="dashboard-header">
        <div>
          <h1 class="welcome-title">
            欢迎回来，<span class="persona-name">{{ personaStore.currentPersona?.name ?? "AI 伴侣" }}</span>
          </h1>
          <p class="date-text">{{ today }}</p>
        </div>
        <div class="header-actions">
          <BaseButton variant="primary" :icon="Plus" @click="handleNewChat">
            新建聊天
          </BaseButton>
          <BaseButton variant="ghost" :icon="Search" @click="openCommandPalette">
            命令面板
          </BaseButton>
        </div>
      </header>

      <section class="dashboard-grid">
        <!-- 最近会话 -->
        <BaseCard class="dashboard-card sessions-card">
          <template #header>
            <div class="card-header">
              <el-icon><ChatDotRound /></el-icon>
              <span>最近会话</span>
            </div>
          </template>
          <div class="session-list">
            <div
              v-for="session in recentSessions"
              :key="session.id"
              class="session-item"
              @click="navigateToSession(session.id)"
            >
              <span v-if="session.isPinned" class="pin">📌</span>
              <div class="session-info">
                <div class="session-title">{{ session.title }}</div>
                <div class="session-time">
                  {{ new Date(session.updatedAt).toLocaleString("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }) }}
                </div>
              </div>
              <el-icon class="session-arrow"><ArrowRight /></el-icon>
            </div>
            <div v-if="!recentSessions.length" class="empty-hint">
              暂无会话，点击“新建聊天”开始吧
            </div>
          </div>
        </BaseCard>

        <!-- 快捷角色 -->
        <BaseCard class="dashboard-card personas-card">
          <template #header>
            <div class="card-header">
              <el-icon><User /></el-icon>
              <span>快捷角色</span>
            </div>
          </template>
          <div class="persona-list">
            <div
              v-for="persona in personaStore.personas"
              :key="persona.id"
              class="persona-item"
              :class="{ active: persona.id === personaStore.currentPersonaId }"
              @click="startPersonaChat(persona.id)"
            >
              <el-avatar :size="40">{{ persona.name[0] }}</el-avatar>
              <div class="persona-info">
                <div class="persona-name">{{ persona.name }}</div>
                <div class="persona-desc">{{ persona.greeting }}</div>
              </div>
            </div>
          </div>
        </BaseCard>

        <!-- 模型状态 -->
        <BaseCard
          class="dashboard-card models-card"
          @click="router.push('/models')"
        >
          <template #header>
            <div class="card-header">
              <el-icon><Cpu /></el-icon>
              <span>模型状态</span>
            </div>
          </template>
          <div class="model-status">
            <div class="status-badge" :class="{ online: onlineModelCount > 0 }">
              {{ onlineModelCount > 0 ? "在线" : "离线" }}
            </div>
            <div class="status-text">{{ modelStatusText }}</div>
          </div>
          <div v-if="activeModels.length" class="model-list">
            <el-tag
              v-for="model in activeModels.slice(0, 3)"
              :key="model.id"
              size="small"
              type="success"
            >
              {{ model.name }}
            </el-tag>
          </div>
        </BaseCard>

        <!-- 微信未读 -->
        <BaseCard
          class="dashboard-card wechat-card"
          @click="router.push('/wechat')"
        >
          <template #header>
            <div class="card-header">
              <el-icon><ChatLineRound /></el-icon>
              <span>微信消息</span>
              <el-badge
                v-if="wechatStore.totalUnread > 0"
                :value="wechatStore.totalUnread"
                :max="99"
                style="margin-left: auto"
              />
            </div>
          </template>
          <div class="wechat-status">
            <div class="wechat-count">
              <span class="count-value">{{ wechatStore.totalUnread }}</span>
              <span class="count-label">条未读</span>
            </div>
            <p class="wechat-preview">{{ latestWechatPreview }}</p>
          </div>
        </BaseCard>
      </section>

      <!-- 快捷动作 -->
      <section class="quick-actions">
        <BaseButton variant="default" :icon="Plus" @click="handleNewChat">
          新建聊天
        </BaseButton>
        <BaseButton variant="default" :icon="Microphone" @click="startVoiceInput">
          语音输入
        </BaseButton>
        <BaseButton variant="default" :icon="Grid" @click="openPluginMarket">
          插件市场
        </BaseButton>
        <BaseButton variant="default" :icon="Setting" @click="router.push('/settings')">
          打开设置
        </BaseButton>
        <BaseButton variant="default" :icon="themeIcon" @click="toggleTheme">
          切换主题
        </BaseButton>
        <BaseButton variant="default" :icon="Search" @click="openCommandPalette">
          打开命令面板
        </BaseButton>
      </section>
    </div>
  </div>
</template>

<style scoped lang="scss">
.dashboard-view {
  position: relative;
  width: 100%;
  height: 100%;
  overflow-y: auto;
}

.dashboard-content {
  position: relative;
  z-index: 1;
  padding: 28px;
  max-width: 1200px;
  margin: 0 auto;
}

.dashboard-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  margin-bottom: 24px;
  gap: 16px;
  flex-wrap: wrap;
}

.welcome-title {
  font-size: 26px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0 0 6px;
}

.welcome-title .persona-name {
  color: var(--color-primary);
}

.date-text {
  margin: 0;
  font-size: 14px;
  color: var(--color-text-secondary);
}

.header-actions {
  display: flex;
  gap: 10px;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  gap: 20px;
  margin-bottom: 20px;
}

.dashboard-card {
  min-height: 220px;
  display: flex;
  flex-direction: column;
  cursor: default;

  &:hover {
    transform: translateY(-2px);
  }
}

.sessions-card {
  grid-column: span 7;
}

.personas-card {
  grid-column: span 5;
}

.models-card,
.wechat-card {
  grid-column: span 6;
  cursor: pointer;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
}

.session-list,
.persona-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.session-item,
.persona-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  background: var(--glass-bg-light);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all var(--transition-base);

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border);
  }
}

.persona-item.active {
  border-color: var(--color-primary);
  box-shadow: var(--glass-glow);
}

.pin {
  font-size: 10px;
  flex-shrink: 0;
}

.session-info,
.persona-info {
  flex: 1;
  min-width: 0;
}

.session-title,
.persona-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-time {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 2px;
}

.persona-desc {
  font-size: 12px;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}

.empty-hint {
  color: var(--color-text-muted);
  font-size: 13px;
  text-align: center;
  padding: 24px 0;
}

.session-arrow {
  color: var(--color-text-muted);
  flex-shrink: 0;
}

.model-status,
.wechat-status {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 14px;
}

.status-badge {
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  background: var(--color-bg-hover);
  color: var(--color-text-secondary);
}

.status-badge.online {
  background: var(--color-success-subtle);
  color: var(--color-success);
}

.status-text {
  font-size: 14px;
  color: var(--color-text-secondary);
}

.model-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.wechat-count {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.count-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--color-primary);
}

.count-label {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.wechat-preview {
  margin: 0;
  font-size: 13px;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

@media (max-width: 900px) {
  .dashboard-grid {
    grid-template-columns: 1fr;
  }

  .sessions-card,
  .personas-card,
  .models-card,
  .wechat-card {
    grid-column: span 1;
  }
}
</style>
