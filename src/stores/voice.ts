import { defineStore } from "pinia";
import { ref } from "vue";
import {
  startRecording as apiStartRecording,
  stopRecording as apiStopRecording,
  stopRecordingAudio as apiStopRecordingAudio,
  cancelRecording as apiCancelRecording,
  type AudioRecordingResult,
} from "@/api/voice";

export type VoiceState = "idle" | "recording" | "processing" | "playing";

export const useVoiceStore = defineStore("voice", () => {
  const state = ref<VoiceState>("idle");
  const waveformData = ref<number[]>(new Array(32).fill(0));
  let waveformTimer: ReturnType<typeof setInterval> | null = null;

  function _updateWaveform() {
    waveformData.value = waveformData.value.map(() => Math.random() * 0.8 + 0.1);
  }

  async function startRecording() {
    state.value = "recording";
    await apiStartRecording().catch((e) => {
      state.value = "idle";
      throw e;
    });
    waveformTimer = setInterval(_updateWaveform, 100);
  }

  async function stopRecording() {
    if (waveformTimer) {
      clearInterval(waveformTimer);
      waveformTimer = null;
    }
    state.value = "processing";
    const text = await apiStopRecording().catch((e) => {
      state.value = "idle";
      throw e;
    });
    state.value = "idle";
    return text;
  }

  async function stopRecordingAudio(): Promise<AudioRecordingResult> {
    if (waveformTimer) {
      clearInterval(waveformTimer);
      waveformTimer = null;
    }
    state.value = "processing";
    const result = await apiStopRecordingAudio().catch((e) => {
      state.value = "idle";
      throw e;
    });
    state.value = "idle";
    return result;
  }

  async function cancelRecording() {
    if (waveformTimer) {
      clearInterval(waveformTimer);
      waveformTimer = null;
    }
    state.value = "idle";
    await apiCancelRecording().catch((e) => console.warn("cancelRecording:", e));
  }

  function startPlaying() {
    state.value = "playing";
    waveformTimer = setInterval(_updateWaveform, 80);
  }

  function stopPlaying() {
    if (waveformTimer) {
      clearInterval(waveformTimer);
      waveformTimer = null;
    }
    state.value = "idle";
  }

  return {
    state,
    waveformData,
    startRecording,
    stopRecording,
    stopRecordingAudio,
    cancelRecording,
    startPlaying,
    stopPlaying,
  };
});
