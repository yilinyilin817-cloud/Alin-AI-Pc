<script setup lang="ts">
import { ref, computed } from "vue";
import { ElMessage } from "element-plus";
import { Plus, Sort, ArrowUp, ArrowDown, Delete } from "@element-plus/icons-vue";
import EmptyStateInline from "./EmptyStateInline.vue";
import type { WorkflowAction } from "@/types";

const props = defineProps<{
  modelValue: WorkflowAction[];
}>();

const emit = defineEmits<{
  "update:modelValue": [actions: WorkflowAction[]];
}>();

const actionTypes: { value: WorkflowAction["type"]; label: string }[] = [
  { value: "retrieve_memory", label: "检索记忆" },
  { value: "query_knowledge", label: "查询知识库" },
  { value: "web_search", label: "网络搜索" },
  { value: "call_skill", label: "调用技能" },
  { value: "send_message", label: "发送消息" },
  { value: "set_context", label: "设置上下文" },
];

const actions = computed({
  get: () => props.modelValue,
  set: (val) => emit("update:modelValue", val),
});

const dialogVisible = ref(false);
const editingIndex = ref<number>(-1);
const editingType = ref<WorkflowAction["type"]>("retrieve_memory");
const editingConfig = ref<Record<string, unknown>>({});

function generateId() {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `action_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

function getTypeLabel(type: WorkflowAction["type"]) {
  return actionTypes.find((t) => t.value === type)?.label ?? type;
}

function getConfigSummary(action: WorkflowAction) {
  const cfg = action.config;
  switch (action.type) {
    case "retrieve_memory":
      return cfg.limit ? `limit=${cfg.limit}` : "默认限制";
    case "query_knowledge":
      return `${cfg.knowledgeBaseId ?? "未选知识库"}${cfg.limit ? `, limit=${cfg.limit}` : ""}`;
    case "web_search":
      return cfg.query ? String(cfg.query).slice(0, 40) : "未填写查询";
    case "call_skill":
      return cfg.skillName ? String(cfg.skillName) : "未选择技能";
    case "send_message":
      return cfg.content ? String(cfg.content).slice(0, 40) : "未填写内容";
    case "set_context":
      return `${cfg.key ?? "未设置键"} = ${cfg.value ?? ""}`;
    default:
      return "";
  }
}

function openAddDialog(type: WorkflowAction["type"]) {
  editingIndex.value = -1;
  editingType.value = type;
  editingConfig.value = getDefaultConfig(type);
  dialogVisible.value = true;
}

function openEditDialog(index: number) {
  editingIndex.value = index;
  const action = actions.value[index];
  editingType.value = action.type;
  editingConfig.value = { ...action.config };
  dialogVisible.value = true;
}

function getDefaultConfig(type: WorkflowAction["type"]): Record<string, unknown> {
  switch (type) {
    case "retrieve_memory":
      return { limit: 5 };
    case "query_knowledge":
      return { knowledgeBaseId: "", limit: 5 };
    case "web_search":
      return { query: "" };
    case "call_skill":
      return { skillName: "", args: {} };
    case "send_message":
      return { content: "" };
    case "set_context":
      return { key: "", value: "" };
    default:
      return {};
  }
}

function saveAction() {
  const config = cleanConfig(editingType.value, editingConfig.value);
  const action: WorkflowAction = {
    id: editingIndex.value >= 0 ? actions.value[editingIndex.value].id : generateId(),
    type: editingType.value,
    config,
  };

  const list = [...actions.value];
  if (editingIndex.value >= 0) {
    list[editingIndex.value] = action;
  } else {
    list.push(action);
  }
  actions.value = list;
  dialogVisible.value = false;
}

function cleanConfig(type: WorkflowAction["type"], cfg: Record<string, unknown>): Record<string, unknown> {
  switch (type) {
    case "retrieve_memory": {
      const limit = cfg.limit === "" || cfg.limit == null ? undefined : Number(cfg.limit);
      return limit != null && !Number.isNaN(limit) ? { limit } : {};
    }
    case "query_knowledge": {
      const knowledgeBaseId = cfg.knowledgeBaseId ? String(cfg.knowledgeBaseId) : undefined;
      const limit = cfg.limit === "" || cfg.limit == null ? undefined : Number(cfg.limit);
      const out: Record<string, unknown> = {};
      if (knowledgeBaseId) out.knowledgeBaseId = knowledgeBaseId;
      if (limit != null && !Number.isNaN(limit)) out.limit = limit;
      return out;
    }
    case "web_search":
      return cfg.query ? { query: String(cfg.query) } : {};
    case "call_skill": {
      const skillName = cfg.skillName ? String(cfg.skillName) : undefined;
      let args = cfg.args;
      if (typeof args === "string") {
        try {
          args = args ? JSON.parse(args) : {};
        } catch {
          ElMessage.error("技能参数必须是合法 JSON 对象");
          throw new Error("Invalid JSON args");
        }
      }
      const out: Record<string, unknown> = {};
      if (skillName) out.skillName = skillName;
      if (args && Object.keys(args as object).length > 0) out.args = args;
      return out;
    }
    case "send_message":
      return cfg.content ? { content: String(cfg.content) } : {};
    case "set_context": {
      const key = cfg.key ? String(cfg.key) : undefined;
      const value = cfg.value !== undefined ? String(cfg.value) : undefined;
      const out: Record<string, unknown> = {};
      if (key) out.key = key;
      if (value !== undefined) out.value = value;
      return out;
    }
    default:
      return cfg;
  }
}

function removeAction(index: number) {
  actions.value = actions.value.filter((_, i) => i !== index);
}

function moveAction(index: number, direction: -1 | 1) {
  const target = index + direction;
  if (target < 0 || target >= actions.value.length) return;
  const list = [...actions.value];
  const temp = list[index];
  list[index] = list[target];
  list[target] = temp;
  actions.value = list;
}

// 原生拖拽排序
const dragIndex = ref<number | null>(null);

function onDragStart(index: number) {
  dragIndex.value = index;
}

function onDragOver(event: DragEvent, index: number) {
  event.preventDefault();
  if (dragIndex.value === null || dragIndex.value === index) return;
  const list = [...actions.value];
  const item = list.splice(dragIndex.value, 1)[0];
  list.splice(index, 0, item);
  dragIndex.value = index;
  actions.value = list;
}

function onDragEnd() {
  dragIndex.value = null;
}
</script>

<template>
  <div class="workflow-node-editor">
    <div class="node-list">
      <div
        v-for="(action, index) in actions"
        :key="action.id"
        class="node-item"
        draggable="true"
        @dragstart="onDragStart(index)"
        @dragover="onDragOver($event, index)"
        @dragend="onDragEnd"
        @click="openEditDialog(index)"
      >
        <div class="node-drag-handle">
          <el-icon><Sort /></el-icon>
        </div>
        <div class="node-index">{{ index + 1 }}</div>
        <div class="node-body">
          <div class="node-type">{{ getTypeLabel(action.type) }}</div>
          <div class="node-summary">{{ getConfigSummary(action) }}</div>
        </div>
        <div class="node-actions" @click.stop>
          <el-button
            link
            size="small"
            :disabled="index === 0"
            @click="moveAction(index, -1)"
          >
            <el-icon><ArrowUp /></el-icon>
          </el-button>
          <el-button
            link
            size="small"
            :disabled="index === actions.length - 1"
            @click="moveAction(index, 1)"
          >
            <el-icon><ArrowDown /></el-icon>
          </el-button>
          <el-button
            link
            type="danger"
            size="small"
            @click="removeAction(index)"
          >
            <el-icon><Delete /></el-icon>
          </el-button>
        </div>
      </div>

      <EmptyStateInline
        v-if="actions.length === 0"
        icon="🧩"
        title="暂无动作节点"
        description="点击下方按钮添加第一个工作流动作"
      />
    </div>

    <div class="add-node-toolbar">
      <el-dropdown trigger="click" placement="bottom-start">
        <el-button type="primary" plain>
          <el-icon class="el-icon--left"><Plus /></el-icon>
          添加动作
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              v-for="t in actionTypes"
              :key="t.value"
              @click="openAddDialog(t.value)"
            >
              {{ t.label }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <el-dialog
      v-model="dialogVisible"
      :title="(editingIndex >= 0 ? '编辑' : '添加') + '动作：' + getTypeLabel(editingType)"
      width="520px"
      destroy-on-close
    >
      <el-form label-position="top">
        <template v-if="editingType === 'retrieve_memory'">
          <el-form-item label="返回条数限制">
            <el-input-number v-model="editingConfig.limit" :min="1" :max="50" />
          </el-form-item>
        </template>

        <template v-if="editingType === 'query_knowledge'">
          <el-form-item label="知识库 ID">
            <el-input v-model="editingConfig.knowledgeBaseId" placeholder="例如：personal_diary" />
          </el-form-item>
          <el-form-item label="返回条数限制">
            <el-input-number v-model="editingConfig.limit" :min="1" :max="50" />
          </el-form-item>
        </template>

        <template v-if="editingType === 'web_search'">
          <el-form-item label="查询语句">
            <el-input v-model="editingConfig.query" placeholder="搜索关键词或问题" />
          </el-form-item>
        </template>

        <template v-if="editingType === 'call_skill'">
          <el-form-item label="技能名称">
            <el-input v-model="editingConfig.skillName" placeholder="例如：weather" />
          </el-form-item>
          <el-form-item label="参数（JSON 对象）">
            <el-input
              v-model="editingConfig.args"
              type="textarea"
              :rows="4"
              placeholder='{"city": "上海"}'
            />
          </el-form-item>
        </template>

        <template v-if="editingType === 'send_message'">
          <el-form-item label="消息内容">
            <el-input
              v-model="editingConfig.content"
              type="textarea"
              :rows="4"
              placeholder="要发送的内容"
            />
          </el-form-item>
        </template>

        <template v-if="editingType === 'set_context'">
          <el-form-item label="键">
            <el-input v-model="editingConfig.key" placeholder="context key" />
          </el-form-item>
          <el-form-item label="值">
            <el-input v-model="editingConfig.value" placeholder="context value" />
          </el-form-item>
        </template>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveAction">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.workflow-node-editor {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.node-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.node-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-base);

  &:hover {
    border-color: var(--color-primary);
    background: var(--color-bg-hover);
  }
}

.node-drag-handle {
  color: var(--color-text-muted);
  cursor: grab;

  &:active {
    cursor: grabbing;
  }
}

.node-index {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--glass-gradient);
  border: 1px solid var(--glass-border);
  font-size: 12px;
  color: var(--color-text-secondary);
  flex-shrink: 0;
}

.node-body {
  flex: 1;
  min-width: 0;
}

.node-type {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
}

.node-summary {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.node-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.add-node-toolbar {
  display: flex;
  justify-content: flex-start;
}

:deep(.el-input-number__decrease),
:deep(.el-input-number__increase) {
  background: var(--color-bg-surface) !important;
  border-color: var(--color-border) !important;
  color: var(--color-text-primary) !important;
}

:deep(.el-dialog) {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);

  .el-dialog__header {
    background: var(--glass-gradient);
    border-bottom: 1px solid var(--glass-border);
    padding: 16px 20px;
  }

  .el-dialog__body {
    background: transparent;
    padding: 20px;
  }

  .el-dialog__footer {
    background: transparent;
    padding: 12px 20px;
    border-top: 1px solid var(--glass-border);
  }
}
</style>
