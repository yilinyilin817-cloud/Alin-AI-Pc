export type ModelType = "llm" | "asr" | "tts" | "emotion" | "embedding";

export type DownloadStatus =
  | "not_downloaded"
  | "downloading"
  | "downloaded"
  | "active";

export interface ModelConfig {
  id: string;
  name: string;
  modelType: ModelType;
  providerId: string;
  description: string;
  vramRequired?: string;
  size?: string;
  status: DownloadStatus;
  progress?: number;
  isActive: boolean;
}

export interface DownloadTask {
  id: string;
  modelId: string;
  modelName: string;
  progress: number;
  status: "pending" | "downloading" | "completed" | "failed";
}

export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
}

export interface GpuInfo {
  vramGb: number;
  recommendation: string;
}

// 云服务商相关类型
export interface CloudModel {
  id: string;
}

export interface CloudProviderConfig {
  id: string;
  name: string;
  providerType: string;
  apiBase: string;
  apiKey: string;
  iconUrl?: string;
  models: CloudModel[];
  isEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateCloudProviderRequest {
  name: string;
  providerType: string;
  apiBase: string;
  apiKey: string;
  iconUrl?: string;
}

export interface UpdateCloudProviderRequest {
  name?: string;
  providerType?: string;
  apiBase?: string;
  apiKey?: string;
  iconUrl?: string;
  isEnabled?: boolean;
}

export interface VerifyCloudProviderResponse {
  success: boolean;
  models: CloudModel[];
  message: string;
}

// 云端 TTS 服务商相关类型
export interface WusoundVoice {
  id: string;
  name: string;
  status: string;
  metadata?: {
    avatar?: string;
    description?: string;
    language?: string[];
    gender?: string;
    tags?: string[];
    prompts?: Array<{
      id: string;
      name: string;
      description?: string;
    }>;
  };
}

export interface WusoundQuota {
  usedChars: number;
  totalChars: number;
  remainingChars: number;
  tier?: string;
  raw?: unknown;
}

export interface WusoundSynthesizeOptions {
  speed?: number;
  pitch?: number;
  volume?: number;
  format?: "wav" | "mp3" | "opus" | "pcm";
  sampleRate?: 8000 | 16000 | 24000 | 48000;
}

export interface CloudTtsProviderConfig {
  id: string;
  name: string;
  providerType: string;
  apiBase: string;
  /** 脱敏的 API Key，仅展示 */
  apiKeyMasked: string;
  /** 是否已配置 API Key */
  hasApiKey: boolean;
  iconUrl?: string;
  voices: WusoundVoice[];
  isEnabled: boolean;
  lastVerifiedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateCloudTtsProviderRequest {
  name: string;
  providerType: string;
  apiBase: string;
  apiKey: string;
  iconUrl?: string;
}

export interface UpdateCloudTtsProviderRequest {
  name?: string;
  providerType?: string;
  apiBase?: string;
  apiKey?: string;
  iconUrl?: string;
  isEnabled?: boolean;
}

export interface VerifyCloudTtsResponse {
  success: boolean;
  voices: WusoundVoice[];
  message: string;
  quota?: WusoundQuota;
  verifiedAt: string;
}

export interface CloudTtsSynthesizeRequest {
  providerId: string;
  text: string;
  voiceId: string;
  promptId?: string;
  speed?: number;
  pitch?: number;
  volume?: number;
  format?: "wav" | "mp3" | "opus" | "pcm";
  sampleRate?: 8000 | 16000 | 24000 | 48000;
}

// ─── 微信 iLink 通道类型 ────────────────────────────

export type WeChatAccountStatus =
  | "offline"
  | "logging_in"
  | "online"
  | "error";

export type WeChatQrcodeStatus =
  | "idle"
  | "pending"
  | "scanned"
  | "confirmed"
  | "expired";

export type WeChatMsgType =
  | "text"
  | "image"
  | "voice"
  | "video"
  | "file"
  | "system"
  | "unknown";

export interface WeChatAccount {
  id: string;
  userId?: string;
  nickname?: string;
  avatarUrl?: string;
  hasBotToken: boolean;
  getUpdatesBuf?: string;
  status: WeChatAccountStatus;
  lastError?: string;
  lastLoginAt?: string;
  lastSyncAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface WeChatAccountView extends WeChatAccount {
  qrcodeUrl?: string;
  qrcodeStatus?: WeChatQrcodeStatus;
}

export interface WeChatQrCode {
  accountId: string;
  qrcodeUrl: string;
  qrcodeKey: string;
  expiresIn: number;
}

export interface WeChatLoginStatus {
  status: WeChatQrcodeStatus | "error";
  accountId: string;
  botToken?: string;
  userId?: string;
  nickname?: string;
  avatarUrl?: string;
  message?: string;
}

export interface WeChatSession {
  id: string;
  accountId: string;
  peerId: string;
  peerType: "user" | "room" | "official";
  peerName?: string;
  peerAvatar?: string;
  lastMsgPreview?: string;
  lastMsgAt?: string;
  unreadCount: number;
  isPinned: boolean;
  isMuted: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface WeChatMessage {
  id: string;
  accountId: string;
  sessionId: string;
  remoteMsgId?: string;
  direction: "inbound" | "outbound";
  msgType: WeChatMsgType;
  content?: string;
  mediaUrl?: string;
  mediaLocalPath?: string;
  senderId?: string;
  senderName?: string;
  contextToken?: string;
  status: "pending" | "sent" | "failed";
  error?: string;
  createdAt: string;
}

export interface WeChatMessageEvent {
  accountId: string;
  sessionId: string;
  message: WeChatMessage;
  isNewSession: boolean;
}

export interface WeChatAccountEvent {
  accountId: string;
  status: WeChatAccountStatus;
  nickname?: string;
  avatarUrl?: string;
  lastError?: string;
}
