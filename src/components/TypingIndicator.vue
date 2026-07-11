<script setup lang="ts">
interface Props {
  text?: string;
}

withDefaults(defineProps<Props>(), {
  text: "AI 正在思考…",
});
</script>

<template>
  <div class="typing-indicator">
    <div class="typing-dots">
      <span class="dot"></span>
      <span class="dot"></span>
      <span class="dot"></span>
    </div>
    <span class="typing-text">{{ text }}</span>
  </div>
</template>

<style scoped lang="scss">
.typing-indicator {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-lg);
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  box-shadow: var(--glass-shadow);
  position: relative;
  overflow: hidden;

  // 渐变流光
  &::before {
    content: "";
    position: absolute;
    inset: 0;
    background: var(--typing-shimmer-gradient);
    transform: translateX(-100%);
    animation: shimmer 2.4s ease-in-out infinite;
    pointer-events: none;
  }
}

.typing-dots {
  display: flex;
  align-items: center;
  gap: 4px;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-primary);
  animation: bounce 1.2s ease-in-out infinite;

  &:nth-child(1) {
    animation-delay: 0s;
  }

  &:nth-child(2) {
    animation-delay: 0.15s;
  }

  &:nth-child(3) {
    animation-delay: 0.3s;
  }
}

.typing-text {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  font-weight: 500;
  letter-spacing: 0.02em;
}

@keyframes bounce {
  0%, 60%, 100% {
    transform: translateY(0);
    opacity: 0.6;
  }
  30% {
    transform: translateY(-6px);
    opacity: 1;
  }
}

@keyframes shimmer {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
}
</style>
