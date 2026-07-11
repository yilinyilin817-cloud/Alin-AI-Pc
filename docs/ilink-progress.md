# iLink 协议实现进度报告

> 目标:通过腾讯官方 iLink 协议(`ilinkai.weixin.qq.com`)实现微信 ClawBot 通道,
> 让本应用能够登录微信、接收消息、发送消息,作为 AI 伴侣多通道之一。
> 文档时间:2026-07-07

---

## 1. 协议基础

| 项目 | 值 |
|---|---|
| 网关 | `https://ilinkai.weixin.qq.com` |
| 媒体 CDN | `https://novac2c.cdn.weixin.qq.com` |
| 端点前缀 | `/ilink/bot/` |
| 加密 | 媒体 AES-128-ECB |
| 参考实现 | npm `@tencent-weixin/openclaw-weixin`、Rust SDK `qingchencloud/wxchat-sdk` |

### 1.1 通用请求头

```
Content-Type:        application/json
AuthorizationType:   ilink_bot_token
Authorization:       Bearer <bot_token>
X-WECHAT-UIN:        base64(uint32→十进制字符串)   每次请求重新生成
```

### 1.2 端点清单(全部已实现)

| # | 方法 | 路径 | 说明 |
|---|---|---|---|
| 1 | GET  | `/ilink/bot/get_bot_qrcode?bot_type=3` | 申请登录二维码 |
| 2 | GET  | `/ilink/bot/get_qrcode_status?qrcode=…&timeout=25` | 长轮询扫码状态 |
| 3 | POST | `/ilink/bot/getupdates` | 长轮询拉新消息 |
| 4 | POST | `/ilink/bot/sendmessage` | 发送消息 |
| 5 | POST | `/ilink/bot/getuploadurl` | 媒体上传凭证 |
| 6 | POST | `/ilink/bot/getconfig` | 拿 typing_ticket |
| 7 | POST | `/ilink/bot/sendtyping` | 发送"正在输入"状态 |
| 附 | GET  | CDN `novac2c.cdn.weixin.qq.com` | 拉加密媒体 + AES-128-ECB 解密 |

---

## 2. 文件清单

### 2.1 新增 / 修改文件

| 路径 | 行数 | 状态 | 用途 |
|---|---|---|---|
| `src-tauri/src/model_bus/ilink/client.rs` | ~870 | ✅ 完成 | iLink 客户端全部 7 个端点 + 媒体解密 |
| `src-tauri/src/model_bus/ilink/mod.rs` | 短 | ✅ 完成 | 模块导出 |
| `src-tauri/src/wechat/mod.rs` | ~100 | ✅ 完成 | WeChatManager:多账号、自动恢复 sync |
| `src-tauri/src/wechat/sync.rs` | ~300 | ✅ 完成 | 长轮询循环、消息入库、事件 emit |
| `src-tauri/src/commands/wechat.rs` | ~450 | ✅ 完成 | Tauri 命令:apply/send/start_sync 等 |
| `src-tauri/src/storage/schema.sql` | +3 张表 | ✅ 完成 | wechat_account / wechat_session / wechat_message / wechat_sync_state |
| `src-tauri/src/storage/repo.rs` | +200 行 | ✅ 完成 | wechat_* CRUD |
| `src-tauri/src/lib.rs` | 末尾 setup | ✅ 完成 | 启动时 spawn `start_all_online_accounts` |
| `src/views/WeChatView.vue` | 完整重写 | ✅ 完成 | 二维码渲染、扫码提示、消息列表、AI 回复触发 |

### 2.2 数据库表

```sql
CREATE TABLE wechat_account (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT,
    nickname            TEXT,
    avatar_url          TEXT,
    bot_token           TEXT,
    qrcode_key          TEXT,
    qrcode_url          TEXT,
    qrcode_status       TEXT DEFAULT 'idle',
    get_updates_buf     TEXT DEFAULT '',          -- 修复:旧默认 '0' 错误
    status              TEXT DEFAULT 'offline',
    last_error          TEXT,
    last_login_at       TEXT,
    last_sync_at        TEXT,
    created_at          TEXT DEFAULT (datetime('now')),
    updated_at          TEXT DEFAULT (datetime('now'))
);

CREATE TABLE wechat_session (
    id                  TEXT PRIMARY KEY,
    account_id          TEXT NOT NULL,
    peer_id             TEXT NOT NULL,
    peer_type           TEXT DEFAULT 'user',
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

CREATE TABLE wechat_message (
    id                  TEXT PRIMARY KEY,
    account_id          TEXT NOT NULL,
    session_id          TEXT NOT NULL,
    remote_msg_id       TEXT,
    direction           TEXT NOT NULL,            -- inbound / outbound
    msg_type            TEXT NOT NULL DEFAULT 'text',
    content             TEXT,
    media_url           TEXT,
    media_local_path    TEXT,
    sender_id           TEXT,
    sender_name         TEXT,
    context_token       TEXT,
    status              TEXT DEFAULT 'sent',
    error               TEXT,
    created_at          TEXT DEFAULT (datetime('now')),
    UNIQUE(account_id, remote_msg_id)
);

CREATE TABLE wechat_sync_state (
    account_id          TEXT PRIMARY KEY,
    get_updates_buf     TEXT NOT NULL,
    last_sync_at        TEXT,
    consecutive_errors  INTEGER DEFAULT 0
);
```

---

## 3. 实现细节(SDK 反编译验证后)

### 3.1 `get_bot_qrcode`

- 响应字段直接在顶级(不在 `data` 包装里):
  - `qrcode` → 32 字符 hex token
  - `qrcode_img_content` → `liteapp.weixin.qq.com` 引导页 URL
- 前端:用 `qrcode` npm 库在浏览器把 URL 渲染成 PNG dataURL
- fallback:`qrcode_url` / `qrcode_img` / `qrcode_key` 多种字段名兼容

### 3.2 `get_qrcode_status`

- 必须带 `iLink-App-ClientVersion: 1` 头
- 长轮询 25 秒超时
- 状态字符串:`wait / scaned / confirmed / expired`
- 凭据在 `credentials.{bot_token, ilink_user_id}` 子对象

### 3.3 `getupdates`(长轮询拉消息)

- Body:`{ "get_updates_buf": "<cursor>", "base_info": { "channel_version": "1.0.0" } }`
- **关键发现**:`channel_version` 真实值是 `"1.0.0"`,来自 `qingchencloud/wxchat-sdk` 的 `SDK_VERSION` 常量
  - 之前误信反编译文章的 "1.0.3" → 改成 1.0.0 后服务端才正常路由
- 业务错误码:
  - `ret: 0` → 成功
  - `ret: -1` → **无新消息**(正常状态,幂等)
  - `ret: 其他` → 真错误
  - `errcode: -14` → session 过期,清 token 跳回登录
- 响应数组字段名兼容:`msgs / messages / updates / msg_list`

### 3.4 `sendmessage`(发消息)

- 4 个"幽灵字段"缺一不可(缺一服务端就 200 + 空 body 静默丢弃):
  ```json
  "msg": {
    "from_user_id": "",          // 空字符串
    "to_user_id": "<peer>",
    "client_id": "bot-<uuid>",   // 每条消息唯一
    "message_type": 2,           // BOT
    "message_state": 0|1|2,      // 0=新建 1=生成中 2=完成
    "context_token": "<base64>", // 24h 窗口 + 上下文
    "item_list": [{ "type": 1, "text_item": { "text": "..." } }]
  }
  ```
- 必须先有该 peer 的 inbound 消息(从而拿到 `context_token`)
- 成功响应是空 body 或 `{}`,我们的 `parse_json_lenient` 自动兼容

### 3.5 `getuploadurl`(媒体上传)

SDK 真实请求体字段(13 个):

```json
{
  "filekey": "<id>",
  "media_type": 1,            // 1=IMAGE 2=VIDEO 3=FILE
  "to_user_id": "<peer>",     // 必填
  "rawsize": 12345,           // 原文件明文大小
  "rawfilemd5": "<md5>",      // 原文件明文 MD5
  "filesize": 12352,          // AES 加密后密文大小
  "thumb_rawsize": 1024,      // 缩略图明文大小(IMAGE/VIDEO 必填)
  "thumb_rawfilemd5": "...",  // 缩略图明文 MD5
  "thumb_filesize": 1040      // 缩略图密文大小
}
```

返回 `{ upload_param, thumb_upload_param }`,用于 CDN PUT 上传加密 query。

### 3.6 `getconfig`(新实现)

- Body:`{ "ilink_user_id": "<bot_user>", "context_token"?: "..." }`
- 响应:`{ "ret": 0, "typing_ticket": "<base64>" }`
- typing_ticket 有效期约 5 秒,调 sendtyping 前必须先刷一次

### 3.7 `sendtyping`(签名修改)

- 旧错误签名:`(to_user_id, context_token)`
- 新正确签名:`(ilink_user_id, typing_ticket, status: 1|2)`
- status:1 = 正在输入,2 = 取消

### 3.8 媒体下载

```
CDN URL + encrypt_query_param → GET → AES-128-ECB(密钥 = media_item.aes_key) → PKCS#7 unpad
```

---

## 4. 错误处理与日志

### 4.1 `parse_json_lenient` helper

服务端把所有 JSON 响应的 `Content-Type` 标成 `application/octet-stream`,
不能依赖 `reqwest::Response::json()`,统一改用 `.text()` + `serde_json::from_str`。
空 body 或 `{}` 解析为 `{ret: 0}`(sendmessage 成功约定)。

### 4.2 关键日志

```
[ilink] {op} status=200 ct=application/octet-stream body[:512]=...
[ilink] getupdates REQUEST: url=... body=...
[ilink] getupdates HEADERS: AuthorizationType=... X-WECHAT-UIN=... Authorization=Bearer ...(N bytes)
```

### 4.3 sync_loop 退避

- 网络错误:2 → 4 → 8 → ... → 60 秒
- `errcode: -14`:session 过期,清 `bot_token`、emit `wechat-account { status: "offline" }`、break loop
- `ret: -1`:正常状态,500ms 后继续

---

## 5. 自动化恢复

应用启动时(`lib.rs` setup 末尾):

```rust
tauri::async_runtime::spawn(async move {
    tokio::time::sleep(Duration::from_millis(500)).await;
    let n = wechat_manager.start_all_online_accounts(db2, app2).await;
    log::info!("wechat 自动恢复完成: {n} 个账号");
});
```

`start_all_online_accounts` SQL 过滤:

```sql
SELECT id FROM wechat_account
WHERE status = 'online' AND bot_token IS NOT NULL AND bot_token != ''
```

> 注:之前误用 `has_bot_token = 1`,schema 没有该字段 → 改成 `bot_token IS NOT NULL AND != ''`

---

## 6. 前端实现(WeChatView.vue)

| 元素 | 说明 |
|---|---|
| 二维码渲染 | `qrcode` 库把 liteapp URL → dataURL → `<img src>` |
| 扫码状态机 | `idle → pending → scanned → confirmed` |
| 提示 | `isLoggedIn && state === 'idle'` 时显示「请在微信中启用 ClawBot 插件」 |
| 消息列表 | 左侧会话 + 右侧消息流(支持文本/图片/语音/视频) |
| AI 回复 | 收消息后通过 Tauri 命令 `chat_send` 调 LLM,流式回写 |
| `nickname` fallback | `account.nickname ?? (status === "online" ? "已登录(等待首条消息获取昵称)" : "未登录")` |

---

## 7. 调试里程碑(踩过的坑)

| # | 错误 | 根因 | 修复 |
|---|---|---|---|
| 1 | `duplicate column name: last_verified_at` | schema.sql `CREATE TABLE` 已有列,migration 又 `ALTER ADD` | 删除重复的 `ALTER` 段 |
| 2 | `404 Not Found: /get_bot_qrcode` | 路径错(wechatbot.dev 文档 `/get_bot_qrcode`,真实 `/ilink/bot/get_bot_qrcode`) | 加 `/ilink/bot/` 前缀 |
| 3 | `二维码 URL 缺失` | 用错字段名 `qrcode_url / qrcode_key` | 改用 `qrcode_img_content / qrcode` |
| 4 | `error decoding response body` | 服务端返回 `Content-Type: application/octet-stream` | 改用 `.text()` + 手动 JSON 解析 |
| 5 | 二维码图片不显示 | `qrcode_img_content` 是 `liteapp.weixin.qq.com` HTML 不是 PNG | 装 `qrcode` 库在浏览器渲染 URL |
| 6 | 持续 `ret: -1` | 把 `ret: -1` 当错误 → 退避,实际是"无新消息" | 改成正常状态,500ms 继续 |
| 7 | `channel_version` 不对 | 文章说 "1.0.3",SDK 源码是 "1.0.0" | 改 `"1.0.0"` |
| 8 | `X-WECHAT-UIN` 格式 | 误用 hex / uuid | 4 字节 → u32 → 十进制字符串 → base64 |
| 9 | `自动恢复完成: 0 个账号` | SQL `has_bot_token = 1`,但 schema 无此列 | 改 `bot_token IS NOT NULL AND != ''` |
| 10 | `sendmessage` 静默失败 | 缺 `from_user_id=""` / `client_id` / `message_type=2` / `message_state` | 加齐 4 字段 |
| 11 | `sendtyping` 一直无效 | 误用 `context_token`,SDK 用 `typing_ticket` | 调 `getconfig` 拿 ticket,再 sendtyping |
| 12 | `getuploadurl` 报缺字段 | 漏 `to_user_id` / `rawsize` / `rawfilemd5` / `filesize` / `thumb_*` | 补齐 SDK 真实字段 |
| 13 | `get_updates_buf` 初始值 | 旧默认 `"0"` | 改 `""` |

---

## 8. 待用户验证(下一步)

### 8.1 必做:重新扫码登录

老 `bot_token` 是用错误的 `channel_version` 注册的 session,服务端直接返 `ret: -1` 静默拒绝。
**所有 fix 都基于 `channel_version: "1.0.0"`,必须重新走一遍登录流程。**

操作:
1. 启动应用 → 微信页
2. 删除老账号 / 点「重新登录」
3. 申请新二维码 → **已启用 ClawBot 插件**的微信扫码
4. 等 `qrcode_status === 'confirmed'`
5. 观察终端 `getupdates` 应正常长轮询(不再持续 `ret: -1`)
6. 给自己发一条消息,验证 `sendmessage` 能正常发出

### 8.2 验证清单

- [ ] 扫码能拿到 token,状态进 `online`
- [ ] 终端 `getupdates` 周期正常,无 `ret: -1` 反复
- [ ] 收到测试消息后,前端 `wechat-message` 事件触发,左侧新增会话
- [ ] 在前端发消息,微信端能收到文本
- [ ] 图片消息上传 → `getuploadurl` 拿到凭证 → CDN PUT 加密文件 → `sendmessage` 带 `image_item` 发送

### 8.3 已知限制

- `getuploadurl` / 媒体上传流程**已实现 API**,但前端尚未集成 UI
- 多账号支持已实现,UI 暂只展示第一个
- `getConfig` + `sendtyping` 已就绪,前端未接"正在输入"指示

---

## 9. 协议参考来源

| 来源 | URL | 用途 |
|---|---|---|
| openilink 官方文档 | https://openilink.com/docs | 端点路径规范 |
| wechatbot.dev 协议 | https://www.wechatbot.dev/zh/protocol#qr-login | (路径有错,作辅) |
| CSDN 反编译文章 | https://blog.csdn.net/xmyfan/article/details/159617944 | 字段名、`getConfig` / `sendTyping` 签名 |
| 今日头条 PaiCLI | https://m.toutiao.com/group/7651428032766427699 | typing_ticket 5s 过期、消息引擎架构 |
| 头条 OpenClaw 解析 | https://m.toutiao.com/group/7620795128331928116 | Python 复刻思路 |
| Rust SDK `wxchat-sdk` | qingchencloud/wxchat-sdk (GitHub) | `SDK_VERSION = "1.0.0"` 真实值 |

---

## 10. 编译状态

✅ `cargo check` 通过(`Finished dev profile in 7.09s`,0 error,69 warning 全部是 unused historical code)
