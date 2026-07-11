import type { AppInfo } from "@/types";
import { isTauri } from "./env";
import { mockGetAppInfo } from "@/mocks/ipc";

export async function getAppInfo(): Promise<AppInfo> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AppInfo>("get_app_info");
  }
  return mockGetAppInfo();
}

export async function healthCheck(): Promise<string> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("health_check");
  }
  return "ok";
}
