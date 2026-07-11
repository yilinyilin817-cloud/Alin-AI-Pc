<script setup lang="ts">
import { ref, computed, onBeforeUnmount, watch } from "vue";
import { Loading, CircleCheck, CircleClose, VideoPlay } from "@element-plus/icons-vue";
import { isTauri } from "@/api/env";

type StepStatus = "pending" | "active" | "done" | "failed";

interface TestStep {
  key: string;
  label: string;
  icon: string;
  status: StepStatus;
  detail?: string;
}

const LLM_STEPS: Omit<TestStep, "status" | "detail">[] = [
  { key: "load_config", label: "加载模型配置", icon: "⚙️" },
  { key: "connect_backend", label: "连接推理后端", icon: "🔗" },
  { key: "run_inference", label: "执行测试推理", icon: "🧠" },
  { key: "verify_output", label: "验证输出结果", icon: "✅" },
];

const WORKER_STEPS: Omit<TestStep, "status" | "detail">[] = [
  { key: "load_config", label: "加载引擎配置", icon: "⚙️" },
  { key: "start_worker", label: "启动工作进程", icon: "🚀" },
  { key: "ping_test", label: "通信与状态检查", icon: "📡" },
  { key: "run_inference", label: "执行测试推理", icon: "🧠" },
  { key: "verify_output", label: "验证结果", icon: "✅" },
];

const OLLAMA_STEPS: Omit<TestStep, "status" | "detail">[] = [
  { key: "load_config", label: "加载模型配置", icon: "⚙️" },
  { key: "connect_ollama", label: "连接 Ollama 服务", icon: "🔗" },
  { key: "load_model", label: "加载模型", icon: "📦" },
  { key: "run_inference", label: "执行推理", icon: "🧠" },
  { key: "verify_output", label: "验证输出", icon: "✅" },
];

const props = defineProps<{
  visible: boolean;
  modelId: string;
  modelName: string;
  modelType: string;
  providerId: string;
}>();

const emit = defineEmits<{
  "update:visible": [value: boolean];
  done: [result: { success: boolean; message: string; latencyMs?: number; audioData?: string; audioMime?: string }];
}>();

const steps = ref<TestStep[]>([]);
const elapsed = ref(0);
const resultMessage = ref("");
const isSuccess = ref(false);
const testLatency = ref<number | undefined>();
const testStarted = ref(false);
const testFinished = ref(false);
const resultAudioData = ref<string>("");
const resultAudioMime = ref<string>("audio/wav");
const audioPlaying = ref(false);
let timer: ReturnType<typeof setInterval> | null = null;
let audioElement: HTMLAudioElement | null = null;
const eventUnlisten = ref<(() => void) | null>(null);

function initSteps() {
  let base: Omit<TestStep, "status" | "detail">[];
  if (props.providerId.startsWith("ollama/")) {
    base = OLLAMA_STEPS;
  } else if (["asr", "tts", "embedding"].includes(props.modelType)) {
    base = WORKER_STEPS;
  } else {
    base = LLM_STEPS;
  }
  steps.value = base.map((s) => ({ ...s, status: "pending" as StepStatus }));
}

function resetState() {
  testStarted.value = false;
  testFinished.value = false;
  resultMessage.value = "";
  testLatency.value = undefined;
  elapsed.value = 0;
  isSuccess.value = false;
  resultAudioData.value = "";
  stopAudio();
  initSteps();
}

function setStepStatus(key: string, status: StepStatus, detail?: string) {
  const step = steps.value.find((s) => s.key === key);
  if (step) {
    step.status = status;
    if (detail) step.detail = detail;
  }
}

function advanceAllTo(key: string, targetStatus: StepStatus) {
  let reached = false;
  for (const s of steps.value) {
    if (s.key === key) reached = true;
    s.status = reached ? s.status : targetStatus === "failed" ? s.status : targetStatus;
  }
}

function simulateSteps(durationPerStep: number[]) {
  const stepKeys = steps.value.map((s) => s.key);
  let offset = 0;
  for (let i = 0; i < stepKeys.length; i++) {
    const delay = durationPerStep[i] ?? 800;
    setTimeout(() => {
      if (testFinished.value) return;
      setStepStatus(stepKeys[i], "active");
      setTimeout(() => {
        if (testFinished.value) return;
        setStepStatus(stepKeys[i], "done");
      }, Math.min(delay - 200, 600));
    }, offset);
    offset += delay;
  }
}

async function listenTauriEvents() {
  if (!isTauri()) return;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{
      modelId: string;
      step: string;
      status: StepStatus;
      detail?: string;
      latencyMs?: number;
      message?: string;
      success?: boolean;
      done?: boolean;
      audioData?: string;
      audioMime?: string;
    }>("model-test:step", (event) => {
      const { modelId, step, status, detail, latencyMs, message, done, success, audioData, audioMime } =
        event.payload;
      if (modelId !== props.modelId) return;

      if (step && status) {
        advanceAllTo(step, "done");
        setStepStatus(step, status, detail);
      }

      if (audioData) {
        resultAudioData.value = audioData;
        resultAudioMime.value = audioMime ?? "audio/wav";
      }

      if (done) {
        finishTest(
          success ?? true,
          message ?? "测试完成",
          latencyMs,
          audioData,
          audioMime,
        );
      }
    });
    eventUnlisten.value = unlisten;
  } catch (e) {
    console.warn("ModelTestDialog: failed to listen to model-test:step", e);
  }
}

function finishTest(success: boolean, message: string, latencyMs?: number, audioData?: string, audioMime?: string) {
  testFinished.value = true;
  isSuccess.value = success;
  resultMessage.value = message;
  testLatency.value = latencyMs;
  if (audioData) {
    resultAudioData.value = audioData;
    resultAudioMime.value = audioMime ?? "audio/wav";
  }
  if (timer) { clearInterval(timer); timer = null; }
  for (const s of steps.value) {
    if (s.status === "pending" || s.status === "active") {
      s.status = success ? "done" : "failed";
    }
  }
}

async function runTest() {
  resetState();
  testStarted.value = true;

  timer = setInterval(() => { elapsed.value += 100; }, 100);
  await listenTauriEvents();

  const hasEventSupport = eventUnlisten.value !== null;
  if (!hasEventSupport) {
    const stepCount = steps.value.length;
    simulateSteps(new Array(stepCount).fill(0).map(() => 600 + Math.floor(Math.random() * 400)));
  }

  try {
    const { testModel } = await import("@/api/model");
    const start = Date.now();
    const res = await testModel(props.modelId);
    const latencyMs = Date.now() - start;

    if (res.audioData) {
      resultAudioData.value = res.audioData;
      resultAudioMime.value = res.audioMime ?? "audio/wav";
    }

    if (!testFinished.value) {
      finishTest(res.success, res.message, latencyMs, res.audioData, res.audioMime);
    } else if (!testLatency.value) {
      testLatency.value = latencyMs;
    }

    emit("done", { ...res, latencyMs });
  } catch (e: any) {
    if (!testFinished.value) {
      finishTest(false, e?.message ?? String(e));
    }
    emit("done", { success: false, message: e?.message ?? String(e) });
  }
}

watch(
  () => props.visible,
  (v) => { if (v) runTest(); },
);

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
  eventUnlisten.value?.();
  stopAudio();
});

function handleClose() {
  if (timer) clearInterval(timer);
  eventUnlisten.value?.();
  stopAudio();
  emit("update:visible", false);
}

const progressPercent = computed(() => {
  const total = steps.value.length;
  if (!total) return 0;
  const done = steps.value.filter(
    (s) => s.status === "done" || s.status === "failed",
  ).length;
  return Math.round((done / total) * 100);
});

const elapsedDisplay = computed(() => {
  const s = Math.floor(elapsed.value / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  return `${m}m ${s % 60}s`;
});

const statusColor = computed(() => (isSuccess.value ? "#67c23a" : "#f56c6c"));

const audioSrc = computed(() => {
  if (!resultAudioData.value) return "";
  const mime = resultAudioMime.value || "audio/wav";
  return `data:${mime};base64,${resultAudioData.value}`;
});

function playAudio() {
  if (!audioSrc.value) return;
  if (!audioElement) {
    audioElement = new Audio(audioSrc.value);
    audioElement.onended = () => { audioPlaying.value = false; };
    audioElement.onerror = () => { audioPlaying.value = false; };
  } else {
    audioElement.src = audioSrc.value;
  }
  audioElement.currentTime = 0;
  audioElement.play().then(() => {
    audioPlaying.value = true;
  }).catch((e) => {
    console.warn("Audio playback failed:", e);
    audioPlaying.value = false;
  });
}

function stopAudio() {
  if (audioElement) {
    audioElement.pause();
    audioElement.currentTime = 0;
    audioElement = null;
  }
  audioPlaying.value = false;
}
</script>

<template>
  <el-dialog
    :model-value="visible"
    :title="`测试模型: ${modelName}`"
    width="560px"
    :close-on-click-modal="false"
    :close-on-press-escape="!testStarted || testFinished"
    :show-close="!testStarted || testFinished"
    @close="handleClose"
  >
    <div class="test-dialog-body">
      <div class="step-list">
        <div
          v-for="(step, idx) in steps"
          :key="step.key"
          class="step-item"
          :class="`step-${step.status}`"
        >
          <div class="step-indicator">
            <span v-if="step.status === 'pending'" class="step-num">{{ idx + 1 }}</span>
            <el-icon v-else-if="step.status === 'active'" class="is-loading step-icon loading">
              <Loading />
            </el-icon>
            <el-icon v-else-if="step.status === 'done'" class="step-icon success">
              <CircleCheck />
            </el-icon>
            <el-icon v-else-if="step.status === 'failed'" class="step-icon failed">
              <CircleClose />
            </el-icon>
          </div>
          <div
            v-if="idx < steps.length - 1"
            class="step-line"
            :class="{
              'line-done': step.status === 'done',
              'line-active': step.status === 'active',
            }"
          />
          <div class="step-label">
            <span class="step-icon-emoji">{{ step.icon }}</span>
            <span class="step-text">{{ step.label }}</span>
            <span v-if="step.detail" class="step-detail">{{ step.detail }}</span>
          </div>
        </div>
      </div>

      <div v-if="testStarted" class="progress-section">
        <el-progress
          :percentage="progressPercent"
          :stroke-width="6"
          :status="testFinished ? (isSuccess ? 'success' : 'exception') : undefined"
          :striped="!testFinished"
          :striped-flow="!testFinished"
        />
        <div class="progress-meta">
          <span class="elapsed">{{ elapsedDisplay }}</span>
          <span v-if="testLatency !== undefined" class="latency">⏱ 响应 {{ testLatency }}ms</span>
        </div>
      </div>

      <div v-if="testFinished" class="result-section" :class="{ success: isSuccess, failed: !isSuccess }">
        <div class="result-header">
          <el-icon :size="24" :color="statusColor">
            <CircleCheck v-if="isSuccess" />
            <CircleClose v-else />
          </el-icon>
          <span class="result-title">{{ isSuccess ? "测试通过" : "测试失败" }}</span>
        </div>
        <p class="result-msg">{{ resultMessage }}</p>
        <div v-if="isSuccess && resultAudioData" class="audio-play-section">
          <el-button type="primary" :icon="VideoPlay" @click="playAudio" :loading="audioPlaying">
            {{ audioPlaying ? "播放中…" : "播放合成语音" }}
          </el-button>
          <span class="audio-hint">点击按钮试听 TTS 合成效果</span>
        </div>
      </div>
    </div>

    <template #footer>
      <el-button @click="handleClose" :disabled="testStarted && !testFinished">
        {{ testFinished ? "关闭" : "取消" }}
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.test-dialog-body {
  padding: 8px 0;
}

.step-list {
  display: flex;
  flex-direction: column;
  gap: 0;
  margin-bottom: 20px;
}

.step-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  min-height: 44px;
  position: relative;
}

.step-indicator {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.35s ease;
  margin-top: 2px;
  position: relative;
  z-index: 1;

  .step-num {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-muted);
    border: 2px solid var(--glass-border);
    background: var(--glass-bg);
  }

  .step-icon {
    font-size: 28px;

    &.loading {
      color: var(--color-primary);
      animation: pulse 1.4s ease-in-out infinite;
    }
    &.success { color: #67c23a; }
    &.failed { color: #f56c6c; }
  }
}

.step-line {
  position: absolute;
  left: 13px;
  top: 30px;
  width: 2px;
  height: 16px;
  background: var(--glass-border);
  transition: background 0.3s;
  z-index: 0;

  &.line-active {
    background: linear-gradient(to bottom, var(--color-primary), var(--glass-border));
  }
  &.line-done {
    background: #67c23a;
  }
}

.step-label {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  padding-top: 4px;
  flex-wrap: wrap;

  .step-icon-emoji { font-size: 15px; flex-shrink: 0; }

  .step-text {
    font-size: 14px;
    color: var(--color-text-muted);
    transition: color 0.3s;
  }

  .step-detail {
    font-size: 12px;
    color: var(--color-text-secondary);
    opacity: 0.8;
    margin-left: 4px;
  }
}

.step-active .step-text { color: var(--color-primary); font-weight: 500; }
.step-done .step-text { color: var(--color-text-primary); }
.step-failed .step-text { color: #f56c6c; }

.progress-section {
  margin-bottom: 16px;

  .progress-meta {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 12px;
    color: var(--color-text-muted);

    .latency { color: var(--color-primary); }
  }
}

.result-section {
  padding: 16px;
  border-radius: var(--radius-md);
  margin-top: 8px;

  &.success {
    background: rgba(103, 194, 58, 0.08);
    border: 1px solid rgba(103, 194, 58, 0.2);
  }
  &.failed {
    background: rgba(245, 108, 108, 0.08);
    border: 1px solid rgba(245, 108, 108, 0.2);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;

    .result-title { font-weight: 600; font-size: 15px; }
  }

  .result-msg {
    font-size: 13px;
    color: var(--color-text-secondary);
    margin: 0 0 12px 0;
    line-height: 1.5;
  }
}

.audio-play-section {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-top: 8px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);

  .audio-hint {
    font-size: 12px;
    color: var(--color-text-muted);
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(1.08); }
}
</style>
