<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import {
  Sunny,
  AlarmClock,
  FolderOpened,
  Search,
  Management,
} from "@element-plus/icons-vue";
import SkillPermissionDialog from "@/components/SkillPermissionDialog.vue";
import PluginCard from "@/components/PluginCard.vue";
import PluginDetail from "@/components/PluginDetail.vue";
import EmptyState from "@/components/EmptyState.vue";
import { usePluginStore } from "@/stores/plugin";
import { mockSkills, mockToolCallLogs } from "@/mocks/data";
import type { SkillDefinition, InstalledPlugin } from "@/types";
import { ElMessage, ElMessageBox } from "element-plus";

const router = useRouter();
const pluginStore = usePluginStore();

const skills = ref<SkillDefinition[]>(structuredClone(mockSkills));
const expandedSkill = ref<string | null>(null);
const permissionDialogVisible = ref(false);
const pendingSkill = ref<SkillDefinition | null>(null);
const activeTab = ref("skills");

const selectedPlugin = ref<InstalledPlugin | null>(null);
const detailVisible = ref(false);

const iconMap: Record<string, typeof Sunny> = {
  Sunny,
  AlarmClock,
  FolderOpened,
  Search,
};

onMounted(() => {
  pluginStore.loadPlugins();
});

function toggleSkill(skill: SkillDefinition) {
  if (
    !skill.enabled &&
    skill.permissions.includes("network") &&
    skill.approvalMode !== "always"
  ) {
    pendingSkill.value = skill;
    permissionDialogVisible.value = true;
    return;
  }
  skill.enabled = !skill.enabled;
}

function handleApprove() {
  if (pendingSkill.value) {
    pendingSkill.value.enabled = true;
    ElMessage.success(`已授权技能: ${pendingSkill.value.name}`);
  }
}

function toggleExpand(id: string) {
  expandedSkill.value = expandedSkill.value === id ? null : id;
}

function openPluginDetail(plugin: InstalledPlugin) {
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

const skillLogs = mockToolCallLogs;
</script>

<template>
  <div class="page-container skill-market">
    <div class="market-header">
      <div>
        <h1 class="page-title">技能市场</h1>
        <p class="page-subtitle">管理 AI 伴侣的工具、技能与扩展插件</p>
      </div>
      <el-button :icon="Management" @click="router.push('/plugins')">
        打开插件管理
      </el-button>
    </div>

    <el-tabs v-model="activeTab" class="market-tabs">
      <el-tab-pane label="技能市场" name="skills">
        <div class="skill-grid">
          <el-card
            v-for="skill in skills"
            :key="skill.id"
            class="skill-card"
            :class="{ expanded: expandedSkill === skill.id }"
            shadow="hover"
            @click="toggleExpand(skill.id)"
          >
            <div class="skill-header">
              <el-icon :size="24" class="skill-icon">
                <component :is="iconMap[skill.icon] ?? Search" />
              </el-icon>
              <div class="skill-info">
                <h3>{{ skill.name }}</h3>
                <p>{{ skill.description }}</p>
              </div>
              <el-switch
                :model-value="skill.enabled"
                @click.stop
                @change="toggleSkill(skill)"
              />
            </div>

            <div class="skill-tags">
              <el-tag
                v-for="perm in skill.permissions"
                :key="perm"
                size="small"
                type="warning"
              >
                {{ perm }}
              </el-tag>
              <el-tag v-if="!skill.permissions.length" size="small">本地</el-tag>
            </div>

            <el-alert
              v-if="skill.permissions.includes('network') && !skill.enabled"
              title="需授权网络"
              type="warning"
              :closable="false"
              show-icon
              class="network-alert"
            />

            <div v-if="expandedSkill === skill.id" class="skill-detail" @click.stop>
              <el-divider />
              <h4>调用日志</h4>
              <el-timeline>
                <el-timeline-item
                  v-for="log in skillLogs.filter((l) => l.skillName === skill.name.replace('get_', '').replace('set_', ''))"
                  :key="log.id"
                  :timestamp="log.createdAt"
                  placement="top"
                  :type="log.status === 'success' ? 'success' : 'danger'"
                >
                  {{ log.argsJson }} → {{ log.resultJson }}
                  <el-tag size="small">{{ log.durationMs }}ms</el-tag>
                </el-timeline-item>
                <el-timeline-item v-if="!skillLogs.length" timestamp="">
                  暂无调用记录
                </el-timeline-item>
              </el-timeline>
            </div>
          </el-card>
        </div>
      </el-tab-pane>

      <el-tab-pane label="已安装插件" name="plugins">
        <div v-if="pluginStore.plugins.length" class="plugin-grid">
          <PluginCard
            v-for="plugin in pluginStore.plugins"
            :key="plugin.id"
            :plugin="plugin"
            @click="openPluginDetail"
            @uninstall="handleUninstall"
            @toggle="handleToggle"
          />
        </div>
        <EmptyState
          v-else
          icon="🧩"
          title="暂无已安装插件"
          description="已安装的插件将在这里显示，也可以前往插件管理页面导入本地插件。"
        >
          <template #actions>
            <el-button :icon="Management" @click="router.push('/plugins')">
              打开插件管理
            </el-button>
          </template>
        </EmptyState>
      </el-tab-pane>
    </el-tabs>

    <SkillPermissionDialog
      v-if="pendingSkill"
      :skill="pendingSkill"
      :visible="permissionDialogVisible"
      @update:visible="permissionDialogVisible = $event"
      @approve="handleApprove"
    />

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
.market-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 8px;
}

.market-tabs {
  :deep(.el-tabs__header) {
    background: var(--glass-bg-light);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: 4px;
    margin-bottom: 20px;
  }
}

.skill-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.skill-card {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
  cursor: pointer;
  transition: all var(--transition);

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border-hover);
    transform: translateY(-2px);
  }

  &.expanded {
    border-color: var(--color-primary);
    box-shadow: var(--glass-shadow), var(--glass-glow);
  }

  :deep(.el-card__body) {
    background: transparent;
  }
}

.skill-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.skill-icon {
  color: var(--color-primary);
  flex-shrink: 0;
  margin-top: 2px;
  filter: var(--skill-icon-glow);
}

.skill-info {
  flex: 1;

  h3 {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  p {
    font-size: 13px;
    color: var(--color-text-secondary);
    line-height: 1.4;
  }
}

.skill-tags {
  display: flex;
  gap: 4px;
  margin-top: 10px;
  flex-wrap: wrap;
}

.network-alert {
  margin-top: 10px;
  background: var(--glass-bg-light);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
}

.skill-detail {
  h4 {
    font-size: 13px;
    color: var(--color-text-secondary);
    margin-bottom: 8px;
  }
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
