<script setup lang="ts">
import { ref, watch } from "vue";
import { Box, Delete, SwitchButton } from "@element-plus/icons-vue";
import type { InstalledPlugin, PluginConfigField } from "@/types";

const props = defineProps<{
  plugin: InstalledPlugin;
}>();

const emit = defineEmits<{
  (e: "uninstall", id: string): void;
  (e: "toggle", id: string): void;
  (e: "save-config", id: string, config: Record<string, unknown>): void;
}>();

const manifest = props.plugin.manifest;
const editingConfig = ref<Record<string, unknown>>({ ...props.plugin.config });

watch(
  () => props.plugin.config,
  (config) => {
    editingConfig.value = { ...config };
  },
  { deep: true },
);

function getFieldValue(field: PluginConfigField): unknown {
  if (editingConfig.value[field.key] !== undefined) {
    return editingConfig.value[field.key];
  }
  return field.default;
}

function setFieldValue(field: PluginConfigField, value: unknown) {
  editingConfig.value[field.key] = value;
}

function saveConfig() {
  emit("save-config", props.plugin.id, { ...editingConfig.value });
}
</script>

<template>
  <div class="plugin-detail">
    <div class="detail-header">
      <div class="detail-icon">
        <el-image v-if="manifest.icon" :src="manifest.icon" fit="cover" class="icon-image" />
        <el-icon v-else :size="36"><Box /></el-icon>
      </div>
      <div class="detail-title">
        <h2>{{ manifest.name }}</h2>
        <div class="detail-meta">
          <el-tag size="small" type="info">v{{ manifest.version }}</el-tag>
          <span v-if="manifest.author" class="detail-author">{{ manifest.author }}</span>
        </div>
      </div>
      <div class="detail-actions">
        <el-switch
          :model-value="plugin.enabled"
          inline-prompt
          :active-icon="SwitchButton"
          active-text="启用"
          inactive-text="禁用"
          @change="emit('toggle', plugin.id)"
        />
        <el-button type="danger" plain :icon="Delete" @click="emit('uninstall', plugin.id)">
          卸载
        </el-button>
      </div>
    </div>

    <p class="detail-description">{{ manifest.description }}</p>

    <el-tabs class="detail-tabs">
      <el-tab-pane label="权限" name="permissions">
        <div v-if="manifest.permissions.length" class="section-list">
          <el-tag
            v-for="perm in manifest.permissions"
            :key="perm"
            size="small"
            type="warning"
            effect="plain"
          >
            {{ perm }}
          </el-tag>
        </div>
        <el-empty v-else description="该插件无需额外权限" />
      </el-tab-pane>

      <el-tab-pane label="技能" name="skills">
        <div v-if="manifest.skills?.length" class="skill-list">
          <div v-for="skill in manifest.skills" :key="skill.id" class="skill-item">
            <div class="skill-name">{{ skill.name }}</div>
            <div class="skill-desc">{{ skill.description }}</div>
          </div>
        </div>
        <el-empty v-else description="该插件未提供技能" />
      </el-tab-pane>

      <el-tab-pane label="命令" name="commands">
        <div v-if="manifest.commands?.length" class="command-list">
          <div v-for="cmd in manifest.commands" :key="cmd.id" class="command-item">
            <div class="command-title">{{ cmd.title }}</div>
            <el-tag v-if="cmd.shortcut" size="small" type="info">{{ cmd.shortcut }}</el-tag>
          </div>
        </div>
        <el-empty v-else description="该插件未注册命令" />
      </el-tab-pane>

      <el-tab-pane v-if="manifest.config?.length" label="配置" name="config">
        <el-form label-position="top" class="config-form">
          <el-form-item
            v-for="field in manifest.config"
            :key="field.key"
            :label="field.label"
            :required="field.required"
          >
            <el-input
              v-if="field.type === 'string' || field.type === 'password'"
              v-model="editingConfig[field.key]"
              :type="field.type === 'password' ? 'password' : 'text'"
              :placeholder="String(field.default ?? '')"
              @change="saveConfig"
            />
            <el-input-number
              v-else-if="field.type === 'number'"
              :model-value="getFieldValue(field) as number"
              @update:model-value="setFieldValue(field, $event)"
              @change="saveConfig"
            />
            <el-switch
              v-else-if="field.type === 'boolean'"
              :model-value="getFieldValue(field) as boolean"
              @update:model-value="setFieldValue(field, $event)"
              @change="saveConfig"
            />
            <el-select
              v-else-if="field.type === 'select'"
              :model-value="getFieldValue(field) as string"
              @update:model-value="setFieldValue(field, $event)"
              @change="saveConfig"
              style="width: 100%"
            >
              <el-option
                v-for="opt in field.options"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
          </el-form-item>
        </el-form>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<style scoped lang="scss">
.plugin-detail {
  padding: 4px;
}

.detail-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}

.detail-icon {
  width: 64px;
  height: 64px;
  border-radius: var(--radius-lg);
  background: var(--tool-icon-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-primary);
  flex-shrink: 0;
  overflow: hidden;
}

.icon-image {
  width: 100%;
  height: 100%;
}

.detail-title {
  flex: 1;
  min-width: 0;

  h2 {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--color-text-primary);
  }
}

.detail-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.detail-author {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.detail-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.detail-description {
  font-size: 14px;
  color: var(--color-text-secondary);
  line-height: 1.6;
  margin-bottom: 20px;
}

.detail-tabs {
  :deep(.el-tabs__header) {
    background: var(--glass-bg-light);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: 4px;
    margin-bottom: 16px;
  }
}

.section-list {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.skill-list,
.command-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.skill-item,
.command-item {
  padding: 12px 14px;
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.skill-name,
.command-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
  margin-bottom: 4px;
}

.skill-desc {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.command-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.config-form {
  :deep(.el-form-item__label) {
    color: var(--color-text-secondary);
  }
}
</style>
