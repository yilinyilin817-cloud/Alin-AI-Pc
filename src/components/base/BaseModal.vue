<script setup lang="ts">
interface Props {
  modelValue: boolean;
  title?: string;
  width?: string;
  showClose?: boolean;
  closeOnClickModal?: boolean;
  closeOnPressEscape?: boolean;
}

withDefaults(defineProps<Props>(), {
  width: "480px",
  showClose: true,
  closeOnClickModal: true,
  closeOnPressEscape: true,
});

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

function onUpdate(value: boolean) {
  emit("update:modelValue", value);
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="title"
    :width="width"
    :show-close="showClose"
    :close-on-click-modal="closeOnClickModal"
    :close-on-press-escape="closeOnPressEscape"
    class="base-modal"
    align-center
    destroy-on-close
    @update:model-value="onUpdate"
  >
    <div class="base-modal__body">
      <slot />
    </div>
    <template v-if="$slots.footer" #footer>
      <div class="base-modal__footer">
        <slot name="footer" />
      </div>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.base-modal {
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.base-modal__body {
  padding: 8px 0;
  color: var(--color-text-primary);
}

.base-modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
}
</style>
