import type { PersonaDefinition } from "@/types";
import { isTauri } from "./env";
import { mockPersonas } from "@/mocks/data";

const LS_KEY = "activePersonaId";

export interface PersonaRow {
  id: string;
  name: string;
  version: string;
  definition: PersonaDefinition;
  isActive: boolean;
}

export async function listPersonas(): Promise<PersonaDefinition[]> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    const rows = await invoke<PersonaRow[]>("list_personas");
    return rows.map((r) => r.definition);
  }
  // Browser mode: try to load from localStorage, fallback to mock
  try {
    const cached = localStorage.getItem("personas");
    if (cached) return JSON.parse(cached);
  } catch {}
  return structuredClone(mockPersonas);
}

export async function getActivePersonaId(): Promise<string | null> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string | null>("get_active_persona_id");
  }
  // Browser mode: read from localStorage
  return localStorage.getItem(LS_KEY);
}

export async function setActivePersona(id: string): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("set_active_persona", { id });
  }
  // Always persist to localStorage as fallback
  localStorage.setItem(LS_KEY, id);
}

export async function updatePersona(persona: PersonaDefinition): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("update_persona", { persona });
  }
  // Browser mode: update in localStorage cache
  try {
    const cached = JSON.parse(localStorage.getItem("personas") || "[]") as PersonaDefinition[];
    const idx = cached.findIndex((p) => p.id === persona.id);
    if (idx >= 0) cached[idx] = persona;
    else cached.push(persona);
    localStorage.setItem("personas", JSON.stringify(cached));
  } catch {}
}
