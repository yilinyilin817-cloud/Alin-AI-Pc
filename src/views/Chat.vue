<script setup lang="ts">
import { ref, watch, nextTick, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { listen } from "@tauri-apps/api/event";
import ChatBubble from "@/components/ChatBubble.vue";
import ChatInput from "@/components/ChatInput.vue";
import EmptyState from "@/components/EmptyState.vue";
import TypingIndicator from "@/components/TypingIndicator.vue";
import CompanionBar from "@/components/CompanionBar.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import { useChat } from "@/composables/useChat";
import { usePersonaStore } from "@/stores/persona";
import { useChatStore } from "@/stores/chat";

const route = useRoute();
const router = useRouter();
const personaStore = usePersonaStore();
const chatStore = useChatStore();
const { currentMessages, isStreaming, selectSession, sendMessage } = useChat();
const companionBarRef = ref<InstanceType<typeof CompanionBar> | null>(null);

const messagesRef = ref<HTMLElement | null>(null);
const chatInputRef = ref<InstanceType<typeof ChatInput> | null>(null);

/** 路由变化时加载对应会话 */
watch(
  () => route.params.sessionId,
  async (id) => {
    if (typeof id === "string") {
      selectSession(id);
      await nextTick();
      scrollToBottom();
    }
  },
  { immediate: true },
);

/** 消息更新时自动滚到底部 */
watch(
  () => currentMessages.value.length,
  async () => {
    await nextTick();
    scrollToBottom();
  },
);

/** 滚动到底部 */
function scrollToBottom() {
  if (messagesRef.value) {
    messagesRef.value.scrollTop = messagesRef.value.scrollHeight;
  }
}

/** 发送消息 */
async function handleSend(content: string, parts?: import("@/types").ContentPart[]) {
  const ok = await sendMessage(content, parts);
  if (ok) {
    chatInputRef.value?.clear();
  }
  await nextTick();
  scrollToBottom();
}

/** 创建新会话 */
async function handleNewSession() {
  const pid = personaStore.currentPersonaId ?? personaStore.personas[0]?.id ?? "persona_aria";
  const session = await chatStore.createSession(pid);
  router.push(`/chat/${session.id}`).catch((e) => console.warn("handleNewSession:", e));
}

async function maybeStartVoiceInput() {
  if (route.query.voice !== "1") return;
  await nextTick();
  chatInputRef.value?.startVoiceInput();
  // 移除 query 避免后退/刷新重复触发
  router.replace({ query: { ...route.query, voice: undefined } }).catch((e) => console.warn("replace voice query:", e));
}

onMounted(() => {
  maybeStartVoiceInput();

  // 监听里程碑达成事件
  listen("milestone-achieved", (e: any) => {
    const ms = e.payload?.milestone;
    if (ms && companionBarRef.value) {
      companionBarRef.value.showMilestonePopup({
        title: ms.title,
        description: ms.description,
        icon: ms.icon,
      });
      companionBarRef.value.load();
    }
  });

  // 监听主动消息（自动切换到对应角色会话）
  listen("proactive-message", async (e: any) => {
    const msg = e.payload;
    if (msg?.personaId && msg.personaId === personaStore.currentPersonaId) {
      // TODO: 创建新会话或自动发一条系统消息
      console.log("Proactive message from", msg.personaName, ":", msg.content);
    }
  });
});

watch(
  () => route.query.voice,
  () => maybeStartVoiceInput(),
);
</script>

<template>
  <div class="chat-view">
    <section class="chat-panel">
      <CompanionBar ref="companionBarRef" />
      <div ref="messagesRef" class="messages-area">
        <EmptyState
          v-if="!currentMessages.length && !isStreaming"
          icon="💬"
          title="开始新对话"
          :description="
            chatStore.currentSessionId
              ? '发送第一条消息，开始和 AI 伴侣聊天吧'
              : (personaStore.currentPersona?.greeting ?? '选择或创建一个会话')
          "
        >
          <template v-if="!chatStore.currentSessionId" #actions>
            <BaseButton variant="primary" @click="handleNewSession">
              新建会话
            </BaseButton>
          </template>
        </EmptyState>
        <ChatBubble
          v-for="msg in currentMessages"
          :key="msg.id"
          :message="msg"
        />

        <div v-if="isStreaming" class="typing-indicator">
          <TypingIndicator text="AI 正在思考…" />
        </div>
      </div>

      <ChatInput ref="chatInputRef" @send="handleSend" />
    </section>
  </div>
</template>

<style scoped lang="scss">
.chat-view {
  display: flex;
  height: 100%;
}

.chat-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.typing-indicator {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}

</style>
