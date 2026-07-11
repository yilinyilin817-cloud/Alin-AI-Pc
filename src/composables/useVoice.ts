import { storeToRefs } from "pinia";
import { useVoiceStore } from "@/stores/voice";

export function useVoice() {
  const voiceStore = useVoiceStore();
  const { state, waveformData } = storeToRefs(voiceStore);

  return {
    state,
    waveformData,
    startRecording: voiceStore.startRecording,
    stopRecording: voiceStore.stopRecording,
    stopRecordingAudio: voiceStore.stopRecordingAudio,
    cancelRecording: voiceStore.cancelRecording,
    startPlaying: voiceStore.startPlaying,
    stopPlaying: voiceStore.stopPlaying,
  };
}
