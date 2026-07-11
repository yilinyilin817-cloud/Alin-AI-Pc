<template>
  <div class="cloud-tts-provider-manager">
    <div class="header">
      <div>
        <h3>云端 TTS 服务商</h3>
        <p class="sub">统一管理悟声等云端 TTS 服务的连接、密钥和音色</p>
      </div>
      <el-button type="primary" @click="showAddDialog">
        <el-icon><Plus /></el-icon>
        添加服务商
      </el-button>
    </div>

    <el-table :data="providers" style="width: 100%" v-loading="loading">
      <el-table-column label="名称" min-width="180">
        <template #default="{ row }">
          <div class="cell-name">
            <el-avatar
              v-if="row.iconUrl"
              :src="row.iconUrl"
              :size="28"
              shape="square"
            />
            <el-avatar v-else :size="28" shape="square">
              {{ row.name.slice(0, 1) }}
            </el-avatar>
            <div class="cell-name-text">
              <div class="name">{{ row.name }}</div>
              <div class="sub">
                <el-tag size="small" :type="getProviderTypeTag(row.providerType)">
                  {{ row.providerType }}
                </el-tag>
              </div>
            </div>
          </div>
        </template>
      </el-table-column>
      <el-table-column prop="apiBase" label="API 地址" min-width="220" />
      <el-table-column label="密钥" width="180">
        <template #default="{ row }">
          <code v-if="row.hasApiKey" class="masked">{{ row.apiKeyMasked }}</code>
          <el-tag v-else type="warning" size="small">未配置</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="音色" width="100">
        <template #default="{ row }">
          {{ row.voices?.length ?? 0 }} 个
        </template>
      </el-table-column>
      <el-table-column label="最后验证" width="170">
        <template #default="{ row }">
          <span v-if="row.lastVerifiedAt" class="time">
            {{ formatTime(row.lastVerifiedAt) }}
          </span>
          <span v-else class="muted">未验证</span>
        </template>
      </el-table-column>
      <el-table-column label="状态" width="100">
        <template #default="{ row }">
          <el-switch v-model="row.isEnabled" @change="handleToggleEnabled(row)" />
        </template>
      </el-table-column>
      <el-table-column label="操作" width="320" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="handleVerify(row.id)" :loading="verifyingId === row.id">
            <el-icon><Check /></el-icon>
            验证
          </el-button>
          <el-button size="small" type="primary" @click="showEditDialog(row)">
            <el-icon><Edit /></el-icon>
            编辑
          </el-button>
          <el-button size="small" @click="openVoicePanel(row)">
            <el-icon><Microphone /></el-icon>
            音色
          </el-button>
          <el-button size="small" type="danger" @click="handleDelete(row.id)">
            <el-icon><Delete /></el-icon>
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 添加 / 编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑云端 TTS 服务商' : '添加云端 TTS 服务商'"
      width="560px"
      :close-on-click-modal="false"
    >
      <el-form :model="form" label-width="100px" label-position="top">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="例如：悟声" maxlength="40" />
        </el-form-item>
        <el-form-item label="类型" required>
          <el-select v-model="form.providerType" placeholder="选择服务商类型">
            <el-option label="悟声 (Wusound)" value="wusound" />
          </el-select>
        </el-form-item>
        <el-form-item label="API 地址" required>
          <el-input
            v-model="form.apiBase"
            placeholder="https://v1.wusound.cn/api"
          />
          <div class="form-hint">必须以 http:// 或 https:// 开头</div>
        </el-form-item>
        <el-form-item label="API Key" required>
          <el-input
            v-model="form.apiKey"
            type="password"
            show-password
            placeholder="sk-..."
          />
          <div v-if="isEdit" class="form-hint">
            留空表示不修改；当前为 <code>{{ form.originalKeyMasked }}</code>
          </div>
        </el-form-item>
        <el-form-item label="图标 URL">
          <el-input v-model="form.iconUrl" placeholder="可选，服务商图标地址" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="form.isEnabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit" :loading="submitting">
          {{ isEdit ? '保存' : '添加' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 验证结果对话框（展示 voices + 配额） -->
    <el-dialog
      v-model="verifyDialogVisible"
      :title="`验证结果 — ${verifyResult?.providerName ?? ''}`"
      width="640px"
    >
      <div v-if="verifyResult" class="verify-result">
        <el-alert
          :type="verifyResult.response.success ? 'success' : 'error'"
          :title="verifyResult.response.message"
          show-icon
          :closable="false"
        />
        <div v-if="verifyResult.response.quota" class="quota">
          <div class="quota-item">
            <div class="label">套餐</div>
            <div class="value">{{ verifyResult.response.quota.tier ?? '—' }}</div>
          </div>
          <div class="quota-item">
            <div class="label">已用 / 总额</div>
            <div class="value">
              {{ verifyResult.response.quota.usedChars }} /
              {{ verifyResult.response.quota.totalChars }} 字符
            </div>
          </div>
          <div class="quota-item">
            <div class="label">剩余</div>
            <div class="value">
              {{ verifyResult.response.quota.remainingChars }} 字符
            </div>
          </div>
        </div>
        <el-divider />
        <h4>语音角色（{{ verifyResult.response.voices.length }}）</h4>
        <div class="voice-grid">
          <el-card
            v-for="v in verifyResult.response.voices"
            :key="v.id"
            class="voice-card"
            shadow="never"
          >
            <div class="voice-name">{{ v.name }}</div>
            <div class="voice-id">{{ v.id }}</div>
            <div class="voice-desc" v-if="v.metadata?.description">
              {{ v.metadata.description }}
            </div>
            <div class="voice-tags" v-if="v.metadata">
              <el-tag v-for="g in v.metadata.language" :key="g" size="small">{{ g }}</el-tag>
              <el-tag v-if="v.metadata.gender" size="small" type="info">
                {{ v.metadata.gender }}
              </el-tag>
            </div>
          </el-card>
        </div>
      </div>
    </el-dialog>

    <!-- 音色试听面板 -->
    <el-drawer
      v-model="voicePanelVisible"
      :title="`音色 — ${activeProvider?.name ?? ''}`"
      size="60%"
    >
      <div v-if="activeProvider" class="voice-panel">
        <div class="preview-box">
          <h4>试听</h4>
          <el-input
            v-model="previewText"
            type="textarea"
            :rows="3"
            placeholder="输入要合成的文本"
            maxlength="500"
            show-word-limit
          />
          <div class="preview-controls">
            <el-select
              v-model="previewVoiceId"
              placeholder="选择音色"
              filterable
              style="flex: 1; margin-right: 8px"
            >
              <el-option
                v-for="v in activeProvider.voices"
                :key="v.id"
                :value="v.id"
                :label="v.name"
              />
            </el-select>
            <el-select
              v-model="previewFormat"
              placeholder="格式"
              style="width: 110px; margin-right: 8px"
            >
              <el-option label="wav" value="wav" />
              <el-option label="mp3" value="mp3" />
              <el-option label="opus" value="opus" />
            </el-select>
            <el-button
              type="primary"
              :loading="previewing"
              :disabled="!previewVoiceId || !previewText.trim()"
              @click="handlePreview"
            >
              合成并播放
            </el-button>
          </div>
          <div v-if="previewAudioUrl" class="audio-player">
            <audio :src="previewAudioUrl" controls autoplay></audio>
          </div>
        </div>

        <el-divider />

        <h4>可用音色（{{ activeProvider.voices.length }}）</h4>
        <el-input
          v-model="voiceSearch"
          placeholder="搜索音色名称或 ID"
          clearable
          style="margin-bottom: 12px"
        />
        <div class="voice-list">
          <el-card
            v-for="v in filteredVoices"
            :key="v.id"
            class="voice-card clickable"
            :class="{ active: previewVoiceId === v.id }"
            shadow="never"
            @click="previewVoiceId = v.id"
          >
            <div class="voice-name">{{ v.name }}</div>
            <div class="voice-id">{{ v.id }}</div>
            <div v-if="v.metadata?.description" class="voice-desc">
              {{ v.metadata.description }}
            </div>
            <div v-if="v.metadata?.prompts?.length" class="voice-prompts">
              <el-tag
                v-for="p in v.metadata.prompts"
                :key="p.id"
                size="small"
                type="info"
                effect="plain"
              >
                {{ p.name }}
              </el-tag>
            </div>
          </el-card>
        </div>
        <el-empty
          v-if="!filteredVoices.length"
          :description="voiceSearch ? '无匹配音色' : '尚无音色，请先点击「验证」拉取'"
        />
      </div>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  Plus,
  Check,
  Edit,
  Delete,
  Microphone,
} from '@element-plus/icons-vue';
import type { CloudTtsProviderConfig, WusoundVoice } from '@/types';
import {
  listCloudTtsProviders,
  createCloudTtsProvider,
  updateCloudTtsProvider,
  deleteCloudTtsProvider,
  verifyCloudTtsProvider,
  cloudTtsPreview,
} from '@/api/cloudTtsProvider';

const providers = ref<CloudTtsProviderConfig[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const isEdit = ref(false);
const submitting = ref(false);
const editingId = ref<string | null>(null);
const verifyingId = ref<string | null>(null);

const verifyDialogVisible = ref(false);
const verifyResult = ref<{
  providerName: string;
  response: Awaited<ReturnType<typeof verifyCloudTtsProvider>>;
} | null>(null);

const voicePanelVisible = ref(false);
const activeProvider = ref<CloudTtsProviderConfig | null>(null);
const previewText = ref('你好，我是悟声 TTS，听听我的声音怎么样？');
const previewVoiceId = ref<string>('');
const previewFormat = ref<'wav' | 'mp3' | 'opus'>('mp3');
const previewing = ref(false);
const previewAudioUrl = ref<string | null>(null);
const voiceSearch = ref('');

const form = ref<{
  name: string;
  providerType: string;
  apiBase: string;
  apiKey: string;
  originalKeyMasked: string;
  iconUrl: string;
  isEnabled: boolean;
}>({
  name: '',
  providerType: 'wusound',
  apiBase: 'https://v1.wusound.cn/api',
  apiKey: '',
  originalKeyMasked: '',
  iconUrl: '',
  isEnabled: true,
});

const filteredVoices = computed<WusoundVoice[]>(() => {
  if (!activeProvider.value) return [];
  const q = voiceSearch.value.trim().toLowerCase();
  if (!q) return activeProvider.value.voices;
  return activeProvider.value.voices.filter(
    (v) => v.name.toLowerCase().includes(q) || v.id.toLowerCase().includes(q),
  );
});

async function loadProviders() {
  loading.value = true;
  try {
    providers.value = await listCloudTtsProviders();
  } catch (error) {
    ElMessage.error('加载云端 TTS 服务商列表失败');
    console.error(error);
  } finally {
    loading.value = false;
  }
}

function getProviderTypeTag(type: string): 'success' | 'info' | 'warning' | 'primary' {
  const map: Record<string, 'success'> = { wusound: 'success' };
  return map[type] ?? 'info';
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleString();
  } catch {
    return iso;
  }
}

function showAddDialog() {
  isEdit.value = false;
  editingId.value = null;
  form.value = {
    name: '',
    providerType: 'wusound',
    apiBase: 'https://v1.wusound.cn/api',
    apiKey: '',
    originalKeyMasked: '',
    iconUrl: '',
    isEnabled: true,
  };
  dialogVisible.value = true;
}

function showEditDialog(provider: CloudTtsProviderConfig) {
  isEdit.value = true;
  editingId.value = provider.id;
  form.value = {
    name: provider.name,
    providerType: provider.providerType,
    apiBase: provider.apiBase,
    apiKey: '',
    originalKeyMasked: provider.apiKeyMasked,
    iconUrl: provider.iconUrl ?? '',
    isEnabled: provider.isEnabled,
  };
  dialogVisible.value = true;
}

async function handleSubmit() {
  if (!form.value.name.trim() || !form.value.apiBase.trim()) {
    ElMessage.warning('请填写名称和 API 地址');
    return;
  }
  if (!/^https?:\/\//.test(form.value.apiBase)) {
    ElMessage.error('API 地址必须以 http:// 或 https:// 开头');
    return;
  }
  if (!isEdit.value && !form.value.apiKey.trim()) {
    ElMessage.warning('请填写 API Key');
    return;
  }
  if (isEdit.value && form.value.apiKey.trim() === '' && !form.value.originalKeyMasked) {
    ElMessage.warning('请填写 API Key');
    return;
  }

  submitting.value = true;
  try {
    if (isEdit.value && editingId.value) {
      const update: Record<string, unknown> = {
        name: form.value.name,
        providerType: form.value.providerType,
        apiBase: form.value.apiBase,
        iconUrl: form.value.iconUrl || undefined,
        isEnabled: form.value.isEnabled,
      };
      if (form.value.apiKey.trim()) {
        update.apiKey = form.value.apiKey;
      }
      await updateCloudTtsProvider(editingId.value, update as never);
      ElMessage.success('更新成功');
    } else {
      await createCloudTtsProvider({
        name: form.value.name,
        providerType: form.value.providerType,
        apiBase: form.value.apiBase,
        apiKey: form.value.apiKey,
        iconUrl: form.value.iconUrl || undefined,
      });
      ElMessage.success('添加成功');
    }
    dialogVisible.value = false;
    await loadProviders();
  } catch (error: any) {
    ElMessage.error(`${isEdit.value ? '更新' : '添加'}失败：${error?.message ?? error}`);
  } finally {
    submitting.value = false;
  }
}

async function handleDelete(id: string) {
  try {
    await ElMessageBox.confirm('确定要删除这个云端 TTS 服务商吗？此操作不可撤销。', '确认', {
      type: 'warning',
    });
    await deleteCloudTtsProvider(id);
    ElMessage.success('删除成功');
    if (activeProvider.value?.id === id) {
      voicePanelVisible.value = false;
      activeProvider.value = null;
    }
    await loadProviders();
  } catch (error) {
    if (error !== 'cancel' && error !== 'close') {
      ElMessage.error('删除失败');
      console.error(error);
    }
  }
}

async function handleVerify(id: string) {
  const provider = providers.value.find((p) => p.id === id);
  verifyingId.value = id;
  try {
    const response = await verifyCloudTtsProvider(id);
    verifyResult.value = {
      providerName: provider?.name ?? '',
      response,
    };
    verifyDialogVisible.value = true;
    if (response.success) {
      ElMessage.success(response.message);
    } else {
      ElMessage.warning(response.message);
    }
    await loadProviders();
  } catch (error: any) {
    ElMessage.error(`验证失败：${error?.message ?? error}`);
  } finally {
    verifyingId.value = null;
  }
}

async function handleToggleEnabled(row: CloudTtsProviderConfig) {
  try {
    await updateCloudTtsProvider(row.id, { isEnabled: row.isEnabled });
    ElMessage.success(row.isEnabled ? '已启用' : '已禁用');
  } catch (error: any) {
    row.isEnabled = !row.isEnabled;
    ElMessage.error(`更新失败：${error?.message ?? error}`);
  }
}

function openVoicePanel(provider: CloudTtsProviderConfig) {
  activeProvider.value = provider;
  previewVoiceId.value = provider.voices[0]?.id ?? '';
  voicePanelVisible.value = true;
}

async function handlePreview() {
  if (!activeProvider.value || !previewVoiceId.value) return;
  if (!previewText.value.trim()) {
    ElMessage.warning('请输入要合成的文本');
    return;
  }
  previewing.value = true;
  releaseAudio();
  try {
    const bytes = await cloudTtsPreview({
      providerId: activeProvider.value.id,
      text: previewText.value,
      voiceId: previewVoiceId.value,
      format: previewFormat.value,
    });
    const blob = new Blob([new Uint8Array(bytes)], {
      type: audioMime(previewFormat.value),
    });
    previewAudioUrl.value = URL.createObjectURL(blob);
  } catch (error: any) {
    ElMessage.error(`合成失败：${error?.message ?? error}`);
  } finally {
    previewing.value = false;
  }
}

function audioMime(format: string): string {
  switch (format) {
    case 'mp3':
      return 'audio/mpeg';
    case 'opus':
      return 'audio/ogg; codecs=opus';
    case 'wav':
    default:
      return 'audio/wav';
  }
}

function releaseAudio() {
  if (previewAudioUrl.value) {
    URL.revokeObjectURL(previewAudioUrl.value);
    previewAudioUrl.value = null;
  }
}

onBeforeUnmount(() => {
  releaseAudio();
});

onMounted(() => {
  loadProviders();
});
</script>

<style scoped lang="scss">
.cloud-tts-provider-manager {
  padding: 20px;

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 20px;

    h3 {
      margin: 0;
      font-size: 18px;
    }

    .sub {
      margin: 4px 0 0;
      color: var(--color-text-muted);
      font-size: 12px;
    }
  }
}

.cell-name {
  display: flex;
  align-items: center;
  gap: 10px;

  .cell-name-text {
    display: flex;
    flex-direction: column;

    .name {
      font-weight: 500;
    }
    .sub {
      margin-top: 2px;
    }
  }
}

.masked {
  font-family: var(--font-mono, 'JetBrains Mono', monospace);
  font-size: 12px;
  color: var(--color-text-secondary);
  background: var(--color-bg-surface);
  padding: 2px 6px;
  border-radius: 4px;
}

.time {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.muted {
  color: var(--color-text-muted);
  font-size: 12px;
}

.form-hint {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;

  code {
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
  }
}

.verify-result {
  .quota {
    display: flex;
    gap: 12px;
    margin: 16px 0;

    .quota-item {
      flex: 1;
      padding: 12px;
      background: var(--glass-bg);
      border: 1px solid var(--glass-border);
      border-radius: var(--radius-md);

      .label {
        font-size: 12px;
        color: var(--color-text-muted);
      }
      .value {
        font-size: 16px;
        font-weight: 600;
        margin-top: 4px;
      }
    }
  }
}

.voice-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.voice-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}

.voice-card {
  border: 1px solid var(--glass-border);
  transition: all 0.2s;

  &.clickable {
    cursor: pointer;
  }
  &.clickable:hover {
    border-color: var(--color-primary);
    transform: translateY(-2px);
  }
  &.active {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px rgba(120, 119, 198, 0.2);
  }

  .voice-name {
    font-size: 14px;
    font-weight: 600;
  }
  .voice-id {
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 11px;
    color: var(--color-text-muted);
    margin-top: 2px;
  }
  .voice-desc {
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-top: 6px;
    line-height: 1.4;
  }
  .voice-tags,
  .voice-prompts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
}

.voice-panel {
  .preview-box {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: 16px;

    h4 {
      margin: 0 0 12px;
      font-size: 14px;
      color: var(--color-text-secondary);
    }

    .preview-controls {
      display: flex;
      align-items: center;
      margin-top: 12px;
    }

    .audio-player {
      margin-top: 12px;

      audio {
        width: 100%;
      }
    }
  }
}
</style>
