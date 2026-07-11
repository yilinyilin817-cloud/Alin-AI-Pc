import type { EmotionTag, Message, Session } from "@/types";
import type { AppInfo } from "@/types/model";
import { mockChatResponses } from "./data";

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function* mockChatStream(
  _prompt: string,
): AsyncGenerator<string> {
  const response =
    mockChatResponses[Math.floor(Math.random() * mockChatResponses.length)];
  for (const char of response) {
    await delay(30 + Math.random() * 40);
    yield char;
  }
}

export function mockRandomEmotion(): EmotionTag {
  const emotions: EmotionTag["emotion"][] = [
    "happy",
    "neutral",
    "sad",
    "surprised",
  ];
  const emotion = emotions[Math.floor(Math.random() * emotions.length)];
  return {
    emotion,
    valence: Math.random() * 2 - 1,
    arousal: Math.random(),
  };
}

export async function mockCreateSession(
  personaId: string,
): Promise<Session> {
  await delay(100);
  const now = new Date().toISOString();
  return {
    id: `sess_${Date.now()}`,
    personaId,
    title: "新对话",
    createdAt: now,
    updatedAt: now,
  };
}

export async function mockSendMessage(
  sessionId: string,
  content: string,
): Promise<Message> {
  await delay(50);
  return {
    id: `msg_${Date.now()}`,
    sessionId,
    role: "user",
    content,
    createdAt: new Date().toISOString(),
  };
}

export async function mockHealthCheck(): Promise<string> {
  await delay(50);
  return "ok";
}

export async function mockGetAppInfo(): Promise<AppInfo> {
  return {
    name: "AI 伴侣",
    version: "0.1.0",
    dataDir: "%APPDATA%/AiCompanion/",
  };
}

export function generateWaveformData(length = 32): number[] {
  return Array.from({ length }, () => Math.random() * 0.8 + 0.1);
}
