<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import ModelDownloadCard from "@/components/ModelDownloadCard.vue";
import CloudProviderManager from "@/components/CloudProviderManager.vue";
import CloudTtsProviderManager from "@/components/CloudTtsProviderManager.vue";
import ModelTestDialog from "@/components/ModelTestDialog.vue";
import {
  getGpuInfo, checkOllama,
  downloadModel as apiDownload, activateModel as apiActivate,
  deleteModel as apiDelete, cancelDownload as apiCancel,
  diagnoseNetwork as apiDiagnose,
  type OllamaStatus,
} from "@/api/model";
import { useModelStore } from "@/stores/model";
import type { ModelConfig, ModelType } from "@/types";
import { ElMessage, ElMessageBox } from "element-plus";
import { Setting, Check } from "@element-plus/icons-vue";

// ── 状态 ──

const modelStore = useModelStore();
const models = ref<ModelConfig[]>([]);
const activeType = ref<ModelType>("llm");
const gpuInfo = ref<{ vramGb: number; recommendation: string } | null>(null);
const ollamaStatus = ref<OllamaStatus | null>(null);
const showCloudProvider = ref(false);
const showCloudTtsProvider = ref(false);

// 测试对话框状态
const testDialogVisible = ref(false);
const testModelId = ref("");
const testModelName = ref("");
const testModelType = ref("");
const testProviderId = ref("");

const typeTabs: { label: string; value: ModelType }[] = [
  { label: "LLM", value: "llm" },
  { label: "ASR", value: "asr" },
  { label: "TTS", value: "tts" },
  { label: "Embedding", value: "embedding" },
];

const filteredModels = computed(() =>
  models.value.filter((m) => m.modelType === activeType.value),
);

const activeLlm = computed(() =>
  models.value.find((m) => m.modelType === "llm" && m.isActive),
);

// ── 初始化 ──

onMounted(async () => {
  // 注册下载进度事件监听
  modelStore.initListener();

  // 额外监听下载进度事件，同步到 models 列表（更新卡片上的进度条/状态）
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
        const m = models.value.find((mm) => mm.id === modelId);
        if (m) {
          m.progress = progress;
          if (done) {
            if (cancelled) {
              m.status = "not_downloaded";
              m.progress = 0;
            } else if (error) {
              m.status = "not_downloaded";
              m.progress = 0;
            } else {
              m.status = "downloaded";
              m.progress = 100;
            }
          } else {
            m.status = "downloading";
          }
        }
      });
    } catch (e) {
      console.warn("ModelCenter: failed to listen download-progress:", e);
    }
  }

  // 加载模型列表
  await refreshModelList();

  // GPU 信息
  try {
    gpuInfo.value = await getGpuInfo();
  } catch (e) {
    console.warn("getGpuInfo:", e);
  }

  // Ollama 状态
  try {
    ollamaStatus.value = await checkOllama();
  } catch (e) {
    ollamaStatus.value = { available: false, version: 'unknown', models: [] };
  }
});

async function refreshModelList() {
  models.value = await modelStore.loadModels();
}

// ── 操作 ──

async function handleDownload(id: string) {
  const model = models.value.find((m) => m.id === id);
  if (!model) return;

  // 使用 store 管理下载任务
  modelStore.addTask(id, model.name);

  try {
    await apiDownload(id);
  } catch (e: any) {
    modelStore.updateProgress(id, 0, true, e.toString());
    ElMessage.error(`下载失败: ${e}`);
  }
}

async function handleCancel(id: string) {
  const model = models.value.find((m) => m.id === id);
  try {
    await apiCancel(id);
    modelStore.updateProgress(id, 0, true, undefined, true);
    ElMessage.info(model ? `已取消下载 ${model.name}` : "已取消下载");
    await refreshModelList();
  } catch (e: any) {
    ElMessage.error(`取消失败: ${e}`);
  }
}

async function handleActivate(id: string) {
  const model = models.value.find((m) => m.id === id);
  if (!model) return;

  try {
    await apiActivate(id);
    await refreshModelList();
    ElMessage.success(`已切换至 ${model.name}`);
  } catch (e: any) {
    modelStore.reportModelError(e.toString(), "模型切换失败");
    ElMessage.error(`切换失败: ${e}`);
  }
}

async function handleTest(id: string) {
  const model = models.value.find((m) => m.id === id);
  if (!model) return;
  testModelId.value = model.id;
  testModelName.value = model.name;
  testModelType.value = model.modelType;
  testProviderId.value = model.providerId;
  testDialogVisible.value = true;
}

const networkDiagnostics = ref<any>(null);
const diagnosticLoading = ref(false);

async function handleDiagnose() {
  diagnosticLoading.value = true;
  try {
    networkDiagnostics.value = await apiDiagnose();
    ElMessage.info("网络诊断完成");
  } catch (e: any) {
    ElMessage.error(`诊断失败: ${e}`);
  } finally {
    diagnosticLoading.value = false;
  }
}

async function handleDelete(id: string) {
  try {
    await ElMessageBox.confirm(
      "确定要删除该模型文件吗？此操作不可恢复。",
      "警告",
      {
        confirmButtonText: "确定删除",
        cancelButtonText: "取消",
        type: "warning",
      }
    );

    await apiDelete(id);
    await refreshModelList();
    ElMessage.success("模型已成功删除");
  } catch (e: any) {
    if (e !== "cancel") {
      ElMessage.error(`删除失败: ${e}`);
    }
  }
}

// 如果 store 中的下载状态变化（特别是完成/取消/失败时），刷新列表
watch(() => modelStore.downloadTasks, (tasks) => {
  const anyDone = tasks.some(t => t.status === "completed" || t.status === "failed" || t.status === "cancelled");
  if (anyDone) {
     setTimeout(refreshModelList, 1000);
  }
}, { deep: true });

function openDownloadDrawer() {
  modelStore.isDrawerVisible = true;
}
</script>

<template>
  <div class="page-container model-center">
    <div class="page-header">
      <div>
        <h1 class="page-title">模型中心</h1>
        <p class="page-subtitle">下载、管理与切换本地 AI 模型</p>
      </div>
      <div class="header-actions">
        <el-button type="info" @click="handleDiagnose" :loading="diagnosticLoading">
          <el-icon><Check /></el-icon>
          网络连接诊断
        </el-button>
        <el-button type="primary" @click="showCloudProvider = true">
          <el-icon><Setting /></el-icon>
          云服务商管理
        </el-button>
        <el-button type="success" @click="showCloudTtsProvider = true">
          <el-icon><Setting /></el-icon>
          云端 TTS 服务商
        </el-button>
      </div>
    </div>

    <!-- 网络诊断结果 -->
    <el-card v-if="networkDiagnostics" class="diagnostic-card" shadow="never">
      <template #header>
        <div class="card-header">
          <span>网络诊断结果</span>
          <el-button link type="primary" @click="networkDiagnostics = null">隐藏</el-button>
        </div>
      </template>
      <div class="diagnostic-grid">
        <div v-for="(res, key) in networkDiagnostics" :key="key" class="diagnostic-item">
          <div class="item-label">{{ String(key).toUpperCase().replace('_', ' ') }}</div>
          <div class="item-value">
            <el-tag :type="res.status ? 'success' : 'danger'" size="small">
              {{ res.status ? '连接正常' : '连接失败' }}
            </el-tag>
            <span class="item-msg">{{ res.message }}</span>
          </div>
        </div>
      </div>
    </el-card>

    <!-- GPU 信息 -->
    <el-alert
      v-if="gpuInfo"
      :title="`检测到 ${gpuInfo.vramGb}GB 显存，推荐使用 ${gpuInfo.recommendation}`"
      type="info"
      show-icon
      :closable="false"
      class="gpu-banner"
    />

    <!-- Ollama 状态 -->
    <el-alert
      v-if="activeType === 'llm' && ollamaStatus"
      :type="ollamaStatus.available ? 'success' : 'warning'"
      :title="ollamaStatus.available
        ? `Ollama 运行中，已安装 ${ollamaStatus.models.length} 个模型`
        : 'Ollama 未运行 — LLM 将使用 Mock 降级'"
      show-icon
      :closable="false"
      class="ollama-banner"
    >
      <template #default v-if="ollamaStatus.available">
        <div class="ollama-info">
          <div class="ollama-models" v-if="ollamaStatus.models.length">
            <el-tag
              v-for="m in ollamaStatus.models"
              :key="m.name"
              size="small"
              style="margin-right: 4px; margin-bottom: 4px"
            >
              {{ m.name }}
            </el-tag>
          </div>
        </div>
      </template>
    </el-alert>

    <!-- 当前 LLM -->
    <div class="active-model" v-if="activeLlm">
      <span>当前 LLM：</span>
      <el-tag type="success">{{ activeLlm.name }}</el-tag>
      <el-button text size="small" @click="openDownloadDrawer">查看下载</el-button>
    </div>

    <!-- 类型 Tabs -->
    <el-tabs v-model="activeType" class="model-tabs">
      <el-tab-pane
        v-for="tab in typeTabs"
        :key="tab.value"
        :label="tab.label"
        :name="tab.value"
      />
    </el-tabs>

    <!-- 模型卡片网格 -->
    <div class="model-grid">
      <ModelDownloadCard
        v-for="model in filteredModels"
        :key="model.id"
        :model="model"
        @download="handleDownload"
        @cancel="handleCancel"
        @activate="handleActivate"
        @test="handleTest"
        @delete="handleDelete"
      />
    </div>

    <!-- 空状态 -->
    <el-empty v-if="!filteredModels.length" description="暂无此类型模型" />

    <!-- 下载任务抽屉 -->
    <el-drawer v-model="modelStore.isDrawerVisible" title="下载任务" direction="btt" size="280px">
      <div v-for="task in modelStore.downloadTasks" :key="task.id" class="download-task">
        <div class="task-header">
          <span class="task-name">{{ task.name }}</span>
          <div class="task-actions">
            <el-tag
              size="small"
              :type="task.status === 'completed' ? 'success'
                : task.status === 'failed' ? 'danger'
                : task.status === 'cancelled' ? 'info'
                : 'warning'"
            >
              {{
                task.status === 'completed' ? '已完成'
                : task.status === 'failed' ? '失败'
                : task.status === 'cancelled' ? '已取消'
                : '下载中'
              }}
            </el-tag>
            <el-button
              v-if="task.status === 'downloading'"
              link
              type="danger"
              size="small"
              @click="handleCancel(task.id)"
            >取消</el-button>
          </div>
        </div>
        <el-progress :percentage="task.progress" :stroke-width="6" :status="task.status === 'failed' ? 'exception' : task.status === 'cancelled' ? 'warning' : undefined" />
        <p v-if="task.error" class="task-error">{{ task.error }}</p>
      </div>
      <p v-if="!modelStore.downloadTasks.length" class="no-tasks">暂无下载任务</p>
      <div class="drawer-actions">
        <el-button v-if="modelStore.downloadTasks.length" size="small" type="danger" plain @click="modelStore.clearCompleted">清除已完成</el-button>
        <el-button size="small" @click="modelStore.isDrawerVisible = false">关闭</el-button>
      </div>
    </el-drawer>

    <!-- 云服务商管理对话框 -->
    <el-dialog
      v-model="showCloudProvider"
      title="云服务商管理"
      width="90%"
      top="5vh"
      :close-on-click-modal="false"
    >
      <CloudProviderManager />
    </el-dialog>

    <!-- 云端 TTS 服务商管理对话框 -->
    <el-dialog
      v-model="showCloudTtsProvider"
      title="云端 TTS 服务商管理"
      width="90%"
      top="5vh"
      :close-on-click-modal="false"
    >
      <CloudTtsProviderManager />
    </el-dialog>

    <!-- 模型测试可视化对话框 -->
    <ModelTestDialog
      v-model:visible="testDialogVisible"
      :model-id="testModelId"
      :model-name="testModelName"
      :model-type="testModelType"
      :provider-id="testProviderId"
    />
  </div>
</template>

<style scoped lang="scss">
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 20px;
}

.diagnostic-card {
  margin-bottom: 16px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
}

.diagnostic-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
}

.diagnostic-item {
  .item-label {
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-bottom: 4px;
    font-weight: 600;
  }
  .item-value {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .item-msg {
    font-size: 11px;
    color: var(--color-text-muted);
  }
}

.gpu-banner, .ollama-banner {
  margin-bottom: 12px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
}

.ollama-info {
  margin-top: 6px;
}

.ollama-models {
  display: flex;
  flex-wrap: wrap;
}

.version-warning {
  margin-top: 8px;
  font-size: 12px;
  color: var(--color-warning);
  display: flex;
  align-items: center;
  gap: 4px;
}

.active-model {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  font-size: 14px;
  color: var(--color-text-secondary);
  padding: 12px 16px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
}

.model-tabs {
  margin-bottom: 16px;

  :deep(.el-tabs__header) {
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: 4px;
  }
}

.model-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.download-task {
  margin-bottom: 16px;
  padding: 10px;
  background: var(--glass-bg-light);
  border-radius: var(--radius-md);

  .task-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .task-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .task-name {
    font-size: 13px;
    font-weight: 500;
  }

  .task-error {
    font-size: 11px;
    color: var(--color-danger);
    margin-top: 4px;
  }
}

.no-tasks {
  color: var(--color-text-muted);
  font-size: 13px;
  text-align: center;
  padding: 20px 0;
}

.drawer-actions {
  display: flex;
  justify-content: center;
  gap: 12px;
  padding-top: 8px;
}
</style>
