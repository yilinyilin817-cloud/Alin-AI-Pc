//! iLink（智联）协议客户端 —— 微信官方 ClawBot API
//!
//! 参考：腾讯微信 ClawBot 插件（@tencent-weixin/openclaw-weixin）协议规范
//!
//! 网关：`https://ilinkai.weixin.qq.com`
//! 媒体 CDN：`https://novac2c.cdn.weixin.qq.com`（AES-128-ECB 加密）
//!
//! 端点（统一前缀 `/ilink/bot/`）：
//! - GET  /ilink/bot/get_bot_qrcode       申请登录二维码
//! - GET  /ilink/bot/get_qrcode_status    长轮询扫码状态
//! - POST /ilink/bot/getupdates           长轮询拉取新消息
//! - POST /ilink/bot/sendmessage          发送消息（必须带 context_token）
//! - POST /ilink/bot/getuploadurl         媒体上传凭证
//! - POST /ilink/bot/sendtyping           发送"正在输入"状态
//!
//! 统一请求头：
//!   Content-Type:        application/json
//!   AuthorizationType:   ilink_bot_token
//!   X-WECHAT-UIN:        base64(randomUint32)   每次变化防重放
//!   Authorization:       Bearer <bot_token>

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub const GATEWAY_DEFAULT: &str = "https://ilinkai.weixin.qq.com";
pub const CDN_DEFAULT: &str = "https://novac2c.cdn.weixin.qq.com";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QrCodeStatus {
    New,
    Scanning,
    Confirmed,
    Expired,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeInfo {
    pub qrcode_url: String,
    pub qrcode_key: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub bot_token: String,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodePollResult {
    pub status: QrCodeStatus,
    pub login: Option<LoginResult>,
    pub message: Option<String>,
}

/// 消息类型（type 字段）
/// 1 文本 / 2 图片 / 3 语音（silk，含 ASR 文本）/ 4 文件 / 5 视频
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MsgType {
    Text,
    Image,
    Voice,
    File,
    Video,
    System,
    Unknown,
}

impl MsgType {
    pub fn from_int(v: i32) -> Self {
        match v {
            1 => MsgType::Text,
            2 => MsgType::Image,
            3 => MsgType::Voice,
            4 => MsgType::File,
            5 => MsgType::Video,
            10000 | 10002 => MsgType::System,
            _ => MsgType::Unknown,
        }
    }
}

/// item_list 中的单项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingItem {
    pub r#type: i32,
    pub text_item: Option<TextItem>,
    pub media_item: Option<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextItem {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub media_url: Option<String>,
    pub aes_key: Option<String>,
    pub thumb_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub remote_msg_id: String,
    pub msg_type: MsgType,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub sender_id: Option<String>,
    pub sender_name: Option<String>,
    pub peer_id: String,
    pub peer_type: String, // user / room
    pub peer_name: Option<String>,
    pub context_token: Option<String>,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUpdatesResult {
    pub messages: Vec<IncomingMessage>,
    pub next_buf: String,
    pub has_more: bool,
}

#[derive(Clone)]
pub struct ILinkClient {
    pub gateway: String,
    pub cdn: String,
    pub client: Client,
    pub bot_token: Option<String>,
}

impl ILinkClient {
    pub fn new(bot_token: Option<&str>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            gateway: GATEWAY_DEFAULT.to_string(),
            cdn: CDN_DEFAULT.to_string(),
            client,
            bot_token: bot_token.map(|s| s.to_string()),
        }
    }

    pub fn with_endpoints(mut self, gateway: &str, cdn: &str) -> Self {
        self.gateway = gateway.to_string();
        self.cdn = cdn.to_string();
        self
    }

    /// 按 wechatbot.dev 协议生成 X-WECHAT-UIN:
    ///   随机 4 字节 → u32 (小端) → 十进制字符串 → base64
    /// 每次请求重新生成，用于防重放。
    fn random_uin_b64() -> String {
        let mut buf = [0u8; 4];
        rand::thread_rng().fill_bytes(&mut buf);
        let n = u32::from_le_bytes(buf);
        B64.encode(n.to_string().as_bytes())
    }

    /// 基础请求头（未带 Authorization，因为登录前没有 bot_token）
    fn base_headers() -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("Content-Type", "application/json".parse().unwrap());
        h.insert("AuthorizationType", "ilink_bot_token".parse().unwrap());
        h.insert("X-WECHAT-UIN", Self::random_uin_b64().parse().unwrap());
        h
    }

    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut h = Self::base_headers();
        if let Some(token) = &self.bot_token {
            h.insert(
                "Authorization",
                format!("Bearer {token}").parse().unwrap(),
            );
        }
        h
    }

    /// 1. 申请登录二维码
    /// GET /ilink/bot/get_bot_qrcode?bot_type=3
    /// 响应（成功）：{ ret: 0, data: { qrcode, qrcode_img_content, expires_in, ... } }
    /// 响应（失败）：{ ret: 1, err_msg: "missing bot_type" }
    /// 注意：服务端把所有响应的 Content-Type 都标成 application/octet-stream，
    ///       不能依赖 reqwest::Response::json()，必须按文本读取后手动 JSON 解析。
    pub async fn get_bot_qrcode(&self) -> Result<QrCodeInfo, String> {
        let url = format!("{}/ilink/bot/get_bot_qrcode", self.gateway);
        let resp = self
            .client
            .get(&url)
            .headers(Self::base_headers())
            .query(&[("bot_type", "3")])
            .send()
            .await
            .map_err(|e| format!("请求二维码失败: {e}"))?;

        let raw = parse_json_lenient(resp, "get_bot_qrcode").await?;

        // 错误响应：ret != 0 + err_msg
        if let Some(ret) = raw.get("ret").and_then(|v| v.as_i64()) {
            if ret != 0 {
                let err = raw
                    .get("err_msg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no err_msg)");
                return Err(format!("get_bot_qrcode 业务错误: ret={ret} err_msg={err}"));
            }
        }

        // 真实响应字段直接在顶级（不在 data 包装里）
        //   qrcode             → 32 字符 hex 的 token
        //   qrcode_img_content → liteapp.weixin.qq.com 上的二维码图片 URL
        let qrcode_url = raw
            .get("qrcode_img_content")
            .or_else(|| raw.get("qrcode_url"))
            .or_else(|| raw.get("qrcode_img"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                format!(
                    "二维码 URL 缺失：响应中未找到 qrcode_img_content/qrcode_url 字段。raw={raw}"
                )
            })?
            .to_string();

        let qrcode_key = raw
            .get("qrcode")
            .or_else(|| raw.get("qrcode_key"))
            .or_else(|| raw.get("qrcodeKey"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                format!(
                    "qrcode 缺失：响应中未找到 qrcode/qrcode_key 字段。raw={raw}"
                )
            })?
            .to_string();

        let expires_in = raw
            .get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(300);

        Ok(QrCodeInfo {
            qrcode_url,
            qrcode_key,
            expires_in,
        })
    }

    /// 2. 长轮询二维码状态
    /// GET /ilink/bot/get_qrcode_status?qrcode=...&timeout=25
    /// 响应：{ ret, status, qrcode_token, credentials?, baseurl?, err_msg? }
    /// 状态值：wait / scaned / confirmed / expired
    /// 必须带 `iLink-App-ClientVersion: 1` 头
    pub async fn poll_qrcode_status(
        &self,
        qrcode_key: &str,
    ) -> Result<QrCodePollResult, String> {
        let url = format!("{}/ilink/bot/get_qrcode_status", self.gateway);
        let mut headers = Self::base_headers();
        headers.insert("iLink-App-ClientVersion", "1".parse().unwrap());
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .query(&[("qrcode", qrcode_key), ("timeout", "25")])
            .send()
            .await
            .map_err(|e| format!("轮询二维码失败: {e}"))?;

        let raw = parse_json_lenient(resp, "get_qrcode_status").await?;

        // 业务错误：ret != 0
        if let Some(ret) = raw.get("ret").and_then(|v| v.as_i64()) {
            if ret != 0 {
                let err = raw
                    .get("err_msg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no err_msg)");
                return Err(format!("get_qrcode_status 业务错误: ret={ret} err_msg={err}"));
            }
        }

        // 状态字符串在顶级
        let status_str = raw
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("wait")
            .to_string();
        let status = match status_str.as_str() {
            "scaned" | "scanning" | "scanned" => QrCodeStatus::Scanning,
            "confirmed" | "ok" | "success" | "logged_in" => QrCodeStatus::Confirmed,
            "expired" => QrCodeStatus::Expired,
            "canceled" | "cancelled" => QrCodeStatus::Canceled,
            _ => QrCodeStatus::New, // wait / new
        };

        let mut login: Option<LoginResult> = None;
        if matches!(status, QrCodeStatus::Confirmed) {
            // 凭据在 credentials 子对象里
            let creds = raw.get("credentials");
            let bot_token = creds
                .and_then(|c| c.get("bot_token").or_else(|| c.get("token")))
                .or_else(|| raw.get("bot_token"))
                .and_then(|v| v.as_str())
                .ok_or("confirmed 但缺少 bot_token")?
                .to_string();
            let user_id = creds
                .and_then(|c| {
                    c.get("ilink_user_id")
                        .or_else(|| c.get("user_id"))
                        .or_else(|| c.get("userid"))
                        .or_else(|| c.get("wxid"))
                })
                .or_else(|| raw.get("ilink_user_id"))
                .or_else(|| raw.get("user_id"))
                .and_then(|v| v.as_str())
                .ok_or("confirmed 但缺少 ilink_user_id")?
                .to_string();
            let ilink_bot_id = creds
                .and_then(|c| c.get("ilink_bot_id"))
                .or_else(|| raw.get("ilink_bot_id"))
                .or_else(|| raw.get("bot_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let nickname = raw
                .get("nickname")
                .or_else(|| raw.get("nick_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let avatar_url = raw
                .get("avatar_url")
                .or_else(|| raw.get("head_img"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            // baseurl 可能覆盖默认网关
            let _ = ilink_bot_id; // 暂不写入 LoginResult，等扩展类型
            login = Some(LoginResult {
                bot_token,
                user_id,
                nickname,
                avatar_url,
            });
        }

        let message = raw
            .get("err_msg")
            .or_else(|| raw.get("message"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(QrCodePollResult {
            status,
            login,
            message,
        })
    }

    /// 3. 长轮询拉取新消息
    /// POST /ilink/bot/getupdates
    /// Body: { "get_updates_buf": "<cursor>", "base_info": { "channel_version": "1.0.0" } }
    /// 响应：{ ret, msgs: [...], get_updates_buf: "<next_cursor>", has_more, err_msg? }
    pub async fn get_updates(
        &self,
        buf: &str,
        timeout_secs: u64,
    ) -> Result<GetUpdatesResult, String> {
        if self.bot_token.is_none() {
            return Err("未登录：缺少 bot_token".into());
        }
        let url = format!("{}/ilink/bot/getupdates", self.gateway);
        let body = serde_json::json!({
            "get_updates_buf": buf,
            "base_info": { "channel_version": "1.0.0" }
        });

        // 统一日志：仅在 debug 级别输出
        log::debug!(
            "[ilink] getupdates REQUEST: url={url} body={}",
            serde_json::to_string(&body).unwrap_or_default()
        );
        let auth_h = self.auth_headers();
        log::debug!(
            "[ilink] getupdates HEADERS: AuthorizationType={:?} X-WECHAT-UIN={:?} Authorization={:?}",
            auth_h.get("AuthorizationType").and_then(|v| v.to_str().ok()),
            auth_h.get("X-WECHAT-UIN").and_then(|v| v.to_str().ok()),
            auth_h.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| if s.len() > 30 { format!("{}...({}bytes)", &s[..20], s.len()) } else { s.to_string() }),
        );

        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .timeout(Duration::from_secs(timeout_secs + 5))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("拉取更新失败: {e}"))?;

        let raw = parse_json_lenient(resp, "getupdates").await?;

        // 业务错误：
        //   ret: 0  = 成功，可能有 msgs
        //   ret: -1 = 无新消息（正常状态，幂等）
        //   ret: 其他 = 真错误
        //   errcode: -14 = session 过期
        //   errcode: -1  = 同 ret: -1
        if let Some(ret) = raw.get("ret").and_then(|v| v.as_i64()) {
            match ret {
                0 => { /* 继续走正常解析 */ }
                -1 => {
                    // 暂无新消息，正常状态
                    log::debug!("getupdates: ret=-1 (无新消息)，下一轮");
                    return Ok(GetUpdatesResult {
                        messages: vec![],
                        next_buf: buf.to_string(),
                        has_more: false,
                    });
                }
                _ => {
                    let err = raw
                        .get("err_msg")
                        .or_else(|| raw.get("errmsg"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("(no err_msg)");
                    return Err(format!("getupdates 业务错误: ret={ret} err_msg={err}"));
                }
            }
        }
        // 兼容 errcode 字段
        if raw.get("errcode").and_then(|v| v.as_i64()) == Some(-14) {
            return Err("session timeout, 需重新登录".into());
        }

        let next_buf = raw
            .get("get_updates_buf")
            .or_else(|| raw.get("next_buf"))
            .or_else(|| raw.get("sync_buf"))
            .and_then(|v| v.as_str())
            .unwrap_or(buf)
            .to_string();

        let has_more = raw
            .get("has_more")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut messages: Vec<IncomingMessage> = Vec::new();
        // 兼容多种数组位置：msgs / messages / updates / msg_list
        if let Some(arr) = raw
            .get("msgs")
            .or_else(|| raw.get("messages"))
            .or_else(|| raw.get("updates"))
            .or_else(|| raw.get("msg_list"))
            .and_then(|v| v.as_array())
        {
            for item in arr {
                if let Some(parsed) = parse_incoming_message(item) {
                    messages.push(parsed);
                }
            }
        }

        Ok(GetUpdatesResult {
            messages,
            next_buf,
            has_more,
        })
    }

    /// 4. 发送消息
    /// POST /ilink/bot/sendmessage
    /// 真实协议（OpenClaw 官方插件反编译）必须包含：
    ///   - from_user_id = ""   空字符串
    ///   - client_id = <UUID>  每条消息唯一 ID（去重/路由）
    ///   - message_type = 2    BOT 消息
    ///   - message_state = 2   FINISH 完成态
    ///   - base_info.channel_version = "1.0.0"
    /// 缺一个服务端就 200 + 空响应{} 静默丢弃
    pub async fn send_message(
        &self,
        to_user_id: &str,
        context_token: &str,
        text: &str,
        message_state: i32,
    ) -> Result<String, String> {
        let client_id = format!("bot-{}", Uuid::new_v4().simple());
        self.send_message_inner(to_user_id, context_token, text, message_state, &client_id).await
    }

    /// 流式发送：使用固定 client_id 支持增量更新
    /// - 第一次调用 message_state=0（新建）
    /// - 中间调用 message_state=1（更新中）
    /// - 最后一次调用 message_state=2（完成）
    pub async fn send_message_stream(
        &self,
        to_user_id: &str,
        context_token: &str,
        text: &str,
        message_state: i32,
        client_id: &str,
    ) -> Result<String, String> {
        self.send_message_inner(to_user_id, context_token, text, message_state, client_id).await
    }

    async fn send_message_inner(
        &self,
        to_user_id: &str,
        context_token: &str,
        text: &str,
        message_state: i32,
        client_id: &str,
    ) -> Result<String, String> {
        if self.bot_token.is_none() {
            return Err("未登录：缺少 bot_token".into());
        }
        let url = format!("{}/ilink/bot/sendmessage", self.gateway);
        let body = serde_json::json!({
            "msg": {
                "from_user_id": "",
                "to_user_id": to_user_id,
                "client_id": client_id,
                "message_type": 2, // BOT 消息
                "message_state": message_state, // 0=新建 1=生成中 2=完成
                "context_token": context_token,
                "item_list": [
                    { "type": 1, "text_item": { "text": text } }
                ]
            },
            "base_info": { "channel_version": "1.0.0" }
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("发送失败: {e}"))?;

        let raw = parse_json_lenient(resp, "sendmessage").await?;
        let msg_id = raw
            .get("msg_id")
            .or_else(|| raw.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(msg_id)
    }

    /// 5. 申请媒体上传凭证
    /// POST /ilink/bot/getuploadurl
    /// SDK 真实请求体字段（@tencent-weixin/openclaw-weixin 反编译）：
    ///   filekey          文件标识
    ///   media_type       1=IMAGE 2=VIDEO 3=FILE
    ///   to_user_id       目标用户 ID（必填）
    ///   rawsize          原文件明文大小
    ///   rawfilemd5       原文件明文 MD5
    ///   filesize         AES-128-ECB 加密后的密文大小
    ///   thumb_rawsize    缩略图明文大小（IMAGE/VIDEO 必填）
    ///   thumb_rawfilemd5 缩略图明文 MD5
    ///   thumb_filesize   缩略图密文大小
    /// 响应：{ upload_param, thumb_upload_param }（给 CDN 上传用的加密 query）
    #[allow(clippy::too_many_arguments)]
    pub async fn get_upload_url(
        &self,
        filekey: &str,
        media_type: i32,
        to_user_id: &str,
        rawsize: i64,
        rawfilemd5: &str,
        filesize: i64,
        thumb_rawsize: Option<i64>,
        thumb_rawfilemd5: Option<&str>,
        thumb_filesize: Option<i64>,
    ) -> Result<serde_json::Value, String> {
        if self.bot_token.is_none() {
            return Err("未登录：缺少 bot_token".into());
        }
        let url = format!("{}/ilink/bot/getuploadurl", self.gateway);
        let mut body = serde_json::json!({
            "filekey": filekey,
            "media_type": media_type,
            "to_user_id": to_user_id,
            "rawsize": rawsize,
            "rawfilemd5": rawfilemd5,
            "filesize": filesize,
        });
        if let Some(s) = thumb_rawsize {
            body["thumb_rawsize"] = serde_json::json!(s);
        }
        if let Some(m) = thumb_rawfilemd5 {
            body["thumb_rawfilemd5"] = serde_json::json!(m);
        }
        if let Some(s) = thumb_filesize {
            body["thumb_filesize"] = serde_json::json!(s);
        }
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("申请上传凭证失败: {e}"))?;
        parse_json_lenient(resp, "getuploadurl").await
    }

    /// 6. 获取账号配置（typing_ticket 等）
    /// POST /ilink/bot/getconfig
    /// 请求体：{ ilink_user_id, context_token? }
    /// 响应：{ ret, typing_ticket, ... }
    /// 备注：typing_ticket 有效期短（约 5 秒），调用 sendtyping 前必须先调这个刷新。
    pub async fn get_config(
        &self,
        ilink_user_id: &str,
        context_token: Option<&str>,
    ) -> Result<String, String> {
        if self.bot_token.is_none() {
            return Err("未登录：缺少 bot_token".into());
        }
        let url = format!("{}/ilink/bot/getconfig", self.gateway);
        let mut body = serde_json::json!({ "ilink_user_id": ilink_user_id });
        if let Some(ct) = context_token {
            body["context_token"] = serde_json::json!(ct);
        }
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("getconfig 失败: {e}"))?;
        let raw = parse_json_lenient(resp, "getconfig").await?;
        if let Some(ret) = raw.get("ret").and_then(|v| v.as_i64()) {
            if ret != 0 {
                let err = raw
                    .get("err_msg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no err_msg)");
                return Err(format!("getconfig 业务错误: ret={ret} err_msg={err}"));
            }
        }
        raw.get("typing_ticket")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                format!(
                    "getconfig 响应中缺少 typing_ticket 字段。raw={raw}"
                )
            })
    }

    /// 7. 发送"正在输入"状态
    /// POST /ilink/bot/sendtyping
    /// SDK 真实请求体：{ ilink_user_id, typing_ticket, status }
    ///   status = 1 → 正在输入
    ///   status = 2 → 取消输入
    /// typing_ticket 必须先调 getconfig 拿，过期会失效。
    pub async fn send_typing(
        &self,
        ilink_user_id: &str,
        typing_ticket: &str,
        status: i32,
    ) -> Result<(), String> {
        if self.bot_token.is_none() {
            return Err("未登录：缺少 bot_token".into());
        }
        let url = format!("{}/ilink/bot/sendtyping", self.gateway);
        let body = serde_json::json!({
            "ilink_user_id": ilink_user_id,
            "typing_ticket": typing_ticket,
            "status": status,
        });
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("sendtyping 失败: {e}"))?;
        // 用 helper 解析（容错 Content-Type）即便我们只关心成功
        let _ = parse_json_lenient(resp, "sendtyping").await?;
        Ok(())
    }

    /// 媒体下载：先从消息 item 拿到 aes_key，再拉取加密媒体并解密
    pub async fn download_media(
        &self,
        cdn_url: &str,
        aes_key_b64: &str,
    ) -> Result<Vec<u8>, String> {
        let key_bytes = B64
            .decode(aes_key_b64)
            .map_err(|e| format!("aes_key base64 解码失败: {e}"))?;
        if key_bytes.len() != 16 {
            return Err(format!("AES-128 key 长度异常: {}", key_bytes.len()));
        }

        let cdn_resp = self
            .client
            .get(cdn_url)
            .send()
            .await
            .map_err(|e| format!("CDN 拉取失败: {e}"))?;
        if !cdn_resp.status().is_success() {
            return Err(format!("CDN 错误: {}", cdn_resp.status()));
        }
        let encrypted = cdn_resp
            .bytes()
            .await
            .map_err(|e| format!("读取 CDN 响应失败: {e}"))?
            .to_vec();

        use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyInit};
        type Aes128Ecb = ecb::Decryptor<aes::Aes128>;
        let cipher = Aes128Ecb::new_from_slice(&key_bytes)
            .map_err(|e| format!("AES 初始化失败: {e}"))?;
        let mut buf = encrypted;
        let decrypted = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .map_err(|e| format!("AES 解密失败: {e}"))?;
        Ok(decrypted.to_vec())
    }
}

/// 服务端把 JSON 响应的 Content-Type 标成 `application/octet-stream`，
/// 强制忽略 Content-Type 解析为 JSON，并在失败时把原始 body 写到日志。
/// 错误响应：HTTP 非 2xx → 直接返回带 body 的错误；
/// 业务错误（ret != 0）由调用方根据 `ret` / `err_msg` 自行判断。
/// 空 body 或 `{}`（sendMessage 成功）当成功解析为 {ret: 0}。
async fn parse_json_lenient(
    resp: reqwest::Response,
    op: &str,
) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("{op}：读取响应体失败: {e}"))?;
    let preview: String = body.chars().take(512).collect();
    log::debug!("[ilink] {op} status={status} ct={ct} body[:512]={preview}");

    if !status.is_success() {
        return Err(format!(
            "{op}：HTTP {status} ct={ct} body={}",
            body.chars().take(256).collect::<String>()
        ));
    }
    // 空 body 或 `{}` 当成 sendMessage 成功响应
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Ok(serde_json::json!({"ret": 0}));
    }
    serde_json::from_str(&body).map_err(|e| {
        format!(
            "{op}：JSON 解析失败: {e} ct={ct} body[:256]={}",
            body.chars().take(256).collect::<String>()
        )
    })
}

fn parse_incoming_message(item: &serde_json::Value) -> Option<IncomingMessage> {
    // 1) msg_id 可能是字符串或数字
    let remote_msg_id: String = {
        let raw = item
            .get("msg_id")
            .or_else(|| item.get("new_msg_id"))
            .or_else(|| item.get("message_id"))
            .or_else(|| item.get("svr_id"))
            .or_else(|| item.get("id"));
        match raw {
            Some(v) if v.is_string() => v.as_str().unwrap().to_string(),
            Some(v) if v.is_i64() => v.as_i64().unwrap().to_string(),
            Some(v) if v.is_u64() => v.as_u64().unwrap().to_string(),
            _ => String::new(),
        }
    };
    if remote_msg_id.is_empty() {
        log::debug!("[ilink] parse_incoming_message: missing msg_id, raw={item}");
        return None;
    }

    let msg_type_int = item
        .get("message_type")
        .or_else(|| item.get("msg_type"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;
    let msg_type = MsgType::from_int(msg_type_int);

    let sender_id = item
        .get("from_user_id")
        .or_else(|| item.get("sender_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 判断消息类型：群聊 vs 私聊
    let is_room = item.get("room_id").is_some()
        || item.get("chat_type").and_then(|v| v.as_str()) == Some("room");
    let peer_type = if is_room { "room" } else { "user" };

    // peer_id: 群消息用 room_id；私聊用 from_user_id（对方 wxid）
    let peer_id = if is_room {
        item.get("room_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        item.get("from_user_id")
            .or_else(|| item.get("peer_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    .or_else(|| sender_id.clone())
    .unwrap_or_default();

    let peer_name = if is_room {
        // 群聊：peer_name 应该是群名
        item.get("room_name")
            .or_else(|| item.get("room_nickname"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        item.get("from_user_name")
            .or_else(|| item.get("sender_name"))
            .or_else(|| item.get("peer_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    // sender_name: 群聊时是发言者个体名；私聊时就是对方
    let sender_name = if is_room {
        item.get("from_user_name")
            .or_else(|| item.get("sender_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        peer_name.clone()
    };

    let context_token = item
        .get("context_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 解析 item_list
    let mut content: Option<String> = None;
    let mut media_url: Option<String> = None;
    if let Some(items) = item.get("item_list").and_then(|v| v.as_array()) {
        for it in items {
            let ty = it.get("type").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if ty == 1 {
                if let Some(t) = it.get("text_item").and_then(|t| t.get("text")).and_then(|v| v.as_str()) {
                    content = Some(t.to_string());
                }
            } else if (2..=5).contains(&ty) {
                if let Some(m) = it.get("media_item") {
                    if media_url.is_none() {
                        media_url = m
                            .get("media_url")
                            .or_else(|| m.get("cdn_url"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }
    }

    let received_at = item
        .get("create_time")
        .or_else(|| item.get("timestamp"))
        .and_then(|v| v.as_i64())
        .map(|t| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(t, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    Some(IncomingMessage {
        remote_msg_id,
        msg_type,
        content,
        media_url,
        sender_id,
            sender_name,
        peer_id,
        peer_type: peer_type.to_string(),
        peer_name,
        context_token,
        received_at,
    })
}
