<template>
  <div class="wechat-view">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="title-row">
          <div class="title">
            <el-icon class="title-icon"><ChatDotRound /></el-icon>
            <span>微信消息</span>
          </div>
          <div class="header-right">
            <el-tag
              v-if="store.account"
              size="small"
              :type="accountTagType"
              effect="dark"
              class="status-tag"
            >
              {{ accountStatusLabel }}
            </el-tag>
            <el-tooltip
              v-if="isLoggedIn"
              :content="bindPanelVisible ? '收起设置' : '展开设置'"
              placement="top"
            >
              <button class="toggle-bind-btn" @click="bindPanelVisible = !bindPanelVisible">
                <el-icon><ArrowUp v-if="bindPanelVisible" /><ArrowDown v-else /></el-icon>
              </button>
            </el-tooltip>
          </div>
        </div>
      </div>

      <transition name="bind-collapse">
        <div v-if="isLoggedIn && bindPanelVisible" class="persona-bind">
        <div class="bind-item">
          <span class="bind-label">角色</span>
          <el-select
            :model-value="store.boundPersonaId"
            size="small"
            placeholder="选择角色"
            clearable
            @change="store.setPersona($event as string | null)"
          >
            <el-option
              v-for="p in store.availablePersonas"
              :key="p.id"
              :label="p.name"
              :value="p.id"
            />
          </el-select>
        </div>
        <div class="bind-item">
          <span class="bind-label">自动回复</span>
          <el-switch
            :model-value="store.autoReply"
            size="small"
            @change="store.setAutoReply($event as boolean)"
          />
        </div>
      </div>
      </transition>

      <div v-if="!isLoggedIn" class="login-box">
        <p class="muted small login-desc">
          基于微信 OpenClaw iLink 协议实现的机器人通道。需用微信扫描二维码登录。
        </p>
        <div v-if="store.account?.qrcodeUrl" class="qrcode-area">
          <img
            v-if="qrcodeDataUrl"
            :src="qrcodeDataUrl"
            alt="qrcode"
            class="qr-img"
          />
          <div v-else class="qr-fallback">
            <el-alert
              :title="qrcodeError || '二维码渲染中…'"
              type="info"
              :closable="false"
              show-icon
            />
            <el-input
              :model-value="store.account.qrcodeUrl"
              readonly
              size="small"
            >
              <template #append>
                <el-button
                  size="small"
                  @click="copyQrcodeUrl(store.account!.qrcodeUrl!)"
                >
                  复制
                </el-button>
              </template>
            </el-input>
          </div>
          <div class="qr-hint small muted">
            用微信扫一扫上方二维码，或复制链接到微信打开
          </div>
          <div class="qr-status">
            <el-tag v-if="store.account.qrcodeStatus === 'pending'" type="warning" size="small">
              等待扫码
            </el-tag>
            <el-tag v-else-if="store.account.qrcodeStatus === 'scanned'" type="primary" size="small">
              已扫码，请确认
            </el-tag>
            <el-tag v-else-if="store.account.qrcodeStatus === 'confirmed'" type="success" size="small">
              登录成功
            </el-tag>
            <el-tag v-else-if="store.account.qrcodeStatus === 'expired'" type="danger" size="small">
              二维码已过期
            </el-tag>
            <el-button size="small" link type="primary" @click="requestQrcode">刷新</el-button>
          </div>
        </div>
        <el-button
          v-else
          type="primary"
          :loading="store.loading"
          class="login-btn"
          @click="requestQrcode"
        >
          申请登录二维码
        </el-button>
        <div v-if="store.account?.qrcodeUrl" class="login-actions">
          <el-button :loading="store.loginPolling" @click="pollLogin">
            检查登录
          </el-button>
        </div>
        <el-alert
          v-if="store.account?.lastError"
          :title="store.account.lastError"
          type="error"
          :closable="false"
          show-icon
        />
      </div>

      <div v-else class="session-list">
        <div class="session-search">
          <el-input
            v-model="searchText"
            placeholder="搜索会话"
            size="small"
            clearable
          >
            <template #prefix><el-icon><Search /></el-icon></template>
          </el-input>
        </div>
        <el-scrollbar class="session-scroll">
          <div
            v-for="s in filteredSessions"
            :key="s.id"
            class="session-item"
            :class="{ active: s.id === store.currentSessionId }"
            @click="openSession(s.id)"
          >
            <el-avatar :src="s.peerAvatar" :size="42" shape="circle" class="peer-avatar">
              {{ (s.peerName ?? s.peerId).slice(0, 1) }}
            </el-avatar>
            <div class="session-meta">
              <div class="row1">
                <span class="name">
                  {{ s.peerName ?? s.peerId }}
                  <el-tag v-if="s.peerType === 'room'" size="small" effect="plain" class="group-tag">群</el-tag>
                </span>
                <span class="time">{{ formatTime(s.lastMsgAt) }}</span>
              </div>
              <div class="row2">
                <span class="preview">{{ s.lastMsgPreview ?? "暂无消息" }}</span>
                <el-badge
                  v-if="s.unreadCount > 0"
                  :value="s.unreadCount"
                  :max="99"
                  class="unread-badge"
                />
              </div>
            </div>
          </div>
          <el-empty
            v-if="!filteredSessions.length"
            description="还没有会话"
            :image-size="60"
          />
        </el-scrollbar>
      </div>
    </aside>

    <main class="main">
      <template v-if="store.currentSession">
        <div class="main-header">
          <div class="title-line">
            <el-avatar :src="store.currentSession.peerAvatar" :size="38" class="header-avatar">
              {{ (store.currentSession.peerName ?? store.currentSession.peerId).slice(0, 1) }}
            </el-avatar>
            <div class="header-info">
              <div class="name">{{ store.currentSession.peerName ?? store.currentSession.peerId }}</div>
              <div class="muted small peer-id">{{ store.currentSession.peerId }}</div>
            </div>
          </div>
          <div class="header-actions">
            <el-button size="small" text @click="refreshMessages">
              <el-icon><Refresh /></el-icon>刷新
            </el-button>
            <el-button size="small" type="danger" text @click="confirmLogout">登出</el-button>
          </div>
        </div>

        <el-scrollbar class="message-list" ref="scrollRef">
          <div class="message-inner">
            <div
              v-for="m in store.currentMessages"
              :key="m.id"
              class="msg-row"
              :class="m.direction"
            >
              <el-avatar
                v-if="m.direction === 'inbound'"
                :size="34"
                :src="store.currentSession.peerAvatar"
                class="msg-avatar"
              >
                {{ (store.currentSession.peerName ?? "U").slice(0, 1) }}
              </el-avatar>
              <div class="msg-content">
                <div class="bubble" :class="[m.msgType, m.direction]">
                  <div v-if="m.msgType === 'text'" class="content">{{ m.content }}</div>
                  <div v-else-if="m.msgType === 'image'" class="media">
                    <el-image
                      :src="m.mediaUrl"
                      :preview-src-list="[m.mediaUrl ?? '']"
                      fit="cover"
                      class="msg-image"
                    />
                    <div v-if="!m.mediaUrl" class="muted">[图片]</div>
                  </div>
                  <div v-else class="media muted">[{{ mediaLabel(m.msgType) }}]</div>
                </div>
                <div class="meta" :class="m.direction">
                  <span class="time">{{ formatTime(m.createdAt) }}</span>
                  <el-tag
                    v-if="m.status === 'pending'"
                    size="small"
                    type="warning"
                    effect="plain"
                  >发送中</el-tag>
                  <el-tag
                    v-else-if="m.status === 'failed'"
                    size="small"
                    type="danger"
                    effect="plain"
                  >{{ m.error ?? "失败" }}</el-tag>
                </div>
              </div>
              <el-avatar
                v-if="m.direction === 'outbound'"
                :size="34"
                :src="store.account?.avatarUrl"
                class="msg-avatar"
              >
                {{ (store.account?.nickname ?? "Me").slice(0, 1) }}
              </el-avatar>
            </div>
            <el-empty
              v-if="!store.currentMessages.length"
              description="还没有消息，发个招呼吧"
              :image-size="80"
            />
          </div>
        </el-scrollbar>

        <div class="input-area">
          <div v-if="isRecording" class="recording-bar">
            <div class="recording-indicator">
              <span class="recording-dot"></span>
              <span class="recording-text">正在录音...</span>
            </div>
            <el-button size="small" type="danger" @click="cancelVoiceRecording">
              <el-icon><Close /></el-icon>取消
            </el-button>
          </div>

          <div class="input-toolbar" v-if="isLoggedIn">
            <div class="toolbar-left">
              <el-button
                v-if="canVoiceMessage"
                size="small"
                circle
                :type="isRecording ? 'danger' : 'default'"
                :class="{ 'recording-btn': isRecording }"
                @click="handleVoiceToggle"
              >
                <el-icon><Microphone v-if="!isRecording" /><VideoPause v-else /></el-icon>
              </el-button>
              <el-button
                v-if="canSegmentedReply"
                size="small"
                :type="segmentedMode ? 'primary' : 'default'"
                @click="toggleSegmentedMode"
              >
                <el-icon class="btn-icon"><ChatDotRound /></el-icon>
                多段回复
              </el-button>
            </div>
          </div>

          <div class="input-wrapper" :class="{ 'segmented-mode': segmentedMode }">
            <template v-if="segmentedMode">
              <div class="segments-list">
                <div
                  v-for="(_, idx) in messageSegments"
                  :key="idx"
                  class="segment-item"
                  :class="{ active: idx === activeSegmentIndex }"
                  @click="activeSegmentIndex = idx"
                >
                  <span class="segment-num">{{ idx + 1 }}</span>
                  <el-input
                    v-model="messageSegments[idx]"
                    type="textarea"
                    :autosize="{ minRows: 1, maxRows: 4 }"
                    :placeholder="`第 ${idx + 1} 条消息...（回车发送，Ctrl+回车新增分段）`"
                    resize="none"
                    class="segment-input"
                    @focus="activeSegmentIndex = idx"
                    @keydown.enter.exact.prevent="handleSend"
                    @keydown.ctrl.enter.prevent="addSegmentFrom(idx)"
                  />
                  <button
                    class="segment-remove"
                    title="删除此段"
                    @click.stop="removeSegment(idx)"
                  >
                    <el-icon><Close /></el-icon>
                  </button>
                </div>
              </div>
              <div class="segments-actions">
                <el-button size="small" text type="primary" @click="addSegment">
                  <el-icon><Plus /></el-icon>添加一段
                </el-button>
                <span class="segments-tip muted small">回车发送 · Ctrl+回车新增分段</span>
              </div>
            </template>
            <template v-else>
              <el-input
                v-model="draft"
                type="textarea"
                :autosize="{ minRows: 1, maxRows: 5 }"
                placeholder="输入消息，回车发送，Shift+回车换行"
                :disabled="!isLoggedIn || isRecording"
                @keydown.enter.exact.prevent="handleSend"
                maxlength="2000"
                resize="none"
                class="msg-textarea"
              />
            </template>
            <div class="input-footer">
              <span class="char-count">
                {{ segmentedMode ? `${totalSegmentChars} 字 / ${messageSegments.length} 段` : `${draft.length} / 2000` }}
              </span>
              <button
                class="send-btn"
                :class="{ 'can-send': (segmentedMode ? hasValidSegments : draft.trim()) && !sending }"
                :disabled="(!segmentedMode && !draft.trim()) || (segmentedMode && !hasValidSegments) || sending"
                @click="handleSend"
              >
                <el-icon v-if="!sending"><Promotion /></el-icon>
                <el-icon v-else class="loading-icon"><Loading /></el-icon>
              </button>
            </div>
          </div>
        </div>
      </template>

      <div v-else class="empty">
        <el-empty
          description="选择一个会话开始聊天"
          :image-size="120"
        >
          <el-button v-if="isLoggedIn" type="primary" @click="store.loadSessions()">
            刷新会话列表
          </el-button>
        </el-empty>
      </div>
    </main>

    <aside class="context">
      <div v-if="store.account" class="context-card">
        <el-avatar :size="52" :src="store.account.avatarUrl" class="context-avatar">
          {{ (store.account.nickname ?? (store.account.status === "online" ? "U" : "?")).slice(0, 1) }}
        </el-avatar>
        <div class="account-meta">
          <div class="account-name">
            {{ store.account.nickname ?? "未登录" }}
          </div>
          <div class="account-status">
            <span class="status-dot" :class="store.account.status"></span>
            {{ accountStatusLabel }}
          </div>
        </div>
      </div>

      <div v-if="isLoggedIn" class="context-section">
        <div class="section-title">消息同步</div>
        <p class="muted small section-desc">
          后台长轮询自动推送消息，网络异常自动重连。
        </p>
        <el-alert
          v-if="isLoggedIn && lastUpdateState === 'idle'"
          type="warning"
          :closable="false"
          show-icon
          class="clawbot-hint"
        >
          <template #title>收不到消息？</template>
          <div class="muted small hint-text">
            打开微信 <strong>我 → 设置 → 插件</strong>，启用 <strong>ClawBot</strong>
          </div>
        </el-alert>
        <el-button size="small" class="sync-btn" @click="manualSync">手动拉取</el-button>
      </div>

      <div class="context-section technical">
        <el-collapse>
          <el-collapse-item title="技术信息">
            <div class="tech-list muted small">
              <div class="tech-item">
                <span class="tech-label">UID</span>
                <span class="tech-value">{{ store.account?.userId ?? "—" }}</span>
              </div>
              <div class="tech-item">
                <span class="tech-label">同步游标</span>
                <span class="tech-value">{{ store.account?.getUpdatesBuf ?? "0" }}</span>
              </div>
              <div class="tech-item">
                <span class="tech-label">网关</span>
                <code>ilinkai.weixin.qq.com</code>
              </div>
              <div class="tech-item">
                <span class="tech-label">CDN</span>
                <code>novac2c.cdn.weixin.qq.com</code>
              </div>
            </div>
          </el-collapse-item>
        </el-collapse>
      </div>
    </aside>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import { ChatDotRound, Search, Refresh, Promotion, Loading, Plus, Close, Microphone, VideoPause, ArrowUp, ArrowDown } from "@element-plus/icons-vue";
import QRCode from "qrcode";
import { useWeChatStore } from "@/stores/wechat";
import { useVoiceStore } from "@/stores/voice";
import {
  listWeChatMessages,
  wechatStartSync,
} from "@/api/wechat";
import { isTauri } from "@/api/env";
import type { WeChatMsgType, PersonaDefinition } from "@/types";

const route = useRoute();
const router = useRouter();
const store = useWeChatStore();
const voiceStore = useVoiceStore();

const searchText = ref("");
const draft = ref("");
const sending = ref(false);
const bindPanelVisible = ref(true);
const scrollRef = ref<any>(null);
const qrcodeDataUrl = ref<string>("");
const qrcodeError = ref<string>("");
const lastUpdateState = ref<"idle" | "live">("idle");
const segmentedMode = ref(false);
const messageSegments = ref<string[]>([""]);
const activeSegmentIndex = ref(0);
const segmentInputRef = ref<HTMLTextAreaElement | null>(null);

watch(
  () => store.currentMessages.length,
  (newLen, oldLen) => {
    if (newLen > (oldLen ?? 0)) {
      lastUpdateState.value = "live";
      const el = scrollRef.value;
      if (el && el.scrollHeight - el.scrollTop - el.clientHeight < 80) {
        scrollToBottom();
      }
    }
  },
);

function getWechatColor(name: string, fallback: string): string {
  if (typeof window === "undefined") return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

async function renderQrcodeDataUrl(url: string) {
  qrcodeError.value = "";
  if (!url) {
    qrcodeDataUrl.value = "";
    return;
  }
  try {
    qrcodeDataUrl.value = await QRCode.toDataURL(url, {
      errorCorrectionLevel: "M",
      width: 240,
      margin: 2,
      color: {
        dark: getWechatColor("--wechat-qrcode-fg", "#000000"),
        light: getWechatColor("--wechat-qrcode-bg", "#ffffff"),
      },
    });
  } catch (e) {
    qrcodeError.value = `二维码渲染失败: ${e instanceof Error ? e.message : String(e)}`;
    qrcodeDataUrl.value = "";
  }
}

watch(
  () => store.account?.qrcodeUrl,
  (url) => { renderQrcodeDataUrl(url ?? ""); },
  { immediate: true },
);

const isLoggedIn = computed(() => store.account?.status === "online");

const currentBoundPersona = computed<PersonaDefinition | null>(() => {
  if (!store.boundPersonaId) return null;
  return store.availablePersonas.find((p) => p.id === store.boundPersonaId) ?? null;
});

const wechatConfig = computed(() => currentBoundPersona.value?.wechat ?? {
  enableSegmentedReply: false,
  segmentDelay: 800,
  enableVoiceMessage: false,
  voiceAutoSend: false,
  voiceAsrEnabled: true,
});

const canSegmentedReply = computed(() => wechatConfig.value.enableSegmentedReply);
const canVoiceMessage = computed(() => wechatConfig.value.enableVoiceMessage);
const isRecording = computed(() => voiceStore.state === "recording");

watch(canSegmentedReply, (enabled) => {
  if (!enabled && segmentedMode.value) {
    segmentedMode.value = false;
    const firstValid = messageSegments.value.find((s) => s.trim());
    draft.value = firstValid ?? "";
    messageSegments.value = [""];
  }
});

watch(canVoiceMessage, (enabled) => {
  if (!enabled && isRecording.value) {
    cancelVoiceRecording();
  }
});

watch(() => store.boundPersonaId, () => {
  draft.value = "";
  segmentedMode.value = false;
  messageSegments.value = [""];
  activeSegmentIndex.value = 0;
});

const totalSegmentChars = computed(() =>
  messageSegments.value.reduce((acc, s) => acc + s.length, 0),
);

const hasValidSegments = computed(() =>
  messageSegments.value.some((s) => s.trim().length > 0),
);

function addSegment() {
  messageSegments.value.push("");
  activeSegmentIndex.value = messageSegments.value.length - 1;
  nextTick(() => segmentInputRef.value?.focus());
}

function addSegmentFrom(idx: number) {
  messageSegments.value.splice(idx + 1, 0, "");
  activeSegmentIndex.value = idx + 1;
  nextTick(() => segmentInputRef.value?.focus());
}

function removeSegment(index: number) {
  if (messageSegments.value.length <= 1) {
    messageSegments.value[0] = "";
    return;
  }
  messageSegments.value.splice(index, 1);
  if (activeSegmentIndex.value >= messageSegments.value.length) {
    activeSegmentIndex.value = messageSegments.value.length - 1;
  }
}

function toggleSegmentedMode() {
  segmentedMode.value = !segmentedMode.value;
  if (segmentedMode.value) {
    const currentDraft = draft.value.trim();
    if (currentDraft) {
      messageSegments.value = [currentDraft];
    } else {
      messageSegments.value = messageSegments.value.some((s) => s.trim())
        ? messageSegments.value
        : [""];
    }
    draft.value = "";
    activeSegmentIndex.value = 0;
  } else {
    const firstValid = messageSegments.value.find((s) => s.trim());
    draft.value = firstValid ?? "";
    messageSegments.value = [""];
  }
}

async function handleVoiceToggle() {
  if (!isTauri()) {
    ElMessage.warning("语音功能仅在桌面端可用");
    return;
  }
  if (isRecording.value) {
    try {
      const text = await voiceStore.stopRecording();
      if (text && text.trim()) {
        if (segmentedMode.value) {
          messageSegments.value[activeSegmentIndex.value] = text;
        } else {
          draft.value = text;
        }
        ElMessage.success("语音已转文字");
      }
    } catch (e: any) {
      ElMessage.error(`语音识别失败: ${e?.message ?? e}`);
    }
  } else {
    try {
      await voiceStore.startRecording();
    } catch (e: any) {
      ElMessage.error(`启动录音失败: ${e?.message ?? e}`);
    }
  }
}

function cancelVoiceRecording() {
  voiceStore.cancelRecording();
}

const accountStatusLabel = computed(() => {
  switch (store.account?.status) {
    case "online": return "在线";
    case "logging_in": return "登录中";
    case "error": return "异常";
    default: return "离线";
  }
});

const accountTagType = computed<"success" | "warning" | "danger" | "info">(() => {
  switch (store.account?.status) {
    case "online": return "success";
    case "logging_in": return "warning";
    case "error": return "danger";
    default: return "info";
  }
});

const filteredSessions = computed(() => {
  const q = searchText.value.trim().toLowerCase();
  if (!q) return store.sessions;
  return store.sessions.filter(
    (s) =>
      (s.peerName ?? "").toLowerCase().includes(q) ||
      s.peerId.toLowerCase().includes(q),
  );
});

onMounted(async () => {
  await store.init();
  const sid = route.params.sessionId as string | undefined;
  if (sid) {
    await store.openSession(sid);
  } else if (store.sessions[0]) {
    await store.openSession(store.sessions[0].id);
  }
  scrollToBottom();
});

watch(
  () => store.currentMessages.length,
  () => nextTick(scrollToBottom),
);

watch(
  () => route.params.sessionId,
  async (sid) => {
    if (typeof sid === "string") {
      await store.openSession(sid);
      scrollToBottom();
    }
  },
);

async function openSession(id: string) {
  await store.openSession(id);
  router.replace({ name: "wechat", params: { sessionId: id } });
  scrollToBottom();
}

async function requestQrcode() {
  try {
    await store.requestQrcode();
  } catch (e: any) {
    ElMessage.error(`申请二维码失败: ${e?.message ?? e}`);
  }
}

async function copyQrcodeUrl(url: string) {
  try {
    await navigator.clipboard.writeText(url);
    ElMessage.success("已复制到剪贴板");
  } catch {
    ElMessage.warning("复制失败，请手动选中复制");
  }
}

async function pollLogin() {
  try {
    const res = await store.pollLogin();
    if (res.status === "confirmed") {
      ElMessage.success("登录成功");
    } else if (res.status === "expired") {
      ElMessage.warning("二维码已过期");
    } else if (res.status === "scanned") {
      ElMessage.info("已扫码，请确认");
    } else if (res.message) {
      ElMessage.info(res.message);
    }
  } catch (e: any) {
    ElMessage.error(`轮询失败: ${e?.message ?? e}`);
  }
}

async function confirmLogout() {
  try {
    await ElMessageBox.confirm("确定登出当前微信账号？", "确认", { type: "warning" });
    await store.logout();
    ElMessage.success("已登出");
  } catch { /* cancel */ }
}

async function handleSend() {
  if (segmentedMode.value) {
    const validTexts = messageSegments.value.filter((s) => s.trim());
    if (!validTexts.length) return;
    sending.value = true;
    try {
      await store.sendMultipleTexts(validTexts, wechatConfig.value.segmentDelay ?? 800);
      messageSegments.value = [""];
      activeSegmentIndex.value = 0;
      await nextTick();
      scrollToBottom();
      ElMessage.success(`已发送 ${validTexts.length} 条消息`);
    } catch (e: any) {
      ElMessage.error(`发送失败: ${e?.message ?? e}`);
    } finally {
      sending.value = false;
    }
    return;
  }

  const text = draft.value.trim();
  if (!text) return;
  sending.value = true;
  try {
    await store.sendText(text);
    draft.value = "";
    await nextTick();
    scrollToBottom();
  } catch (e: any) {
    ElMessage.error(`发送失败: ${e?.message ?? e}`);
  } finally {
    sending.value = false;
  }
}

async function refreshMessages() {
  if (!store.currentSessionId) return;
  try {
    const list = await listWeChatMessages(store.currentSessionId, 200);
    store.messagesBySession[store.currentSessionId] = list;
    await nextTick();
    scrollToBottom();
  } catch (e) {
    ElMessage.error("刷新失败");
  }
}

async function manualSync() {
  try {
    await wechatStartSync();
    ElMessage.success("已重新拉起同步任务");
  } catch (e: any) {
    ElMessage.error(`重启失败: ${e?.message ?? e}`);
  }
}

async function scrollToBottom() {
  if (!scrollRef.value) return;
  await nextTick();
  const scrollInstance = scrollRef.value;
  if (scrollInstance.setScrollTop) {
    scrollInstance.setScrollTop(1000000);
  }
}

function formatTime(iso?: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const now = new Date();
  if (d.toDateString() === now.toDateString()) {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return d.toLocaleDateString();
}

function mediaLabel(t: WeChatMsgType): string {
  switch (t) {
    case "image": return "图片";
    case "voice": return "语音";
    case "video": return "视频";
    case "file": return "文件";
    case "system": return "系统消息";
    default: return "未知";
  }
}
</script>

<style scoped lang="scss">
.wechat-view {
  display: grid;
  grid-template-columns: 300px 1fr 260px;
  height: 100%;
  width: 100%;
  background: var(--bg-gradient-base);
  overflow: hidden;
}

/* ========== Sidebar ========== */
.sidebar {
  border-right: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

.sidebar-header {
  padding: 16px 16px 12px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 16px;

  .title-icon {
    font-size: 18px;
    color: #07c160;
  }
}

.status-tag {
  border-radius: 10px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-bind-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  background: transparent;
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.18s ease;

  .el-icon {
    font-size: 13px;
  }

  &:hover {
    background: var(--glass-hover-bg);
    color: var(--color-primary);
    border-color: var(--color-primary);
  }
}

.persona-bind {
  padding: 10px 16px;
  border-bottom: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--glass-bg-light);
  flex-shrink: 0;
  overflow: hidden;
}

.bind-collapse-enter-active,
.bind-collapse-leave-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.bind-collapse-enter-from,
.bind-collapse-leave-to {
  opacity: 0;
  padding-top: 0;
  padding-bottom: 0;
  max-height: 0;
}

.bind-collapse-enter-to,
.bind-collapse-leave-from {
  opacity: 1;
  max-height: 100px;
}

.bind-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.bind-label {
  font-size: 12px;
  color: var(--color-text-secondary);
  flex-shrink: 0;
}

.bind-item :deep(.el-select) {
  width: 150px;
}

.login-box {
  padding: 20px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  align-items: center;
}

.login-desc {
  text-align: center;
  line-height: 1.6;
}

.qrcode-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.qr-img {
  width: 220px;
  height: 220px;
  border: 1px solid var(--glass-border);
  border-radius: 12px;
  background: #fff;
  padding: 8px;
  box-shadow: 0 2px 12px rgba(0,0,0,0.08);
}

.qr-hint {
  text-align: center;
  max-width: 240px;
  line-height: 1.5;
}

.qr-fallback {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.qr-status {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  justify-content: center;
}

.login-btn {
  width: 100%;
  border-radius: 10px;
  height: 40px;
  font-weight: 500;
}

.login-actions {
  display: flex;
  gap: 8px;
  width: 100%;
  justify-content: center;
}

.session-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.session-search {
  padding: 12px 12px 8px;
  flex-shrink: 0;

  :deep(.el-input__wrapper) {
    border-radius: 10px;
    background: var(--glass-bg-light);
    box-shadow: 0 0 0 1px var(--glass-border);
  }
}

.session-scroll {
  flex: 1;
}

.session-item {
  display: flex;
  gap: 10px;
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s ease;
  border-radius: 0;

  &:hover {
    background: var(--glass-hover-bg);
  }

  &.active {
    background: var(--wechat-session-active-bg);
  }

  .peer-avatar {
    flex-shrink: 0;
    background: linear-gradient(135deg, #07c160, #2ba471);
    color: #fff;
    font-weight: 600;
    font-size: 16px;
  }

  .session-meta {
    flex: 1;
    min-width: 0;

    .row1 {
      display: flex;
      justify-content: space-between;
      align-items: center;

      .name {
        font-weight: 500;
        font-size: 14px;
        display: flex;
        align-items: center;
        gap: 4px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .group-tag {
        font-size: 10px !important;
        padding: 0 4px !important;
        height: 16px !important;
        line-height: 16px !important;
        border-radius: 4px !important;
      }

      .time {
        font-size: 11px;
        color: var(--color-text-muted);
        flex-shrink: 0;
        margin-left: 6px;
      }
    }

    .row2 {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-top: 3px;

      .preview {
        font-size: 12px;
        color: var(--color-text-secondary);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 170px;
      }

      .unread-badge {
        flex-shrink: 0;
        :deep(.el-badge__content) {
          font-size: 10px;
          height: 16px;
          line-height: 16px;
          padding: 0 5px;
          border-radius: 8px;
        }
      }
    }
  }
}

/* ========== Main Chat Area ========== */
.main {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  overflow: hidden;
  background: var(--bg-gradient-base);
}

.main-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  flex-shrink: 0;
}

.title-line {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-avatar {
  background: linear-gradient(135deg, #07c160, #2ba471);
  color: #fff;
  font-weight: 600;
  flex-shrink: 0;
}

.header-info {
  .name {
    font-weight: 600;
    font-size: 15px;
    color: var(--color-text-primary);
  }
  .peer-id {
    font-size: 11px;
    margin-top: 1px;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.header-actions {
  display: flex;
  gap: 4px;
}

.message-list {
  flex: 1;
  min-height: 0;
}

.message-inner {
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.msg-row {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  max-width: 100%;

  &.outbound {
    flex-direction: row-reverse;
  }
}

.msg-avatar {
  flex-shrink: 0;
  margin-top: 2px;

  &.inbound ~ .msg-content,
  &.outbound ~ .msg-content {
    /* noop */
  }
}

.msg-content {
  display: flex;
  flex-direction: column;
  max-width: 70%;
  min-width: 0;
}

.outbound .msg-content {
  align-items: flex-end;
}

.bubble {
  padding: 10px 14px;
  border-radius: 4px 14px 14px 14px;
  font-size: 14px;
  line-height: 1.6;
  word-break: break-word;
  position: relative;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);

  &.inbound {
    background: var(--glass-bg-light);
    color: var(--color-text-primary);
    border: 1px solid var(--glass-border);
    border-radius: 4px 14px 14px 14px;
  }

  &.outbound {
    background: #95ec69;
    color: #1a1a1a;
    border: none;
    border-radius: 14px 4px 14px 14px;
  }

  .content {
    white-space: pre-wrap;
  }

  .msg-image {
    max-width: 220px;
    border-radius: 8px;
    display: block;
  }
}

.meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
  font-size: 11px;
  color: var(--color-text-muted);
  padding: 0 4px;

  &.outbound {
    justify-content: flex-end;
  }

  :deep(.el-tag) {
    height: 18px;
    line-height: 18px;
    padding: 0 6px;
    font-size: 10px;
    border-radius: 4px;
  }
}

/* ========== Input Area ========== */
.input-area {
  padding: 12px 24px 16px;
  background: transparent;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.recording-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: linear-gradient(135deg, rgba(245,108,108,0.1), rgba(245,108,108,0.05));
  border: 1px solid rgba(245,108,108,0.3);
  border-radius: 12px;
}

.recording-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
}

.recording-dot {
  width: 10px;
  height: 10px;
  background: #f56c6c;
  border-radius: 50%;
  animation: pulse-recording 1s ease-in-out infinite;
}

@keyframes pulse-recording {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(1.2); }
}

.recording-text {
  font-size: 13px;
  color: #f56c6c;
  font-weight: 500;
}

.input-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.toolbar-left {
  display: flex;
  gap: 8px;
  align-items: center;
}

.recording-btn {
  animation: pulse-btn 1.5s ease-in-out infinite;
}

@keyframes pulse-btn {
  0%, 100% { box-shadow: 0 0 0 0 rgba(245,108,108,0.4); }
  50% { box-shadow: 0 0 0 8px rgba(245,108,108,0); }
}

.btn-icon {
  margin-right: 4px;
}

.input-wrapper {
  background: var(--glass-bg-light);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid var(--glass-border);
  border-radius: 18px;
  padding: 8px 12px 6px;
  transition: all 0.25s cubic-bezier(0.4,0,0.2,1);

  &:focus-within {
    border-color: #07c160;
    box-shadow: 0 0 0 3px rgba(7,193,96,0.1), 0 4px 16px rgba(0,0,0,0.08);
  }

  &.segmented-mode {
    padding: 12px;
  }
}

.segments-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 280px;
  overflow-y: auto;
  padding-right: 4px;
}

.segment-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px;
  border-radius: 10px;
  border: 1px solid transparent;
  transition: all 0.15s ease;
  background: transparent;

  &:hover, &.active {
    background: var(--glass-bg);
    border-color: var(--glass-border);
  }

  &.active {
    border-color: #07c160;
  }
}

.segment-num {
  width: 22px;
  height: 22px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--glass-bg-light);
  border: 1px solid var(--glass-border);
  border-radius: 6px;
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-top: 4px;
}

.segment-input {
  flex: 1;
  min-width: 0;

  :deep(.el-textarea__inner) {
    background: transparent !important;
    border: none !important;
    box-shadow: none !important;
    padding: 4px 0 !important;
    font-size: 14px;
    line-height: 1.6;
    resize: none;
    color: var(--color-text-primary);

    &::placeholder {
      color: var(--color-text-muted);
    }
  }
}

.segment-remove {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--color-text-muted);
  cursor: pointer;
  margin-top: 2px;
  transition: all 0.15s ease;
  opacity: 0;

  .segment-item:hover & {
    opacity: 1;
  }

  &:hover {
    background: rgba(245,108,108,0.1);
    color: #f56c6c;
  }
}

.segments-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px dashed var(--glass-border);
  margin-top: 4px;
}

.segments-tip {
  margin-left: auto;
  font-size: 12px;
  opacity: 0.6;
}

.msg-textarea {
  :deep(.el-textarea__inner) {
    background: transparent;
    border: none;
    box-shadow: none !important;
    color: var(--color-text-primary);
    padding: 4px 4px;
    font-size: 14px;
    line-height: 1.6;
    min-height: auto !important;
    resize: none;

    &::placeholder {
      color: var(--color-text-muted);
    }

    &:hover, &:focus {
      border: none;
      box-shadow: none !important;
    }
  }
}

.input-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 2px 4px 0;
}

.char-count {
  font-size: 11px;
  color: var(--color-text-muted);
}

.send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 10px;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4,0,0.2,1);

  .el-icon {
    font-size: 16px;
    transform: rotate(-45deg);
    transition: transform 0.2s ease;
  }

  .loading-icon {
    animation: spin 1s linear infinite;
    transform: none;
  }

  &:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  &.can-send {
    background: #07c160;
    color: #fff;
    box-shadow: 0 2px 8px rgba(7,193,96,0.3);

    &:hover:not(:disabled) {
      transform: scale(1.08);
      box-shadow: 0 4px 14px rgba(7,193,96,0.45);
    }

    &:active:not(:disabled) {
      transform: scale(0.94);
    }

    .el-icon {
      transform: rotate(0deg);
    }
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ========== Right Context Panel ========== */
.context {
  border-left: 1px solid var(--glass-border);
  padding: 16px 14px;
  overflow-y: auto;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.context-card {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 14px;
  background: var(--glass-bg-light);
  border-radius: 14px;
  border: 1px solid var(--glass-border);
}

.context-avatar {
  flex-shrink: 0;
  background: linear-gradient(135deg, #07c160, #2ba471);
  color: #fff;
  font-weight: 600;
}

.account-meta {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-status {
  font-size: 12px;
  color: var(--color-text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-muted);

  &.online { background: #07c160; }
  &.logging_in { background: #e6a23c; animation: pulse-dot 1.5s infinite; }
  &.error { background: #f56c6c; }
}

@keyframes pulse-dot {
  0%,100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.context-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.section-desc {
  line-height: 1.5;
  margin: 0;
}

.clawbot-hint {
  margin: 4px 0;
  border-radius: 10px;
}

.hint-text {
  line-height: 1.5;
}

.sync-btn {
  align-self: flex-start;
  border-radius: 8px;
}

.technical {
  margin-top: auto;
  padding-top: 8px;

  :deep(.el-collapse) {
    border: none;
    background: transparent;
  }

  :deep(.el-collapse-item__header) {
    font-size: 12px;
    color: var(--color-text-muted);
    font-weight: 500;
    height: 32px;
    line-height: 32px;
    border-bottom-color: var(--glass-border);
    background: transparent;
    padding-left: 0;
  }

  :deep(.el-collapse-item__wrap) {
    background: transparent;
    border-bottom: none;
  }

  :deep(.el-collapse-item__content) {
    padding: 8px 0 4px;
  }
}

.tech-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tech-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  font-size: 11px;

  .tech-label {
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .tech-value {
    font-family: var(--font-family-code);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-secondary);
    text-align: right;
    max-width: 140px;
  }

  code {
    font-family: var(--font-family-code);
    font-size: 10px;
    background: var(--color-bg-surface);
    padding: 1px 5px;
    border-radius: 4px;
    color: var(--color-text-secondary);
  }
}

.muted {
  color: var(--color-text-muted);
}
.small {
  font-size: 12px;
}
</style>
