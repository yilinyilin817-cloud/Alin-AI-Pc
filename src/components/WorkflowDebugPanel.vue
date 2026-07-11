<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { Workflow, WorkflowAction } from "@/types";

const props = defineProps<{
  visible: boolean;
  workflow: Workflow;
  testMessage?: string;
}>();

const emit = defineEmits<{
  "update:visible": [val: boolean];
}>();

interface StepResult {
  id: string;
  type: WorkflowAction["type"];
  typeLabel: string;
  input: Record<string, unknown>;
  output: string;
  elapsedMs: number;
  status: "pending" | "running" | "done" | "error";
}

const drawerVisible = computed({
  get: () => props.visible,
  set: (val) => emit("update:visible", val),
});

const testInput = ref(props.testMessage ?? "");
const running = ref(false);
const results = ref<StepResult[]>([]);

const actionTypeLabels: Record<WorkflowAction["type"], string> = {
  retrieve_memory: "检索记忆",
  query_knowledge: "查询知识库",
  web_search: "网络搜索",
  call_skill: "调用技能",
  send_message: "发送消息",
  set_context: "设置上下文",
};

watch(
  () => props.testMessage,
  (val) => {
    if (val !== undefined) testInput.value = val;
  },
);

watch(
  () => props.workflow,
  () => {
    results.value = [];
  },
);

function getSimulatedOutput(action: WorkflowAction, inputText: string): string {
  switch (action.type) {
    case "retrieve_memory": {
      const limit = action.config.limit ?? 5;
      return `检索到 ${limit} 条与「${inputText.slice(0, 20) || "当前上下文"}」相关的长期记忆。`;
    }
    case "query_knowledge": {
      const kb = action.config.knowledgeBaseId ?? "默认知识库";
      const limit = action.config.limit ?? 5;
      return `从「${kb}」命中 ${limit} 条知识片段。`;
    }
    case "web_search":
      return `搜索「${action.config.query || inputText}」完成，返回 3 条结果摘要。`;
    case "call_skill":
      return `调用技能「${action.config.skillName ?? "未指定"}」，参数 ${JSON.stringify(action.config.args ?? {})}，执行成功。`;
    case "send_message":
      return `发送消息：${action.config.content ?? ""}`;
    case "set_context":
      return `设置上下文 ${action.config.key ?? "?"} = ${action.config.value ?? ""}`;
    default:
      return "完成";
  }
}

async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function runWorkflow() {
  if (running.value) return;
  running.value = true;
  results.value = [];

  const inputText = testInput.value.trim();
  const startTime = performance.now();

  for (const action of props.workflow.actions) {
    const stepStart = performance.now();
    const step: StepResult = {
      id: action.id,
      type: action.type,
      typeLabel: actionTypeLabels[action.type],
      input: { ...action.config },
      output: "",
      elapsedMs: 0,
      status: "running",
    };
    results.value.push(step);

    // 模拟动作执行耗时
    await sleep(400 + Math.random() * 400);

    step.output = getSimulatedOutput(action, inputText);
    step.elapsedMs = Math.round(performance.now() - stepStart);
    step.status = "done";
  }

  const totalElapsed = Math.round(performance.now() - startTime);
  results.value.push({
    id: "__summary__",
    type: "send_message",
    typeLabel: "流程完成",
    input: { trigger: props.workflow.trigger, message: inputText },
    output: `工作流「${props.workflow.name}」执行完毕，共 ${props.workflow.actions.length} 步，总耗时 ${totalElapsed}ms。`,
    elapsedMs: totalElapsed,
    status: "done",
  });

  running.value = false;
}

function formatConfig(cfg: Record<string, unknown>): string {
  return JSON.stringify(cfg, null, 2);
}
</script>

<template>
  <el-drawer
    v-model="drawerVisible"
    :title="`调试：${workflow.name}`"
    size="560px"
    destroy-on-close
  >
    <div class="debug-panel">
      <div class="debug-meta">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="触发条件">
            <el-tag size="small">{{ workflow.trigger.type }}</el-tag>
            <span class="trigger-detail">
              <template v-if="workflow.trigger.type === 'message'">
                {{ workflow.trigger.pattern ?? "任意消息" }}
              </template>
              <template v-else-if="workflow.trigger.type === 'scheduled'">
                {{ workflow.trigger.cron }}
              </template>
              <template v-else-if="workflow.trigger.type === 'event'">
                {{ workflow.trigger.eventName }}
              </template>
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="动作数">
            {{ workflow.actions.length }}
          </el-descriptions-item>
        </el-descriptions>
      </div>

      <div class="debug-input">
        <el-input
          v-model="testInput"
          placeholder="输入测试消息（触发类型为 message 时生效）"
          :disabled="running"
        >
          <template #append>
            <el-button
              type="primary"
              :loading="running"
              @click="runWorkflow"
            >
              测试运行
            </el-button>
          </template>
        </el-input>
      </div>

      <div v-if="results.length === 0" class="debug-empty">
        点击“测试运行”查看每步执行结果
      </div>

      <div v-else class="debug-timeline">
        <div
          v-for="(step, index) in results"
          :key="step.id"
          class="debug-step"
          :class="{ 'is-summary': step.id === '__summary__' }"
        >
          <div class="step-header">
            <div class="step-index">{{ index + 1 }}</div>
            <div class="step-title">{{ step.typeLabel }}</div>
            <div class="step-time">{{ step.elapsedMs }}ms</div>
          </div>
          <div class="step-body">
            <div class="step-section">
              <div class="section-label">输入</div>
              <pre class="section-code">{{ formatConfig(step.input) }}</pre>
            </div>
            <div class="step-section">
              <div class="section-label">输出</div>
              <div class="section-output">
                <el-skeleton v-if="step.status === 'running'" :rows="2" animated />
                <span v-else>{{ step.output }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </el-drawer>
</template>

<style scoped lang="scss">
.debug-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.debug-meta {
  :deep(.el-descriptions__body) {
    background: var(--color-bg-surface);
  }

  :deep(.el-descriptions__label) {
    background: var(--glass-bg-light);
    color: var(--color-text-secondary);
  }

  :deep(.el-descriptions__content) {
    color: var(--color-text-primary);
  }
}

.trigger-detail {
  margin-left: 8px;
  color: var(--color-text-secondary);
  font-size: 13px;
}

.debug-input {
  :deep(.el-input-group__append) {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: #fff;
  }
}

.debug-empty {
  padding: 40px 20px;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 13px;
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
}

.debug-timeline {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.debug-step {
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;

  &.is-summary {
    border-color: var(--color-primary);
    background: var(--glass-gradient);
  }
}

.step-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--glass-bg-light);
  border-bottom: 1px solid var(--color-border);
}

.step-index {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--color-primary);
  color: #fff;
  font-size: 12px;
  flex-shrink: 0;
}

.step-title {
  flex: 1;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
}

.step-time {
  font-size: 12px;
  color: var(--color-text-muted);
}

.step-body {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.step-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.section-label {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.section-code {
  margin: 0;
  padding: 8px 10px;
  background: var(--color-bg-base);
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  color: var(--color-text-primary);
  font-family: var(--font-family-code);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
}

.section-output {
  padding: 8px 10px;
  background: var(--color-bg-base);
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  color: var(--color-text-primary);
  font-size: 13px;
  line-height: 1.5;
}

:deep(.el-drawer) {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

:deep(.el-drawer__header) {
  background: var(--glass-gradient);
  border-bottom: 1px solid var(--glass-border);
  padding: 16px 20px;
  margin-bottom: 0;
  color: var(--color-text-primary);
}

:deep(.el-drawer__body) {
  padding: 20px;
  background: transparent;
}
</style>
