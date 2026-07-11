import { isTauri } from "./env";

export async function captureScreen(): Promise<Uint8Array> {
  if (!isTauri()) throw new Error("Not in Tauri");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<number[]>("capture_screen").then((arr) => new Uint8Array(arr));
}

export async function captureCamera(): Promise<Uint8Array> {
  if (!isTauri()) throw new Error("Not in Tauri");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<number[]>("capture_camera").then((arr) => new Uint8Array(arr));
}
