<script setup lang="ts">
import { computed } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { Message } from "@/types";
import { usePersonaStore } from "@/stores/persona";
import { useChatStore } from "@/stores/chat";
import { useChat } from "@/composables/useChat";
import MessageRenderer from "./MessageRenderer.vue";
import SegmentedMessage from "./SegmentedMessage.vue";
import VoiceMessageBubble from "./VoiceMessageBubble.vue";
import ToolCallCard from "./ToolCallCard.vue";

const props = defineProps<{
  message: Message;
}>();

const personaStore = usePersonaStore();
const chatStore = useChatStore();
const { sendMessage } = useChat();

const emotionEmoji: Record<string, string> = {
  happy: "😊",
  sad: "😢",
  angry: "😠",
  fearful: "😨",
  surprised: "😲",
  disgusted: "🤢",
  neutral: "😐",
};

const avatarText = computed(() => {
  if (props.message.role === 'user') return '我';
  const name = personaStore.currentPersona?.name;
  return name ? name[0] : 'AI';
});

const formattedTime = computed(() => {
  if (!props.message.createdAt) return '';
  const date = new Date(props.message.createdAt);
  const now = new Date();
  const isToday = date.toDateString() === now.toDateString();

  if (isToday) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  }
  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }) + ' ' +
         date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
});

function bytesToBase64(bytes: number[] | Uint8Array): string {
  let binary = "";
  const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  for (let i = 0; i < arr.length; i++) {
    binary += String.fromCharCode(arr[i]);
  }
  return btoa(binary);
}

async function copyMessage() {
  const text = props.message.content;
  try {
    await navigator.clipboard.writeText(text);
    ElMessage.success('已复制到剪贴板');
  } catch {
    ElMessage.error('复制失败');
  }
}

async function regenerateMessage() {
  if (props.message.role !== 'assistant') return;
  const sessionId = chatStore.currentSessionId;
  if (!sessionId) return;

  const msgs = chatStore.messagesBySession[sessionId] ?? [];
  const idx = msgs.findIndex((m) => m.id === props.message.id);
  if (idx < 0) return;

  // 找到当前 AI 消息前面的最近一条用户消息
  let userIdx = -1;
  for (let i = idx - 1; i >= 0; i--) {
    if (msgs[i].role === 'user') {
      userIdx = i;
      break;
    }
  }
  if (userIdx < 0) {
    ElMessage.warning('未找到可重新生成的上下文');
    return;
  }

  const userMsg = msgs[userIdx];
  // 删除当前 AI 消息
  msgs.splice(idx, 1);
  // 重新发送用户消息（保留多模态附件）
  const ok = await sendMessage(userMsg.content, userMsg.parts);
  if (!ok) {
    ElMessage.error('重新生成失败');
  }
}

async function deleteMessage() {
  try {
    await ElMessageBox.confirm('确定要删除这条消息吗？', '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    });
  } catch {
    return;
  }

  const sessionId = chatStore.currentSessionId;
  if (!sessionId) return;
  const msgs = chatStore.messagesBySession[sessionId] ?? [];
  const idx = msgs.findIndex((m) => m.id === props.message.id);
  if (idx >= 0) {
    msgs.splice(idx, 1);
    ElMessage.success('已删除');
  }
}
</script>

<template>
  <div
    class="chat-bubble"
    :class="{
      user: message.role === 'user',
      assistant: message.role === 'assistant',
      tool: message.role === 'tool',
    }"
  >
    <div v-if="message.role === 'assistant'" class="avatar">
      {{ avatarText }}
    </div>

    <div class="bubble-content">
      <!-- 消息内容渲染 -->
      <div class="bubble-text">
        <SegmentedMessage
          v-if="message.role === 'assistant' && message.segments?.length"
          :segments="message.segments"
          :streaming="message.streaming"
        />
        <MessageRenderer
          v-else-if="message.role === 'assistant' && (message.content || message.parts?.length)"
          :content="message.content"
          :streaming="message.streaming"
          :parts="message.parts"
        />
        <p v-else-if="message.content" class="plain-text">
          {{ message.content }}
          <span v-if="message.streaming" class="cursor">|</span>
        </p>
      </div>

      <!-- 图片附件 -->
      <div v-if="message.parts?.length" class="bubble-parts">
        <template v-for="(part, i) in message.parts" :key="i">
          <img
            v-if="part.type === 'image_url'"
            :src="part.url"
            class="bubble-image"
            alt="attachment"
          />
          <img
            v-else-if="part.type === 'image_bytes'"
            :src="'data:image/png;base64,' + bytesToBase64(part.data)"
            class="bubble-image"
            alt="image"
          />
          <VoiceMessageBubble
            v-else-if="part.type === 'audio_bytes'"
            :part="part"
            :position="message.role === 'user' ? 'right' : 'left'"
            class="bubble-audio"
          />
        </template>
      </div>

      <!-- 工具调用卡片 -->
      <div v-if="message.toolCalls?.length" class="tool-calls">
        <ToolCallCard
          v-for="tc in message.toolCalls"
          :key="tc.id"
          :tool-name="tc.name"
          :status="tc.status || 'success'"
          :arguments="tc.arguments"
          :result="tc.result"
        />
      </div>

      <!-- 消息底部：时间戳和操作按钮 -->
      <div v-if="message.role !== 'tool' && message.content" class="bubble-footer">
        <span v-if="formattedTime" class="message-time">{{ formattedTime }}</span>
        <div class="message-actions">
          <button class="action-btn" title="复制" @click="copyMessage">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
          </button>
          <button
            v-if="message.role === 'assistant'"
            class="action-btn"
            title="重新生成"
            @click="regenerateMessage"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6"></path>
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
            </svg>
          </button>
          <button class="action-btn" title="删除" @click="deleteMessage">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="3 6 5 6 21 6"></polyline>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <div
      v-if="message.emotionTag && message.role === 'assistant'"
      class="emotion-badge"
      :title="message.emotionTag.emotion"
    >
      {{ emotionEmoji[message.emotionTag.emotion] ?? "😐" }}
    </div>
  </div>
</template>

<style scoped lang="scss">
.chat-bubble {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--chat-message-gap, 12px);
  max-width: 75%;
  animation: message-enter 0.3s ease-out;

  &.user {
    margin-left: auto;
    flex-direction: row-reverse;

    .bubble-content {
      background: linear-gradient(135deg,
        var(--color-primary) 0%,
        var(--color-primary-dark) 100%);
      color: var(--color-text-inverse);
      border: none;
      border-radius: var(--radius-lg) var(--radius-lg) 4px var(--radius-lg);
      box-shadow: var(--shadow-bubble-user);
    }

    .bubble-footer {
      justify-content: flex-end;
    }

    .message-time {
      color: var(--color-text-inverse-muted);
    }

    .action-btn {
      color: var(--color-text-inverse-secondary);

      &:hover {
        background: var(--color-inverse-hover);
        color: var(--color-text-inverse);
      }
    }
  }

  &.assistant {
    .avatar {
      width: 36px;
      height: 36px;
      flex-shrink: 0;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 14px;
      font-weight: 600;
      color: var(--color-text-inverse);
      background: linear-gradient(135deg, var(--color-primary), var(--color-accent));
      box-shadow: var(--shadow-avatar);
      align-self: flex-start;
      margin-top: var(--spacing-xs);
    }

    .bubble-content {
      background: var(--glass-bg);
      backdrop-filter: blur(var(--glass-blur));
      -webkit-backdrop-filter: blur(var(--glass-blur));
      border: 1px solid var(--glass-border);
      border-radius: var(--radius-lg) var(--radius-lg) var(--radius-lg) 4px;
      box-shadow: var(--shadow-bubble-assistant);
    }
  }

  &.tool {
    max-width: 90%;
    margin: 0 auto;

    .bubble-content {
      background: var(--glass-bg-light);
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
      border: 1px dashed var(--glass-border);
      border-radius: var(--radius-md);
      font-size: var(--font-size-sm);
      color: var(--color-text-secondary);
    }
  }

  &:hover .bubble-content {
    transform: translateY(-1px);
    box-shadow: var(--shadow-bubble-hover);
  }
}

.bubble-content {
  padding: var(--chat-bubble-padding, 12px 16px);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.bubble-text {
  line-height: 1.6;
  word-break: break-word;
  color: var(--color-text-primary);
  font-weight: 400;
  text-shadow: var(--shadow-text);

  .plain-text {
    margin: 0;
    white-space: pre-wrap;
  }

  :deep(p) {
    & + p {
      margin-top: var(--spacing-sm);
    }
  }
}

.cursor {
  animation: blink 0.8s infinite;
  color: var(--color-primary);
  font-weight: bold;
}

@keyframes blink {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0;
  }
}

@keyframes message-enter {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.bubble-image {
  max-width: 240px;
  border-radius: var(--radius-md);
  margin-top: var(--spacing-sm);
}

.bubble-audio {
  margin-top: var(--spacing-sm);
}

.tool-calls {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-md);
}

.bubble-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: var(--spacing-sm);
  padding-top: var(--spacing-sm);
  border-top: 1px solid var(--glass-border);
  gap: var(--spacing-md);
}

.message-time {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  opacity: 0.7;
}

.message-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;

  .chat-bubble:hover & {
    opacity: 1;
  }
}

.action-btn {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.2s;

  svg {
    width: 14px;
    height: 14px;
  }

  &:hover {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  &:active {
    transform: scale(0.95);
  }
}

.emotion-badge {
  font-size: 18px;
  align-self: flex-end;
}

// 密度覆盖
:global(html.density-compact) .chat-bubble {
  margin-bottom: var(--chat-message-gap, 6px);

  .bubble-content {
    padding: var(--chat-bubble-padding, 8px 12px);
  }
}

:global(html.density-cozy) .chat-bubble {
  margin-bottom: var(--chat-message-gap, 20px);

  .bubble-content {
    padding: var(--chat-bubble-padding, 16px 20px);
  }
}
</style>
