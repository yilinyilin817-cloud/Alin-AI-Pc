import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type {
  WeChatAccountView,
  WeChatMessage,
  WeChatSession,
  WeChatMessageEvent,
  WeChatAccountEvent,
} from "@/types";
import { isTauri } from "@/api/env";
import {
  getWeChatAccount,
  listWeChatSessions,
  listWeChatMessages,
  markWeChatSessionRead,
  wechatRequestQrcode,
  wechatPollLogin,
  wechatLogout,
  wechatStartSync,
  sendWeChatText,
  setWechatPersona,
  getWechatPersona,
} from "@/api/wechat";
import { listPersonas } from "@/api/persona";
import type { PersonaDefinition } from "@/types";

let unlistenMessage: (() => void) | null = null;
let unlistenAccount: (() => void) | null = null;

export const useWeChatStore = defineStore("wechat", () => {
  const account = ref<WeChatAccountView | null>(null);
  const sessions = ref<WeChatSession[]>([]);
  const messagesBySession = ref<Record<string, WeChatMessage[]>>({});
  const currentSessionId = ref<string | null>(null);
  const loading = ref(false);
  const loginPolling = ref(false);

  // 自动回复 + 角色绑定
  const autoReply = ref(false);
  const boundPersonaId = ref<string | null>(null);
  const availablePersonas = ref<PersonaDefinition[]>([]);

  const currentSession = computed(() =>
    sessions.value.find((s) => s.id === currentSessionId.value) ?? null,
  );

  const currentMessages = computed<WeChatMessage[]>(
    () => messagesBySession.value[currentSessionId.value ?? ""] ?? [],
  );

  const totalUnread = computed(() =>
    sessions.value.reduce((acc, s) => acc + (s.unreadCount ?? 0), 0),
  );

  async function init() {
    if (!isTauri()) return;
    await loadAccount();
    await loadSessions();
    await loadPersonas();
    await loadPersonaBinding();
    await ensureListeners();
    // 如果账号已登录但同步任务被中断，主动拉起
    if (account.value?.status === "online") {
      try {
        await wechatStartSync();
      } catch (e) {
        console.warn("failed to start wechat sync", e);
      }
    }
  }

  async function loadAccount() {
    loading.value = true;
    try {
      account.value = await getWeChatAccount();
    } catch (e) {
      console.warn("load wechat account failed", e);
    } finally {
      loading.value = false;
    }
  }

  async function loadSessions() {
    try {
      sessions.value = await listWeChatSessions();
    } catch (e) {
      console.warn("load wechat sessions failed", e);
    }
  }

  async function openSession(id: string) {
    currentSessionId.value = id;
    if (!messagesBySession.value[id]) {
      try {
        messagesBySession.value[id] = await listWeChatMessages(id, 200);
      } catch (e) {
        messagesBySession.value[id] = [];
        console.warn("load messages failed", e);
      }
    }
    // 标记已读
    const sess = sessions.value.find((s) => s.id === id);
    if (sess && sess.unreadCount > 0) {
      try {
        await markWeChatSessionRead(id);
        sess.unreadCount = 0;
      } catch (e) {
        console.warn("mark read failed", e);
      }
    }
  }

  async function requestQrcode() {
    await loadAccount();
    const qr = await wechatRequestQrcode();
    if (account.value) {
      account.value.qrcodeUrl = qr.qrcodeUrl;
      account.value.qrcodeStatus = "pending";
      account.value.status = "logging_in";
    }
    return qr;
  }

  async function pollLogin() {
    loginPolling.value = true;
    try {
      const res = await wechatPollLogin();
      if (account.value) {
        account.value.qrcodeStatus = res.status as never;
        if (res.status === "confirmed") {
          account.value.status = "online";
          account.value.nickname = res.nickname;
          account.value.avatarUrl = res.avatarUrl;
          account.value.userId = res.userId;
          account.value.hasBotToken = true;
        } else if (res.status === "expired") {
          account.value.qrcodeStatus = "expired";
        }
      }
      if (res.status === "confirmed") {
        await loadSessions();
      }
      return res;
    } finally {
      loginPolling.value = false;
    }
  }

  async function logout() {
    await wechatLogout();
    if (account.value) {
      account.value.status = "offline";
      account.value.qrcodeStatus = "idle";
      account.value.qrcodeUrl = undefined;
      account.value.hasBotToken = false;
    }
    sessions.value = [];
    messagesBySession.value = {};
    currentSessionId.value = null;
  }

  async function reconnect() {
    await init();
  }

  async function sendText(text: string) {
    if (!currentSessionId.value) return null;
    const sess = currentSession.value;
    if (!sess) return null;
    const placeholder: WeChatMessage = {
      id: `tmp_${Date.now()}`,
      accountId: account.value?.id ?? "",
      sessionId: sess.id,
      direction: "outbound",
      msgType: "text",
      content: text,
      status: "pending",
      createdAt: new Date().toISOString(),
    };
    if (!messagesBySession.value[sess.id]) messagesBySession.value[sess.id] = [];
    messagesBySession.value[sess.id].push(placeholder);

    try {
      const final = await sendWeChatText(sess.id, text);
      const list = messagesBySession.value[sess.id];
      const idx = list.findIndex((m) => m.id === placeholder.id);
      if (idx >= 0) list[idx] = final;
      else list.push(final);
      // 更新会话预览
      sess.lastMsgPreview = text;
      sess.lastMsgAt = final.createdAt;
      return final;
    } catch (e) {
      const list = messagesBySession.value[sess.id];
      const idx = list.findIndex((m) => m.id === placeholder.id);
      if (idx >= 0) {
        list[idx] = { ...list[idx], status: "failed", error: String(e) };
      }
      throw e;
    }
  }

  async function sendMultipleTexts(texts: string[], delayMs: number = 800): Promise<WeChatMessage[]> {
    if (!currentSessionId.value) return [];
    const results: WeChatMessage[] = [];
    for (let i = 0; i < texts.length; i++) {
      const text = texts[i].trim();
      if (!text) continue;
      try {
        const msg = await sendText(text);
        if (msg) results.push(msg);
        if (i < texts.length - 1 && delayMs > 0) {
          await new Promise((resolve) => setTimeout(resolve, delayMs));
        }
      } catch (e) {
        console.error(`Failed to send segment ${i + 1}:`, e);
        throw e;
      }
    }
    return results;
  }

  async function ensureListeners() {
    if (!isTauri()) return;
    if (unlistenMessage && unlistenAccount) return;
    const { listen } = await import("@tauri-apps/api/event");
    if (!unlistenMessage) {
      unlistenMessage = await listen<WeChatMessageEvent>(
        "wechat-message",
        async (e) => {
          const { sessionId, message, isNewSession } = e.payload;
          if (isNewSession) {
            await loadSessions();
          }
          if (!messagesBySession.value[sessionId]) {
            messagesBySession.value[sessionId] = [];
          }
          // 远程消息可能因为去重而重复，用 remote_msg_id 判重
          const list = messagesBySession.value[sessionId];
          if (
            message.direction === "inbound" &&
            message.remoteMsgId &&
            list.some(
              (m) => m.remoteMsgId && m.remoteMsgId === message.remoteMsgId,
            )
          ) {
            return;
          }
          list.push(message);

          const sess = sessions.value.find((s) => s.id === sessionId);
          if (sess) {
            sess.lastMsgPreview =
              message.msgType === "text" ? message.content ?? "" : `[${message.msgType}]`;
            sess.lastMsgAt = message.createdAt;
            if (
              message.direction === "inbound" &&
              sessionId !== currentSessionId.value
            ) {
              sess.unreadCount = (sess.unreadCount ?? 0) + 1;
            }
          } else {
            // 新会话
            await loadSessions();
          }
        },
      );
    }
    if (!unlistenAccount) {
      unlistenAccount = await listen<WeChatAccountEvent>(
        "wechat-account",
        (e) => {
          if (!account.value) return;
          if (e.payload.accountId !== account.value.id) return;
          account.value.status = e.payload.status;
          if (e.payload.nickname !== undefined)
            account.value.nickname = e.payload.nickname;
          if (e.payload.avatarUrl !== undefined)
            account.value.avatarUrl = e.payload.avatarUrl;
          if (e.payload.lastError !== undefined)
            account.value.lastError = e.payload.lastError;
        },
      );
    }
  }

  // ─── 角色绑定 ────────────────────────────────

  async function loadPersonas() {
    try {
      availablePersonas.value = await listPersonas();
    } catch (e) {
      console.warn("load personas failed", e);
    }
  }

  async function loadPersonaBinding() {
    try {
      boundPersonaId.value = await getWechatPersona();
      // 从 settings 加载自动回复开关
      const { loadSettings } = await import("@/api/settings");
      const settings = await loadSettings();
      const ar = settings?.wechat_auto_reply;
      autoReply.value = ar === true || ar === "true";
    } catch (e) {
      console.warn("load persona binding failed", e);
    }
  }

  async function setAutoReply(enabled: boolean) {
    autoReply.value = enabled;
    try {
      const { loadSettings } = await import("@/api/settings");
      const settings = await loadSettings();
      await (await import("@/api/settings")).saveSettings({ ...settings, wechat_auto_reply: enabled });
    } catch (e) {
      console.warn("save auto reply failed", e);
    }
  }

  async function setPersona(personaId: string | null) {
    boundPersonaId.value = personaId;
    try {
      await setWechatPersona(account.value?.id ?? "default", personaId);
    } catch (e) {
      console.warn("set wechat persona failed", e);
    }
  }

  return {
    account,
    sessions,
    currentSession,
    currentSessionId,
    currentMessages,
    messagesBySession,
    totalUnread,
    loading,
    loginPolling,
    // 角色绑定
    autoReply,
    boundPersonaId,
    availablePersonas,
    // 方法
    init,
    loadAccount,
    loadSessions,
    openSession,
    requestQrcode,
    pollLogin,
    logout,
    reconnect,
    sendText,
    sendMultipleTexts,
    setAutoReply,
    setPersona,
  };
});
