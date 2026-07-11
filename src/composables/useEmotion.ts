import { ref } from "vue";
import type { EmotionTag } from "@/types";
import { mockRandomEmotion } from "@/mocks/ipc";

export function useEmotion() {
  const currentEmotion = ref<EmotionTag>({
    emotion: "neutral",
    valence: 0,
    arousal: 0.3,
  });

  function refreshEmotion() {
    currentEmotion.value = mockRandomEmotion();
  }

  return { currentEmotion, refreshEmotion };
}
