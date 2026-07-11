<script setup lang="ts">
import type { PersonaDefinition } from "@/types";

defineProps<{
  persona: PersonaDefinition;
  selected?: boolean;
}>();

</script>

<template>
  <el-card
    class="persona-card"
    :class="{ selected }"
    shadow="hover"
  >
    <div class="card-body">
      <el-avatar :size="56" class="avatar">
        {{ persona.name[0] }}
      </el-avatar>
      <div class="info">
        <h3 class="name">{{ persona.name }}</h3>
        <div class="tags">
          <el-tag
            v-for="trait in persona.personality"
            :key="trait"
            size="small"
            effect="plain"
          >
            {{ trait }}
          </el-tag>
        </div>
        <p class="greeting">{{ persona.greeting }}</p>
        <div class="meta">
          <el-tag size="small" type="info">{{ persona.llm.provider }}</el-tag>
        </div>
      </div>
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.persona-card {
  position: relative;
  overflow: hidden;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
  transition: all var(--transition-base);

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border-hover);
    transform: translateY(-2px);
  }

  &.selected {
    border-color: var(--color-primary);
    box-shadow: var(--glass-shadow), var(--glass-glow);
  }

  :deep(.el-card__body) {
    background: transparent;
  }
}

.card-body {
  position: relative;
  z-index: 1;
  display: flex;
  gap: 16px;
}

.avatar {
  background: linear-gradient(135deg, var(--color-primary), var(--color-accent));
  color: var(--color-text-inverse);
  font-size: 22px;
  flex-shrink: 0;
  box-shadow: var(--shadow-avatar);
}

.name {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 6px;
}

.tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}

.greeting {
  font-size: 13px;
  color: var(--color-text-secondary);
  margin-bottom: 8px;
  line-height: 1.4;
}

.meta {
  display: flex;
  gap: 4px;
}
</style>
