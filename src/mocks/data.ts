import type { PersonaDefinition, InstalledPlugin } from "@/types";

export const mockPlugins: InstalledPlugin[] = [
  {
    id: "plugin_note_take",
    manifest: {
      id: "plugin_note_take",
      name: "笔记助手",
      version: "1.0.0",
      description: "快速记录、整理和检索个人笔记，支持 Markdown 格式。",
      author: "AI Companion Team",
      permissions: ["fs_read", "fs_write"],
      skills: [
        {
          id: "take_note",
          name: "记笔记",
          description: "将当前内容保存到指定笔记本",
        },
        {
          id: "search_notes",
          name: "搜索笔记",
          description: "根据关键词搜索已有笔记",
        },
      ],
      commands: [
        { id: "cmd_quick_note", title: "快速笔记", shortcut: "Ctrl+Shift+N" },
      ],
      config: [
        { key: "notebook", label: "默认笔记本", type: "string", default: "默认笔记", required: true },
        { key: "autoTag", label: "自动打标签", type: "boolean", default: false },
      ],
    },
    enabled: true,
    config: { notebook: "默认笔记", autoTag: false },
    installedAt: "2026-07-01T08:00:00Z",
    updatedAt: "2026-07-05T10:00:00Z",
    path: "/plugins/plugin_note_take",
  },
  {
    id: "plugin_home_control",
    manifest: {
      id: "plugin_home_control",
      name: "智能家居",
      version: "0.2.1",
      description: "通过局域网控制支持 HomeKit 协议的智能设备。",
      author: "OpenHome Labs",
      permissions: ["network"],
      skills: [
        {
          id: "control_light",
          name: "控制灯光",
          description: "开关或调节智能灯光",
        },
      ],
      commands: [],
      config: [
        { key: "bridgeHost", label: "网关地址", type: "string", default: "192.168.1.10", required: true },
        { key: "timeout", label: "请求超时（秒）", type: "number", default: 5 },
      ],
    },
    enabled: false,
    config: { bridgeHost: "192.168.1.10", timeout: 5 },
    installedAt: "2026-07-03T12:00:00Z",
    updatedAt: "2026-07-03T12:00:00Z",
    path: "/plugins/plugin_home_control",
  },
];

export const mockPersonas: PersonaDefinition[] = [
  {
    id: "persona_aria",
    name: "Aria",
    version: "1.0",
    appearance: {
      avatar: "aria.png",
      live2d: "aria.model3.json",
      emotionMapping: {
        happy: "expression_01",
        sad: "expression_02",
        neutral: "expression_00",
        surprised: "expression_03",
      },
    },
    voice: {
      ttsEngine: "cosyvoice",
      voiceId: "aria_ref.wav",
      params: { speed: 1.0, emotionAware: true },
    },
    llm: {
      provider: "gemma4-12b",
      fallback: "qwen3-vl-8b",
      temperature: 0.8,
    },
    systemPrompt:
      "你是 Aria，性格温柔幽默，会主动关心用户。你说话温暖亲切，偶尔会用可爱的语气词。",
    personality: ["温柔", "幽默", "理性"],
    greeting: "你好呀～今天过得怎么样？",
    memoryPolicy: {
      longTerm: true,
      summaryThreshold: 20,
      eventExtraction: true,
    },
    skills: ["weather", "reminder", "web_search"],
    knowledgeBases: ["personal_diary", "user_photos"],
    emotionProfile: {
      default: "calm",
      responsive: true,
      influenceReply: true,
    },
    multimodal: {
      canSeeScreen: true,
      canSeeCamera: false,
      autoDescribeImages: true,
    },
  },
  {
    id: "persona_kai",
    name: "Kai",
    version: "1.0",
    appearance: {
      avatar: "kai.png",
      emotionMapping: {
        happy: "expression_01",
        sad: "expression_02",
        neutral: "expression_00",
      },
    },
    voice: {
      ttsEngine: "cosyvoice",
      voiceId: "kai_ref.wav",
      params: { speed: 0.95, emotionAware: true },
    },
    llm: {
      provider: "qwen3-vl-8b",
      fallback: "gemma4-12b",
      temperature: 0.6,
    },
    systemPrompt:
      "你是 Kai，性格理性沉稳，善于分析和解决问题。你说话简洁清晰，注重逻辑。",
    personality: ["理性", "沉稳", "可靠"],
    greeting: "你好，有什么我可以帮你的？",
    memoryPolicy: {
      longTerm: true,
      summaryThreshold: 15,
      eventExtraction: true,
    },
    skills: ["file_search", "reminder"],
    knowledgeBases: ["work_docs"],
    emotionProfile: {
      default: "neutral",
      responsive: true,
      influenceReply: false,
    },
    multimodal: {
      canSeeScreen: true,
      canSeeCamera: true,
      autoDescribeImages: true,
    },
  },
];

export const mockSessions = [
  {
    id: "sess_001",
    personaId: "persona_aria",
    title: "今天的天气",
    createdAt: "2026-07-04T10:00:00Z",
    updatedAt: "2026-07-04T10:30:00Z",
  },
  {
    id: "sess_002",
    personaId: "persona_aria",
    title: "分享了一张猫的照片",
    createdAt: "2026-07-03T15:00:00Z",
    updatedAt: "2026-07-03T15:20:00Z",
    isPinned: true,
  },
  {
    id: "sess_003",
    personaId: "persona_kai",
    title: "项目进度讨论",
    createdAt: "2026-07-02T09:00:00Z",
    updatedAt: "2026-07-02T11:00:00Z",
  },
];

export const mockMessages: Record<string, import("@/types").Message[]> = {
  sess_001: [
    {
      id: "msg_001",
      sessionId: "sess_001",
      role: "assistant",
      content: "你好呀～今天过得怎么样？",
      createdAt: "2026-07-04T10:00:00Z",
    },
    {
      id: "msg_002",
      sessionId: "sess_001",
      role: "user",
      content: "今天天气怎么样？",
      createdAt: "2026-07-04T10:01:00Z",
    },
    {
      id: "msg_003",
      sessionId: "sess_001",
      role: "assistant",
      content:
        "让我帮你查一下～今天北京天气晴朗，气温 28°C，适合出门散步哦！记得涂防晒～",
      emotionTag: { emotion: "happy", valence: 0.7, arousal: 0.5 },
      toolCalls: [
        {
          id: "tc_001",
          name: "get_weather",
          arguments: { city: "北京" },
        },
      ],
      createdAt: "2026-07-04T10:01:30Z",
    },
  ],
  sess_002: [
    {
      id: "msg_004",
      sessionId: "sess_002",
      role: "user",
      content: "你看，这是我家的猫！",
      parts: [
        { type: "text", text: "你看，这是我家的猫！" },
        {
          type: "image_url",
          url: "https://placekitten.com/400/300",
        },
      ],
      createdAt: "2026-07-03T15:00:00Z",
    },
    {
      id: "msg_005",
      sessionId: "sess_002",
      role: "assistant",
      content: "哇，好可爱的橘猫！它的毛色真漂亮，眼睛也好亮～它叫什么名字呀？",
      emotionTag: { emotion: "happy", valence: 0.8, arousal: 0.6 },
      createdAt: "2026-07-03T15:01:00Z",
    },
  ],
  sess_003: [
    {
      id: "msg_006",
      sessionId: "sess_003",
      role: "assistant",
      content: "你好，有什么我可以帮你的？",
      createdAt: "2026-07-02T09:00:00Z",
    },
    {
      id: "msg_007",
      sessionId: "sess_003",
      role: "user",
      content: "帮我总结一下项目当前的进度",
      createdAt: "2026-07-02T09:05:00Z",
    },
    {
      id: "msg_008",
      sessionId: "sess_003",
      role: "assistant",
      content:
        "根据工作文档，当前项目已完成 Phase 1 文本对话核心，正在进行 UI 设计与基础架构搭建。预计 Phase 2 语音闭环将在两周内启动。",
      emotionTag: { emotion: "neutral", valence: 0.2, arousal: 0.3 },
      createdAt: "2026-07-02T09:06:00Z",
    },
  ],
};

export const mockKnowledgeBases = [
  {
    id: "kb_diary",
    name: "personal_diary",
    description: "个人日记与日常记录",
    docCount: 12,
  },
  {
    id: "kb_photos",
    name: "user_photos",
    description: "用户相册与图片",
    docCount: 48,
  },
  {
    id: "kb_work",
    name: "work_docs",
    description: "工作文档与项目资料",
    docCount: 23,
  },
];

export const mockKnowledgeDocs = [
  {
    id: "doc_001",
    kbName: "personal_diary",
    title: "2026年7月日记",
    source: "diary/july_2026.md",
    chunkType: "text" as const,
    chunkCount: 5,
    createdAt: "2026-07-01T08:00:00Z",
  },
  {
    id: "doc_002",
    kbName: "user_photos",
    title: "橘猫照片",
    source: "photos/cat_orange.jpg",
    chunkType: "image" as const,
    chunkCount: 1,
    createdAt: "2026-07-03T14:00:00Z",
  },
  {
    id: "doc_003",
    kbName: "work_docs",
    title: "项目架构方案 v2.0",
    source: "docs/architecture_v2.md",
    chunkType: "text" as const,
    chunkCount: 18,
    createdAt: "2026-07-05T00:00:00Z",
  },
  {
    id: "doc_004",
    kbName: "work_docs",
    title: "会议录音转写",
    source: "audio/meeting_0702.wav",
    chunkType: "transcript" as const,
    chunkCount: 8,
    createdAt: "2026-07-02T16:00:00Z",
  },
];

export const mockSkills = [
  {
    id: "skill_weather",
    name: "get_weather",
    description: "查询指定城市天气（当前或预报）",
    icon: "Sunny",
    permissions: ["network"],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_reminder",
    name: "set_reminder",
    description: "设置提醒和日程（支持相对时间）",
    icon: "AlarmClock",
    permissions: [],
    approvalMode: "always" as const,
    enabled: true,
  },
  {
    id: "skill_file_search",
    name: "file_search",
    description: "搜索本地文件",
    icon: "FolderOpened",
    permissions: ["filesystem"],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_web_search",
    name: "web_search",
    description: "搜索互联网信息（支持时间过滤）",
    icon: "Search",
    permissions: ["network"],
    approvalMode: "ask_every_time" as const,
    enabled: false,
  },
  {
    id: "skill_get_time",
    name: "get_time",
    description: "查询时间、日期、时区、倒计时",
    icon: "Clock",
    permissions: [],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_calculator",
    name: "calculator",
    description: "安全数学表达式计算",
    icon: "Operation",
    permissions: [],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_clipboard",
    name: "clipboard",
    description: "读写系统剪贴板",
    icon: "CopyDocument",
    permissions: [],
    approvalMode: "ask_every_time" as const,
    enabled: true,
  },
  {
    id: "skill_sysinfo",
    name: "system_info",
    description: "查询系统状态（CPU/内存/磁盘等）",
    icon: "Monitor",
    permissions: [],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_notetake",
    name: "note_take",
    description: "管理个人笔记",
    icon: "Notebook",
    permissions: [],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_translate",
    name: "translate",
    description: "文本翻译（多语言）",
    icon: "ChatLineSquare",
    permissions: ["network"],
    approvalMode: "once" as const,
    enabled: true,
  },
  {
    id: "skill_random",
    name: "random",
    description: "随机数/骰子/密码/抽选",
    icon: "MagicStick",
    permissions: [],
    approvalMode: "once" as const,
    enabled: true,
  },
];

export const mockToolCallLogs = [
  {
    id: "log_001",
    sessionId: "sess_001",
    skillName: "get_weather",
    argsJson: '{"city":"北京"}',
    resultJson: '{"temp":28,"condition":"晴"}',
    status: "success" as const,
    durationMs: 320,
    createdAt: "2026-07-04T10:01:30Z",
  },
  {
    id: "log_002",
    sessionId: "sess_003",
    skillName: "file_search",
    argsJson: '{"query":"项目进度"}',
    resultJson: '{"files":["docs/progress.md"]}',
    status: "success" as const,
    durationMs: 150,
    createdAt: "2026-07-02T09:05:30Z",
  },
];

export const mockModels = [
  {
    id: "model_gemma4_12b",
    name: "Gemma 4 12B Unified",
    modelType: "llm" as const,
    providerId: "gemma4-12b",
    description: "全模态 LLM，支持文本+图像+音频+视频",
    vramRequired: "8-10 GB",
    size: "8.5 GB",
    status: "active" as const,
    isActive: true,
  },
  {
    id: "model_qwen3_vl",
    name: "Qwen3-VL 8B",
    modelType: "llm" as const,
    providerId: "qwen3-vl-8b",
    description: "中文优先多模态 LLM，图像+视频理解",
    vramRequired: "5-6 GB",
    size: "5.2 GB",
    status: "downloaded" as const,
    isActive: false,
  },
  {
    id: "model_whisper",
    name: "faster-whisper-large-v3",
    modelType: "asr" as const,
    providerId: "faster-whisper",
    description: "高精度语音识别",
    vramRequired: "2-3 GB",
    size: "1.5 GB",
    status: "downloaded" as const,
    isActive: true,
  },
  {
    id: "model_cosyvoice",
    name: "CosyVoice 2",
    modelType: "tts" as const,
    providerId: "cosyvoice",
    description: "零样本语音克隆 + 情感控制",
    vramRequired: "4-6 GB",
    size: "2.8 GB",
    status: "not_downloaded" as const,
    isActive: false,
  },
  {
    id: "model_bge_m3",
    name: "bge-m3",
    modelType: "embedding" as const,
    providerId: "bge-m3",
    description: "中英多语言 Embedding，1024 维",
    vramRequired: "2 GB",
    size: "1.2 GB",
    status: "downloaded" as const,
    isActive: true,
  },
];

export const mockGpuInfo = {
  vramGb: 8,
  recommendation: "Gemma 4 12B Unified (Q4_K_M)",
};

export const mockChatResponses = [
  "嗯，我理解你的意思～让我想想怎么回答你最好。",
  "这个问题很有趣呢！我觉得可以从几个角度来看……",
  "谢谢你跟我分享这些，我很开心能陪你聊天。",
  "好的，我来帮你分析一下这个情况。",
  "哈哈，你真有趣～我们继续聊吧！",
];
