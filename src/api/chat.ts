import type { ContentPart, Message, MessageSegment, Session, ToolCall } from "@/types";
import { isTauri } from "./env";
import {
  mockCreateSession,
  mockChatStream,
  mockRandomEmotion,
  mockSendMessage,
} from "@/mocks/ipc";
import { mockSessions, mockMessages } from "@/mocks/data";

export interface ChatChunkPayload {
  sessionId: string;
  messageId: string;
  chunk: string;
  done: boolean;
  /** 当后端复用 chat-chunk 通道发送分段信息时携带 */
  segment?: MessageSegment;
  segmentIndex?: number;
}

export interface ChatSegmentPayload {
  sessionId: string;
  messageId: string;
  segment: MessageSegment;
  segmentIndex: number;
  done: boolean;
}

export interface ToolCallStartPayload {
  sessionId: string;
  messageId: string;
  toolCalls: ToolCall[];
}

export interface ToolResultPayload {
  sessionId: string;
  toolCallId: string;
  status: "success" | "error";
  result: string;
}

type ChunkHandler = (payload: ChatChunkPayload) => void;
type SegmentHandler = (payload: ChatSegmentPayload) => void;
type ToolCallStartHandler = (payload: ToolCallStartPayload) => void;
type ToolResultHandler = (payload: ToolResultPayload) => void;

let chunkHandler: ChunkHandler | null = null;
let unlistenChunkFn: (() => void) | null = null;
let segmentHandler: SegmentHandler | null = null;
let unlistenSegmentFn: (() => void) | null = null;
let toolCallStartHandler: ToolCallStartHandler | null = null;
let unlistenToolCallStartFn: (() => void) | null = null;
let toolResultHandler: ToolResultHandler | null = null;
let unlistenToolResultFn: (() => void) | null = null;

export async function setupChatStreamListener(handler: ChunkHandler) {
  chunkHandler = handler;
  if (!isTauri()) return;

  const { listen } = await import("@tauri-apps/api/event");
  if (unlistenChunkFn) unlistenChunkFn();
  unlistenChunkFn = await listen<ChatChunkPayload>("chat-chunk", (event) => {
    chunkHandler?.(event.payload);
  });
}

export async function setupChatSegmentListener(handler: SegmentHandler) {
  segmentHandler = handler;
  if (!isTauri()) return;

  const { listen } = await import("@tauri-apps/api/event");
  if (unlistenSegmentFn) unlistenSegmentFn();
  unlistenSegmentFn = await listen<ChatSegmentPayload>("chat-segment", (event) => {
    segmentHandler?.(event.payload);
  });
}

export async function setupToolCallStartListener(handler: ToolCallStartHandler) {
  toolCallStartHandler = handler;
  if (!isTauri()) return;

  const { listen } = await import("@tauri-apps/api/event");
  if (unlistenToolCallStartFn) unlistenToolCallStartFn();
  unlistenToolCallStartFn = await listen<ToolCallStartPayload>(
    "tool-call-start",
    (event) => {
      toolCallStartHandler?.(event.payload);
    },
  );
}

export async function setupToolResultListener(handler: ToolResultHandler) {
  toolResultHandler = handler;
  if (!isTauri()) return;

  const { listen } = await import("@tauri-apps/api/event");
  if (unlistenToolResultFn) unlistenToolResultFn();
  unlistenToolResultFn = await listen<ToolResultPayload>("tool-result", (event) => {
    toolResultHandler?.(event.payload);
  });
}

export async function listSessions(
  personaId?: string,
): Promise<Session[]> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<Session[]>("list_sessions", { personaId: personaId ?? null });
  }
  return personaId
    ? mockSessions.filter((s) => s.personaId === personaId)
    : [...mockSessions];
}

export async function createSession(
  personaId: string,
  title?: string,
): Promise<Session> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<Session>("create_session", { personaId, title: title ?? null });
  }
  return mockCreateSession(personaId);
}

export async function deleteSession(sessionId: string): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<void>("delete_session", { sessionId });
  }
}

export async function getMessages(sessionId: string): Promise<Message[]> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<Message[]>("get_messages", { sessionId });
  }
  return mockMessages[sessionId] ?? [];
}

/** 发送消息（支持多模态 parts） */
export async function sendMessage(
  sessionId: string,
  content: string,
  parts?: ContentPart[],
  onChunk?: ChunkHandler,
): Promise<Message> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    if (onChunk) chunkHandler = onChunk;
    return invoke<Message>("send_message", {
      sessionId,
      content,
      parts: parts ?? null,
    });
  }

  // Mock fallback
  await mockSendMessage(sessionId, content);
  const assistantId = `msg_${Date.now()}_assistant`;
  let fullContent = "";
  for await (const chunk of mockChatStream(content)) {
    fullContent += chunk;
    onChunk?.({
      sessionId,
      messageId: assistantId,
      chunk,
      done: false,
    });
  }
  onChunk?.({ sessionId, messageId: assistantId, chunk: "", done: true });
  return {
    id: assistantId,
    sessionId,
    role: "assistant",
    content: fullContent,
    emotionTag: mockRandomEmotion(),
    createdAt: new Date().toISOString(),
  };
}
