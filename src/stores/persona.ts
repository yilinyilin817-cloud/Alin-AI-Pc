import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { PersonaDefinition, Workflow } from "@/types";
import { listPersonas, getActivePersonaId, setActivePersona, updatePersona as apiUpdatePersona } from "@/api/persona";
import {
  listWorkflows as apiListWorkflows,
  createWorkflow as apiCreateWorkflow,
  updateWorkflow as apiUpdateWorkflow,
  deleteWorkflow as apiDeleteWorkflow,
} from "@/api/workflow";
import { mockPersonas } from "@/mocks/data";

export const usePersonaStore = defineStore("persona", () => {
  const personas = ref<PersonaDefinition[]>([]);
  const currentPersonaId = ref<string | null>(null);
  const loading = ref(false);
  const initialized = ref(false);

  const currentPersona = computed<PersonaDefinition | null>(
    () =>
      personas.value.find((p) => p.id === currentPersonaId.value) ??
      personas.value[0] ??
      null,
  );

  function ensureWorkflows(persona: PersonaDefinition): PersonaDefinition {
    const p = { ...persona };
    if (!Array.isArray(p.workflows)) {
      p.workflows = [];
    }
    if (!p.wechat) {
      p.wechat = {
        enableSegmentedReply: false,
        segmentDelay: 800,
        enableVoiceMessage: false,
        voiceAutoSend: false,
        voiceAsrEnabled: true,
        actionDescriptionMode: 'inline',
      };
    } else {
      if (!p.wechat.actionDescriptionMode) {
        p.wechat.actionDescriptionMode = 'inline';
      }
    }
    return p;
  }

  async function loadPersonas() {
    if (initialized.value && !loading.value) return;
    loading.value = true;
    try {
      const list = await listPersonas();
      personas.value = list.map(ensureWorkflows);
      const activeId = await getActivePersonaId();
      if (activeId) currentPersonaId.value = activeId;
      initialized.value = true;
    } catch (e) {
      console.warn("Failed to load personas from IPC, using defaults", e);
      personas.value = structuredClone(mockPersonas).map(ensureWorkflows);
      // 使用 mock 中第一个角色作为默认，而非硬编码
      if (!currentPersonaId.value && personas.value.length > 0) {
        currentPersonaId.value = personas.value[0].id;
      }
      initialized.value = true;
    } finally {
      loading.value = false;
    }
  }

  function selectPersona(id: string) {
    currentPersonaId.value = id;
    setActivePersona(id).catch((e) => console.warn("setActivePersona:", e));
  }

  async function updatePersona(id: string, updates: Partial<PersonaDefinition>) {
    const existing = personas.value.find((p) => p.id === id);
    if (!existing) return;
    // 深拷贝并脱敏响应式代理，避免 IPC/API 序列化异常，同时保留现有工作流
    const base = JSON.parse(JSON.stringify(existing)) as PersonaDefinition;
    const incoming = JSON.parse(JSON.stringify(updates)) as Partial<PersonaDefinition>;
    const updated = { ...base, ...incoming };
    await apiUpdatePersona(updated);
    const idx = personas.value.findIndex((p) => p.id === id);
    if (idx >= 0) {
      personas.value[idx] = ensureWorkflows(updated);
    }
  }

  function getPersonaById(id: string) {
    return personas.value.find((p) => p.id === id);
  }

  async function loadWorkflows(personaId: string) {
    const workflows = await apiListWorkflows(personaId);
    const idx = personas.value.findIndex((p) => p.id === personaId);
    if (idx >= 0) {
      personas.value[idx] = ensureWorkflows(personas.value[idx]);
      personas.value[idx].workflows = workflows;
    }
    return workflows;
  }

  async function createWorkflow(workflow: Workflow) {
    const created = await apiCreateWorkflow(workflow);
    const persona = getPersonaById(created.personaId);
    if (persona) {
      const list = persona.workflows ?? [];
      list.push(created);
      persona.workflows = list;
    }
    return created;
  }

  async function updateWorkflow(workflow: Workflow) {
    const updated = await apiUpdateWorkflow(workflow);
    const persona = getPersonaById(updated.personaId);
    if (persona) {
      const list = persona.workflows ?? [];
      const idx = list.findIndex((w) => w.id === updated.id);
      if (idx >= 0) {
        list[idx] = updated;
      } else {
        list.push(updated);
      }
      persona.workflows = list;
    }
    return updated;
  }

  async function deleteWorkflow(workflowId: string) {
    await apiDeleteWorkflow(workflowId);
    for (const persona of personas.value) {
      if (!persona.workflows) continue;
      const idx = persona.workflows.findIndex((w) => w.id === workflowId);
      if (idx >= 0) {
        persona.workflows.splice(idx, 1);
        break;
      }
    }
  }

  return {
    personas,
    currentPersonaId,
    currentPersona,
    loading,
    initialized,
    loadPersonas,
    selectPersona,
    updatePersona,
    getPersonaById,
    loadWorkflows,
    createWorkflow,
    updateWorkflow,
    deleteWorkflow,
  };
});
