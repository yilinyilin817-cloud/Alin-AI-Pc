import type { SkillDefinition, ToolCallLog } from "@/types/skill";
import { isTauri } from "./env";
import { mockSkills, mockToolCallLogs } from "@/mocks/data";

export async function listSkills(): Promise<SkillDefinition[]> {
  if (!isTauri()) return structuredClone(mockSkills);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SkillDefinition[]>("list_skills");
}

export async function toggleSkill(skillName: string, enabled: boolean): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("toggle_skill", { skillName, enabled });
}

export async function approveSkillPermission(skillName: string, status: string): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("approve_skill_permission", { skillName, status });
}

export async function listToolCallLogs(sessionId?: string, limit?: number): Promise<ToolCallLog[]> {
  if (!isTauri()) {
    return structuredClone(mockToolCallLogs).filter(
      (l) => !sessionId || l.sessionId === sessionId,
    );
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ToolCallLog[]>("list_tool_call_logs", {
    sessionId: sessionId ?? null,
    limit: limit ?? null,
  });
}

export async function runSkillManual(sessionId: string, skillName: string, args: Record<string, unknown>): Promise<string> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("run_skill_manual", { sessionId, skillName, args });
}
