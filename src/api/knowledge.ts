import type { KnowledgeBase, KnowledgeDoc, SearchResult } from "@/types/knowledge";
import { isTauri } from "./env";
import { mockKnowledgeBases, mockKnowledgeDocs } from "@/mocks/data";

function genId(prefix: string) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

export async function listKnowledgeBases(): Promise<KnowledgeBase[]> {
  if (!isTauri()) return structuredClone(mockKnowledgeBases);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<KnowledgeBase[]>("list_knowledge_bases");
}

export async function createKnowledgeBase(name: string, description?: string): Promise<KnowledgeBase> {
  if (!isTauri()) {
    const kb: KnowledgeBase = {
      id: genId("kb"),
      name,
      description: description ?? "",
      docCount: 0,
    };
    mockKnowledgeBases.push(kb);
    return { ...kb };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<KnowledgeBase>("create_knowledge_base", { name, description: description ?? null });
}

export async function listKnowledgeDocs(kbId: string): Promise<KnowledgeDoc[]> {
  if (!isTauri()) {
    return structuredClone(mockKnowledgeDocs).filter((d) => d.kbName === kbId);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<KnowledgeDoc[]>("list_knowledge_docs", { kbId });
}

export async function importDocument(
  kbId: string,
  title: string,
  source: string,
  content: string,
  chunkType?: string,
): Promise<string> {
  if (!isTauri()) {
    const doc: KnowledgeDoc = {
      id: genId("doc"),
      kbName: kbId,
      title,
      source,
      chunkType: (chunkType as KnowledgeDoc["chunkType"]) ?? "text",
      chunkCount: Math.max(1, Math.ceil(content.length / 500)),
      createdAt: new Date().toISOString(),
    };
    mockKnowledgeDocs.push(doc);
    const kb = mockKnowledgeBases.find((k) => k.id === kbId || k.name === kbId);
    if (kb) kb.docCount += 1;
    return doc.id;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("import_document", {
    kbId,
    title,
    source,
    content,
    chunkType: chunkType ?? null,
  });
}

export async function deleteDoc(docId: string): Promise<void> {
  if (!isTauri()) {
    const idx = mockKnowledgeDocs.findIndex((d) => d.id === docId);
    if (idx >= 0) {
      const doc = mockKnowledgeDocs[idx];
      mockKnowledgeDocs.splice(idx, 1);
      const kb = mockKnowledgeBases.find((k) => k.name === doc.kbName);
      if (kb && kb.docCount > 0) kb.docCount -= 1;
    }
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke("delete_doc", { docId });
}

export async function searchKnowledge(
  query: string,
  kbId?: string,
  topK?: number,
): Promise<SearchResult[]> {
  if (!isTauri()) {
    const q = query.toLowerCase();
    const docs = mockKnowledgeDocs.filter((d) => !kbId || d.kbName === kbId);
    return docs
      .map((d) => ({
        chunkId: genId("chunk"),
        text: `[${d.title}] ${d.source}`,
        score: Math.random() * 0.5 + 0.3,
        docTitle: d.title,
      }))
      .filter((r) => r.text.toLowerCase().includes(q))
      .slice(0, topK ?? 5);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SearchResult[]>("search_knowledge", {
    query,
    kbId: kbId ?? null,
    topK: topK ?? null,
  });
}
