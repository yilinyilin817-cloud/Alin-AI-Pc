# 下一轮功能扩展方案 Spec

## Why

当前应用已完成设计系统、Dashboard、命令面板、通知中心等基础体验升级。为了进一步提升对话深度与角色智能化程度，下一轮聚焦**对话能力升级**：让角色能听会说、能分段表达、能按工作流自主执行，并支持通过插件扩展能力边界，同时补齐对应的 UI 与交互细节。

## What Changes

- **语音消息**：聊天支持发送/接收语音消息，消息气泡内可直接播放，显示波形、时长与转写文本；语音输入从"录音转文字"升级为可选"发送语音消息"。
- **多段式回复**：AI 回复可拆分为多个段落（文本、代码、图片、思考、工具结果），每段独立渲染、可折叠/展开，支持流式逐段输出。
- **角色工作流**：在角色卡中新增工作流编排，支持按触发条件（收到消息、定时、事件）执行动作序列（记忆检索、知识库查询、网络搜索、调用技能、发送消息）。
- **插件系统**：引入可安装/启用/配置的插件机制，插件通过声明式 manifest 注册技能、命令与设置项；提供本地插件目录与插件市场入口。
- **UI 优化**：新增语音波形组件、分段消息渲染器、工作流编辑器、插件卡片与详情页；统一空状态、加载态与错误态视觉。

## Impact

- 受影响页面：`src/views/Chat.vue`、`src/views/PersonaEditor.vue`、`src/views/SkillMarket.vue`、新增 `src/views/PluginManager.vue`、新增 `src/views/WorkflowEditor.vue`。
- 受影响组件：`ChatBubble.vue`、`MessageRenderer.vue`、`ChatInput.vue`、`VoiceVisualizer.vue`、新增 `SegmentedMessage.vue`、`VoiceMessageBubble.vue`、`WorkflowNodeEditor.vue`、`PluginCard.vue`。
- 受影响后端：`src-tauri/src/commands/voice.rs`（语音消息上传/播放）、`src-tauri/src/commands/chat.rs`（分段输出协议）、`src-tauri/src/commands/persona.rs`（工作流持久化）、新增 `src-tauri/src/plugin/*`。
- 受影响类型：`src/types/chat.ts`（Message 结构扩展分段）、`src/types/persona.ts`（新增 Workflow 定义）、新增 `src/types/plugin.ts`。
- 新增产物：插件 manifest 规范、工作流 DSL、语音消息存储与转写流水线。

## ADDED Requirements

### Requirement: 语音消息

The system SHALL 在聊天中支持发送、接收和播放语音消息，并提供语音转写与语音输入模式切换。

#### Scenario: 发送语音消息
- **WHEN** 用户在聊天输入区按住麦克风按钮录音，松开后选择"发送语音"
- **THEN** 消息列表出现一条语音消息气泡，显示时长与播放按钮

#### Scenario: 播放语音消息
- **WHEN** 用户点击语音消息气泡上的播放按钮
- **THEN** 播放对应音频，波形随播放进度高亮，再次点击暂停

#### Scenario: 语音转文字显示
- **WHEN** 语音消息发送成功或收到 AI/他人语音消息
- **THEN** 气泡下方显示转写文本（可选折叠），转写失败时显示"转写中…/转写失败"

#### Scenario: 语音输入模式切换
- **WHEN** 用户在设置中开启"语音消息优先"
- **THEN** 点击麦克风按钮默认发送语音消息，而非转换为文字后发送

### Requirement: 多段式回复

The system SHALL 支持 AI 将单次回复拆分为多个可独立渲染、可折叠的段落。

#### Scenario: 分段渲染
- **WHEN** AI 返回包含多个 segment 的回复
- **THEN** 每个 segment 按类型（text、code、image、think、tool_result）独立渲染，并支持分段流式追加

#### Scenario: 折叠思考过程
- **WHEN** segment 类型为 think 或 reasoning
- **THEN** 默认折叠为"思考过程"标签，用户点击后展开查看详情

#### Scenario: 复制单段内容
- **WHEN** 用户悬停在某一段落上
- **THEN** 显示复制按钮，仅复制该段内容

#### Scenario: 引用与来源
- **WHEN** segment 携带 source 字段（知识库/网页/记忆）
- **THEN** 在段落末尾显示来源徽章，点击可跳转或浮窗预览

### Requirement: 角色工作流

The system SHALL 允许角色配置触发条件与动作序列，使角色能在特定场景下自动执行多步操作。

#### Scenario: 创建工作流
- **WHEN** 用户在角色编辑页切换到"工作流"标签
- **THEN** 可通过拖拽或表单新建工作流：填写触发条件、添加动作节点、连线排序

#### Scenario: 触发工作流
- **WHEN** 收到用户消息且满足某条工作流的触发条件
- **THEN** 后端按顺序执行动作序列，并将中间结果注入上下文，最终生成回复

#### Scenario: 工作流调试
- **WHEN** 用户点击工作流旁的"测试运行"按钮
- **THEN** 弹出调试面板，展示每步输入/输出与耗时，支持单步重试

#### Scenario: 启用与禁用
- **WHEN** 用户关闭某条工作流开关
- **THEN** 该工作流不再触发，但配置保留

### Requirement: 插件系统

The system SHALL 提供可扩展的插件机制，使第三方或本地脚本能够安全地注册新技能与命令。

#### Scenario: 安装本地插件
- **WHEN** 用户在插件管理页选择"导入本地插件"并指定插件目录
- **THEN** 系统读取 manifest，校验权限与入口，注册成功后出现在插件列表

#### Scenario: 启用/禁用插件
- **WHEN** 用户切换插件开关
- **THEN** 该插件提供的技能与命令在全局即时生效或失效

#### Scenario: 插件配置
- **WHEN** 插件 manifest 声明了配置项
- **THEN** 在插件详情页自动生成表单，修改后持久化

#### Scenario: 插件市场入口
- **WHEN** 用户进入 SkillMarket 页面
- **THEN** 顶部显示"插件市场"与"已安装"两个标签，可浏览、安装、更新插件

### Requirement: UI 优化

The system SHALL 为新功能提供一致、美观且符合设计系统的界面。

#### Scenario: 语音消息气泡
- **WHEN** 语音消息出现在聊天中
- **THEN** 气泡使用设计系统颜色，波形动画流畅，播放进度可视，布局不突兀

#### Scenario: 分段消息容器
- **WHEN** AI 回复包含多段
- **THEN** 各段之间使用 subtle divider 分隔，类型图标清晰，整体保持阅读节奏

#### Scenario: 空状态与加载态
- **WHEN** 工作流编辑器或插件市场无数据
- **THEN** 显示统一的空状态插画与提示文案

## MODIFIED Requirements

### Requirement: 聊天消息模型

The system SHALL 扩展现有 `Message` 与 `ContentPart` 类型以支持语音分段消息。

- `ContentPart` 新增 `audio_bytes` 的元数据字段（duration、transcript、mime）。
- `Message` 新增可选字段 `segments?: MessageSegment[]`，与现有 `content` 字段兼容；当 `segments` 存在时优先渲染分段内容。

### Requirement: 角色编辑器

The system SHALL 在角色编辑器新增"工作流"与"插件"两个配置标签。

- "工作流"标签：展示该角色关联的工作流列表，支持增删改查与测试。
- "插件"标签：展示已安装插件中该角色可启用的插件与技能，支持勾选。

### Requirement: 语音命令

The system SHALL 扩展后端语音命令以支持语音消息的上传、下载与播放。

- `start_recording` / `stop_recording` 保持 ASR 能力。
- 新增 `save_voice_message`、`play_voice_message`、`get_voice_transcript` 命令。

## REMOVED Requirements

无删除需求。本方案为增量扩展，不废弃现有功能。
