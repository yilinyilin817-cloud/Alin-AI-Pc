-- 用户表
CREATE TABLE IF NOT EXISTS user (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    avatar      TEXT,
    config_json TEXT,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now'))
);

-- 角色表
CREATE TABLE IF NOT EXISTS persona (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    version         TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    is_active       INTEGER DEFAULT 0,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now'))
);

-- 角色工作流
CREATE TABLE IF NOT EXISTS workflows (
    id            TEXT PRIMARY KEY,
    persona_id    TEXT NOT NULL,
    name          TEXT NOT NULL,
    description   TEXT,
    enabled       INTEGER NOT NULL DEFAULT 1,
    trigger_json  TEXT NOT NULL,
    actions_json  TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflows_persona ON workflows(persona_id);

-- 会话表
CREATE TABLE IF NOT EXISTS session (
    id          TEXT PRIMARY KEY,
    persona_id  TEXT NOT NULL REFERENCES persona(id),
    title       TEXT,
    summary     TEXT,
    is_pinned   INTEGER DEFAULT 0,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_session_persona ON session(persona_id);
CREATE INDEX IF NOT EXISTS idx_session_updated ON session(updated_at DESC);

-- 消息表
CREATE TABLE IF NOT EXISTS message (
    id            TEXT PRIMARY KEY,
    session_id    TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    role          TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'tool', 'system')),
    content       TEXT NOT NULL,
    content_parts TEXT,
    segments      TEXT,
    emotion_tag   TEXT,
    tokens        INTEGER DEFAULT 0,
    tool_calls    TEXT,
    created_at    TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_message_session ON message(session_id, created_at);

-- 知识库
CREATE TABLE IF NOT EXISTS knowledge_base (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);

-- 知识库文档（一个 kb 包含多个文档）
CREATE TABLE IF NOT EXISTS knowledge_doc (
    id          TEXT PRIMARY KEY,
    kb_id       TEXT NOT NULL REFERENCES knowledge_base(id) ON DELETE CASCADE,
    title       TEXT,
    source      TEXT,
    chunk_type  TEXT DEFAULT 'text' CHECK(chunk_type IN ('text', 'image', 'transcript')),
    chunk_count INTEGER DEFAULT 0,
    meta_json   TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_knowledge_doc_kb ON knowledge_doc(kb_id);

-- 知识块（带向量索引）
CREATE TABLE IF NOT EXISTS knowledge_chunk (
    id          TEXT PRIMARY KEY,
    doc_id      TEXT NOT NULL REFERENCES knowledge_doc(id) ON DELETE CASCADE,
    seq         INTEGER NOT NULL,
    text        TEXT NOT NULL,
    embedding_id TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chunk_doc ON knowledge_chunk(doc_id);

-- 知识块 FTS5 全文检索
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_chunk_fts USING fts5(
    text,
    content=knowledge_chunk,
    content_rowid=rowid
);

-- 技能注册表
CREATE TABLE IF NOT EXISTS skill (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    definition_yaml TEXT NOT NULL,
    enabled         INTEGER DEFAULT 1,
    config_json     TEXT,
    created_at      TEXT DEFAULT (datetime('now'))
);

-- 技能权限审批记录
CREATE TABLE IF NOT EXISTS skill_permission (
    id          TEXT PRIMARY KEY,
    skill_name  TEXT NOT NULL,
    status      TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'denied')),
    approved_at TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skill_perm_name ON skill_permission(skill_name);

-- 工具调用日志
CREATE TABLE IF NOT EXISTS tool_call_log (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES session(id),
    message_id  TEXT REFERENCES message(id),
    skill_name  TEXT NOT NULL,
    args_json   TEXT,
    result_json TEXT,
    status      TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'success', 'error', 'rejected')),
    duration_ms INTEGER,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tool_call_session ON tool_call_log(session_id, created_at);

-- 个人笔记
CREATE TABLE IF NOT EXISTS note (
    id          TEXT PRIMARY KEY,
    content     TEXT NOT NULL,
    tags        TEXT DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_note_tags ON note(tags);
CREATE INDEX IF NOT EXISTS idx_note_created ON note(created_at DESC);

-- 长期记忆
CREATE TABLE IF NOT EXISTS memory (
    id           TEXT PRIMARY KEY,
    persona_id   TEXT NOT NULL REFERENCES persona(id),
    session_id   TEXT REFERENCES session(id),
    type         TEXT NOT NULL CHECK(type IN ('summary', 'event', 'preference', 'knowledge')),
    content      TEXT NOT NULL,
    embedding_id TEXT,
    importance   REAL DEFAULT 0.5,
    access_count INTEGER DEFAULT 0,
    last_recall  TEXT,
    created_at   TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_memory_persona ON memory(persona_id);
CREATE INDEX IF NOT EXISTS idx_memory_type    ON memory(type);
CREATE INDEX IF NOT EXISTS idx_memory_import  ON memory(importance DESC);

-- 模型配置
CREATE TABLE IF NOT EXISTS model_config (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    model_type    TEXT NOT NULL CHECK(model_type IN ('llm', 'asr', 'tts', 'emotion', 'embedding')),
    provider_id   TEXT NOT NULL,
    model_path    TEXT,
    vram_required TEXT,
    size_mb       INTEGER,
    status        TEXT DEFAULT 'not_downloaded' CHECK(status IN ('not_downloaded', 'downloading', 'downloaded', 'active')),
    is_active     INTEGER DEFAULT 0,
    config_json   TEXT,
    created_at    TEXT DEFAULT (datetime('now'))
);

-- 下载任务
CREATE TABLE IF NOT EXISTS download_task (
    id          TEXT PRIMARY KEY,
    model_id    TEXT NOT NULL REFERENCES model_config(id),
    progress    INTEGER DEFAULT 0,
    status      TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'downloading', 'completed', 'failed')),
    error_msg   TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);

-- 应用设置（key-value）
CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value_json  TEXT NOT NULL,
    updated_at  TEXT DEFAULT (datetime('now'))
);

-- 提醒
CREATE TABLE IF NOT EXISTS reminder (
    id          TEXT PRIMARY KEY,
    persona_id  TEXT NOT NULL REFERENCES persona(id),
    fire_at     TEXT NOT NULL,
    content     TEXT NOT NULL,
    done        INTEGER DEFAULT 0,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_reminder_fire ON reminder(fire_at);

-- 情绪日志
CREATE TABLE IF NOT EXISTS emotion_log (
    id          TEXT PRIMARY KEY,
    session_id  TEXT REFERENCES session(id),
    message_id  TEXT REFERENCES message(id),
    source      TEXT CHECK(source IN ('text', 'voice', 'fused')),
    emotion     TEXT NOT NULL,
    valence     REAL,
    arousal     REAL,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_emotion_session ON emotion_log(session_id, created_at);

-- 云端 TTS 服务商配置
CREATE TABLE IF NOT EXISTS cloud_tts_provider (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    provider_type   TEXT NOT NULL DEFAULT 'wusound',
    api_base        TEXT NOT NULL,
    api_key         TEXT NOT NULL DEFAULT '',
    icon_url        TEXT,
    voices_json     TEXT DEFAULT '[]',
    is_enabled      INTEGER DEFAULT 1,
    last_verified_at TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now'))
);

-- 微信 iLink 通道 ───────────────────────────────

-- 微信账号（机器人）登录状态
CREATE TABLE IF NOT EXISTS wechat_account (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT,
    nickname            TEXT,
    avatar_url          TEXT,
    bot_token           TEXT,
    qrcode_key          TEXT,
    qrcode_url          TEXT,
    qrcode_status       TEXT DEFAULT 'idle', -- idle/pending/scanned/confirmed/expired
    get_updates_buf     TEXT DEFAULT '',
    persona_id          TEXT,                -- 绑定的角色 ID，NULL=不自动回复
    status              TEXT DEFAULT 'offline', -- offline/logging_in/online/error
    last_error          TEXT,
    last_login_at       TEXT,
    last_sync_at        TEXT,
    created_at          TEXT DEFAULT (datetime('now')),
    updated_at          TEXT DEFAULT (datetime('now'))
);

-- 微信会话（好友 / 群 / 公众号）
CREATE TABLE IF NOT EXISTS wechat_session (
    id                  TEXT PRIMARY KEY,
    account_id          TEXT NOT NULL,
    peer_id             TEXT NOT NULL,
    peer_type           TEXT DEFAULT 'user', -- user/room/official
    peer_name           TEXT,
    peer_avatar         TEXT,
    last_msg_preview    TEXT,
    last_msg_at         TEXT,
    unread_count        INTEGER DEFAULT 0,
    is_pinned           INTEGER DEFAULT 0,
    is_muted            INTEGER DEFAULT 0,
    created_at          TEXT DEFAULT (datetime('now')),
    updated_at          TEXT DEFAULT (datetime('now')),
    UNIQUE(account_id, peer_id)
);

-- 微信消息
CREATE TABLE IF NOT EXISTS wechat_message (
    id                  TEXT PRIMARY KEY,
    account_id          TEXT NOT NULL,
    session_id          TEXT NOT NULL,
    remote_msg_id       TEXT,
    direction           TEXT NOT NULL, -- inbound / outbound
    msg_type            TEXT NOT NULL DEFAULT 'text', -- text/image/voice/video/file/system
    content             TEXT,
    media_url           TEXT,
    media_local_path    TEXT,
    sender_id           TEXT,
    sender_name         TEXT,
    context_token       TEXT,
    status              TEXT DEFAULT 'sent', -- pending/sent/failed
    error               TEXT,
    created_at          TEXT DEFAULT (datetime('now')),
    UNIQUE(account_id, remote_msg_id)
);

CREATE INDEX IF NOT EXISTS idx_wechat_msg_session ON wechat_message(session_id, created_at);

-- 同步游标（冗余备份，防止长轮询异常丢游标）
CREATE TABLE IF NOT EXISTS wechat_sync_state (
    account_id          TEXT PRIMARY KEY,
    get_updates_buf     TEXT NOT NULL,
    last_sync_at        TEXT,
    consecutive_errors  INTEGER DEFAULT 0
);

-- ════════════════════════════════════════════
-- 种子数据
-- ════════════════════════════════════════════

-- 默认用户
INSERT OR IGNORE INTO user (id, name) VALUES (1, '默认用户');

-- 默认模型配置
INSERT OR IGNORE INTO model_config (id, name, model_type, provider_id, status, is_active, vram_required, size_mb) VALUES
    -- LLM
    ('model_gemma4_12b',  'Gemma 4 12B Unified',     'llm',      'ollama/gemma4:12b',   'not_downloaded', 1, '8-10 GB', 8704),
    ('model_gemma4_e4b',  'Gemma 4 E4B',              'llm',      'ollama/gemma4:e4b',   'not_downloaded', 0, '5 GB',    5100),
    ('model_gemma4_e2b',  'Gemma 4 E2B',              'llm',      'ollama/gemma4:e2b',   'not_downloaded', 0, '3 GB',    2662),
    ('model_qwen3_vl_8b', 'Qwen3-VL 8B',              'llm',      'ollama/qwen3-vl:8b',  'not_downloaded', 0, '5-6 GB',  5325),
    ('model_qwen3_vl_4b', 'Qwen3-VL 4B',              'llm',      'ollama/qwen3-vl:4b',  'not_downloaded', 0, '3-4 GB',  2867),
    ('model_llama4_scout','Llama 4 Scout 17B MoE',    'llm',      'ollama/llama4:scout', 'not_downloaded', 0, '10-12 GB',11264),
    ('model_minicpm_o',   'MiniCPM-o 2.6',            'llm',      'ollama/minicpm-o:2.6','not_downloaded', 0, '5-7 GB',  5018),
    ('model_phi4_mm',     'Phi-4 Multimodal 5.6B',    'llm',      'ollama/phi4:multimodal','not_downloaded',0, '4 GB',   3686),
    ('model_internvl3_8b','InternVL3 8B',             'llm',      'ollama/internvl3:8b', 'not_downloaded', 0, '5-6 GB',  5120),
    -- ASR
    ('model_whisper',     'faster-whisper-large-v3',  'asr',      'faster-whisper',      'not_downloaded', 0, '2-3 GB',  1536),
    ('model_whisper_med', 'faster-whisper-medium',    'asr',      'faster-whisper',      'not_downloaded', 0, '1-2 GB',  819),
    ('model_funasr',      'FunASR Paraformer-zh',     'asr',      'funasr',              'not_downloaded', 0, '1-2 GB',  1024),
    -- TTS
    ('model_cosyvoice',   'CosyVoice 2',              'tts',      'cosyvoice',           'not_downloaded', 0, '4-6 GB',  2867),
    ('model_chattts',     'ChatTTS',                  'tts',      'chattts',             'not_downloaded', 0, '2-3 GB',  1229),
    ('model_piper',       'Piper TTS',                'tts',      'piper',               'not_downloaded', 0, '0.2 GB',  205),
    -- Embedding
    ('model_bge_m3',      'bge-m3',                   'embedding','bge-m3',              'not_downloaded', 0, '2 GB',    1229),
    ('model_bge_small',   'bge-small-zh-v1.5',        'embedding','bge-small',           'not_downloaded', 0, '0.5 GB',  31),
    ('model_jina_v3',     'jina-embeddings-v3',       'embedding','jina-v3',             'not_downloaded', 0, '1 GB',    512);

-- 内置技能
INSERT OR IGNORE INTO skill (id, name, description, definition_yaml, enabled) VALUES
    ('skill_weather',    'get_weather',    '查询指定城市天气（当前或预报）',   'name: get_weather\ndescription: 查询指定城市天气信息（当前或预报）\nparameters:\n  - name: city\n    type: string\n    required: true\n    description: 城市名\n  - name: date\n    type: string\n    required: false\n    description: 查询日期 today/tomorrow/ISO日期\n  - name: days\n    type: integer\n    required: false\n    description: 预报天数\npermissions: [network]\napproval_mode: once\nexecutor_type: rust\n', 1),
    ('skill_reminder',   'set_reminder',   '设置提醒和日程（支持相对时间）',   'name: set_reminder\ndescription: 设置提醒（支持绝对和相对时间）\nparameters:\n  - name: content\n    type: string\n    required: true\n  - name: fire_at\n    type: string\n    required: false\n    description: ISO 8601 时间\n  - name: relative_time\n    type: string\n    required: false\n    description: 相对时间如 in 30 minutes\n  - name: repeat\n    type: string\n    required: false\npermissions: []\napproval_mode: always\nexecutor_type: rust\n', 1),
    ('skill_file_search','file_search',    '搜索本地文件',                    'name: file_search\ndescription: 搜索本地文件\nparameters:\n  - name: query\n    type: string\n    required: true\n  - name: dir\n    type: string\n    required: false\npermissions: [filesystem]\napproval_mode: once\nexecutor_type: rust\n', 1),
    ('skill_web_search', 'web_search',     '搜索互联网信息（支持时间过滤）',   'name: web_search\ndescription: 搜索互联网信息\nparameters:\n  - name: query\n    type: string\n    required: true\n  - name: top_n\n    type: integer\n    required: false\n    description: 返回结果数\n  - name: time_range\n    type: string\n    required: false\n    description: 时间范围 day/week/month/year\npermissions: [network]\napproval_mode: ask_every_time\nexecutor_type: rust\n', 1),
    ('skill_get_time',   'get_time',       '查询时间、日期、时区、倒计时',     'name: get_time\ndescription: 查询时间日期时区及日期计算\nparameters:\n  - name: operation\n    type: string\n    required: false\n    description: 操作类型 current/add/diff/countdown/timezone\n  - name: days\n    type: integer\n    required: false\n  - name: timezone\n    type: string\n    required: false\npermissions: []\napproval_mode: once\nexecutor_type: rust\n', 1),
    ('skill_calculator', 'calculator',     '安全数学表达式计算',              'name: calculator\ndescription: 安全计算数学表达式\nparameters:\n  - name: expression\n    type: string\n    required: true\n    description: 数学表达式\npermissions: []\napproval_mode: once\nexecutor_type: rust\n', 1),
    ('skill_clipboard',  'clipboard',      '读写系统剪贴板',                  'name: clipboard\ndescription: 读写系统剪贴板\nparameters:\n  - name: action\n    type: string\n    required: true\n    description: read/write\n  - name: text\n    type: string\n    required: false\n    description: 写入文本\npermissions: []\napproval_mode: ask_every_time\nexecutor_type: rust\n', 1),
    ('skill_sysinfo',    'system_info',    '查询系统状态（CPU/内存/磁盘等）', 'name: system_info\ndescription: 查询系统状态信息\nparameters:\n  - name: query\n    type: string\n    required: false\n    description: cpu/memory/disk/battery/os/all\npermissions: []\napproval_mode: once\nexecutor_type: rust\n', 1),
    ('skill_notetake',   'note_take',      '管理个人笔记',                    'name: note_take\ndescription: 管理个人笔记\nparameters:\n  - name: action\n    type: string\n    required: true\n    description: save/search/list/delete\n  - name: content\n    type: string\n    required: false\n  - name: keyword\n    type: string\n    required: false\n  - name: tags\n    type: string\n    required: false\npermissions: []\napproval_mode: once\nexecutor_type: rust\n', 1),
    ('skill_translate',  'translate',      '文本翻译（多语言）',              'name: translate\ndescription: 翻译文本\nparameters:\n  - name: text\n    type: string\n    required: true\n  - name: from_lang\n    type: string\n    required: false\n  - name: to_lang\n    type: string\n    required: false\npermissions: [network]\napproval_mode: once\nexecutor_type: python\n', 1),
    ('skill_random',     'random',         '随机数/骰子/密码/抽选',           'name: random\ndescription: 随机内容生成\nparameters:\n  - name: type\n    type: string\n    required: true\n    description: number/dice/password/pick\n  - name: min\n    type: integer\n    required: false\n  - name: max\n    type: integer\n    required: false\n  - name: faces\n    type: integer\n    required: false\n  - name: length\n    type: integer\n    required: false\n  - name: items\n    type: string\n    required: false\npermissions: []\napproval_mode: once\nexecutor_type: rust\n', 1);

UPDATE skill SET enabled = 1, definition_yaml = 'name: web_search\ndescription: 搜索互联网信息（支持时间过滤）\nparameters:\n  - name: query\n    type: string\n    required: true\n    description: 搜索关键词\n  - name: top_n\n    type: integer\n    required: false\n    description: 返回结果数量（默认 5，最大 10）\n  - name: time_range\n    type: string\n    required: false\n    description: 时间范围过滤 day/week/month/year\npermissions: [network]\napproval_mode: ask_every_time\nexecutor_type: rust\n' WHERE name = 'web_search';
UPDATE skill SET definition_yaml = 'name: get_weather\ndescription: 查询指定城市天气（当前或预报）\nparameters:\n  - name: city\n    type: string\n    required: true\n    description: 城市名\n  - name: date\n    type: string\n    required: false\n    description: today/tomorrow/ISO日期\n  - name: days\n    type: integer\n    required: false\n    description: 预报天数（默认1）\npermissions: [network]\napproval_mode: once\nexecutor_type: rust\n' WHERE name = 'get_weather';
UPDATE skill SET definition_yaml = 'name: set_reminder\ndescription: 设置提醒（支持绝对时间和相对时间）\nparameters:\n  - name: content\n    type: string\n    required: true\n    description: 提醒内容\n  - name: fire_at\n    type: string\n    required: false\n    description: ISO 8601 时间\n  - name: relative_time\n    type: string\n    required: false\n    description: 相对时间如 "in 30 minutes" "tomorrow 9am"\n  - name: repeat\n    type: string\n    required: false\n    description: 重复周期 none/daily/weekly\npermissions: []\napproval_mode: always\nexecutor_type: rust\n' WHERE name = 'set_reminder';
UPDATE skill SET definition_yaml = 'name: file_search\ndescription: 搜索本地文件\nparameters:\n  - name: query\n    type: string\n    required: true\n    description: 文件名关键词\n  - name: dir\n    type: string\n    required: false\n    description: 搜索目录\npermissions: [filesystem]\napproval_mode: once\nexecutor_type: rust\n' WHERE name = 'file_search';
