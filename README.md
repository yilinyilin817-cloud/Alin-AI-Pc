# AI 伴侣 · 全本地化多模态桌面应用

> **零数据外泄 · 断网可用 · 可自定义角色 · 支持工具调用**

全本地化运行的桌面端 AI 伴侣，所有模型（LLM / ASR / TTS / 情绪识别）均在本地推理。支持多模态对话（文字/语音/图像/截屏/摄像头帧）、自定义角色、知识库 RAG、Skill 工具调用。

---

## ✨ 功能矩阵

| 功能 | 状态 | 说明 |
|------|------|------|
| **Gemma 4 多模态 LLM** | ✅ 代码就绪 | Ollama（推荐，`ollama pull gemma4:12b`）+ Python llama.cpp 双后端，按角色切换 |
| **Qwen3-VL 备选** | ✅ 代码就绪 | 中文优先时切 `ollama pull qwen3-vl:8b` |
| **ASR 语音识别** | ✅ 代码就绪 | faster-whisper 实时转录 |
| **TTS 语音合成** | ✅ 代码就绪 | CosyVoice 情感语音 + pyttsx3 兜底 |
| **语音 VAD 打断** | ✅ 代码就绪 | silero-vad 流式检测，barge-in 即时打断 |
| **情绪识别（文本）** | ✅ 代码就绪 | LLM 内联分类 JSON 输出 |
| **情绪识别（语音）** | ✅ 代码就绪 | Wav2Vec2 SER + 文本融合 |
| **SQLite 数据库** | ✅ 已实现 | 用户/角色/会话/消息/记忆/技能/设置全部持久化 |
| **角色系统** | ✅ 已实现 | 自定义角色卡（人设/音色/技能/知识库），Aria + Kai 内置 |
| **知识库 RAG** | ✅ 代码就绪 | 多模态知识库导入 + 向量+FTS5 混合检索 |
| **Skill 工具调用** | ✅ 代码就绪 | 天气/提醒/文件搜索/网络搜索/家居控制 + 用户审批 |
| **长期记忆** | ✅ 代码就绪 | 事件抽取 + 向量召回，跨会话记忆 |
| **情绪记忆曲线** | ✅ 代码就绪 | 情绪追踪与分析 |
| **截屏/摄像头** | ✅ 代码就绪 | 屏幕截取 + 摄像头帧感知（feature gate） |
| **模型管理中心** | ✅ 代码就绪 | 显存检测 + 模型下载 + 激活切换 |
| **Live2D 角色形象** | ⏳ 占位 | CSS 头像 + 表情映射，SDK 接口预留 |
| **Ollama 集成** | ✅ 默认 | 最简启动路径 |
| **跨平台** | ✅ 架构 | Tauri 2 — Windows / macOS / Linux |

> ✅ 代码就绪 = 真实实现，需安装依赖或下载模型后全功能运行
> ⏳ 占位 = 接口预留，暂无完整实现

---

## 🆕 功能特性

| 特性 | 说明 |
|------|------|
| **设计系统与 CSS 变量** | 建立统一的设计令牌（颜色/字号/间距/阴影），全平台通过 CSS 变量维护一致的视觉层级与组件风格。 |
| **主题切换** | 支持浅色、深色与跟随系统三种模式，偏好设置自动持久化。 |
| **聊天界面美化** | Markdown 渲染、语法高亮代码块、一键复制、行号显示、打字机效果与空状态插画，提升长对话可读性。 |
| **首页仪表盘** | 汇总最近会话、快捷入口、运行状态与推荐操作，打开应用即可一键继续。 |
| **全局搜索与命令面板** | 按 `Ctrl/Cmd + K` 唤起搜索面板，快速跳转页面、检索会话或执行常用命令。 |
| **生成艺术氛围背景** | 基于 p5.js 的轻量生成艺术背景，为聊天与仪表盘营造动态氛围。 |
| **桌面端体验优化** | 恢复上次窗口状态、系统托盘菜单、全局快捷键、拖拽文件直接发送，贴合原生桌面使用习惯。 |
| **通知中心** | 统一的消息通知聚合面板，支持历史通知查看与一键清除。 |

---

## 🔄 下一轮功能

本阶段重点补齐四大能力，让对话与角色系统更完整：

| 能力 | 说明 |
|------|------|
| **语音消息** | 前端支持录音/转文字/发送语音，后端持久化 WAV 文件并提供播放与 ASR 转写，设置页可开启“语音消息优先”。 |
| **多段式回复** | 后端 Pipeline 将回复拆分为文本/代码/图片/思考/工具结果等 segment，前端流式逐段渲染，支持折叠、复制与来源徽章。 |
| **角色工作流** | 为角色配置触发式自动化流程（消息/定时/事件），支持记忆检索、知识库查询、网络搜索、技能调用、发送消息等动作节点，并可在编辑器中调试运行。 |
| **插件系统** | 定义插件 manifest 规范，后端实现安装/启用/配置生命周期，前端提供插件市场、插件管理与角色级插件技能挂载。 |

## 🏗 架构总览

```
Frontend (Vue3 + Element Plus + Pinia)
    ↕ Tauri IPC (invoke / event / stream)
Rust Core (对话编排 / RAG / 技能 / 记忆 / 情绪 / 30+ 命令)
    ↕ ModelBus trait (Ollama HTTP | Python Worker IPC)
Python Workers (LLM / ASR / TTS / Embedding / VAD / Emotion)
    ↕
SQLite + sqlite-vec + 文件系统
```

**11 个 Rust 模块** · **7 个 Python Worker** · **5 个内置技能** · **30+ IPC 命令**

---

## 🚀 快速启动

### 前提条件

| 需求 | 说明 |
|------|------|
| Node.js ≥ 18 | 前端构建 |
| Rust ≥ 1.75 | Tauri 后端（需 MSVC 工具链，详见 [docs/SETUP.md](docs/SETUP.md)） |
| Python ≥ 3.10 | 模型推理 Worker（可选，不用 ASR/TTS 时可跳过） |
| Ollama | 推荐 LLM 后端（可选，可用 Mock 降级） |

### 步骤

```bash
# 1. 安装前端依赖
cd ai-companion
npm install

# 2. 安装 Python 依赖（可选，否则降级 Mock）
pip install -r workers/requirements.txt

# 3. 拉取 LLM 模型（推荐）
ollama pull gemma4:12b
# 或中文优先
ollama pull qwen3-vl:8b

# 4. 下载模型权重（可选，语音/RAG 需要）
python workers/download_models.py --list          # 查看可选
python workers/download_models.py whisper         # 下载 ASR 模型
python workers/download_models.py bge-m3          # 下载 Embedding

# 5. 启动开发服务器
npm run tauri dev
```

### 模型降级行为

| 场景 | 表现 |
|------|------|
| **Ollama 未启动** | Chat 用 Mock 回复（流式），其他功能正常 |
| **Python Worker 不可用** | ASR/TTS 不可用，Chat 可文字交流+Rust 内置技能 |
| **Embedding 模型未下载** | RAG 检索降级为纯关键词（FTS5），长期记忆降级 |
| **ASR 模型未下载** | 语音输入不可用，文字输入正常 |
| **TTS 未安装** | 语音合成不可用，文字显示正常 |

---

## 📦 项目结构

```
ai-companion/
├── src-tauri/              # Rust 后端（11 个模块）
│   ├── src/
│   │   ├── model_bus/      # ModelProvider trait + Ollama/LlamaCpp
│   │   ├── worker/         # Python Worker IPC 池
│   │   ├── vector/         # VectorStore trait + sqlite-vec/Qdrant
│   │   ├── rag/            # 知识库索引+检索
│   │   ├── skill/          # 技能注册+执行+校验
│   │   ├── memory/         # 三级记忆系统
│   │   ├── emotion/        # 文本+语音情绪融合
│   │   ├── perception/     # 截屏/摄像头/音频I/O
│   │   ├── orchestrator/   # 对话编排 Pipeline
│   │   ├── commands/       # 30+ IPC 命令
│   │   └── storage/        # SQLite + repo 层
│   └── Cargo.toml
├── src/                    # Vue 3 前端
│   ├── views/              # Chat / PersonaEditor / KnowledgeBase / SkillMarket / ModelCenter / Settings
│   ├── stores/             # Pinia 状态管理（chat/persona/voice/settings）
│   ├── api/                # Tauri IPC 封装层（7 个模块：chat/persona/knowledge/skill/model/voice/capture/settings/memory）
│   ├── composables/        # useChat / useVoice / useEmotion / useCapture
│   └── components/         # 20+ UI 组件
├── workers/                # Python 推理 Worker（7 个）
│   ├── protocol.py         # MessagePack IPC 协议
│   ├── llm_worker.py       # LLM 推理（llama.cpp / Ollama）
│   ├── asr_worker.py       # 语音识别（faster-whisper）
│   ├── tts_worker.py       # 语音合成（CosyVoice / pyttsx3）
│   ├── embedding_worker.py # 向量化（bge-m3）
│   ├── vad_worker.py       # 语音活动检测（silero-vad）
│   ├── emotion_worker.py   # 情绪识别（Wav2Vec2）
│   ├── download_models.py  # 模型下载脚本
│   └── requirements.txt
├── skills/                 # 内置技能实现（5 个 Python）
├── data/
│   ├── skills/             # 技能 YAML 定义
│   └── models/             # 模型权重缓存
└── docs/
    ├── ARCHITECTURE.md     # 完整架构方案 v2.0
    └── SETUP.md            # 分平台安装指南
```

---

## 🧩 核心技术规格

### 推荐模型配置

| 模型 | 类型 | 显存 (Q4) | 推荐场景 |
|------|------|-----------|---------|
| Gemma 4 12B Unified | 多模态 LLM | 8–10 GB | **主推** 全模态（图+音+视频） |
| Gemma 4 E4B | 轻量 LLM | 5 GB | 显存受限 + 音频 |
| Qwen3-VL 8B | 视觉 LLM | 5–6 GB | 中文优先场景 |
| faster-whisper-lv3 | ASR | 2–3 GB | 语音识别 |
| CosyVoice 2 | TTS | 4–6 GB | 情感语音合成 |
| bge-m3 | Embedding | 2 GB | RAG 向量化 |

### 系统要求

- **GPU**：推荐 ≥ 8 GB 显存（RTX 3060/4060+）
- **CPU 模式**：Q4 量化 + 小模型可在 16 GB 内存上运行（速度较慢）
- **磁盘**：模型文件共约 15–20 GB（可选下载）
- **跨平台**：Windows 10+ / macOS 13+ / Ubuntu 22.04+

---

## 🔧 开发命令

```bash
npm run tauri dev       # 开发模式（热更新）
npm run tauri build     # 打包发布
cargo check             # Rust 编译检查（需 MSVC 工具链）
npm run build           # 前端构建（vue-tsc + vite）
```

---

## 📄 许可

MIT License

---

*AI 伴侣 · 由你自己的大模型陪伴 · v0.1.0*
