import type { InstalledPlugin } from "@/types/plugin";
import { isTauri } from "./env";
import { mockPlugins } from "@/mocks/data";

export async function listPlugins(): Promise<InstalledPlugin[]> {
  if (!isTauri()) return structuredClone(mockPlugins);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<InstalledPlugin[]>("list_plugins");
}

export async function getPlugin(id: string): Promise<InstalledPlugin | null> {
  if (!isTauri()) {
    return structuredClone(mockPlugins.find((p) => p.id === id) ?? null);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<InstalledPlugin | null>("get_plugin", { id });
}

export async function installPlugin(sourcePath: string): Promise<InstalledPlugin> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<InstalledPlugin>("install_plugin", { sourcePath });
}

export async function uninstallPlugin(id: string): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("uninstall_plugin", { id });
}

export async function enablePlugin(id: string, enabled: boolean): Promise<InstalledPlugin> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<InstalledPlugin>("enable_plugin", { id, enabled });
}

export async function configurePlugin(
  id: string,
  config: Record<string, unknown>,
): Promise<InstalledPlugin> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<InstalledPlugin>("configure_plugin", { id, config });
}
