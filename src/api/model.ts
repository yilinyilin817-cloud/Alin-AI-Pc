import type { ModelConfig, GpuInfo } from "@/types/model";
import { isTauri } from "./env";
import { mockModels, mockGpuInfo } from "@/mocks/data";

/** Ollama 状态 */
export interface OllamaStatus {
  available: boolean;
  version: string;
  models: { name: string; size?: string }[];
  error?: string;
}

export async function listModels(): Promise<ModelConfig[]> {
  if (!isTauri()) return structuredClone(mockModels);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ModelConfig[]>("list_models");
}

export async function getGpuInfo(): Promise<GpuInfo> {
  if (!isTauri()) return mockGpuInfo;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<GpuInfo>("get_gpu_info");
}

export async function checkOllama(): Promise<OllamaStatus> {
  if (!isTauri()) return { available: false, version: 'unknown', models: [] };
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<OllamaStatus>("check_ollama");
}

export async function downloadModel(modelId: string): Promise<void> {
  if (!isTauri()) {
    console.warn("downloadModel: not in Tauri environment");
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("download_model", { modelId });
}

export async function cancelDownload(modelId: string): Promise<void> {
  if (!isTauri()) {
    console.warn("cancelDownload: not in Tauri environment");
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("cancel_download", { modelId });
}

export async function activateModel(modelId: string): Promise<void> {
  if (!isTauri()) {
    console.warn("activateModel: not in Tauri environment");
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("activate_model", { modelId });
}

export async function getModelStatus(modelId: string): Promise<ModelConfig | null> {
  if (!isTauri()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ModelConfig>("get_model_status", { modelId });
}

export interface ModelTestResult {
  success: boolean;
  message: string;
  latencyMs?: number;
  audioData?: string;
  audioMime?: string;
}

export async function testModel(modelId: string): Promise<ModelTestResult> {
  if (!isTauri()) return { success: true, message: "Mock 测试成功" };
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ModelTestResult>("test_model", { modelId });
}


/** 监听模型测试过程中的步骤事件（返回取消监听的函数） */
export async function listenModelTest(
  modelId: string,
  onStep: (payload: {
    modelId: string;
    step: string;
    status: "pending" | "active" | "done" | "failed";
    detail?: string;
    latencyMs?: number;
    message?: string;
    success?: boolean;
    done?: boolean;
    audioData?: string;
    audioMime?: string;
  }) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{
      modelId: string;
      step: string;
      status: string;
      detail?: string;
      latencyMs?: number;
      message?: string;
      success?: boolean;
      done?: boolean;
    }>("model-test:step", (event) => {
      const payload = event.payload;
      if (payload.modelId === modelId) {
        onStep({
          modelId: payload.modelId,
          step: payload.step,
          status: payload.status as "pending" | "active" | "done" | "failed",
          detail: payload.detail,
          latencyMs: payload.latencyMs,
          message: payload.message,
          success: payload.success,
          done: payload.done,
        });
      }
    });
    return unlisten;
  } catch {
    return () => {};
  }
}


export async function deleteModel(modelId: string): Promise<void> {
  if (!isTauri()) {
    console.warn("deleteModel: not in Tauri environment");
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("delete_model", { modelId });
}

export interface NetworkDiagnosis {
  ollamaReachable: boolean;
  huggingfaceReachable: boolean;
  latencyMs: number;
  proxyDetected: boolean;
}

export async function diagnoseNetwork(): Promise<NetworkDiagnosis> {
  if (!isTauri()) {
    return { ollamaReachable: false, huggingfaceReachable: false, latencyMs: 0, proxyDetected: false };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<NetworkDiagnosis>("diagnose_network");
}
