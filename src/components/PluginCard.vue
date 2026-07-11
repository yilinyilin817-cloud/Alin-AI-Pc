<script setup lang="ts">
import { Box, Delete } from "@element-plus/icons-vue";
import type { InstalledPlugin } from "@/types";

const props = defineProps<{
  plugin: InstalledPlugin;
}>();

const emit = defineEmits<{
  (e: "click", plugin: InstalledPlugin): void;
  (e: "uninstall", id: string): void;
  (e: "toggle", id: string): void;
}>();

const manifest = props.plugin.manifest;
</script>

<template>
  <el-card class="plugin-card" shadow="hover" @click="emit('click', plugin)">
    <div class="card-body">
      <div class="plugin-icon">
        <el-image v-if="manifest.icon" :src="manifest.icon" fit="cover" class="icon-image" />
        <el-icon v-else :size="28"><Box /></el-icon>
      </div>
      <div class="plugin-info">
        <div class="plugin-header">
          <h3 class="plugin-name">{{ manifest.name }}</h3>
          <el-tag size="small" type="info">v{{ manifest.version }}</el-tag>
        </div>
        <p class="plugin-desc">{{ manifest.description }}</p>
        <div class="plugin-meta">
          <span v-if="manifest.author" class="plugin-author">{{ manifest.author }}</span>
          <span v-else class="plugin-author">未知作者</span>
          <div class="permission-tags">
            <el-tag
              v-for="perm in manifest.permissions"
              :key="perm"
              size="small"
              type="warning"
              effect="plain"
            >
              {{ perm }}
            </el-tag>
            <el-tag v-if="!manifest.permissions.length" size="small" effect="plain">本地</el-tag>
          </div>
        </div>
      </div>
      <div class="plugin-actions" @click.stop>
        <el-switch
          :model-value="plugin.enabled"
          @change="emit('toggle', plugin.id)"
        />
        <el-button
          type="danger"
          link
          size="small"
          :icon="Delete"
          @click="emit('uninstall', plugin.id)"
        >
          卸载
        </el-button>
      </div>
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.plugin-card {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
  cursor: pointer;
  transition: all var(--transition-base);

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border-hover);
    transform: translateY(-2px);
  }

  :deep(.el-card__body) {
    background: transparent;
    padding: 16px;
  }
}

.card-body {
  display: flex;
  align-items: flex-start;
  gap: 14px;
}

.plugin-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
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

.plugin-info {
  flex: 1;
  min-width: 0;
}

.plugin-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.plugin-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.plugin-desc {
  font-size: 13px;
  color: var(--color-text-secondary);
  line-height: 1.4;
  margin-bottom: 10px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.plugin-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.plugin-author {
  font-size: 12px;
  color: var(--color-text-muted);
}

.permission-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.plugin-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 10px;
  flex-shrink: 0;
  margin-left: 8px;
}
</style>
