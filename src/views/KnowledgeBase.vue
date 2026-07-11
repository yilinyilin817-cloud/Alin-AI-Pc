<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { Upload, FolderOpened, Plus, Delete, Document, Search } from "@element-plus/icons-vue";
import { listKnowledgeBases, listKnowledgeDocs, deleteDoc, importDocument, createKnowledgeBase } from "@/api/knowledge";
import type { KnowledgeBase, KnowledgeDoc } from "@/types/knowledge";

const searchQuery = ref("");
const selectedKbId = ref("");
const treeData = ref<KnowledgeBase[]>([]);
const docs = ref<KnowledgeDoc[]>([]);
const uploading = ref(false);
const uploadDialogVisible = ref(false);
const createKbDialogVisible = ref(false);
const newKbName = ref("");
const newKbDesc = ref("");
const fileInputRef = ref<HTMLInputElement | null>(null);
const pendingFiles = ref<File[]>([]);

const SUPPORTED_EXTS = [".txt", ".md", ".markdown", ".json", ".csv", ".log", ".xml", ".html", ".htm", ".yaml", ".yml", ".rst", ".tex", ".py", ".js", ".ts", ".java", ".c", ".cpp", ".h", ".rs", ".go", ".rb", ".sh", ".bat", ".ps1"];

const chunkTypeLabel: Record<string, string> = {
  text: "文本",
  image: "图片",
  transcript: "转写",
};

const filteredDocs = computed(() => {
  if (!searchQuery.value) return docs.value;
  const q = searchQuery.value.toLowerCase();
  return docs.value.filter((d) => d.title?.toLowerCase().includes(q) || d.source?.toLowerCase().includes(q));
});

const selectedKb = computed(() =>
  treeData.value.find((k) => k.id === selectedKbId.value || k.name === selectedKbId.value)
);

const selectedKbName = computed(() => selectedKb.value?.name ?? "");

const pendingFileNames = computed(() => pendingFiles.value.map((f) => f.name));

onMounted(async () => {
  await refreshKbList();
});

async function refreshKbList() {
  try {
    const bases = await listKnowledgeBases();
    treeData.value = bases;
    if (bases.length > 0 && !selectedKbId.value) {
      selectedKbId.value = bases[0].id || bases[0].name;
      await loadDocs();
    }
  } catch (e) {
    console.warn("load kb:", e);
  }
}

function handleNodeClick(data: KnowledgeBase) {
  selectedKbId.value = data.id || data.name;
  loadDocs();
}

async function loadDocs() {
  if (!selectedKbId.value) return;
  try {
    docs.value = await listKnowledgeDocs(selectedKbId.value);
  } catch (e) {
    console.warn("load docs:", e);
  }
}

function triggerFilePick() {
  fileInputRef.value?.click();
}

function onFileSelected(e: Event) {
  const input = e.target as HTMLInputElement;
  if (!input.files) return;
  const files = Array.from(input.files);
  const valid: File[] = [];
  for (const f of files) {
    const ext = "." + f.name.split(".").pop()?.toLowerCase();
    if (!SUPPORTED_EXTS.includes(ext) && f.type !== "text/plain" && !f.type.startsWith("text/")) {
      ElMessage.warning(`文件 "${f.name}" 格式暂不支持，已跳过`);
      continue;
    }
    if (f.size > 10 * 1024 * 1024) {
      ElMessage.warning(`文件 "${f.name}" 超过 10MB，已跳过`);
      continue;
    }
    valid.push(f);
  }
  pendingFiles.value = valid;
  input.value = "";
}

function removePendingFile(index: number) {
  pendingFiles.value.splice(index, 1);
}

async function confirmUpload() {
  if (!selectedKbId.value) {
    ElMessage.warning("请先选择一个知识库");
    return;
  }
  if (pendingFiles.value.length === 0) {
    ElMessage.warning("请先选择要上传的文件");
    return;
  }

  uploading.value = true;
  const kbRef = selectedKbId.value;
  let successCount = 0;
  let failCount = 0;

  for (const file of pendingFiles.value) {
    try {
      const content = await readFileAsText(file);
      const title = file.name.replace(/\.[^.]+$/, "");
      await importDocument(kbRef, title, file.name, content, "text");
      successCount++;
    } catch (e) {
      console.warn("import file:", file.name, e);
      failCount++;
    }
  }

  uploading.value = false;
  uploadDialogVisible.value = false;
  pendingFiles.value = [];

  if (successCount > 0) {
    ElMessage.success(`成功导入 ${successCount} 个文件${failCount > 0 ? `，${failCount} 个失败` : ""}`);
    await loadDocs();
    await refreshKbList();
  } else {
    ElMessage.error("文件导入失败");
  }
}

function readFileAsText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(file, "utf-8");
  });
}

function openCreateKbDialog() {
  newKbName.value = "";
  newKbDesc.value = "";
  createKbDialogVisible.value = true;
}

async function confirmCreateKb() {
  const name = newKbName.value.trim();
  if (!name) {
    ElMessage.warning("请输入知识库名称");
    return;
  }
  try {
    const kb = await createKnowledgeBase(name, newKbDesc.value.trim() || undefined);
    ElMessage.success("知识库已创建");
    await refreshKbList();
    selectedKbId.value = kb.id || kb.name;
    await loadDocs();
    createKbDialogVisible.value = false;
  } catch (e) {
    ElMessage.error("创建知识库失败: " + String(e));
  }
}

async function handleDelete(docId: string) {
  try {
    await ElMessageBox.confirm("确定删除该文档？删除后不可恢复。", "确认删除", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    await deleteDoc(docId);
    ElMessage.success("已删除");
    await loadDocs();
    await refreshKbList();
  } catch (e) {
    if (e !== "cancel") {
      ElMessage.error("删除失败");
    }
  }
}
</script>

<template>
  <div class="page-container knowledge-page">
    <div class="kb-page-header">
      <div>
        <h1 class="page-title">知识库</h1>
        <p class="page-subtitle">管理 AI 伴侣的知识来源</p>
      </div>
      <div class="kb-header-actions">
        <el-button type="primary" :icon="Upload" :disabled="!selectedKbId" @click="uploadDialogVisible = true">
          上传文件
        </el-button>
        <el-button :icon="Plus" @click="openCreateKbDialog">
          新建知识库
        </el-button>
      </div>
    </div>

    <div class="kb-layout">
      <aside class="kb-sidebar">
        <div class="kb-tree-header">
          <span>知识库列表</span>
          <el-button size="small" text :icon="Plus" @click="openCreateKbDialog" />
        </div>
        <el-tree
          :data="treeData"
          node-key="id"
          :props="{ label: 'name', children: 'children' }"
          @node-click="handleNodeClick"
          :highlight-current="true"
          :default-expanded-keys="treeData.map((t) => t.id)"
        >
          <template #default="{ data }">
            <div class="kb-tree-node">
              <el-icon><FolderOpened /></el-icon>
              <span class="kb-tree-name">{{ data.name }}</span>
              <span class="kb-tree-count">{{ data.docCount }}</span>
            </div>
          </template>
        </el-tree>
      </aside>

      <div class="kb-main">
        <div class="kb-toolbar">
          <el-input
            v-model="searchQuery"
            placeholder="搜索文档…"
            clearable
            class="kb-search"
            size="large"
            :prefix-icon="Search"
          />
          <span v-if="selectedKbName" class="kb-current-kb">
            <el-icon><FolderOpened /></el-icon>
            {{ selectedKbName }}
          </span>
        </div>

        <el-table :data="filteredDocs" stripe class="kb-doc-table">
          <el-table-column label="标题" min-width="240">
            <template #default="scope">
              <div class="doc-title-cell">
                <el-icon class="doc-icon"><Document /></el-icon>
                <span>{{ scope.row.title }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="source" label="来源" min-width="180" />
          <el-table-column label="类型" width="80">
            <template #default="scope">
              <el-tag :type="scope.row.chunkType === 'text' ? '' : 'warning'" size="small">
                {{ chunkTypeLabel[scope.row.chunkType] ?? scope.row.chunkType }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="chunkCount" label="块数" width="70" />
          <el-table-column label="日期" width="110">
            <template #default="scope">
              {{ new Date(scope.row.createdAt).toLocaleDateString() }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="scope">
              <el-button text size="small" type="danger" @click="handleDelete(scope.row.id)">
                删除
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </div>

    <input
      ref="fileInputRef"
      type="file"
      multiple
      accept=".txt,.md,.markdown,.json,.csv,.log,.xml,.html,.htm,.yaml,.yml,.rst,.tex,.py,.js,.ts,.java,.c,.cpp,.h,.rs,.go,.rb,.sh,.bat,.ps1,text/*"
      style="display: none"
      @change="onFileSelected"
    />

    <el-dialog
      v-model="uploadDialogVisible"
      title="上传文档到知识库"
      width="520px"
      :close-on-click-modal="false"
      class="kb-upload-dialog"
    >
      <div v-if="selectedKbName" class="upload-target-kb">
        目标知识库：<strong>{{ selectedKbName }}</strong>
      </div>

      <div v-if="pendingFiles.length === 0" class="upload-dropzone" @click="triggerFilePick">
        <el-icon class="upload-icon"><Upload /></el-icon>
        <p class="upload-hint-main">点击选择文件</p>
        <p class="upload-hint-sub">支持 .txt / .md / .json / .csv / .py 等文本格式，单个文件不超过 10MB</p>
      </div>

      <div v-else class="upload-file-list">
        <div v-for="(name, idx) in pendingFileNames" :key="name" class="upload-file-item">
          <el-icon class="file-icon"><Document /></el-icon>
          <span class="file-name">{{ name }}</span>
          <el-button text size="small" type="danger" :icon="Delete" @click="removePendingFile(idx)" />
        </div>
      </div>

      <div class="upload-actions">
        <el-button v-if="pendingFiles.length > 0" @click="pendingFiles = []">清空列表</el-button>
        <el-button @click="triggerFilePick">{{ pendingFiles.length > 0 ? '继续添加' : '选择文件' }}</el-button>
        <el-button type="primary" :loading="uploading" :disabled="pendingFiles.length === 0" @click="confirmUpload">
          开始导入
        </el-button>
      </div>
    </el-dialog>

    <el-dialog
      v-model="createKbDialogVisible"
      title="新建知识库"
      width="440px"
      :close-on-click-modal="false"
    >
      <el-form label-position="top">
        <el-form-item label="知识库名称">
          <el-input v-model="newKbName" placeholder="例如：个人笔记、项目文档" maxlength="50" />
        </el-form-item>
        <el-form-item label="描述（可选）">
          <el-input v-model="newKbDesc" type="textarea" :rows="2" placeholder="简短描述知识库内容" maxlength="200" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createKbDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmCreateKb">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.knowledge-page {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.kb-page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  flex-shrink: 0;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.kb-header-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
  flex-wrap: wrap;
}

.kb-layout {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 20px;
}

.kb-sidebar {
  width: 260px;
  flex-shrink: 0;
  padding: 16px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  overflow-y: auto;
}

.kb-tree-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--color-text-secondary);
  font-weight: 500;
}

.kb-tree-node {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
  padding-right: 4px;

  .el-icon {
    color: var(--color-primary);
    font-size: 14px;
    flex-shrink: 0;
  }
}

.kb-tree-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}

.kb-tree-count {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--color-text-muted);
  background: var(--glass-bg-light);
  padding: 1px 6px;
  border-radius: 10px;
  min-width: 20px;
  text-align: center;
}

:deep(.el-tree-node__content) {
  height: 34px;
  border-radius: var(--radius-sm);
}

:deep(.el-tree-node.is-current > .el-tree-node__content) {
  background: var(--glass-hover-bg) !important;
}

.kb-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.kb-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.kb-search {
  flex: 1;
  max-width: 400px;

  :deep(.el-input__wrapper) {
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    border: 1px solid var(--glass-border);
    box-shadow: none;

    &:hover { border-color: var(--glass-border-hover); }
    &.is-focus { border-color: var(--color-primary); box-shadow: var(--glass-glow); }
  }
}

.kb-current-kb {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--color-text-secondary);
  background: var(--glass-bg-light);
  padding: 6px 12px;
  border-radius: var(--radius-md);
  white-space: nowrap;

  .el-icon { color: var(--color-primary); }
}

.kb-doc-table {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.doc-title-cell {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;

  .doc-icon {
    color: var(--color-primary);
    font-size: 16px;
    flex-shrink: 0;
  }

  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

:deep(.el-table) {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);

  .el-table__header-wrapper th {
    background: var(--glass-gradient) !important;
    border-bottom: 1px solid var(--glass-border);
  }

  .el-table__body-wrapper tr {
    background: transparent !important;
    transition: background var(--transition);

    &:hover > td {
      background: var(--glass-hover-bg) !important;
    }
  }

  .el-table__body-wrapper td {
    border-bottom: 1px solid var(--glass-border);
  }
}

.upload-target-kb {
  margin-bottom: 16px;
  padding: 10px 14px;
  background: var(--glass-bg-light);
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--color-text-secondary);

  strong {
    color: var(--color-primary);
  }
}

.upload-dropzone {
  border: 2px dashed var(--glass-border);
  border-radius: var(--radius-lg);
  padding: 40px 20px;
  text-align: center;
  cursor: pointer;
  transition: all var(--transition);

  &:hover {
    border-color: var(--color-primary);
    background: var(--glass-hover-bg);
  }

  .upload-icon {
    font-size: 40px;
    color: var(--color-primary);
    margin-bottom: 12px;
  }

  .upload-hint-main {
    font-size: 15px;
    color: var(--color-text-primary);
    margin: 0 0 6px;
    font-weight: 500;
  }

  .upload-hint-sub {
    font-size: 12px;
    color: var(--color-text-muted);
    margin: 0;
  }
}

.upload-file-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 280px;
  overflow-y: auto;
  margin-bottom: 16px;
}

.upload-file-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--glass-bg-light);
  border-radius: var(--radius-md);
  border: 1px solid var(--glass-border);

  .file-icon {
    color: var(--color-primary);
    font-size: 16px;
    flex-shrink: 0;
  }

  .file-name {
    flex: 1;
    font-size: 13px;
    color: var(--color-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
}

.upload-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding-top: 12px;
  border-top: 1px solid var(--glass-border);
}
</style>
