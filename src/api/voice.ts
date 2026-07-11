import { isTauri } from "./env";

export async function startRecording(): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("start_recording");
}

export async function stopRecording(): Promise<string> {
  if (!isTauri()) return "";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("stop_recording");
}

export interface AudioRecordingResult {
  audio: string;
  duration: number;
  mime: string;
  transcript?: string;
}

export async function stopRecordingAudio(): Promise<AudioRecordingResult> {
  if (!isTauri()) {
    return {
      audio: "",
      duration: 0,
      mime: "audio/wav",
    };
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AudioRecordingResult>("stop_recording_audio");
  } catch {
    // 后端命令尚未实现时返回 mock 数据，保证 UI 可联调
    return {
      audio: "",
      duration: 0,
      mime: "audio/wav",
    };
  }
}

export async function cancelRecording(): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("cancel_recording");
}

export async function synthesizeSpeech(text: string, voiceId?: string, personaId?: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("synthesize_speech", { text, voiceId: voiceId ?? null, personaId: personaId ?? null });
}

export async function listAudioDevices(): Promise<string[]> {
  if (!isTauri()) return ["默认麦克风 (Mock)", "默认扬声器 (Mock)"];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string[]>("list_audio_devices");
}
