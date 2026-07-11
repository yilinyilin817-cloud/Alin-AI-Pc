import type { Workflow } from "@/types";
import { isTauri } from "./env";

export interface WorkflowRow {
  id: string;
  personaId: string;
  name: string;
  description?: string;
  enabled: boolean;
  triggerJson: string;
  actionsJson: string;
  createdAt: string;
  updatedAt: string;
}

function parseWorkflow(row: WorkflowRow): Workflow {
  return {
    id: row.id,
    personaId: row.personaId,
    name: row.name,
    description: row.description,
    enabled: row.enabled,
    trigger: JSON.parse(row.triggerJson),
    actions: JSON.parse(row.actionsJson),
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

function toRow(workflow: Workflow): WorkflowRow {
  return {
    id: workflow.id,
    personaId: workflow.personaId,
    name: workflow.name,
    description: workflow.description,
    enabled: workflow.enabled,
    triggerJson: JSON.stringify(workflow.trigger),
    actionsJson: JSON.stringify(workflow.actions),
    createdAt: workflow.createdAt,
    updatedAt: workflow.updatedAt,
  };
}

export async function listWorkflows(personaId: string): Promise<Workflow[]> {
  if (!isTauri()) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  const rows = await invoke<WorkflowRow[]>("list_workflows", { personaId });
  return rows.map(parseWorkflow);
}

export async function getWorkflow(workflowId: string): Promise<Workflow | null> {
  if (!isTauri()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  const row = await invoke<WorkflowRow | null>("get_workflow", { workflowId });
  return row ? parseWorkflow(row) : null;
}

export async function createWorkflow(workflow: Workflow): Promise<Workflow> {
  if (!isTauri()) return workflow;
  const { invoke } = await import("@tauri-apps/api/core");
  const row = await invoke<WorkflowRow>("create_workflow", { payload: toRow(workflow) });
  return parseWorkflow(row);
}

export async function updateWorkflow(workflow: Workflow): Promise<Workflow> {
  if (!isTauri()) return workflow;
  const { invoke } = await import("@tauri-apps/api/core");
  const row = await invoke<WorkflowRow>("update_workflow", { payload: toRow(workflow) });
  return parseWorkflow(row);
}

export async function deleteWorkflow(workflowId: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("delete_workflow", { workflowId });
}
