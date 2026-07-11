import { isTauri } from "./env";

export interface AppSettings {
  theme: "light" | "dark" | "system";
  language: string;
  networkEnabled: boolean;
  launchOnStartup: boolean;
  inputDevice: string;
  outputDevice: string;
  fontSize?: "small" | "default" | "large";
  messageDensity?: "compact" | "default" | "cozy";
  generativeArtEnabled?: boolean;
  generativeArtSeed?: number;
  voiceMessagePreferred?: boolean;
  modelsDir?: string;
  [key: string]: unknown;
}

export interface ModelDirInfo {
  path: string;
  totalSize: number;
  totalSizeFormatted: string;
  modelCount: number;
  models: Array<{
    name: string;
    size: number;
    sizeFormatted: string;
  }>;
}

export interface MigrationProgress {
  progress: number;
  currentModel?: string;
  completed?: number;
  total?: number;
  done?: boolean;
  success?: boolean;
  failed?: Array<{ name: string; error: string }>;
}

export async function loadSettings(): Promise<Partial<AppSettings>> {
  if (!isTauri()) return {};
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Partial<AppSettings>>("load_settings");
}

export async function saveSettings(settings: Partial<AppSettings>): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("save_settings", { settings });
}

export async function getModelsDirInfo(): Promise<ModelDirInfo> {
  if (!isTauri()) {
    return {
      path: "",
      totalSize: 0,
      totalSizeFormatted: "0 B",
      modelCount: 0,
      models: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ModelDirInfo>("get_models_dir_info");
}

export async function setModelsDir(path: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("set_models_dir", { path });
}

export async function migrateModels(
  oldPath: string,
  newPath: string,
  onProgress?: (progress: MigrationProgress) => void
): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");

  return new Promise<void>((resolve, reject) => {
    let unlisten: (() => void) | undefined;

    listen<MigrationProgress>("migration-progress", (event) => {
      const p = event.payload;
      if (onProgress) {
        onProgress(p);
      }
      if (p.done) {
        if (unlisten) unlisten();
        if (p.success) {
          resolve();
        } else {
          reject(new Error(p.failed?.map(f => `${f.name}: ${f.error}`).join('\n') || "迁移失败"));
        }
      }
    }).then((unsub) => {
      unlisten = unsub;
      invoke("migrate_models", { oldPath, newPath }).catch((e) => {
        if (unlisten) unlisten();
        reject(e);
      });
    });
  });
}
