<script setup lang="ts">
import { ref, computed, watch, toRaw, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { Plus } from "@element-plus/icons-vue";
import PersonaCard from "@/components/PersonaCard.vue";
import WorkflowNodeEditor from "@/components/WorkflowNodeEditor.vue";
import WorkflowDebugPanel from "@/components/WorkflowDebugPanel.vue";
import EmptyStateInline from "@/components/EmptyStateInline.vue";
import { usePersonaStore } from "@/stores/persona";
import { usePluginStore } from "@/stores/plugin";
import type { PersonaDefinition, Workflow, WorkflowTrigger, InstalledPlugin } from "@/types";

const route = useRoute();
const router = useRouter();
const personaStore = usePersonaStore();
const pluginStore = usePluginStore();

const activeTab = ref<string>((route.query.tab as string) || "basic");

watch(
  () => route.query.tab,
  (tab) => {
    if (typeof tab === "string" && tab) {
      activeTab.value = tab;
    }
  },
);
const editingPersona = ref<PersonaDefinition | null>(null);
const isDirty = ref(false);
const saving = ref(false);
const loadingPersona = ref(false);
const selectedPersonaId = ref<string>("");
let syncSeq = 0;

const workflowDialogVisible = ref(false);
const workflowSaving = ref(false);
const isNewWorkflow = ref(false);
const editingWorkflow = ref<Workflow | null>(null);
const debugPanelVisible = ref(false);
const debugWorkflow = ref<Workflow | null>(null);
const debugTestMessage = ref("");

/** 深拷贝为普通对象，避免 reactive 与 ref 嵌套导致的响应式追踪问题 */
function clonePersona(p: PersonaDefinition): PersonaDefinition {
  const cloned = structuredClone(toRaw(p)) as PersonaDefinition;
  if (!cloned.wechat) {
    cloned.wechat = {
      enableSegmentedReply: false,
      segmentDelay: 800,
      enableVoiceMessage: false,
      voiceAutoSend: false,
      voiceAsrEnabled: true,
      actionDescriptionMode: 'inline',
    };
  }
  if (!cloned.wechat.actionDescriptionMode) {
    cloned.wechat.actionDescriptionMode = 'inline';
  }
  return cloned;
}

const messageTrigger = computed({
  get: () =>
    editingWorkflow.value?.trigger.type === "message"
      ? (editingWorkflow.value.trigger as { type: "message"; pattern?: string })
      : { type: "message" as const, pattern: "" },
  set: (val) => {
    if (editingWorkflow.value) editingWorkflow.value.trigger = val;
  },
});

const scheduledTrigger = computed({
  get: () =>
    editingWorkflow.value?.trigger.type === "scheduled"
      ? (editingWorkflow.value.trigger as { type: "scheduled"; cron: string })
      : { type: "scheduled" as const, cron: "" },
  set: (val) => {
    if (editingWorkflow.value) editingWorkflow.value.trigger = val;
  },
});

const eventTrigger = computed({
  get: () =>
    editingWorkflow.value?.trigger.type === "event"
      ? (editingWorkflow.value.trigger as { type: "event"; eventName: string })
      : { type: "event" as const, eventName: "" },
  set: (val) => {
    if (editingWorkflow.value) editingWorkflow.value.trigger = val;
  },
});

const personalityOptions = [
  "温柔", "幽默", "理性", "沉稳", "可靠", "活泼", "冷静", "热情",
];

const llmProviders = [
  { label: "Gemma 4 E4B", value: "gemma4:e4b" },
  { label: "Gemma 4 12B Unified", value: "gemma4-12b" },
  { label: "Qwen3-VL 8B", value: "qwen3-vl-8b" },
];

const skillOptions = [
  { label: "天气查询", value: "weather" },
  { label: "提醒", value: "reminder" },
  { label: "文件搜索", value: "file_search" },
  { label: "网络搜索", value: "web_search" },
];

const kbOptions = [
  { label: "个人日记", value: "personal_diary" },
  { label: "用户相册", value: "user_photos" },
  { label: "工作文档", value: "work_docs" },
];

const routePersonaId = computed(() => {
  const id = route.params.id;
  return typeof id === "string" && id ? id : "";
});

const personas = computed(() => personaStore.personas);

async function syncEditing(targetId: string) {
  if (!personaStore.initialized) return;
  const list = personas.value;
  if (list.length === 0) return;
  if (!targetId) return;

  const mySeq = ++syncSeq;
  loadingPersona.value = true;

  try {
    let p = personaStore.getPersonaById(targetId);
    let resolvedId = targetId;

    if (!p) {
      const first = list[0];
      if (first) {
        resolvedId = first.id;
        p = first;
      } else {
        return;
      }
    }

    await personaStore.loadWorkflows(resolvedId).catch(() => {});
    if (mySeq !== syncSeq) return;

    selectedPersonaId.value = resolvedId;
    personaStore.selectPersona(resolvedId);
    editingPersona.value = clonePersona(p);
    isDirty.value = false;
  } finally {
    if (mySeq === syncSeq) {
      loadingPersona.value = false;
    }
  }
}

function switchToPersona(id: string) {
  if (id === selectedPersonaId.value && editingPersona.value) return;
  router.replace({ params: { id } });
  syncEditing(id);
}

onMounted(async () => {
  if (!personaStore.initialized) {
    await personaStore.loadPersonas();
  }
  if (!pluginStore.initialized) {
    await pluginStore.loadPlugins();
  }
  const initialId = routePersonaId.value || personaStore.currentPersonaId || "";
  selectedPersonaId.value = initialId;
  if (initialId) {
    await syncEditing(initialId);
  }
});

watch(routePersonaId, (newId) => {
  if (!newId || !personaStore.initialized) return;
  if (newId === selectedPersonaId.value) return;
  syncEditing(newId);
});

async function savePersona() {
  if (!editingPersona.value) return;
  saving.value = true;
  try {
    await personaStore.updatePersona(editingPersona.value.id, toRaw(editingPersona.value));
    isDirty.value = false;
    ElMessage.success("角色已保存");
  } catch (e) {
    ElMessage.error("保存失败: " + String(e));
  } finally {
    saving.value = false;
  }
}

function pluginSkillId(pluginId: string, skillId: string): string {
  return `plugin:${pluginId}:${skillId}`;
}

function isPluginSkillEnabled(plugin: InstalledPlugin, skillId: string): boolean {
  if (!editingPersona.value) return false;
  return editingPersona.value.skills.includes(pluginSkillId(plugin.id, skillId));
}

function togglePluginSkill(plugin: InstalledPlugin, skillId: string) {
  if (!editingPersona.value) return;
  const id = pluginSkillId(plugin.id, skillId);
  const idx = editingPersona.value.skills.indexOf(id);
  if (idx >= 0) {
    editingPersona.value.skills.splice(idx, 1);
  } else {
    editingPersona.value.skills.push(id);
  }
  isDirty.value = true;
}

function generateWorkflowId() {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `wf_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

function nowIso() {
  return new Date().toISOString();
}

function createEmptyWorkflow(personaId: string): Workflow {
  return {
    id: generateWorkflowId(),
    personaId,
    name: "",
    description: "",
    enabled: true,
    trigger: { type: "message" },
    actions: [],
    createdAt: nowIso(),
    updatedAt: nowIso(),
  };
}

function formatTriggerLabel(trigger: WorkflowTrigger) {
  switch (trigger.type) {
    case "message":
      return `收到消息${trigger.pattern ? `（匹配 ${trigger.pattern}）` : ""}`;
    case "scheduled":
      return `定时：${trigger.cron}`;
    case "event":
      return `事件：${trigger.eventName}`;
    default:
      return "未知触发";
  }
}

function openNewWorkflow() {
  if (!editingPersona.value) return;
  isNewWorkflow.value = true;
  editingWorkflow.value = createEmptyWorkflow(editingPersona.value.id);
  workflowDialogVisible.value = true;
}

function openEditWorkflow(workflow: Workflow) {
  isNewWorkflow.value = false;
  editingWorkflow.value = { ...workflow, actions: workflow.actions.map((a) => ({ ...a })) };
  workflowDialogVisible.value = true;
}

async function saveWorkflow() {
  if (!editingWorkflow.value || !editingPersona.value) return;
  workflowSaving.value = true;
  try {
    const payload = { ...editingWorkflow.value, updatedAt: nowIso() };
    if (isNewWorkflow.value) {
      await personaStore.createWorkflow(payload);
    } else {
      await personaStore.updateWorkflow(payload);
    }
    // 同步到当前编辑角色
    const refreshed = personaStore.getPersonaById(editingPersona.value.id);
    if (refreshed) {
      editingPersona.value.workflows = (refreshed.workflows ?? []).map((w) => ({ ...w }));
    }
    workflowDialogVisible.value = false;
    ElMessage.success("工作流已保存");
  } catch (e) {
    ElMessage.error("保存工作流失败: " + String(e));
  } finally {
    workflowSaving.value = false;
  }
}

async function removeWorkflow(workflow: Workflow) {
  if (!editingPersona.value) return;
  try {
    await personaStore.deleteWorkflow(workflow.id);
    editingPersona.value.workflows = (editingPersona.value.workflows ?? []).filter(
      (w) => w.id !== workflow.id,
    );
    ElMessage.success("工作流已删除");
  } catch (e) {
    ElMessage.error("删除工作流失败: " + String(e));
  }
}

function openDebugPanel(workflow: Workflow) {
  debugWorkflow.value = { ...workflow, actions: workflow.actions.map((a) => ({ ...a })) };
  debugTestMessage.value = workflow.trigger.type === "message" && workflow.trigger.pattern
    ? workflow.trigger.pattern
    : "";
  debugPanelVisible.value = true;
}

function onTriggerTypeChange(type: WorkflowTrigger["type"]) {
  if (!editingWorkflow.value) return;
  switch (type) {
    case "message":
      editingWorkflow.value.trigger = { type: "message" };
      break;
    case "scheduled":
      editingWorkflow.value.trigger = { type: "scheduled", cron: "" };
      break;
    case "event":
      editingWorkflow.value.trigger = { type: "event", eventName: "" };
      break;
  }
}
</script>

<template>
  <div class="page-container persona-editor">
    <div class="page-header">
      <h1 class="page-title">角色编辑器</h1>
      <el-select
        v-model="selectedPersonaId"
        size="small"
        style="width: 180px"
        @change="switchToPersona"
      >
        <el-option
          v-for="p in personas"
          :key="p.id"
          :label="p.name"
          :value="p.id"
        />
      </el-select>
    </div>
    <p class="page-subtitle">自定义 AI 伴侣的人设、声音与能力</p>

    <div v-if="editingPersona" v-loading="loadingPersona" class="editor-layout">
      <div class="editor-form">
        <el-tabs v-model="activeTab">
          <el-tab-pane label="基础" name="basic">
            <el-form label-width="100px" label-position="top">
              <el-form-item label="名称">
                <el-input v-model="editingPersona.name" />
              </el-form-item>
              <el-form-item label="性格标签">
                <el-select
                  v-model="editingPersona.personality"
                  multiple
                  placeholder="选择性格"
                  style="width: 100%"
                >
                  <el-option
                    v-for="opt in personalityOptions"
                    :key="opt"
                    :label="opt"
                    :value="opt"
                  />
                </el-select>
              </el-form-item>
              <el-form-item label="问候语">
                <el-input v-model="editingPersona.greeting" />
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="人设" name="prompt">
            <el-form label-position="top">
              <el-form-item label="系统提示词">
                <el-input
                  v-model="editingPersona.systemPrompt"
                  type="textarea"
                  :rows="8"
                  placeholder="描述角色的性格、说话方式…"
                />
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="声音" name="voice">
            <el-form label-width="120px" label-position="top">
              <el-form-item label="TTS 引擎">
                <el-select v-model="editingPersona.voice.ttsEngine" style="width: 100%">
                  <el-option label="CosyVoice 2" value="cosyvoice" />
                  <el-option label="GPT-SoVITS" value="gpt-sovits" />
                </el-select>
              </el-form-item>
              <el-form-item label="参考音频">
                <el-upload drag :auto-upload="false" accept="audio/*">
                  <div class="upload-hint">拖拽或点击上传 3-10 秒参考音频</div>
                </el-upload>
              </el-form-item>
              <el-form-item label="语速">
                <el-slider
                  v-model="editingPersona.voice.params.speed"
                  :min="0.5"
                  :max="2"
                  :step="0.05"
                  show-input
                />
              </el-form-item>
              <el-button type="primary" plain>试听（Mock）</el-button>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="模型" name="model">
            <el-form label-position="top">
              <el-form-item label="LLM Provider">
                <el-select v-model="editingPersona.llm.provider" style="width: 100%">
                  <el-option
                    v-for="p in llmProviders"
                    :key="p.value"
                    :label="p.label"
                    :value="p.value"
                  />
                </el-select>
              </el-form-item>
              <el-form-item label="温度">
                <el-slider
                  v-model="editingPersona.llm.temperature"
                  :min="0"
                  :max="2"
                  :step="0.1"
                  show-input
                />
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="能力" name="skills">
            <el-form label-position="top">
              <el-form-item label="启用技能">
                <el-checkbox-group v-model="editingPersona.skills">
                  <el-checkbox
                    v-for="s in skillOptions"
                    :key="s.value"
                    :label="s.value"
                  >
                    {{ s.label }}
                  </el-checkbox>
                </el-checkbox-group>
              </el-form-item>
              <el-form-item label="挂载知识库">
                <el-checkbox-group v-model="editingPersona.knowledgeBases">
                  <el-checkbox
                    v-for="kb in kbOptions"
                    :key="kb.value"
                    :label="kb.value"
                  >
                    {{ kb.label }}
                  </el-checkbox>
                </el-checkbox-group>
              </el-form-item>
              <el-form-item label="多模态">
                <el-switch
                  v-model="editingPersona.multimodal.canSeeScreen"
                  active-text="可看屏幕"
                />
                <el-switch
                  v-model="editingPersona.multimodal.canSeeCamera"
                  active-text="可看摄像头"
                  style="margin-left: 16px"
                />
                <el-switch
                  v-model="editingPersona.multimodal.autoDescribeImages"
                  active-text="自动描述图片"
                  style="margin-left: 16px"
                />
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="微信" name="wechat">
            <el-form label-position="top">
              <el-form-item label="多段式回复">
                <el-switch
                  v-model="editingPersona.wechat!.enableSegmentedReply"
                  active-text="启用"
                />
                <div class="form-help">启用后可在微信聊天输入框编辑多条消息逐条发送</div>
              </el-form-item>
              <el-form-item v-if="editingPersona.wechat!.enableSegmentedReply" label="发送间隔（毫秒）">
                <el-slider
                  v-model="editingPersona.wechat!.segmentDelay"
                  :min="300"
                  :max="3000"
                  :step="100"
                  show-input
                />
                <div class="form-help">每条消息之间的延迟时间，模拟真人打字节奏</div>
              </el-form-item>
              <el-form-item label="语音消息功能">
                <el-switch
                  v-model="editingPersona.wechat!.enableVoiceMessage"
                  active-text="启用"
                />
                <div class="form-help">启用后在微信聊天界面显示语音录制按钮，支持语音转文字</div>
              </el-form-item>
              <template v-if="editingPersona.wechat!.enableVoiceMessage">
                <el-form-item label="语音自动转文字">
                  <el-switch
                    v-model="editingPersona.wechat!.voiceAsrEnabled"
                    active-text="自动识别"
                  />
                  <div class="form-help">录音结束后自动进行语音识别转文字</div>
                </el-form-item>
                <el-form-item label="自动填入发送框">
                  <el-switch
                    v-model="editingPersona.wechat!.voiceAutoSend"
                    active-text="自动填入"
                  />
                  <div class="form-help">识别完成后自动将文字填入输入框（不会自动发送）</div>
                </el-form-item>
              </template>
              <el-form-item label="动作描述处理">
                <el-radio-group v-model="editingPersona.wechat!.actionDescriptionMode">
                  <el-radio value="inline">保留在原文中</el-radio>
                  <el-radio value="separate">独立发送</el-radio>
                  <el-radio value="remove">移除动作描写</el-radio>
                </el-radio-group>
                <div class="form-help">
                  AI 回复中的括号动作描写（如"（微微一笑）"）如何处理：
                  <br>• 保留：括号内容跟随前后文字发送在同一条消息
                  <br>• 独立发送：括号动作单独作为一条消息
                  <br>• 移除：去掉括号中的动作描写，只发送对话内容
                </div>
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="插件" name="plugins">
            <el-form label-position="top">
              <el-form-item label="插件技能">
                <div v-if="pluginStore.plugins.length === 0" class="plugin-empty">
                  暂无已安装插件，前往
                  <el-button link type="primary" @click="router.push('/plugins')">
                    插件管理
                  </el-button>
                  安装
                </div>
                <div v-else class="plugin-list">
                  <div
                    v-for="plugin in pluginStore.plugins"
                    :key="plugin.id"
                    class="plugin-option"
                  >
                    <div class="plugin-option-header">
                      <el-tag size="small" :type="plugin.enabled ? 'success' : 'info'">
                        {{ plugin.enabled ? '已启用' : '已禁用' }}
                      </el-tag>
                      <span class="plugin-option-name">{{ plugin.manifest.name }}</span>
                      <span class="plugin-option-version">v{{ plugin.manifest.version }}</span>
                    </div>
                    <div v-if="plugin.manifest.skills?.length" class="plugin-skills">
                      <el-checkbox
                        v-for="skill in plugin.manifest.skills"
                        :key="skill.id"
                        :model-value="isPluginSkillEnabled(plugin, skill.id)"
                        :disabled="!plugin.enabled"
                        @update:model-value="togglePluginSkill(plugin, skill.id)"
                      >
                        {{ skill.name }}
                        <span class="plugin-skill-desc">{{ skill.description }}</span>
                      </el-checkbox>
                    </div>
                    <div v-else class="plugin-no-skills">
                      该插件未提供可配置技能
                    </div>
                  </div>
                </div>
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="外观" name="appearance">
            <el-form label-position="top">
              <el-form-item label="头像文件名">
                <el-input
                  v-model="editingPersona.appearance.avatar"
                  placeholder="例如：aria.png"
                />
              </el-form-item>
              <el-form-item label="生成艺术背景">
                <el-switch
                  v-model="editingPersona.appearance.generativeArt"
                  active-text="启用"
                />
                <div class="form-help">为角色卡片启用 Luminous Drift 算法背景</div>
              </el-form-item>
            </el-form>
          </el-tab-pane>

          <el-tab-pane label="工作流" name="workflows">
            <div class="workflow-tab">
              <div class="workflow-header">
                <h3 class="workflow-title">角色工作流</h3>
                <el-button type="primary" size="small" @click="openNewWorkflow">
                  <el-icon class="el-icon--left"><Plus /></el-icon>
                  新建工作流
                </el-button>
              </div>

              <EmptyStateInline
                v-if="(editingPersona.workflows ?? []).length === 0"
                icon="🧩"
                title="暂无工作流"
                description="点击右上角按钮创建"
              />

              <div v-else class="workflow-list">
                <div
                  v-for="workflow in editingPersona.workflows"
                  :key="workflow.id"
                  class="workflow-card"
                >
                  <div class="workflow-card-main">
                    <div class="workflow-name">
                      {{ workflow.name || "未命名工作流" }}
                      <el-tag v-if="!workflow.enabled" type="info" size="small">已禁用</el-tag>
                    </div>
                    <div class="workflow-trigger">{{ formatTriggerLabel(workflow.trigger) }}</div>
                    <div v-if="workflow.description" class="workflow-desc">
                      {{ workflow.description }}
                    </div>
                  </div>
                  <div class="workflow-card-actions">
                    <el-switch
                      v-model="workflow.enabled"
                      @change="personaStore.updateWorkflow(workflow)"
                    />
                    <el-button link size="small" @click="openDebugPanel(workflow)">
                      测试运行
                    </el-button>
                    <el-button link size="small" @click="openEditWorkflow(workflow)">
                      编辑
                    </el-button>
                    <el-button link type="danger" size="small" @click="removeWorkflow(workflow)">
                      删除
                    </el-button>
                  </div>
                </div>
              </div>
            </div>
          </el-tab-pane>
        </el-tabs>

        <div class="form-actions">
          <el-button type="primary" @click="savePersona">保存角色</el-button>
        </div>
      </div>

      <aside class="editor-preview">
        <h3 class="preview-title">预览</h3>
        <PersonaCard :persona="editingPersona" selected />
      </aside>

      <!-- 工作流编辑弹窗 -->
      <el-dialog
        v-model="workflowDialogVisible"
        :title="isNewWorkflow ? '新建工作流' : '编辑工作流'"
        width="640px"
        destroy-on-close
      >
        <el-form v-if="editingWorkflow" label-position="top">
          <el-form-item label="名称">
            <el-input v-model="editingWorkflow.name" placeholder="例如：每日早报" />
          </el-form-item>
          <el-form-item label="描述">
            <el-input
              v-model="editingWorkflow.description"
              type="textarea"
              :rows="2"
              placeholder="工作流用途说明"
            />
          </el-form-item>
          <el-form-item label="启用">
            <el-switch v-model="editingWorkflow.enabled" />
          </el-form-item>

          <el-form-item label="触发条件">
          <el-radio-group v-model="editingWorkflow.trigger.type" @change="onTriggerTypeChange">
            <el-radio-button label="message">收到消息</el-radio-button>
            <el-radio-button label="scheduled">定时</el-radio-button>
            <el-radio-button label="event">事件</el-radio-button>
          </el-radio-group>
        </el-form-item>

        <el-form-item v-if="editingWorkflow.trigger.type === 'message'" label="匹配规则（可选）">
          <el-input v-model="messageTrigger.pattern" placeholder="留空表示任意消息；支持正则或关键词" />
        </el-form-item>

        <el-form-item v-if="editingWorkflow.trigger.type === 'scheduled'" label="Cron 表达式">
          <el-input v-model="scheduledTrigger.cron" placeholder="例如：0 9 * * *" />
        </el-form-item>

        <el-form-item v-if="editingWorkflow.trigger.type === 'event'" label="事件名称">
          <el-input v-model="eventTrigger.eventName" placeholder="例如：user_login" />
        </el-form-item>

          <el-form-item label="动作序列">
            <WorkflowNodeEditor v-model="editingWorkflow.actions" />
          </el-form-item>
        </el-form>

        <template #footer>
          <el-button @click="workflowDialogVisible = false">取消</el-button>
          <el-button type="primary" :loading="workflowSaving" @click="saveWorkflow">
            保存
          </el-button>
        </template>
      </el-dialog>

      <!-- 工作流调试面板 -->
      <WorkflowDebugPanel
        v-if="debugWorkflow"
        v-model:visible="debugPanelVisible"
        :workflow="debugWorkflow"
        :test-message="debugTestMessage"
      />
    </div>

    <!-- 首次加载中 -->
    <el-skeleton v-else-if="personaStore.loading || !personaStore.initialized" :rows="8" animated style="padding: 0 20px" />

    <!-- 无角色 -->
    <el-empty v-else description="暂无角色，请先创建角色" />
  </div>
</template>

<style scoped lang="scss">
.persona-editor {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-layout {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 24px;
}

.editor-form {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  padding: 20px;
}

.editor-preview {
  width: 320px;
  flex-shrink: 0;
  overflow-y: auto;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  padding: 16px;
}

.preview-title {
  font-size: 14px;
  color: var(--color-text-secondary);
  margin-bottom: 12px;
}

.form-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--glass-border);
}

.upload-hint {
  padding: 20px;
  color: var(--color-text-muted);
  font-size: 13px;
}

.emotion-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

:deep(.el-tabs__header) {
  background: var(--glass-bg-light);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
  padding: 4px;
  margin-bottom: 20px;
}

:deep(.el-form-item__label) {
  color: var(--color-text-secondary);
}

:deep(.el-input__wrapper),
:deep(.el-textarea__inner) {
  background: var(--color-bg-surface) !important;
  border: 1px solid var(--color-border) !important;
  box-shadow: none !important;

  &:hover {
    border-color: var(--color-primary) !important;
  }

  &.is-focus {
    border-color: var(--color-primary) !important;
    box-shadow: 0 0 0 1px var(--color-primary) !important;
  }
}

:deep(.el-textarea__inner) {
  color: var(--color-text-primary) !important;
}

:deep(.el-input__inner) {
  color: var(--color-text-primary) !important;
}

:deep(.el-select__wrapper) {
  background: var(--color-bg-surface) !important;
  border: 1px solid var(--color-border) !important;
  box-shadow: none !important;

  &:hover {
    border-color: var(--color-primary) !important;
  }

  &.is-focused {
    border-color: var(--color-primary) !important;
    box-shadow: 0 0 0 1px var(--color-primary) !important;
  }
}

:deep(.el-upload-dragger) {
  background: var(--color-bg-surface) !important;
  border-color: var(--color-border) !important;

  &:hover {
    border-color: var(--color-primary) !important;
  }
}

:deep(.el-checkbox) {
  color: var(--color-text-primary) !important;
}

:deep(.el-switch__label) {
  color: var(--color-text-primary) !important;
}

:deep(.el-slider__runway) {
  background-color: var(--color-border) !important;
}

:deep(.el-slider__bar) {
  background-color: var(--color-primary) !important;
}

:deep(.el-slider__button) {
  border-color: var(--color-primary) !important;
}

:deep(.el-input-number__decrease),
:deep(.el-input-number__increase) {
  background: var(--color-bg-surface) !important;
  border-color: var(--color-border) !important;
  color: var(--color-text-primary) !important;
}

:deep(.el-tabs__active-bar) {
  background-color: var(--color-primary) !important;
}

:deep(.el-tabs__item) {
  color: var(--color-text-secondary) !important;

  &:hover {
    color: var(--color-primary) !important;
  }

  &.is-active {
    color: var(--color-primary) !important;
  }
}

.form-help {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;
}

.workflow-tab {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.workflow-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.workflow-title {
  font-size: 16px;
  font-weight: 500;
  color: var(--color-text-primary);
  margin: 0;
}

.workflow-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.workflow-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  transition: all var(--transition-base);

  &:hover {
    border-color: var(--color-primary);
    background: var(--color-bg-hover);
  }
}

.workflow-card-main {
  flex: 1;
  min-width: 0;
}

.workflow-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
  display: flex;
  align-items: center;
  gap: 8px;
}

.workflow-trigger {
  font-size: 13px;
  color: var(--color-text-secondary);
  margin-top: 4px;
}

.workflow-desc {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.workflow-card-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

:deep(.el-radio-button__inner) {
  background: var(--color-bg-surface) !important;
  border-color: var(--color-border) !important;
  color: var(--color-text-secondary) !important;
}

:deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
  background: var(--color-primary) !important;
  border-color: var(--color-primary) !important;
  color: var(--color-text-inverse) !important;
  box-shadow: -1px 0 0 0 var(--color-primary) !important;
}

.plugin-empty {
  padding: 24px;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 13px;
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
}

.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.plugin-option {
  padding: 14px 16px;
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.plugin-option-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.plugin-option-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
}

.plugin-option-version {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-left: auto;
}

.plugin-skills {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.plugin-skill-desc {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-left: 4px;
}

.plugin-no-skills {
  font-size: 13px;
  color: var(--color-text-muted);
}
</style>
