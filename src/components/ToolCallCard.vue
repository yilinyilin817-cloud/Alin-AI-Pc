<template>
  <div class="tool-call-card">
    <div class="tool-header" @click="expanded = !expanded">
      <div class="tool-info">
        <div class="tool-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>
          </svg>
        </div>
        <div class="tool-details">
          <div class="tool-name">{{ toolName }}</div>
          <div class="tool-status" :class="statusClass">
            <span class="status-dot"></span>
            {{ statusText }}
          </div>
        </div>
      </div>
      <div class="tool-expand">
        <svg :class="{ rotated: expanded }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="6 9 12 15 18 9"></polyline>
        </svg>
      </div>
    </div>
    
    <div v-if="expanded" class="tool-content">
      <div v-if="arguments" class="tool-section">
        <div class="section-title">参数</div>
        <pre class="section-content">{{ formattedArguments }}</pre>
      </div>
      
      <div v-if="result" class="tool-section">
        <div class="section-title">结果</div>
        <pre class="section-content">{{ result }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';

interface Props {
  toolName: string;
  status: 'pending' | 'running' | 'success' | 'error';
  arguments?: Record<string, any>;
  result?: string;
}

const props = defineProps<Props>();

const expanded = ref(false);

const statusClass = computed(() => `status-${props.status}`);

const statusText = computed(() => {
  const map = {
    pending: '等待执行',
    running: '执行中...',
    success: '执行成功',
    error: '执行失败'
  };
  return map[props.status];
});

const formattedArguments = computed(() => {
  if (!props.arguments) return '';
  return JSON.stringify(props.arguments, null, 2);
});
</script>

<style scoped lang="scss">
.tool-call-card {
  margin: 8px 0;
  border: 1px solid var(--tool-border);
  border-radius: 8px;
  background: var(--tool-bg);
  overflow: hidden;
  transition: all 0.2s;

  &:hover {
    border-color: var(--tool-border-hover);
    box-shadow: var(--tool-shadow-hover);
  }
}

.tool-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  cursor: pointer;
  user-select: none;
  transition: background 0.2s;

  &:hover {
    background: var(--tool-header-hover-bg);
  }
}

.tool-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.tool-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: var(--tool-icon-bg);
  color: var(--color-primary);

  svg {
    width: 18px;
    height: 18px;
  }
}

.tool-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tool-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
}

.tool-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  &.status-pending {
    color: var(--color-text-secondary);
  }

  &.status-running {
    color: var(--color-warning);
    
    .status-dot {
      animation: pulse 1.5s infinite;
    }
  }

  &.status-success {
    color: var(--color-success);
  }

  &.status-error {
    color: var(--color-danger);
  }
}

.tool-expand {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-secondary);
  transition: transform 0.2s;

  svg {
    width: 16px;
    height: 16px;
    transition: transform 0.2s;

    &.rotated {
      transform: rotate(180deg);
    }
  }
}

.tool-content {
  border-top: 1px solid var(--tool-content-border);
  padding: 12px 16px;
  background: var(--tool-content-bg);
}

.tool-section {
  margin-bottom: 12px;

  &:last-child {
    margin-bottom: 0;
  }
}

.section-title {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.section-content {
  margin: 0;
  padding: 10px 12px;
  font-family: var(--font-family-code);
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text-primary);
  background: var(--tool-section-bg);
  border-radius: 6px;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}
</style>
