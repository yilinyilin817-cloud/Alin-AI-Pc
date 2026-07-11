<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Upload, ArrowLeft } from "@element-plus/icons-vue";
import { useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import { usePluginStore } from "@/stores/plugin";
import PluginCard from "@/components/PluginCard.vue";
import PluginDetail from "@/components/PluginDetail.vue";
import EmptyState from "@/components/EmptyState.vue";
import type { InstalledPlugin } from "@/types";
import { isTauri } from "@/api/env";

const router = useRouter();
const pluginStore = usePluginStore();

const selectedPlugin = ref<InstalledPlugin | null>(null);
const detailVisible = ref(false);

onMounted(() => {
  pluginStore.loadPlugins();
});

async function importPlugin() {
  if (!isTauri()) {
    ElMessage.warning("当前环境不支持文件选择，请在 Tauri 桌面端使用");
    return;
  }

  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择插件目录",
    });
    if (typeof selected === "string") {
      await pluginStore.installPlugin(selected);
      ElMessage.success("插件导入成功");
    }
  } catch (e) {
    ElMessage.error("导入插件失败: " + String(e));
  }
}

function openDetail(plugin: InstalledPlugin) {
  selectedPlugin.value = plugin;
  detailVisible.value = true;
}

async function handleUninstall(id: string) {
  try {
    await ElMessageBox.confirm("卸载后该插件提供的技能与命令将不可用，是否继续？", "确认卸载", {
      confirmButtonText: "卸载",
      cancelButtonText: "取消",
      type: "warning",
    });
    await pluginStore.uninstallPlugin(id);
    if (selectedPlugin.value?.id === id) {
      detailVisible.value = false;
      selectedPlugin.value = null;
    }
    ElMessage.success("插件已卸载");
  } catch (e) {
    if (e !== "cancel") {
      console.warn("uninstall plugin:", e);
    }
  }
}

async function handleToggle(id: string) {
  try {
    await pluginStore.toggleEnabled(id);
  } catch (e) {
    ElMessage.error("切换插件状态失败: " + String(e));
  }
}

async function handleSaveConfig(id: string, config: Record<string, unknown>) {
  try {
    await pluginStore.updateConfig(id, config);
    ElMessage.success("配置已保存");
  } catch (e) {
    ElMessage.error("保存配置失败: " + String(e));
  }
}
</script>

<template>
  <div class="page-container plugin-manager">
    <div class="page-header-row">
      <div>
        <h1 class="page-title">插件管理</h1>
        <p class="page-subtitle">安装、配置与管理 AI 伴侣的扩展插件</p>
      </div>
      <div class="page-actions">
        <el-button :icon="ArrowLeft" @click="router.push('/skills')">
          返回技能市场
        </el-button>
        <el-button type="primary" :icon="Upload" @click="importPlugin">
          导入本地插件
        </el-button>
      </div>
    </div>

    <div v-loading="pluginStore.loading" class="plugin-list-wrap">
      <div v-if="pluginStore.plugins.length" class="plugin-grid">
        <PluginCard
          v-for="plugin in pluginStore.plugins"
          :key="plugin.id"
          :plugin="plugin"
          @click="openDetail"
          @uninstall="handleUninstall"
          @toggle="handleToggle"
        />
      </div>

      <EmptyState
        v-else
        icon="🧩"
        title="暂无已安装插件"
        description="点击右上角“导入本地插件”按钮，选择插件目录进行安装。"
      >
      <template #actions>
        <el-button type="primary" :icon="Upload" @click="importPlugin">
          导入本地插件
        </el-button>
      </template>
    </EmptyState>
    </div>

    <el-dialog
      v-model="detailVisible"
      title="插件详情"
      width="640px"
      destroy-on-close
      class="plugin-detail-dialog"
    >
      <PluginDetail
        v-if="selectedPlugin"
        :plugin="selectedPlugin"
        @uninstall="handleUninstall"
        @toggle="handleToggle"
        @save-config="handleSaveConfig"
      />
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.page-header-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 8px;
}

.page-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}

.plugin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  gap: 16px;
}

.plugin-detail-dialog {
  :deep(.el-dialog) {
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }

  :deep(.el-dialog__title) {
    color: var(--color-text-primary);
  }
}
</style>
