import { defineStore } from "pinia";
import { ref } from "vue";
import type { ModelConfig } from "@/types/model";
import { listModels, cancelDownload as apiCancelDownload } from "@/api/model";
import { useNotificationStore } from "@/stores/notification";

export interface DownloadTask {
  id: string; // modelId
  name: string;
  progress: number;
  status: "downloading" | "completed" | "failed" | "cancelled";
  error?: string;
}

export const useModelStore = defineStore("model", () => {
  const downloadTasks = ref<DownloadTask[]>([]);
  const isDrawerVisible = ref(false);

  function addTask(id: string, name: string) {
    const existing = downloadTasks.value.find((t) => t.id === id);
    if (existing) {
      existing.status = "downloading";
      existing.progress = 0;
      existing.error = undefined;
    } else {
      downloadTasks.value.push({
        id,
        name,
        progress: 0,
        status: "downloading",
      });
    }
    isDrawerVisible.value = true;
  }

  function updateProgress(id: string, progress: number, done?: boolean, error?: string, cancelled?: boolean) {
    const task = downloadTasks.value.find((t) => t.id === id);
    if (task) {
      task.progress = progress;
      if (done) {
        if (cancelled) {
          task.status = "cancelled";
        } else if (error) {
          task.status = "failed";
          task.error = error;
        } else {
          task.status = "completed";
        }
      }
    }
  }

  function clearCompleted() {
    downloadTasks.value = downloadTasks.value.filter((t) => t.status === "downloading");
  }

  async function cancelDownload(id: string) {
    try {
      await apiCancelDownload(id);
    } catch (e) {
      console.warn("cancelDownload error:", e);
    }
  }

  async function loadModels(): Promise<ModelConfig[]> {
    try {
      return await listModels();
    } catch (e: any) {
      const notificationStore = useNotificationStore();
      const notification = notificationStore.add({
        type: "error",
        source: "model",
        title: "模型连接失败",
        body: e?.message ? String(e.message) : String(e),
        action: {
          label: "重试",
          handler: () => {
            notificationStore.markRead(notification.id);
            loadModels();
          },
        },
      });
      return [];
    }
  }

  function reportModelError(error: string, title = "模型同步错误") {
    const notificationStore = useNotificationStore();
    const notification = notificationStore.add({
      type: "error",
      source: "model",
      title,
      body: error,
      action: {
        label: "重试",
        handler: () => {
          notificationStore.markRead(notification.id);
          loadModels();
        },
      },
    });
  }

  async function initListener() {
    if (typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__) {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        await listen<{
          modelId: string;
          progress: number;
          done?: boolean;
          error?: string;
          cancelled?: boolean;
        }>("download-progress", (event) => {
          const { modelId, progress, done, error, cancelled } = event.payload;
          updateProgress(modelId, progress, done, error, cancelled);
        });

        await listen<{ error?: string; message?: string }>("model-error", (event) => {
          const payload = event.payload;
          reportModelError(payload.error || payload.message || "模型发生错误");
        });

        await listen<{ modelId?: string; error?: string }>("model-disconnected", (event) => {
          const payload = event.payload;
          reportModelError(
            payload.error || `模型 ${payload.modelId || ""} 连接断开`,
            "模型连接断开",
          );
        });
      } catch (e) {
        console.warn("ModelStore: failed to setup listen:", e);
      }
    }
  }

  return {
    downloadTasks,
    isDrawerVisible,
    addTask,
    updateProgress,
    clearCompleted,
    cancelDownload,
    loadModels,
    reportModelError,
    initListener,
  };
});
