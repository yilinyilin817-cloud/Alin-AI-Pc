# AI 伴侣

> 全本地化 · 多模态 · 桌面端 AI 伴侣

**零数据外泄** — 所有模型在本地推理，无需联网即可对话。支持文字 / 语音 / 图像 / 截屏 / 摄像头帧多模态交互，自带角色系统、知识库 RAG、技能工具调用、情绪感知与长期记忆。

**架构快速一览：**

```
Frontend (Vue 3 + Element Plus + Pinia)
    | Tauri IPC (invoke / event / stream)
Rust Core (对话编排 / RAG / 技能 / 记忆 / 情绪 / 30+ 命令)
    | ModelBus trait (Ollama HTTP | Worker IPC)
Python Workers (LLM / ASR / TTS / Embedding / VAD / Emotion)
    |
SQLite + sqlite-vec + 文件系统
```

- **11 个 Rust 模块** + **7 个 Python Worker** + **10+ 内置技能** + **18 个 IPC 封装模块**
- **17 张数据库表**：用户 / 角色 / 会话 / 消息 / 知识库 / 技能 / 笔记 / 记忆 / 情绪 / 插件 / 工作流

---

## 功能

| 能力 | 状态 | 说明 |
|---|---|---|
| 多模态对话 | 完成 | 文字、语音、图像、截屏、摄像头帧，Ollama 后端或 Python llama.cpp 后端 |
| 角色系统 | 完成 | 自定义角色卡（人设 / 音色 / 技能绑定 / 知识库），Aria、Kai、婉清 三套内置 |
| 知识库 RAG | 完成 | 多种格式文件导入、自动分块、向量 + FTS5 混合检索、多知识库管理 |
| 技能工具调用 | 完成 | 天气 / 提醒 / 笔记 / 计算 / 剪贴板 / 系统信息 / 翻译 / 随机 / 文件搜索 / 网络搜索（可审批） |
| 语音交互 | 完成 | faster-whisper ASR、CosyVoice / pyttsx3 TTS、silero-vad 实时打断 |
| 情绪感知 | 完成 | LLM 文本情绪分类 + Wav2Vec2 语音情绪识别 + 跨模态融合，情绪历史追踪 |
| 三级记忆 | 完成 | 短期记忆（上下文窗口管理）、长期记忆（事件抽取 + 向量召回）、记忆摘要 |
| 模型管理 | 完成 | 多 Provider 切换（Ollama / Cloud / iLink / WuSound）、显存检测、模型下载 |
| Workflow 引擎 | 完成 | 可视化节点编辑器，触发器 + 动作链（记忆/知识库/技能/网络搜索/发送消息） |
| 插件系统 | 完成 | Manifest 规范、安装 / 启用 / 配置生命周期、角色级挂载 |
| 主动推送 | 完成 | 基于情绪、时间、事件触发的 proactive 消息推送 |
| 微信集成 | 完成 | 微信联系人浏览、聊天记录同步、本地搜索与分析 |
| 主题切换 | 完成 | 浅色 / 深色 / 跟随系统，偏好持久化 |
| 命令面板 | 完成 | Ctrl+K 全局搜索，快速跳转页面、检索会话、执行命令 |
| 系统托盘 | 完成 | 最小化到托盘，后台常驻 |
| 全状态恢复 | 完成 | 窗口大小 / 位置、上次角色、会话持久化 |

> 所有"完成"项均为真实实现，安装模型依赖后即可全功能运行。

### 降级策略

| 场景 | 表现 |
|---|---|
| Ollama 未安装 / 未启动 | 自动切换 Mock 回复（流式），其余功能正常 |
| Python Worker 不可用 | ASR / TTS / Embedding 功能不可用，Chat 走 Rust 内置 + 技能调用 |
| 未下载 ASR 模型 | 语音输入不可用，文字输入正常 |
| 未下载 Embedding 模型 | RAG 降级为纯 FTS5 关键词搜索，长期记忆降级 |
| 未下载 TTS 模型 | 语音合成不可用，文字回复正常 |

---

## 快速启动

### 环境要求

| 依赖 | 版本 | 说明 |
|---|---|---|
| Node.js | >= 18 | 前端构建 |
| Rust | >= 1.75 | Tauri 后端（需 MSVC 工具链） |
| Python | >= 3.10 | 模型推理 Worker（可选，不用 ASR/TTS 时可跳过） |
| Ollama | 最新 | 推荐 LLM 后端（可选，可用 Mock 降级） |

### 安装与运行

```bash
# 1. 安装前端依赖
cd ai-companion
npm install

# 2. 安装 Python Worker 依赖（按需）
pip install faster-whisper pyttsx3
pip install -r workers/requirements.txt

# 3. 拉取 LLM 模型（推荐）
ollama pull gemma4:12b
# 或中文优先
ollama pull qwen3-vl:8b

# 4. 下载模型权重（可选）
python workers/download_models.py --list
python workers/download_models.py whisper
python workers/download_models.py bge-m3

# 5. 启动开发调试
npm run tauri dev
```

### 模型下载与部署

桌面端内置“模型中心”，按 LLM / ASR / TTS / Embedding 分类展示模型状态、下载进度、部署命令、本地目录和运行依赖。下载完成后可直接在卡片上启用模型或运行功能测试。

| 类型 | 模型中心入口 | 推荐模型 | 部署依赖 |
|---|---|---|---|
| LLM | 对话模型 | Gemma 4 12B、Qwen3-VL 8B、MiniCPM-o 2.6 | Ollama 服务 |
| ASR | 语音识别 | faster-whisper-large-v3、faster-whisper-medium、FunASR Paraformer-zh | Python Worker、faster-whisper 或 FunASR |
| TTS | 语音合成 | CosyVoice 2、ChatTTS、Piper TTS | Python Worker、pyttsx3，按需安装 CosyVoice / Piper / ChatTTS |
| Embedding | 向量检索 | bge-m3、bge-small-zh-v1.5、jina-embeddings-v3 | sentence-transformers |

常用命令：

```bash
# 查看脚本支持的模型
python workers/download_models.py --list

# 下载 TTS 模型
python workers/download_models.py cosyvoice
python workers/download_models.py chattts
python workers/download_models.py piper

# 下载其他模型
python workers/download_models.py whisper-medium
python workers/download_models.py funasr
python workers/download_models.py bge-m3
python workers/download_models.py jina-v3

# 批量下载全部非内置模型
python workers/download_models.py --all
```

模型中心会自动识别下载目录并同步数据库状态。Ollama 模型使用 `ollama pull <tag>`，其他模型默认下载到 `data/models/<model-id>`；也可以在设置中调整模型目录。

### 打包发布

```bash
npm run tauri build
```

---

## 项目结构

```
ai-companion/
  src-tauri/                   # Rust 后端
    src/
      commands/                # 30+ Tauri IPC 命令
      model_bus/               # ModelProvider trait + 多后端
        ollama.rs              # Ollama HTTP 客户端
        cloud.rs               # 云端 Provider（OpenAI 兼容）
        scheduler.rs           # 多模型调度
        ilink/                 # iLink 协议客户端
      worker/                  # Python Worker IPC 池
      orchestrator/            # 对话编排 Pipeline + Workflow 引擎
      perception/              # 截屏 / 音频 I/O / 摄像头
      rag/                     # 知识库索引 + 向量检索
      skill/                   # 技能注册 / 执行 / 校验
      memory/                  # 三级记忆系统
      emotion/                 # 文本 + 语音情绪融合
      vector/                  # VectorStore trait + sqlite-vec / Qdrant
      relationship/            # 关系亲密度与情绪记忆曲线
      proactive/               # 主动推送引擎
      plugin/                  # 插件生命周期管理
      wechat/                  # 微信数据同步
      storage/                 # SQLite repo 层 + 17 张表
      context.rs               # 1M token 上下文窗口管理
      state.rs                 # 全局 AppState
      lib.rs                   # Tauri setup / 模块组装
    Cargo.toml                 # 40+ 依赖
  src/                         # Vue 3 前端
    views/                     # 9 个页面视图
    components/                # 30 个组件（含 3 个 base 组件）
    stores/                    # 8 个 Pinia store
    api/                       # 18 个 Tauri IPC 封装模块
    composables/               # 6 个组合式函数
    styles/                    # 设计令牌 + 全局样式
    types/                     # TypeScript 类型定义
    router/                    # 路由配置
    mocks/                     # 离线 mock 数据
  workers/                     # Python 推理 Worker
    llm_worker.py              # LLM 推理（llama.cpp）
    asr_worker.py              # 语音识别（faster-whisper）
    tts_worker.py              # 语音合成（CosyVoice / pyttsx3）
    embedding_worker.py        # 向量化（bge-m3）
    vad_worker.py              # 语音活动检测（silero-vad）
    emotion_worker.py          # 语音情绪识别（Wav2Vec2）
    download_models.py         # 模型下载脚本
    protocol.py                # MessagePack IPC 协议
  skills/                      # 内置 Python 技能实现
  data/
    personas/                  # 内置角色定义
    skills/                    # 技能 YAML 定义
  docs/                        # 文档
```

---

## 配置推荐

| 模型 | 显存 (Q4) | 推荐场景 |
|---|---|---|
| Gemma 4 12B | 8-10 GB | 主力模型，全模态（图 / 音 / 视频帧） |
| Gemma 4 E4B | 5 GB | 显存受限设备 |
| Qwen3-VL 8B | 5-6 GB | 中文优先场景 |
| faster-whisper large-v3 | 2-3 GB | 语音识别 |
| CosyVoice 2 | 4-6 GB | 情感语音合成 |
| BGE-M3 | 2 GB | RAG 向量化 |

大语言模型使用 Ollama 作为默认后端；Python llama.cpp Worker 作为备选（按角色切换）。

---

## 开发命令

```bash
npm run tauri dev         # 开发模式（Vite HMR + Rust 热重载）
npm run tauri build       # 打包为安装包
cargo check               # Rust 编译检查（需 MSVC 工具链）
```

---

## 技术栈

- **桌面框架**：Tauri 2（Rust + WebView）
- **前端**：Vue 3 + TypeScript + Element Plus + Pinia + Vue Router
- **可视化**：Three.js（生成艺术背景）
- **后端语言**：Rust（异步运行时：tokio）
- **数据库**：SQLite（rusqlite + bundled）+ sqlite-vec 向量扩展 + FTS5
- **Worker IPC**：MessagePack（rmp-serde）over stdin/stdout
- **Python 推理**：faster-whisper / llama.cpp / CosyVoice / sentence-transformers / silero-vad

---

## 许可

MIT License

---

*AI 伴侣 · 由你自己的大模型陪伴 · v0.1.0*
