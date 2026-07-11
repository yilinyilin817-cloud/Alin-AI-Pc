<script setup lang="ts">
import { nextTick, ref } from "vue";
import {
  Picture,
  Microphone,
  Monitor,
  Promotion,
  ChatDotRound,
} from "@element-plus/icons-vue";
import { useVoice } from "@/composables/useVoice";
import { useCapture } from "@/composables/useCapture";
import { useSettingsStore } from "@/stores/settings";

const emit = defineEmits<{
  send: [content: string, parts?: import("@/types").ContentPart[]];
}>();

const text = ref("");
const imageAttachments = ref<import("@/types").ContentPart[]>([]);
const fileInput = ref<HTMLInputElement | null>(null);
const isDragOver = ref(false);
const dragCounter = ref(0);
const settingsStore = useSettingsStore();
const voiceMode = ref<"transcribe" | "audio">(
  settingsStore.voiceMessagePreferred ? "audio" : "transcribe",
);
const { state: voiceState, startRecording, stopRecording, stopRecordingAudio } = useVoice();
const { captureScreen } = useCapture();

const TEXT_EXTENSIONS = new Set([
  ".txt", ".md", ".json", ".js", ".ts", ".jsx", ".tsx", ".py",
  ".html", ".css", ".scss", ".vue", ".rs", ".c", ".cpp", ".java",
  ".go", ".yaml", ".yml", ".xml", ".sql", ".sh", ".ps1", ".bat",
]);

function isTextFile(name: string): boolean {
  const lower = name.toLowerCase();
  return Array.from(TEXT_EXTENSIONS).some((ext) => lower.endsWith(ext));
}

function bytesToBase64(bytes: number[] | Uint8Array): string {
  let binary = "";
  const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  for (let i = 0; i < arr.length; i++) {
    binary += String.fromCharCode(arr[i]);
  }
  return btoa(binary);
}

function getImagePreview(part: import("@/types").ContentPart): string {
  if (part.type === "image_url") return part.url;
  if (part.type === "image_bytes") {
    const mime = part.mime || "image/png";
    return `data:${mime};base64,${bytesToBase64(part.data)}`;
  }
  return "";
}

function removeAttachment(i: number) {
  imageAttachments.value.splice(i, 1);
}

function handleSend() {
  if (!text.value.trim() && imageAttachments.value.length === 0) return;
  const parts = imageAttachments.value.length > 0 ? [...imageAttachments.value] : undefined;
  emit("send", text.value, parts);
}

function clear() {
  text.value = "";
  imageAttachments.value = [];
}

defineExpose({ clear, startVoiceInput });

async function startVoiceInput() {
  if (voiceState.value === "recording") return;
  voiceMode.value = "transcribe";
  settingsStore.voiceMessagePreferred = false;
  settingsStore.save();
  await startRecording();
}

function getTextarea(): HTMLTextAreaElement | null {
  return document.querySelector(".chat-input .el-textarea__inner");
}

function insertTextAtCursor(value: string) {
  const textarea = getTextarea();
  const start = textarea?.selectionStart ?? text.value.length;
  const end = textarea?.selectionEnd ?? start;
  const before = text.value.slice(0, start);
  const after = text.value.slice(end);
  text.value = before + value + after;
  if (textarea) {
    void nextTick(() => {
      const pos = start + value.length;
      textarea.setSelectionRange(pos, pos);
      textarea.focus();
    });
  }
}

function readImageFile(file: File) {
  const reader = new FileReader();
  reader.onload = () => {
    const data = reader.result as string;
    const base64 = data.split(",")[1];
    if (base64) {
      const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
      imageAttachments.value.push({ type: "image_bytes", data: bytes, mime: file.type });
    }
  };
  reader.readAsDataURL(file);
}

function readTextFile(file: File) {
  const reader = new FileReader();
  reader.onload = () => {
    insertTextAtCursor(reader.result as string);
  };
  reader.readAsText(file);
}

function handleDragEnter(e: DragEvent) {
  e.preventDefault();
  dragCounter.value++;
  isDragOver.value = true;
}

function handleDragOver(e: DragEvent) {
  e.preventDefault();
}

function handleDragLeave(e: DragEvent) {
  e.preventDefault();
  dragCounter.value--;
  if (dragCounter.value <= 0) {
    dragCounter.value = 0;
    isDragOver.value = false;
  }
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  dragCounter.value = 0;
  isDragOver.value = false;
  const files = e.dataTransfer?.files;
  if (!files) return;
  for (const file of Array.from(files)) {
    if (file.type.startsWith("image/")) {
      readImageFile(file);
    } else if (isTextFile(file.name)) {
      readTextFile(file);
    }
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
}

async function toggleVoice() {
  if (voiceState.value === "recording") {
    if (voiceMode.value === "audio") {
      const { audio, duration, mime } = await stopRecordingAudio();
      const bytes = audio ? Uint8Array.from(atob(audio), (c) => c.charCodeAt(0)) : new Uint8Array(0);
      emit("send", "", [{ type: "audio_bytes", data: bytes, duration, mime }]);
    } else {
      const transcript = await stopRecording();
      insertTextAtCursor(transcript);
    }
  } else {
    await startRecording();
  }
}

function handleVoiceModeChange(command: string | number) {
  const mode = command === "audio" ? "audio" : "transcribe";
  voiceMode.value = mode;
  settingsStore.voiceMessagePreferred = mode === "audio";
  settingsStore.save();
}

function handleImageUpload() {
  fileInput.value?.click();
}

function onFileSelected(e: Event) {
  const input = e.target as HTMLInputElement;
  if (!input.files?.length) return;
  readImageFile(input.files[0]);
  input.value = "";
}
</script>

<template>
  <div class="chat-input-wrapper">
    <div
      class="chat-input"
      :class="{ 'drag-over': isDragOver, 'recording': voiceState === 'recording' }"
      @dragenter="handleDragEnter"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
    >
      <input
        ref="fileInput"
        type="file"
        accept="image/*"
        style="display: none"
        @change="onFileSelected"
      />

      <div v-if="imageAttachments.length" class="attachment-preview">
        <div
          v-for="(att, i) in imageAttachments"
          :key="i"
          class="attachment-thumb"
        >
          <img :src="getImagePreview(att)" alt="attachment" />
          <button class="attachment-remove" title="移除" @click="removeAttachment(i)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </div>

      <el-input
        v-model="text"
        type="textarea"
        :rows="1"
        :autosize="{ minRows: 1, maxRows: 6 }"
        placeholder="输入消息… (Enter 发送，Shift+Enter 换行)"
        resize="none"
        class="message-textarea"
        @keydown="handleKeydown"
      />

      <div class="input-footer">
        <div class="input-actions">
          <el-tooltip content="附加图片" placement="top">
            <button class="action-btn" @click="handleImageUpload">
              <el-icon><Picture /></el-icon>
            </button>
          </el-tooltip>
          <el-tooltip content="截屏" placement="top">
            <button class="action-btn" @click="captureScreen">
              <el-icon><Monitor /></el-icon>
            </button>
          </el-tooltip>
          <el-tooltip :content="voiceState === 'recording' ? '停止录音' : '语音输入'" placement="top">
            <button
              class="action-btn"
              :class="{ 'is-recording': voiceState === 'recording' }"
              @click="toggleVoice"
            >
              <el-icon><Microphone /></el-icon>
            </button>
          </el-tooltip>
          <el-dropdown
            trigger="click"
            :disabled="voiceState === 'recording'"
            placement="top"
            @command="handleVoiceModeChange"
          >
            <button class="action-btn" :title="voiceMode === 'audio' ? '当前：发送语音' : '当前：转文字'">
              <el-icon><ChatDotRound /></el-icon>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="transcribe" :disabled="voiceMode === 'transcribe'">
                  转文字
                </el-dropdown-item>
                <el-dropdown-item command="audio" :disabled="voiceMode === 'audio'">
                  发语音
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>

        <button
          class="send-btn"
          :class="{ 'can-send': text.trim() || imageAttachments.length }"
          :disabled="!text.trim() && !imageAttachments.length"
          @click="handleSend"
        >
          <el-icon><Promotion /></el-icon>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.chat-input-wrapper {
  padding: 12px 20px 16px;
  background: transparent;
}

.chat-input {
  display: flex;
  flex-direction: column;
  background: var(--glass-bg-light);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid var(--glass-border);
  border-radius: 20px;
  padding: 10px 12px 8px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);

  &:focus-within {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(124, 107, 240, 0.12), 0 4px 20px rgba(0, 0, 0, 0.12);
  }

  &.drag-over {
    border-color: var(--color-primary);
    background: rgba(124, 107, 240, 0.06);
    box-shadow: 0 0 0 3px rgba(124, 107, 240, 0.2);
  }

  &.recording {
    border-color: var(--color-danger);
    box-shadow: 0 0 0 3px rgba(245, 108, 108, 0.15);
  }
}

.attachment-preview {
  display: flex;
  gap: 8px;
  padding: 4px 4px 8px;
  flex-wrap: wrap;
}

.attachment-thumb {
  position: relative;
  width: 52px;
  height: 52px;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid var(--glass-border);
  transition: all 0.2s ease;

  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}

.attachment-remove {
  position: absolute;
  top: 3px;
  right: 3px;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  background: rgba(0, 0, 0, 0.6);
  border: none;
  border-radius: 50%;
  color: #fff;
  cursor: pointer;
  opacity: 0;
  transition: all 0.15s ease;

  .attachment-thumb:hover & {
    opacity: 1;
  }

  svg {
    width: 10px;
    height: 10px;
  }

  &:hover {
    background: var(--color-danger);
    transform: scale(1.1);
  }
}

.message-textarea {
  :deep(.el-textarea__inner) {
    background: transparent;
    border: none;
    box-shadow: none !important;
    color: var(--color-text-primary);
    padding: 6px 8px 4px;
    font-size: 14px;
    line-height: 1.6;
    min-height: auto !important;
    resize: none;

    &::placeholder {
      color: var(--color-text-muted);
    }

    &:hover, &:focus {
      border: none;
      box-shadow: none !important;
    }
  }
}

.input-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 4px 0;
}

.input-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 10px;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.18s ease;

  .el-icon {
    font-size: 17px;
  }

  &:hover {
    background: rgba(124, 107, 240, 0.1);
    color: var(--color-primary);
  }

  &:active {
    transform: scale(0.92);
  }

  &.is-recording {
    color: var(--color-danger);
    background: rgba(245, 108, 108, 0.1);
    animation: pulse-recording 1.5s ease-in-out infinite;
  }
}

@keyframes pulse-recording {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

.send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  padding: 0;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: 12px;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

  .el-icon {
    font-size: 16px;
    transform: rotate(-45deg);
    transition: transform 0.2s ease;
  }

  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  &.can-send {
    background: linear-gradient(135deg, var(--color-primary) 0%, #9b8af5 100%);
    border-color: transparent;
    color: #fff;
    box-shadow: 0 2px 8px rgba(124, 107, 240, 0.35);

    &:hover:not(:disabled) {
      transform: scale(1.06);
      box-shadow: 0 4px 16px rgba(124, 107, 240, 0.5);
    }

    &:active:not(:disabled) {
      transform: scale(0.96);
    }

    .el-icon {
      transform: rotate(0deg);
    }
  }
}
</style>
