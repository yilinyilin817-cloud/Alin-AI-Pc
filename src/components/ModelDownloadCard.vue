<script setup lang="ts">
import type { ModelConfig } from "@/types";
import { computed } from "vue";
import { Close, Download, FolderOpened, Link, Microphone, Cpu, Service, ChatDotRound } from "@element-plus/icons-vue";

const props = defineProps<{
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

const typeLabel: Record<string, string> = {
  llm: "对话模型",
  asr: "语音识别",
  tts: "语音合成",
  embedding: "向量检索",
  emotion: "情绪识别",
};

const typeIcon = computed(() => {
  if (props.model.modelType === "tts") return Microphone;
  if (props.model.modelType === "embedding") return Cpu;
  if (props.model.modelType === "asr") return Service;
  return ChatDotRound;
});

const statusType = computed(() => {
  if (props.model.isActive) return "success";
  if (props.model.status === "downloading") return "warning";
  if (props.model.status === "downloaded") return "success";
  return "info";
});
</script>

<template>
  <el-card class="model-card" :class="{ 'is-active': model.isActive }" shadow="hover">
    <div class="card-header">
      <div class="title-group">
        <el-icon class="type-icon"><component :is="typeIcon" /></el-icon>
        <div>
          <h3 class="model-name">{{ model.name }}</h3>
          <span class="model-type">{{ typeLabel[model.modelType] || model.modelType }}</span>
        </div>
      </div>
      <el-tag :type="statusType" size="small">
        {{ statusLabel[model.status] }}
      </el-tag>
    </div>

    <p class="model-desc">{{ model.description }}</p>

    <div class="model-meta">
      <el-tag v-if="model.source" size="small" effect="plain">
        <el-icon><Link /></el-icon>
        {{ model.source }}
      </el-tag>
      <el-tag v-if="model.vramRequired" size="small" effect="plain">显存 {{ model.vramRequired }}</el-tag>
      <el-tag v-if="model.size" size="small" effect="plain">{{ model.size }}</el-tag>
    </div>

    <p v-if="model.runtimeHint" class="runtime-hint">{{ model.runtimeHint }}</p>

    <div class="deploy-info">
      <div v-if="model.installCommand" class="deploy-row">
        <el-icon><Download /></el-icon>
        <code>{{ model.installCommand }}</code>
      </div>
      <div v-if="model.localPath" class="deploy-row">
        <el-icon><FolderOpened /></el-icon>
        <span>{{ model.localPath }}</span>
      </div>
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
          v-if="model.isActive || model.status === 'downloaded'"
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
  gap: 12px;
}

.title-group {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.type-icon {
  width: 32px;
  height: 32px;
  flex: 0 0 32px;
  border-radius: var(--radius-md);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--color-primary-subtle);
  color: var(--color-primary);
}

.model-name {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
  line-height: 1.25;
}

.model-type {
  font-size: 12px;
  color: var(--color-text-muted);
}

.model-desc {
  font-size: 13px;
  color: var(--color-text-secondary);
  margin-bottom: 12px;
  line-height: 1.4;
}

.model-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}

.runtime-hint {
  margin: 0 0 12px;
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.5;
}

.deploy-info {
  display: grid;
  gap: 6px;
  margin-bottom: 12px;
}

.deploy-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  font-size: 11px;
  color: var(--color-text-muted);

  code,
  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  code {
    padding: 2px 5px;
    border-radius: 5px;
    background: var(--glass-bg-light);
    color: var(--color-text-secondary);
  }
}

.progress {
  margin-bottom: 12px;
}

.card-actions {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
