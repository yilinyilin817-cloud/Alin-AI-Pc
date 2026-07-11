import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { 
  loadSettings as apiLoadSettings, 
  saveSettings as apiSaveSettings,
  getModelsDirInfo,
  setModelsDir as apiSetModelsDir,
  migrateModels as apiMigrateModels,
  type ModelDirInfo,
  type MigrationProgress
} from "@/api/settings";

const LAST_ROUTE_STORAGE_KEY = "app:last-route";
const VOICE_MESSAGE_PREFERRED_KEY = "app:voice-message-preferred";

export type ThemeMode = "dark" | "light" | "system";
export type FontSize = "small" | "default" | "large";
export type MessageDensity = "compact" | "default" | "cozy";
export type Language = "zh-CN" | "en-US";

type ResolvedTheme = "dark" | "light";

function getSystemTheme(): ResolvedTheme {
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function readVoiceMessagePreferred(): boolean {
  if (typeof window === "undefined") return false;
  const raw = localStorage.getItem(VOICE_MESSAGE_PREFERRED_KEY);
  return raw === "true";
}

function writeVoiceMessagePreferred(value: boolean) {
  if (typeof window === "undefined") return;
  localStorage.setItem(VOICE_MESSAGE_PREFERRED_KEY, String(value));
}

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<ThemeMode>(
    (localStorage.getItem("theme") as ThemeMode) ?? "dark",
  );
  const language = ref<Language>("zh-CN");
  const networkEnabled = ref(false);
  const launchOnStartup = ref(false);
  const inputDevice = ref("default");
  const outputDevice = ref("default");
  const fontSize = ref<FontSize>("default");
  const messageDensity = ref<MessageDensity>("default");
  const generativeArtEnabled = ref(false);
  const generativeArtSeed = ref<number | undefined>(undefined);
  const voiceMessagePreferred = ref<boolean>(readVoiceMessagePreferred());
  const lastRoute = ref<string | undefined>(undefined);
  const loaded = ref(false);

  const modelsDir = ref("");
  const originalModelsDir = ref("");
  const modelDirInfo = ref<ModelDirInfo | null>(null);
  const migrationProgress = ref(0);
  const migrationCurrentModel = ref("");
  const migrationStatus = ref<'idle' | 'migrating' | 'success' | 'error'>('idle');
  const migrationError = ref("");

  const effectiveTheme = computed<ResolvedTheme>(() =>
    theme.value === "system" ? getSystemTheme() : theme.value,
  );

  /** 应用主题：html.light 亮色 / html.dark 暗色 */
  function applyTheme() {
    if (typeof document === "undefined") return;
    const root = document.documentElement;
    if (effectiveTheme.value === "light") {
      root.classList.add("light");
      root.classList.remove("dark");
    } else {
      root.classList.remove("light");
      root.classList.add("dark");
    }
    localStorage.setItem("theme", theme.value);
  }

  function setTheme(mode: ThemeMode) {
    theme.value = mode;
    applyTheme();
    save();
  }

  function toggleTheme() {
    const current = effectiveTheme.value;
    setTheme(current === "dark" ? "light" : "dark");
  }

  function setLastRoute(route: string | undefined) {
    lastRoute.value = route;
    // 同时写入 localStorage 作为持久化回退
    if (route) {
      localStorage.setItem(LAST_ROUTE_STORAGE_KEY, route);
    } else {
      localStorage.removeItem(LAST_ROUTE_STORAGE_KEY);
    }
    save();
  }

  async function load() {
    try {
      const s = await apiLoadSettings();
      if (s.theme) theme.value = s.theme as ThemeMode;
      if (s.language) language.value = s.language as Language;
      if (s.networkEnabled !== undefined) networkEnabled.value = s.networkEnabled as boolean;
      if (s.launchOnStartup !== undefined) launchOnStartup.value = s.launchOnStartup as boolean;
      if (s.inputDevice) inputDevice.value = s.inputDevice as string;
      if (s.outputDevice) outputDevice.value = s.outputDevice as string;
      if (s.fontSize) fontSize.value = s.fontSize as FontSize;
      if (s.messageDensity) messageDensity.value = s.messageDensity as MessageDensity;
      if (s.generativeArtEnabled !== undefined) generativeArtEnabled.value = s.generativeArtEnabled as boolean;
      if (s.generativeArtSeed !== undefined) generativeArtSeed.value = s.generativeArtSeed as number;
      if (s.voiceMessagePreferred !== undefined) voiceMessagePreferred.value = s.voiceMessagePreferred as boolean;
      if (s.lastRoute !== undefined) lastRoute.value = s.lastRoute as string;
      if (s.modelsDir) modelsDir.value = s.modelsDir as string;
      loaded.value = true;
    } catch (e) {
      console.warn("loadSettings:", e);
    }
    // 后端读取失败或没有对应字段时，回退到 localStorage
    if (voiceMessagePreferred.value === undefined || voiceMessagePreferred.value === null) {
      voiceMessagePreferred.value = readVoiceMessagePreferred();
    }
    if (!lastRoute.value) {
      lastRoute.value = localStorage.getItem(LAST_ROUTE_STORAGE_KEY) ?? undefined;
    }
    // 确保初始主题应用
    applyTheme();
    // 加载模型目录信息
    await refreshModelDirInfo();
  }

  async function save() {
    writeVoiceMessagePreferred(voiceMessagePreferred.value);
    try {
      await apiSaveSettings({
        theme: theme.value,
        language: language.value,
        networkEnabled: networkEnabled.value,
        launchOnStartup: launchOnStartup.value,
        inputDevice: inputDevice.value,
        outputDevice: outputDevice.value,
        fontSize: fontSize.value,
        messageDensity: messageDensity.value,
        generativeArtEnabled: generativeArtEnabled.value,
        generativeArtSeed: generativeArtSeed.value,
        voiceMessagePreferred: voiceMessagePreferred.value,
        lastRoute: lastRoute.value,
        modelsDir: modelsDir.value,
      });
    } catch (e) {
      console.warn("saveSettings:", e);
    }
  }

  async function refreshModelDirInfo() {
    try {
      modelDirInfo.value = await getModelsDirInfo();
      if (!modelsDir.value && modelDirInfo.value.path) {
        modelsDir.value = modelDirInfo.value.path;
        originalModelsDir.value = modelDirInfo.value.path;
      }
    } catch (e) {
      console.warn("getModelsDirInfo:", e);
    }
  }

  async function selectModelDir(): Promise<boolean> {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择模型存储目录",
      });
      if (typeof selected === "string" && selected) {
        modelsDir.value = selected;
        return true;
      }
    } catch (e) {
      console.error("selectModelDir:", e);
    }
    return false;
  }

  async function applyModelDir() {
    if (modelsDir.value === originalModelsDir.value) return;
    try {
      await apiSetModelsDir(modelsDir.value);
      originalModelsDir.value = modelsDir.value;
      await refreshModelDirInfo();
    } catch (e) {
      throw new Error(String(e));
    }
  }

  async function startMigration() {
    if (!originalModelsDir.value || !modelsDir.value) return;
    if (modelsDir.value === originalModelsDir.value) return;
    
    migrationStatus.value = 'migrating';
    migrationProgress.value = 0;
    migrationCurrentModel.value = "";
    migrationError.value = "";

    try {
      await apiMigrateModels(
        originalModelsDir.value,
        modelsDir.value,
        (progress: MigrationProgress) => {
          migrationProgress.value = progress.progress;
          if (progress.currentModel) {
            migrationCurrentModel.value = progress.currentModel;
          }
          if (progress.done) {
            migrationCurrentModel.value = "";
            if (progress.success) {
              migrationStatus.value = 'success';
              originalModelsDir.value = modelsDir.value;
            } else {
              migrationStatus.value = 'error';
              migrationError.value = progress.failed?.map(f => `${f.name}: ${f.error}`).join('\n') || "迁移失败";
            }
          }
        }
      );
      migrationStatus.value = 'success';
      originalModelsDir.value = modelsDir.value;
      migrationCurrentModel.value = "";
      await refreshModelDirInfo();
    } catch (e) {
      migrationStatus.value = 'error';
      migrationError.value = String(e);
      migrationCurrentModel.value = "";
    }
  }

  function resetMigrationStatus() {
    migrationStatus.value = 'idle';
    migrationProgress.value = 0;
    migrationCurrentModel.value = "";
    migrationError.value = "";
  }

  // 初始化时立即应用
  applyTheme();

  return {
    theme,
    language,
    networkEnabled,
    launchOnStartup,
    inputDevice,
    outputDevice,
    fontSize,
    messageDensity,
    generativeArtEnabled,
    generativeArtSeed,
    voiceMessagePreferred,
    lastRoute,
    loaded,
    effectiveTheme,
    modelsDir,
    originalModelsDir,
    modelDirInfo,
    migrationProgress,
    migrationCurrentModel,
    migrationStatus,
    migrationError,
    load,
    save,
    setTheme,
    toggleTheme,
    applyTheme,
    setLastRoute,
    refreshModelDirInfo,
    selectModelDir,
    applyModelDir,
    startMigration,
    resetMigrationStatus,
  };
});
