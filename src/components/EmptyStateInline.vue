<script setup lang="ts">
defineProps<{
  title?: string;
  description?: string;
  icon?: string;
}>();
</script>

<template>
  <div class="empty-state-inline">
    <div class="icon">{{ icon ?? "📭" }}</div>
    <div class="content">
      <h3 class="title">{{ title ?? "暂无内容" }}</h3>
      <p v-if="description" class="description">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="actions">
      <slot name="actions" />
    </div>
    <slot />
  </div>
</template>

<style scoped lang="scss">
.empty-state-inline {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-sm);
  min-height: 160px;
  padding: var(--spacing-xl) var(--spacing-lg);
  text-align: center;
  border-radius: var(--radius-md);
  border: 1px dashed var(--color-border);
  background: var(--color-bg-surface);
  color: var(--color-text-muted);
}

.icon {
  font-size: 32px;
  filter: var(--icon-glow);
}

.title {
  font-size: var(--font-size-sm);
  font-weight: 500;
  color: var(--color-text-secondary);
  margin: 0;
}

.description {
  font-size: var(--font-size-xs);
  color: var(--color-text-muted);
  max-width: 320px;
  line-height: 1.5;
  margin: 0;
}

.actions {
  display: flex;
  gap: var(--spacing-sm);
  flex-wrap: wrap;
  justify-content: center;
  margin-top: var(--spacing-xs);
}
</style>
