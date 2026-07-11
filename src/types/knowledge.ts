export interface KnowledgeBase {
  id: string;
  name: string;
  description: string;
  docCount: number;
  children?: KnowledgeBase[];
}

export interface KnowledgeDoc {
  id: string;
  kbName: string;
  title: string;
  source: string;
  chunkType: "text" | "image" | "transcript";
  chunkCount: number;
  createdAt: string;
}

export interface SearchResult {
  chunkId: string;
  text: string;
  score: number;
  docTitle?: string;
}
