import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { Message, Session, ContentPart } from "@/types";
import {
  listSessions as apiListSessions,
  createSession as apiCreateSession,
  deleteSession as apiDeleteSession,
  getMessages as apiGetMessages,
  sendMessage as apiSendMessage,
  setupChatStreamListener,
  setupChatSegmentListener,
  setupToolCallStartListener,
  setupToolResultListener,
} from "@/api/chat";
import { usePersonaStore } from "@/stores/persona";
import { indexChatExchange } from "@/composables/useKnowledgeIndex";

export const useChatStore = defineStore("chat", () => {
  const sessions = ref<Session[]>([]);
  const messagesBySession = ref<Record<string, Message[]>>({});
  const currentSessionId = ref<string | null>(null);
  const isStreaming = ref(false);
  const streamingMessageId = ref<string | null>(null);
  const initialized = ref(false);

  let unlistenStream: (() => void) | null = null;
  let unlistenSegment: (() => void) | null = null;

  const pendingUserMessage = ref<{ content: string; personaId: string; sessionTitle?: string } | null>(null);
  const indexedMessageIds = ref<Set<string>>(new Set());

  const currentSession = computed(() =>
    sessions.value.find((s) => s.id === currentSessionId.value) ?? null,
  );

  const currentMessages = computed(() => {
    if (!currentSessionId.value) return [];
    return messagesBySession.value[currentSessionId.value] ?? [];
  });

  function triggerIndex(sessionId: string, assistantMsgId: string) {
    const msgs = messagesBySession.value[sessionId];
    if (!msgs) return;
    const assistantMsg = msgs.find((m) => m.id === assistantMsgId);
    if (!assistantMsg || !assistantMsg.content.trim()) return;
    if (indexedMessageIds.value.has(assistantMsgId)) return;

    const assistantIdx = msgs.indexOf(assistantMsg);
    let lastUserMsg: Message | undefined;
    for (let i = assistantIdx - 1; i >= 0; i--) {
      if (msgs[i].role === "user") {
        lastUserMsg = msgs[i];
        break;
      }
    }
    if (!lastUserMsg) return;

    const session = sessions.value.find((s) => s.id === sessionId);
    const personaId = session?.personaId ?? pendingUserMessage.value?.personaId;
    if (!personaId) return;

    indexedMessageIds.value.add(assistantMsgId);
    indexChatExchange(personaId, lastUserMsg.content, assistantMsg.content, session?.title);
  }

  async function init(personaId?: string) {
    if (initialized.value) return;

    unlistenStream?.();
    setupChatStreamListener((payload) => {
      const { sessionId, messageId, chunk, done } = payload;
      if (!messagesBySession.value[sessionId]) {
        messagesBySession.value[sessionId] = [];
      }
      const msgs = messagesBySession.value[sessionId];
      let idx = msgs.findIndex((m) => m.id === messageId);
      if (idx < 0) {
        idx = msgs.length;
        msgs.push({
          id: messageId,
          sessionId,
          role: "assistant",
          content: "",
          streaming: true,
          createdAt: new Date().toISOString(),
        });
        if (isStreaming.value) {
          streamingMessageId.value = messageId;
        }
      }
      if (chunk) msgs[idx].content += chunk;
      if (done) {
        msgs[idx].streaming = false;
        if (streamingMessageId.value === messageId) {
          isStreaming.value = false;
          streamingMessageId.value = null;
        }
        triggerIndex(sessionId, messageId);
      }
    });

    unlistenSegment?.();
    setupChatSegmentListener((payload) => {
      const { sessionId, messageId, segment, segmentIndex, done } = payload;
      if (!messagesBySession.value[sessionId]) {
        messagesBySession.value[sessionId] = [];
      }
      const msgs = messagesBySession.value[sessionId];
      let idx = msgs.findIndex((m) => m.id === messageId);
      if (idx < 0) {
        idx = msgs.length;
        msgs.push({
          id: messageId,
          sessionId,
          role: "assistant",
          content: "",
          segments: [],
          streaming: true,
          createdAt: new Date().toISOString(),
        });
        if (isStreaming.value) {
          streamingMessageId.value = messageId;
        }
      }
      const msg = msgs[idx];
      if (!msg.segments) msg.segments = [];
      if (segmentIndex < msg.segments.length) {
        msg.segments[segmentIndex] = segment;
      } else if (segmentIndex === msg.segments.length) {
        msg.segments.push(segment);
      }
      if (segment?.type === "text" && segment.content) {
        msg.content += segment.content;
      }
      if (done) {
        msg.streaming = false;
        if (streamingMessageId.value === messageId) {
          isStreaming.value = false;
          streamingMessageId.value = null;
        }
        triggerIndex(sessionId, messageId);
      }
    });

    setupToolCallStartListener((payload) => {
      const { sessionId, messageId, toolCalls } = payload;
      if (!messagesBySession.value[sessionId]) {
        messagesBySession.value[sessionId] = [];
      }
      const msgs = messagesBySession.value[sessionId];
      msgs.push({
        id: messageId,
        sessionId,
        role: "assistant",
        content: "",
        toolCalls: toolCalls.map((tc) => ({
          ...tc,
          status: "pending" as const,
        })),
        streaming: false,
        createdAt: new Date().toISOString(),
      });
    });

    setupToolResultListener((payload) => {
      const { sessionId, toolCallId, status, result } = payload;
      if (!messagesBySession.value[sessionId]) return;
      const msgs = messagesBySession.value[sessionId];
      for (const msg of msgs) {
        if (msg.role !== "assistant" || !msg.toolCalls) continue;
        const tc = msg.toolCalls.find((t) => t.id === toolCallId);
        if (tc) {
          tc.status = status;
          tc.result = result;
          break;
        }
      }
    });

    try {
      sessions.value = await apiListSessions(personaId);
      initialized.value = true;
    } catch (e) {
      console.warn("Failed to load sessions", e);
    }
  }

  function selectSession(sessionId: string) {
    if (sessionId === currentSessionId.value) return;
    currentSessionId.value = sessionId;
    apiGetMessages(sessionId).then((msgs) => {
      messagesBySession.value[sessionId] = msgs;
    }).catch((e) => console.warn("getMessages:", e));
  }

  async function createSession(personaId: string): Promise<Session> {
    const session = await apiCreateSession(personaId);
    sessions.value.unshift(session);
    messagesBySession.value[session.id] = [];
    currentSessionId.value = session.id;
    return session;
  }

  async function deleteSession(sessionId: string): Promise<void> {
    await apiDeleteSession(sessionId);
    const idx = sessions.value.findIndex((s) => s.id === sessionId);
    if (idx >= 0) {
      sessions.value.splice(idx, 1);
    }
    delete messagesBySession.value[sessionId];
    if (currentSessionId.value === sessionId) {
      currentSessionId.value = null;
    }
  }

  async function sendMessage(content: string, parts?: ContentPart[]): Promise<boolean> {
    if (!content.trim() && (!parts || parts.length === 0)) return false;

    if (!currentSessionId.value) {
      const personaStore = usePersonaStore();
      const personaId = personaStore.currentPersonaId ?? personaStore.personas[0]?.id ?? "persona_aria";
      await createSession(personaId);
    }

    const sessionId = currentSessionId.value!;
    const session = sessions.value.find((s) => s.id === sessionId);
    const personaId = session?.personaId ?? "persona_aria";

    const userMsg: Message = {
      id: `msg_${Date.now()}_user`,
      sessionId,
      role: "user",
      content: content.trim(),
      parts,
      createdAt: new Date().toISOString(),
    };

    if (!messagesBySession.value[sessionId]) {
      messagesBySession.value[sessionId] = [];
    }
    messagesBySession.value[sessionId].push(userMsg);

    pendingUserMessage.value = {
      content: content.trim(),
      personaId,
      sessionTitle: session?.title,
    };

    if (session && (session.title === "新对话" || !session.title)) {
      session.title = content.trim().slice(0, 20);
    }
    if (session) session.updatedAt = new Date().toISOString();

    isStreaming.value = true;

    try {
      await apiSendMessage(sessionId, content, parts);
      return true;
    } catch (e) {
      console.error("sendMessage error:", e);
      isStreaming.value = false;
      streamingMessageId.value = null;
      pendingUserMessage.value = null;
      return false;
    }
  }

  return {
    sessions,
    messagesBySession,
    currentSessionId,
    currentSession,
    currentMessages,
    isStreaming,
    streamingMessageId,
    initialized,
    init,
    selectSession,
    createSession,
    deleteSession,
    sendMessage,
  };
});
