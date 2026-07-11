import { isTauri } from "./env";

export interface MemoryItem {
  id: string;
  type: string;
  content: string;
  importance: number;
  createdAt: string;
}

export async function listMemories(personaId: string): Promise<MemoryItem[]> {
  if (!isTauri()) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<MemoryItem[]>("list_memories", { personaId });
}

export async function deleteMemory(memoryId: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("delete_memory", { memoryId });
}
