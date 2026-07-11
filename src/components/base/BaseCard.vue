<script setup lang="ts">
interface Props {
  hoverable?: boolean;
}

withDefaults(defineProps<Props>(), {
  hoverable: true,
});
</script>

<template>
  <div class="base-card" :class="{ 'is-hoverable': hoverable }">
    <div v-if="$slots.header" class="base-card__header">
      <slot name="header" />
    </div>
    <div class="base-card__body">
      <slot />
    </div>
    <div v-if="$slots.footer" class="base-card__footer">
      <slot name="footer" />
    </div>
  </div>
</template>

<style scoped lang="scss">
.base-card {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
  transition: all var(--transition-base);
  overflow: hidden;

  &.is-hoverable:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border-hover);
    transform: translateY(-2px);
    box-shadow: var(--glass-shadow), var(--glass-glow);
  }
}

.base-card__header {
  padding: 14px 20px;
  background: var(--glass-gradient);
  border-bottom: 1px solid var(--glass-border);
  font-weight: 600;
  font-size: var(--font-size-md);
  color: var(--color-text-primary);
}

.base-card__body {
  padding: 20px;
  color: var(--color-text-primary);
}

.base-card__footer {
  padding: 12px 20px;
  border-top: 1px solid var(--glass-border);
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
}
</style>
