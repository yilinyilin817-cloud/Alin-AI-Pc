import { storeToRefs } from "pinia";
import { useChatStore } from "@/stores/chat";

export function useChat() {
  const chatStore = useChatStore();
  const {
    sessions,
    currentSessionId,
    currentSession,
    currentMessages,
    isStreaming,
  } = storeToRefs(chatStore);

  return {
    sessions,
    currentSessionId,
    currentSession,
    currentMessages,
    isStreaming,
    selectSession: chatStore.selectSession,
    createSession: chatStore.createSession,
    sendMessage: chatStore.sendMessage,
  };
}
