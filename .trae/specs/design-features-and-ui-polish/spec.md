# 功能扩展与 UI 美化方案 Spec

## Why

当前应用已具备 AI 聊天、角色管理、模型中心、微信通道、TTS 等核心能力，但缺少统一的视觉语言与高频桌面效率入口。为了在保持功能稳定的前提下提升产品质感与日常可用性，需要制定一套聚焦“视觉一致性 + 桌面效率 + 生成艺术氛围”的增量方案。

## What Changes

- **设计系统（Design System）**：建立一套可复用的颜色、字体、间距、圆角、阴影与组件规范，统一前端所有页面。
- **聊天界面美化**：优化消息气泡、Markdown 渲染、代码高亮、引用样式、加载动画与空状态。
- **首页仪表盘（Dashboard）**：新增启动后的主入口，聚合最近会话、快捷角色切换、模型状态与未读消息。
- **全局搜索与命令面板**：`Ctrl/Cmd + K` 唤起，支持跳转会话、角色、模型、设置与动作命令。
- **生成艺术氛围背景**：在登录/启动页、角色封面与加载状态引入 p5.js 算法生成背景，呼应“算法艺术”理念。
- **桌面端体验优化**：窗口状态恢复、托盘菜单、原生右键菜单、文件拖拽发送与快捷键注册。
- **主题切换**：支持浅色/深色/跟随系统，并以 CSS 变量驱动。
- **通知中心**：集中展示消息、同步错误与系统事件，避免弹窗打断。

## Impact

- 受影响页面：`src/views/*`、`src/components/*`、`src/App.vue`。
- 受影响后端：`src-tauri/src/lib.rs`（快捷键、托盘）、`src-tauri/src/commands/*`（搜索接口、窗口状态）。
- 新增产物：`src/styles/design-system.css`、算法艺术独立 HTML 组件、仪表盘组件。
- 受影响能力：微信消息事件、Tauri 窗口事件、角色/模型 store。

## ADDED Requirements

### Requirement: 设计系统

The system SHALL 提供一套 CSS 变量与基础组件规范，使所有页面在颜色、字体、间距、圆角、阴影上保持一致。

#### Scenario: 主题切换
- **WHEN** 用户在设置页切换浅色/深色/跟随系统
- **THEN** 全站颜色立即响应，且重启后保持选择

#### Scenario: 组件复用
- **WHEN** 开发者新增页面
- **THEN** 可直接使用现有 Button/Input/Card/Modal/Toast 样式而无需重写

### Requirement: 聊天界面美化

The system SHALL 优化聊天界面可读性与视觉层次，使长时间对话更舒适。

#### Scenario: Markdown 消息
- **WHEN** AI 返回 Markdown 内容
- **THEN** 正确渲染标题、列表、链接、代码块、引用与表格，并带语法高亮

#### Scenario: 代码块
- **WHEN** 消息包含代码
- **THEN** 显示语言标签、复制按钮与行号（可选）

#### Scenario: 加载与空状态
- **WHEN** AI 正在生成回复
- **THEN** 显示优雅的打字机动画而非静态 loading

### Requirement: 首页仪表盘

The system SHALL 在启动后展示仪表盘，聚合最常用的信息与入口。

#### Scenario: 进入应用
- **WHEN** 应用启动或点击首页
- **THEN** 显示最近会话、快捷角色、在线模型状态、微信未读与快捷动作

#### Scenario: 快捷切换
- **WHEN** 用户点击仪表盘上的角色卡片
- **THEN** 直接进入与该角色的新聊天

### Requirement: 全局搜索与命令面板

The system SHALL 支持 `Ctrl/Cmd + K` 唤起命令面板，用于快速导航与操作。

#### Scenario: 搜索导航
- **WHEN** 用户输入关键词
- **THEN** 实时匹配会话、角色、模型、知识与设置项，回车跳转

#### Scenario: 快捷命令
- **WHEN** 用户输入 `/` 或选择命令
- **THEN** 可执行“新建聊天”“切换主题”“打开设置”等动作

### Requirement: 生成艺术氛围背景

The system SHALL 在启动页、角色封面与加载状态使用算法生成的动态背景，增强科技感与品牌识别。

#### Scenario: 启动页
- **WHEN** 应用启动
- **THEN** 显示基于种子的 p5.js 生成艺术背景，每次启动略有不同但可复现

#### Scenario: 角色封面
- **WHEN** 用户查看角色详情或编辑角色
- **THEN** 角色卡片展示与其气质匹配的生成式封面

### Requirement: 桌面端体验优化

The system SHALL 提供原生桌面应用应有的窗口、托盘与交互能力。

#### Scenario: 窗口恢复
- **WHEN** 用户关闭窗口后重新打开
- **THEN** 恢复上次的位置、尺寸与所在页面

#### Scenario: 托盘菜单
- **WHEN** 用户点击系统托盘图标
- **THEN** 显示“显示主窗口”“新建聊天”“退出”等菜单

#### Scenario: 文件拖拽
- **WHEN** 用户拖拽文件到聊天输入区
- **THEN** 提示上传/发送（先支持图片与文本文件）

### Requirement: 通知中心

The system SHALL 在界面右上角提供通知中心入口，集中展示事件。

#### Scenario: 收到微信消息
- **WHEN** 收到微信消息且应用非当前会话
- **THEN** 通知中心增加未读条目，可选桌面通知

#### Scenario: 同步错误
- **WHEN** 微信同步失败或模型连接断开
- **THEN** 通知中心显示错误卡片，提供重试入口

## MODIFIED Requirements

### Requirement: 设置页

The system SHALL 在现有设置页中新增“外观”分组，包含主题、字体大小、消息密度与生成艺术开关。

- **WHEN** 用户修改外观设置
- **THEN** 设置即时生效并持久化到本地存储

## REMOVED Requirements

无删除需求。本方案为增量优化，不废弃现有功能。
