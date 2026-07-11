export type Mood = "energetic" | "warm" | "calm" | "mystic";

export type WorkflowTrigger =
  | { type: "message"; pattern?: string }
  | { type: "scheduled"; cron: string }
  | { type: "event"; eventName: string };

export type WorkflowAction = {
  id: string;
  type: "retrieve_memory" | "query_knowledge" | "web_search" | "call_skill" | "send_message" | "set_context";
  config: Record<string, unknown>;
  nextActionId?: string;
};

export interface Workflow {
  id: string;
  personaId: string;
  name: string;
  description?: string;
  enabled: boolean;
  trigger: WorkflowTrigger;
  actions: WorkflowAction[];
  createdAt: string;
  updatedAt: string;
}

/** 角色卡定义 */
export interface PersonaDefinition {
  id: string;
  name: string;
  version: string;
  appearance: {
    avatar: string;
    live2d?: string;
    model3d?: string;
    emotionMapping?: Record<string, string>;
    /** 是否启用生成艺术背景 */
    generativeArt?: boolean;
  };
  voice: {
    ttsEngine: string;
    voiceId: string;
    params: { speed: number; emotionAware: boolean };
  };
  llm: {
    provider: string;
    fallback?: string;
    temperature: number;
  };
  systemPrompt: string;
  personality: string[];
  greeting: string;
  memoryPolicy: {
    longTerm: boolean;
    summaryThreshold: number;
    eventExtraction: boolean;
  };
  /** 启用的技能/插件能力 ID 列表；插件技能格式为 plugin:{pluginId}:{skillId} */
  skills: string[];
  knowledgeBases: string[];
  workflows?: Workflow[];
  emotionProfile: {
    default: string;
    responsive: boolean;
    influenceReply: boolean;
  };
  multimodal: {
    canSeeScreen: boolean;
    canSeeCamera: boolean;
    autoDescribeImages: boolean;
  };
  wechat?: {
    enableSegmentedReply: boolean;
    segmentDelay: number;
    enableVoiceMessage: boolean;
    voiceAutoSend: boolean;
    voiceAsrEnabled: boolean;
    /** 动作描述（括号内容）处理方式：separate=独立成段 / inline=保留在原文中 / remove=移除 */
    actionDescriptionMode: 'separate' | 'inline' | 'remove';
  };
}
