/** 多模态内容片段 */
export type ContentPart =
  | { type: "text"; text: string }
  | { type: "image_url"; url: string }
  | { type: "image_bytes"; data: Uint8Array; mime?: string }
  | ({ type: "audio_bytes"; data: Uint8Array } & AudioPartMeta);

/** 语音片段元数据 */
export interface AudioPartMeta {
  duration?: number;
  transcript?: string;
  mime?: string;
}

/** 分段消息来源 */
export interface MessageSegmentSource {
  name: string;
  url?: string;
}

/** 消息分段（用于多段式回复） */
export type MessageSegment =
  | { type: "text"; content: string; source?: MessageSegmentSource }
  | { type: "code"; content: string; language?: string; source?: MessageSegmentSource }
  | {
      type: "image";
      content?: string;
      imageUrl?: string;
      imageBytes?: Uint8Array;
      source?: MessageSegmentSource;
    }
  | { type: "think"; content?: string; collapsed?: boolean; source?: MessageSegmentSource }
  | {
      type: "tool_result";
      content?: string;
      toolCallId?: string;
      source?: MessageSegmentSource;
    };

/** 情绪标签 */
export interface EmotionTag {
  emotion:
    | "happy"
    | "sad"
    | "angry"
    | "fearful"
    | "surprised"
    | "disgusted"
    | "neutral";
  valence: number;
  arousal: number;
}

/** 工具调用 */
export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  status?: 'pending' | 'running' | 'success' | 'error';
  result?: string;
}

/** 消息 */
export interface Message {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "tool" | "system";
  content: string;
  parts?: ContentPart[];
  segments?: MessageSegment[];
  emotionTag?: EmotionTag;
  toolCalls?: ToolCall[];
  streaming?: boolean;
  createdAt?: string;
}

/** 会话 */
export interface Session {
  id: string;
  personaId: string;
  title: string;
  summary?: string;
  isPinned?: boolean;
  createdAt: string;
  updatedAt: string;
}
