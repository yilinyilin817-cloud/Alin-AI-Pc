<script setup lang="ts">
defineProps<{
  title?: string;
  description?: string;
  icon?: string;
}>();
</script>

<template>
  <div class="empty-state">
    <div class="empty-state__content">
      <div class="icon">{{ icon ?? "📭" }}</div>
      <h3 class="title">{{ title ?? "暂无内容" }}</h3>
      <p v-if="description" class="description">{{ description }}</p>
      <div v-if="$slots.actions" class="actions">
        <slot name="actions" />
      </div>
      <slot />
    </div>
  </div>
</template>

<style scoped lang="scss">
.empty-state {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 280px;
  padding: var(--spacing-2xl) var(--spacing-xl);
  text-align: center;
  border-radius: var(--radius-xl);
  border: 1px solid var(--glass-border);
  background: var(--glass-bg-light);
  overflow: hidden;
  box-shadow: var(--glass-shadow);
}

.empty-state__content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.icon {
  font-size: 56px;
  margin-bottom: var(--spacing-md);
  filter: var(--icon-glow);
  animation: float 3s ease-in-out infinite;
}

.title {
  font-size: var(--font-size-xl);
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0 0 var(--spacing-sm);
  text-shadow: var(--shadow-text);
}

.description {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  max-width: 360px;
  line-height: 1.6;
  margin: 0 0 var(--spacing-md);
}

.actions {
  display: flex;
  gap: var(--spacing-sm);
  flex-wrap: wrap;
  justify-content: center;
}

@keyframes float {
  0%, 100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-8px);
  }
}

</style>
