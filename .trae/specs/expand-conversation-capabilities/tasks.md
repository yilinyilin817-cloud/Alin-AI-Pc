# Tasks

- [x] Task 1: 扩展消息类型与内容模型
  - [x] SubTask 1.1: 在 `src/types/chat.ts` 定义 `MessageSegment` 与 `AudioPartMeta` 类型
  - [x] SubTask 1.2: 扩展 `Message` 与 `ContentPart` 类型以兼容语音与分段
  - [x] SubTask 1.3: 更新后端 `storage/models.rs` 与 `repo.rs` 的序列化/反序列化逻辑
  - [x] SubTask 1.4: 运行 `cargo check` 与 `npm run dev` 确保类型兼容

- [x] Task 2: 语音消息（前端）
  - [x] SubTask 2.1: 在 `ChatInput.vue` 增加"发送语音"与"转文字"模式切换
  - [x] SubTask 2.2: 创建 `VoiceMessageBubble.vue`，支持播放/暂停、进度波形、时长显示
  - [x] SubTask 2.3: 在 `MessageRenderer.vue` 中识别 `audio_bytes` 并渲染语音气泡
  - [x] SubTask 2.4: 在设置页新增"语音消息优先"开关

- [x] Task 3: 语音消息（后端）
  - [x] SubTask 3.1: 新增 `save_voice_message` 命令，保存语音文件到应用数据目录
  - [x] SubTask 3.2: 新增 `play_voice_message` 命令，读取并播放本地语音文件
  - [x] SubTask 3.3: 新增 `get_voice_transcript` 命令，复用 ASR Worker 对语音消息转写
  - [x] SubTask 3.4: 在 `chat.rs` 发送逻辑中支持携带 `audio_bytes` 的消息

- [x] Task 4: 多段式回复（数据协议）
  - [x] SubTask 4.1: 定义后端流式分段输出协议（SSE/事件格式）
  - [x] SubTask 4.2: 修改 `orchestrator/pipeline.rs` 支持按 segment 输出到前端
  - [x] SubTask 4.3: 更新 `src/api/chat.ts` 流式监听器以解析 segment 事件

- [x] Task 5: 多段式回复（UI）
  - [x] SubTask 5.1: 创建 `SegmentedMessage.vue` 组件，根据 segment 类型分发渲染
  - [x] SubTask 5.2: 实现文本段、代码段、图片段、工具结果段的渲染
  - [x] SubTask 5.3: 实现 think/reasoning 段的默认折叠与展开
  - [x] SubTask 5.4: 为每段添加悬停复制按钮与来源徽章
  - [x] SubTask 5.5: 在 `ChatBubble.vue` 中优先使用 `segments` 渲染，回退到 `content`

- [x] Task 6: 角色工作流（类型与存储）
  - [x] SubTask 6.1: 在 `src/types/persona.ts` 定义 `Workflow`、`WorkflowTrigger`、`WorkflowAction` 类型
  - [x] SubTask 6.2: 在 `src-tauri/src/storage/models.rs` 新增工作流表与角色关联
  - [x] SubTask 6.3: 实现 `src-tauri/src/commands/workflow.rs` 的 CRUD 命令
  - [x] SubTask 6.4: 在 `persona.rs` 加载角色时一并加载工作流

- [x] Task 7: 角色工作流（执行引擎）
  - [x] SubTask 7.1: 创建 `src-tauri/src/orchestrator/workflow_engine.rs`
  - [x] SubTask 7.2: 实现触发条件匹配（收到消息、定时、事件）
  - [x] SubTask 7.3: 实现动作节点执行：记忆检索、知识库查询、网络搜索、调用技能、发送消息
  - [x] SubTask 7.4: 将工作流中间结果注入 LLM 上下文并生成回复

- [x] Task 8: 角色工作流（UI 编辑器）
  - [x] SubTask 8.1: 在 `PersonaEditor.vue` 新增"工作流"标签
  - [x] SubTask 8.2: 创建 `WorkflowNodeEditor.vue`，支持添加/删除/排序动作节点
  - [x] SubTask 8.3: 实现工作流启用/禁用开关与保存
  - [x] SubTask 8.4: 实现"测试运行"调试面板，展示每步输入/输出/耗时

- [x] Task 9: 插件系统（规范与后端）
  - [x] SubTask 9.1: 新增 `src/types/plugin.ts`，定义 `PluginManifest`、`PluginPermission`、`PluginConfigField`
  - [x] SubTask 9.2: 创建 `src-tauri/src/plugin/manifest.rs` 解析与校验 manifest
  - [x] SubTask 9.3: 创建 `src-tauri/src/plugin/registry.rs` 管理已安装插件生命周期
  - [x] SubTask 9.4: 实现插件命令：`install_plugin`、`uninstall_plugin`、`enable_plugin`、`configure_plugin`

- [x] Task 10: 插件系统（前端与市场）
  - [x] SubTask 10.1: 创建 `PluginCard.vue` 与 `PluginDetail.vue` 组件
  - [x] SubTask 10.2: 新增 `src/views/PluginManager.vue` 管理已安装插件
  - [x] SubTask 10.3: 改造 `SkillMarket.vue`，增加"插件市场"与"已安装"标签
  - [x] SubTask 10.4: 在 `PersonaEditor.vue` 新增"插件"标签，配置角色可用插件

- [x] Task 11: UI 优化与统一
  - [x] SubTask 11.1: 为语音消息气泡、分段消息容器应用设计系统 CSS 变量
  - [x] SubTask 11.2: 为工作流编辑器、插件市场设计空状态与加载态
  - [x] SubTask 11.3: 更新命令面板数据源，新增"工作流"与"插件"搜索分类
  - [x] SubTask 11.4: 更新 Dashboard 快捷动作，新增"语音输入"与"插件市场"入口

- [x] Task 12: 验证与收尾
  - [x] SubTask 12.1: 按 `checklist.md` 逐项验证
  - [x] SubTask 12.2: 运行 `cargo check` 与 `npm run dev` 确保无新增错误
  - [x] SubTask 12.3: 更新 README.md，补充语音消息、分段回复、角色工作流、插件系统说明

# Task Dependencies

- Task 1 是后续所有任务的基础。
- Task 2 与 Task 3 可并行，但需在 Task 1 之后。
- Task 4 与 Task 5 可并行，但 UI 需在协议确定后细化。
- Task 6 与 Task 7 可并行开发，但执行引擎需在存储完成后联调。
- Task 8 依赖 Task 6。
- Task 9 与 Task 10 可并行，但前端需在 registry 完成后联调。
- Task 11 依赖 Task 2/3/5/8/10。
- Task 12 依赖所有前置任务。
