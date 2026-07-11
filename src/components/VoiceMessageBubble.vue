<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { VideoPlay, VideoPause } from "@element-plus/icons-vue";
import type { ContentPart } from "@/types";
import VoiceVisualizer from "./VoiceVisualizer.vue";

interface Props {
  part: ContentPart & { type: "audio_bytes" };
  position?: "left" | "right";
}

const props = withDefaults(defineProps<Props>(), {
  position: "left",
});

const isPlaying = ref(false);
const audioRef = ref<HTMLAudioElement | null>(null);
const blobUrl = ref<string | null>(null);
const waveformData = ref<number[]>([]);

const durationText = computed(() => {
  const seconds = Math.round(props.part.duration ?? 0);
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
});

const transcriptText = computed(() => {
  return props.part.transcript?.trim() || "无转写";
});

function generateWaveform() {
  const data = new Array(32).fill(0);
  const bytes = props.part.data;
  if (!bytes.length) {
    return data.map(() => 0.2 + Math.random() * 0.3);
  }
  // 从音频 bytes 采样生成伪波形，并混入少量随机让静止时更自然
  for (let i = 0; i < data.length; i++) {
    const idx = Math.floor((i / data.length) * bytes.length);
    const val = Math.abs(bytes[idx] - 128) / 128;
    data[i] = Math.min(1, Math.max(0.15, val * 0.8 + Math.random() * 0.15));
  }
  return data;
}

function ensureAudio() {
  if (audioRef.value) return;
  const mime = props.part.mime || "audio/wav";
  const blob = new Blob([props.part.data], { type: mime });
  blobUrl.value = URL.createObjectURL(blob);
  const audio = new Audio(blobUrl.value);
  audio.addEventListener("ended", () => {
    isPlaying.value = false;
  });
  audio.addEventListener("pause", () => {
    isPlaying.value = false;
  });
  audio.addEventListener("play", () => {
    isPlaying.value = true;
  });
  audioRef.value = audio;
}

async function togglePlay() {
  ensureAudio();
  const audio = audioRef.value;
  if (!audio) return;
  if (isPlaying.value) {
    audio.pause();
  } else {
    try {
      await audio.play();
    } catch (e) {
      console.warn("play audio failed:", e);
    }
  }
}

onMounted(() => {
  waveformData.value = generateWaveform();
});

onUnmounted(() => {
  if (audioRef.value) {
    audioRef.value.pause();
    audioRef.value = null;
  }
  if (blobUrl.value) {
    URL.revokeObjectURL(blobUrl.value);
    blobUrl.value = null;
  }
});
</script>

<template>
  <div class="voice-message-bubble" :class="position">
    <button class="voice-play-btn" :title="isPlaying ? '暂停' : '播放'" @click="togglePlay">
      <el-icon :size="18">
        <VideoPause v-if="isPlaying" />
        <VideoPlay v-else />
      </el-icon>
    </button>

    <div class="voice-body">
      <div class="voice-waveform">
        <VoiceVisualizer :data="waveformData" :active="isPlaying" />
      </div>
      <div class="voice-meta">
        <span class="voice-duration">{{ durationText }}</span>
        <span class="voice-transcript">{{ transcriptText }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.voice-message-bubble {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-lg);
  background: var(--glass-bg-light);
  border: 1px solid var(--glass-border);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  max-width: 320px;

  &.right {
    flex-direction: row-reverse;
    background: var(--voice-message-right-bg);
  }
}

.voice-play-btn {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: var(--color-primary);
  color: var(--color-text-inverse);
  cursor: pointer;
  transition: all var(--transition-base);
  box-shadow: var(--shadow-primary-sm);

  &:hover {
    background: var(--color-primary-light);
    transform: scale(1.05);
    box-shadow: var(--shadow-primary-hover);
  }

  &:active {
    transform: scale(0.96);
  }
}

.voice-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.voice-waveform {
  width: 180px;
  height: 40px;

  :deep(.voice-visualizer) {
    width: 100%;
    height: 100%;
  }
}

.voice-meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.voice-duration {
  font-variant-numeric: tabular-nums;
  min-width: 32px;
}

.voice-transcript {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  opacity: 0.85;
}
</style>
