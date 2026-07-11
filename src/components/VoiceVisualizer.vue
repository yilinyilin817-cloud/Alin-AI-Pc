<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";

const props = defineProps<{
  data: number[];
  active?: boolean;
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let animFrame: number | null = null;

function getCssColor(name: string, fallback: string): string {
  if (typeof window === "undefined") return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

function draw() {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const { width, height } = canvas;
  ctx.clearRect(0, 0, width, height);

  const barCount = props.data.length;
  const barWidth = width / barCount - 2;
  const color = props.active
    ? getCssColor("--color-primary", "#7c6bf0")
    : getCssColor("--color-text-muted", "#9ca3b8");

  props.data.forEach((val, i) => {
    const barHeight = val * height * 0.8;
    const x = i * (barWidth + 2);
    const y = (height - barHeight) / 2;

    ctx.fillStyle = color;
    ctx.globalAlpha = props.active ? 0.7 + val * 0.3 : 0.4;
    ctx.beginPath();
    ctx.roundRect(x, y, barWidth, barHeight, 2);
    ctx.fill();
  });

  ctx.globalAlpha = 1;
}

function animate() {
  draw();
  if (props.active) {
    animFrame = requestAnimationFrame(animate);
  }
}

watch(() => [props.data, props.active], () => {
  if (animFrame) cancelAnimationFrame(animFrame);
  if (props.active) animate();
  else draw();
}, { deep: true });

onMounted(() => {
  if (props.active) animate();
  else draw();
});

onUnmounted(() => {
  if (animFrame) cancelAnimationFrame(animFrame);
});
</script>

<template>
  <canvas ref="canvasRef" class="voice-visualizer" width="200" height="48" />
</template>

<style scoped lang="scss">
.voice-visualizer {
  width: 100%;
  height: 48px;
  border-radius: var(--radius-md);
  background: var(--glass-bg-light);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid var(--glass-border);
  padding: 4px;
}
</style>
