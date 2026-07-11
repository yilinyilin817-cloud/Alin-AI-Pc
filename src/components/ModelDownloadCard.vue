<script setup lang="ts">
import type { ModelConfig } from "@/types";
import { Close } from "@element-plus/icons-vue";

defineProps<{
  model: ModelConfig;
}>();

const emit = defineEmits<{
  download: [id: string];
  activate: [id: string];
  test: [id: string];
  delete: [id: string];
  cancel: [id: string];
}>();

const statusLabel: Record<string, string> = {
  not_downloaded: "未下载",
  downloading: "下载中",
  downloaded: "已下载",
  active: "使用中",
};
</script>

<template>
  <el-card class="model-card" :class="{ 'is-active': model.isActive }" shadow="hover">
    <div class="card-header">
      <h3 class="model-name">{{ model.name }}</h3>
      <el-tag
        :type="model.isActive ? 'success' : model.status === 'downloading' ? 'warning' : 'info'"
        size="small"
      >
        {{ statusLabel[model.status] }}
      </el-tag>
    </div>

    <p class="model-desc">{{ model.description }}</p>

    <div class="model-meta">
      <el-tag v-if="model.vramRequired" size="small" effect="plain">显存 {{ model.vramRequired }}</el-tag>
      <el-tag v-if="model.size" size="small" effect="plain">{{ model.size }}</el-tag>
    </div>

    <el-progress
      v-if="model.status === 'downloading'"
      :percentage="model.progress ?? 0"
      :stroke-width="6"
      :striped="true"
      :striped-flow="true"
      class="progress"
    />

    <div class="card-actions">
      <!-- 未下载：下载按钮 -->
      <el-button
        v-if="model.status === 'not_downloaded'"
        type="primary"
        size="small"
        @click="emit('download', model.id)"
      >
        下载
      </el-button>

      <!-- 下载中：取消按钮 -->
      <el-button
        v-else-if="model.status === 'downloading'"
        type="danger"
        size="small"
        plain
        :icon="Close"
        @click="emit('cancel', model.id)"
      >
        取消下载
      </el-button>

      <!-- 已下载/使用中：启用/测试/删除 -->
      <template v-else-if="model.status === 'downloaded' || model.status === 'active'">
        <el-button
          v-if="!model.isActive"
          type="primary"
          size="small"
          plain
          @click="emit('activate', model.id)"
        >
          启用
        </el-button>

        <el-button
          v-if="model.isActive"
          size="small"
          type="success"
          plain
          @click="emit('test', model.id)"
        >
          功能测试
        </el-button>

        <el-button
          size="small"
          type="danger"
          plain
          @click="emit('delete', model.id)"
        >
          删除
        </el-button>
      </template>
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.model-card {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
  transition: all var(--transition);

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border-hover);
    transform: translateY(-2px);
  }

  &.is-active {
    border-color: var(--color-primary);
  }

  :deep(.el-card__body) {
    background: transparent;
  }
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.model-name {
  font-size: 15px;
  font-weight: 600;
}

.model-desc {
  font-size: 13px;
  color: var(--color-text-secondary);
  margin-bottom: 12px;
  line-height: 1.4;
}

.model-meta {
  display: flex;
  gap: 6px;
  margin-bottom: 12px;
}

.progress {
  margin-bottom: 12px;
}

.card-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
