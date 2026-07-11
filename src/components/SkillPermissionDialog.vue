<script setup lang="ts">
import type { SkillDefinition } from "@/types";

defineProps<{
  skill: SkillDefinition;
  visible: boolean;
}>();

const emit = defineEmits<{
  "update:visible": [val: boolean];
  approve: [];
  reject: [];
}>();
</script>

<template>
  <el-dialog
    :model-value="visible"
    title="技能权限审批"
    width="420px"
    @update:model-value="emit('update:visible', $event)"
  >
    <div class="dialog-body">
      <p>
        技能 <strong>{{ skill.name }}</strong> 请求以下权限：
      </p>
      <div class="permissions">
        <el-tag
          v-for="perm in skill.permissions"
          :key="perm"
          type="warning"
          size="small"
        >
          {{ perm }}
        </el-tag>
        <el-tag v-if="!skill.permissions.length" size="small">无特殊权限</el-tag>
      </div>
      <p class="hint">审批模式：{{ skill.approvalMode }}</p>
    </div>

    <template #footer>
      <el-button @click="emit('reject'); emit('update:visible', false)">
        拒绝
      </el-button>
      <el-button
        type="primary"
        @click="emit('approve'); emit('update:visible', false)"
      >
        允许
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.dialog-body {
  line-height: 1.6;
}

.permissions {
  display: flex;
  gap: 6px;
  margin: 12px 0;
  flex-wrap: wrap;
}

.hint {
  font-size: 12px;
  color: var(--color-text-muted);
}

:deep(.el-dialog) {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);

  .el-dialog__header {
    background: var(--glass-gradient);
    border-bottom: 1px solid var(--glass-border);
    padding: 16px 20px;
  }

  .el-dialog__body {
    background: transparent;
    padding: 20px;
  }

  .el-dialog__footer {
    background: transparent;
    padding: 12px 20px;
    border-top: 1px solid var(--glass-border);
  }
}
</style>
