import { invoke } from "@tauri-apps/api/core";
import type {
  WeChatAccountView,
  WeChatLoginStatus,
  WeChatMessage,
  WeChatQrCode,
  WeChatSession,
} from "@/types";

export async function listWeChatAccounts(): Promise<WeChatAccountView[]> {
  return invoke("list_wechat_accounts");
}

export async function getWeChatAccount(
  accountId?: string,
): Promise<WeChatAccountView> {
  return invoke("get_wechat_account", { accountId: accountId ?? null });
}

export async function wechatRequestQrcode(
  accountId?: string,
): Promise<WeChatQrCode> {
  return invoke("wechat_request_qrcode", { accountId: accountId ?? null });
}

export async function wechatPollLogin(
  accountId?: string,
): Promise<WeChatLoginStatus> {
  return invoke("wechat_poll_login", { accountId: accountId ?? null });
}

export async function wechatLogout(accountId?: string): Promise<void> {
  return invoke("wechat_logout", { accountId: accountId ?? null });
}

export async function wechatStartSync(accountId?: string): Promise<void> {
  return invoke("wechat_start_sync", { accountId: accountId ?? null });
}

export async function listWeChatSessions(
  accountId?: string,
): Promise<WeChatSession[]> {
  return invoke("list_wechat_sessions", { accountId: accountId ?? null });
}

export async function listWeChatMessages(
  sessionId: string,
  limit?: number,
): Promise<WeChatMessage[]> {
  return invoke("list_wechat_messages", { sessionId, limit: limit ?? null });
}

export async function markWeChatSessionRead(sessionId: string): Promise<void> {
  return invoke("mark_wechat_session_read", { sessionId });
}

export async function sendWeChatText(
  sessionId: string,
  text: string,
): Promise<WeChatMessage> {
  return invoke("send_wechat_text", { sessionId, text });
}

/// 绑定角色到微信账号
export async function setWechatPersona(accountId: string, personaId: string | null): Promise<void> {
  return invoke("set_wechat_persona", { accountId: accountId ?? null, personaId });
}

/// 查询微信账号绑定的角色
export async function getWechatPersona(accountId?: string): Promise<string | null> {
  return invoke("get_wechat_persona", { accountId: accountId ?? null });
}
