export type ApprovalMode = "once" | "always" | "ask_every_time";

export interface SkillDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  permissions: string[];
  approvalMode: ApprovalMode;
  enabled: boolean;
  config?: Record<string, unknown>;
}

export interface ToolCallLog {
  id: string;
  sessionId: string;
  skillName: string;
  argsJson: string;
  resultJson: string;
  status: "pending" | "success" | "error" | "rejected";
  durationMs: number;
  createdAt: string;
}
