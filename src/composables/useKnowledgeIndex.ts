import { listKnowledgeBases, createKnowledgeBase, importDocument } from "@/api/knowledge";
import { usePersonaStore } from "@/stores/persona";

const PERSONA_KB_PREFIX = "chat_";
const PERSONA_KB_DESC_SUFFIX = "的对话记忆";

function personaKbName(personaId: string): string {
  return `${PERSONA_KB_PREFIX}${personaId}`;
}

export async function ensurePersonaKnowledgeBase(personaId: string): Promise<string> {
  const kbName = personaKbName(personaId);
  const bases = await listKnowledgeBases();
  const existing = bases.find((b) => b.name === kbName || b.id === kbName);
  if (existing) {
    const kbRef = existing.id || existing.name;
    await ensureKbLinkedToPersona(personaId, kbRef);
    return kbRef;
  }

  const personaStore = usePersonaStore();
  const persona = personaStore.getPersonaById(personaId);
  const displayName = persona?.name ?? personaId;
  const kb = await createKnowledgeBase(kbName, `${displayName}${PERSONA_KB_DESC_SUFFIX}`);
  const kbRef = kb.id || kb.name;
  await ensureKbLinkedToPersona(personaId, kbRef);
  return kbRef;
}

async function ensureKbLinkedToPersona(personaId: string, kbRef: string) {
  const personaStore = usePersonaStore();
  const persona = personaStore.getPersonaById(personaId);
  if (!persona) return;
  const existing = persona.knowledgeBases ?? [];
  const kbName = personaKbName(personaId);
  if (!existing.includes(kbRef) && !existing.includes(kbName)) {
    const updated = [...existing, kbRef];
    await personaStore.updatePersona(personaId, { knowledgeBases: updated });
  }
}

export async function indexChatExchange(
  personaId: string,
  userMessage: string,
  assistantReply: string,
  sessionTitle?: string,
): Promise<void> {
  if (!userMessage.trim() || !assistantReply.trim()) return;
  try {
    const kbRef = await ensurePersonaKnowledgeBase(personaId);
    const now = new Date();
    const dateStr = now.toISOString().slice(0, 10);
    const timeStr = now.toTimeString().slice(0, 5);
    const title = `${sessionTitle ? sessionTitle + " - " : ""}${dateStr} ${timeStr}`;
    const source = `chat/${personaId}/${now.toISOString().replace(/[:.]/g, "-")}.md`;
    const content = [
      `# 对话记录 - ${dateStr} ${timeStr}`,
      sessionTitle ? `\n## 会话\n${sessionTitle}` : "",
      `\n## 用户\n${userMessage.trim()}`,
      `\n## 回复\n${assistantReply.trim()}`,
      `\n---\n记录时间: ${now.toLocaleString()}`,
    ].filter(Boolean).join("\n");

    await importDocument(kbRef, title, source, content, "text");
  } catch (e) {
    console.warn("indexChatExchange failed:", e);
  }
}
