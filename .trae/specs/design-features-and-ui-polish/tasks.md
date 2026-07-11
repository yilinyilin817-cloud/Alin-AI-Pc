# Tasks

- [x] Task 1: 建立设计系统基础
  - [x] SubTask 1.1: 创建 `src/styles/design-system.css`，定义颜色/字体/间距/圆角/阴影 CSS 变量
  - [x] SubTask 1.2: 在 `main.ts` 或 `App.vue` 中全局引入设计系统
  - [x] SubTask 1.3: 用 CSS 变量替换现有硬编码颜色（优先聊天页与设置页）
  - [x] SubTask 1.4: 封装基础组件 `BaseButton.vue`、`BaseCard.vue`、`BaseModal.vue`

- [x] Task 2: 主题切换能力
  - [x] SubTask 2.1: 在 Pinia store 新增 `appearanceStore`，持久化主题（light/dark/system）
  - [x] SubTask 2.2: 实现 `useTheme()` 组合式函数，监听系统主题变化
  - [x] SubTask 2.3: 在设置页新增“外观”分组与主题下拉框
  - [x] SubTask 2.4: 验证切换后全站颜色即时响应且重启保持

- [x] Task 3: 聊天界面美化
  - [x] SubTask 3.1: 引入 Markdown 渲染库与代码高亮库
  - [x] SubTask 3.2: 重构 `MessageBubble.vue`，区分用户/AI/系统消息样式
  - [x] SubTask 3.3: 实现代码块组件：语言标签、复制按钮、可选行号
  - [x] SubTask 3.4: 替换静态 loading 为打字机动画（三点呼吸/流式文字）
  - [x] SubTask 3.5: 设计并应用空状态插画/提示

- [x] Task 4: 首页仪表盘
  - [x] SubTask 4.1: 设计 Dashboard 信息架构（最近会话、快捷角色、模型状态、微信未读、快捷动作）
  - [x] SubTask 4.2: 创建 `DashboardView.vue` 页面与路由
  - [x] SubTask 4.3: 实现数据聚合：从 chat/persona/model/wechat store 读取最新状态
  - [x] SubTask 4.4: 实现角色卡片点击跳转新建聊天
  - [x] SubTask 4.5: 将启动后默认路由改为 Dashboard

- [x] Task 5: 全局搜索与命令面板
  - [x] SubTask 5.1: 注册全局快捷键 `Ctrl/Cmd + K`（Tauri globalShortcut + 前端兜底）
  - [x] SubTask 5.2: 创建 `CommandPalette.vue` 组件（输入框 + 分类结果列表）
  - [x] SubTask 5.3: 聚合搜索数据源：会话、角色、模型、知识库、设置项、动作命令
  - [x] SubTask 5.4: 实现键盘导航（↑↓选择、Enter 执行、Esc 关闭）
  - [x] SubTask 5.5: 支持 `/` 前缀快捷命令

- [x] Task 6: 生成艺术氛围背景
  - [x] SubTask 6.1: 制定算法艺术哲学文档 `docs/algorithmic-art-philosophy.md`
  - [x] SubTask 6.2: 创建可复用的 `GenerativeArtBackground.vue` 组件（基于 p5.js）
  - [x] SubTask 6.3: 为启动页应用生成艺术背景，支持 seed 复现
  - [x] SubTask 6.4: 为角色封面生成基于角色气质的封面图
  - [x] SubTask 6.5: 为加载状态添加小型生成动画

- [x] Task 7: 桌面端体验优化
  - [x] SubTask 7.1: 使用 Tauri `WindowStatePlugin` 或手动存储恢复窗口位置/尺寸
  - [x] SubTask 7.2: 实现系统托盘图标与托盘菜单
  - [x] SubTask 7.3: 注册原生右键菜单（聊天消息、输入框）
  - [x] SubTask 7.4: 实现聊天输入区文件拖拽发送（先支持图片与文本文件）
  - [x] SubTask 7.5: 新增常用全局快捷键（新建聊天、切换主题、聚焦搜索）

- [x] Task 8: 通知中心
  - [x] SubTask 8.1: 设计通知数据模型与 store
  - [x] SubTask 8.2: 在顶部导航添加通知铃铛与抽屉
  - [x] SubTask 8.3: 接入微信消息事件，非当前会话时生成通知
  - [x] SubTask 8.4: 接入模型断开、同步错误事件
  - [x] SubTask 8.5: 可选：集成 Tauri 原生系统通知

- [x] Task 9: 验证与收尾
  - [x] SubTask 9.1: 运行 `npm run dev` 与 `cargo check`，确保无新增编译/构建错误
  - [x] SubTask 9.2: 按 checklist.md 逐项验证
  - [x] SubTask 9.3: 更新相关文档或 README 中的新功能说明

# Task Dependencies

- Task 2（主题切换）依赖 Task 1（设计系统）
- Task 3（聊天美化）可与 Task 1 并行，但需在 Task 2 之后做深色模式适配
- Task 4（仪表盘）依赖 Task 3 的消息数据结构
- Task 5（命令面板）依赖 Task 4 的数据源
- Task 6（生成艺术）可独立并行
- Task 7（桌面端优化）与 Task 8（通知中心）可并行
- Task 9 依赖前面所有任务完成

- [x] Task 10: 修复验证未通过的 checklist 项
  - [x] SubTask 10.1: 清理 WeChatView.vue 等页面中的硬编码颜色，迁移到 CSS 变量（优先 #fff、渐变背景、#0c2340 等显著硬编码）
  - [x] SubTask 10.2: 实现 Tauri 窗口重启后恢复上次页面/路由（在关闭时持久化当前路由，启动时读取并跳转）
  - [x] SubTask 10.3: 完善模型错误通知：为 modelStore.loadModels 错误通知添加重试入口；监听后端模型断开/同步错误事件并生成带重试 action 的通知
  - [x] SubTask 10.4: 更新 README.md，补充设计系统、主题切换、Dashboard、命令面板、生成艺术、通知中心等新功能说明
