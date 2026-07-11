<script setup lang="ts">
import { computed } from "vue";
import type { EmotionTag } from "@/types";

const props = defineProps<{
  emotion: EmotionTag;
  size?: "small" | "medium";
}>();

const emojiMap: Record<string, string> = {
  happy: "😊",
  sad: "😢",
  angry: "😠",
  fearful: "😨",
  surprised: "😲",
  disgusted: "🤢",
  neutral: "😐",
};

const hue = computed(() => {
  const v = props.emotion.valence;
  return Math.round(((v + 1) / 2) * 120);
});

const ringSize = computed(() =>
  props.size === "small" ? 32 : 48,
);
</script>

<template>
  <div class="emotion-indicator" :class="size ?? 'medium'">
    <div
      class="emotion-ring"
      :style="{
        width: ringSize + 'px',
        height: ringSize + 'px',
        background: `conic-gradient(hsl(${hue}, 70%, 60%) ${emotion.arousal * 360}deg, var(--color-border) 0deg)`,
      }"
    >
      <span class="emoji">{{ emojiMap[emotion.emotion] ?? "😐" }}</span>
    </div>
    <span v-if="size !== 'small'" class="label">{{ emotion.emotion }}</span>
  </div>
</template>

<style scoped lang="scss">
.emotion-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px;
  border-radius: 20px;
  background: var(--glass-bg-light);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid var(--glass-border);
}

.emotion-ring {
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  box-shadow: var(--glow-primary-sm);

  &::after {
    content: "";
    position: absolute;
    inset: 3px;
    border-radius: 50%;
    background: var(--glass-bg);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
}

.emoji {
  position: relative;
  z-index: 1;
  font-size: 16px;
  filter: var(--glow-primary-xs);
}

.small .emoji {
  font-size: 14px;
}

.label {
  font-size: 12px;
  color: var(--color-text-secondary);
  text-transform: capitalize;
}
</style>
