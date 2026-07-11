<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useSettingsStore, type FontSize } from "@/stores/settings";
import { listAudioDevices } from "@/api/voice";
import { mockGetAppInfo } from "@/mocks/ipc";
import type { AppInfo } from "@/types";
import { ElMessage, ElMessageBox } from "element-plus";

const settings = useSettingsStore();
const appInfo = ref<AppInfo | null>(null);
const inputDevices = ref<{ label: string; value: string }[]>([]);
const outputDevices = ref<{ label: string; value: string }[]>([]);
const fontSizeMarks = { 0: "小", 1: "默认", 2: "大" };
const fontSizeMap: Record<number, FontSize> = { 0: "small", 1: "default", 2: "large" };
const reverseFontSizeMap: Record<FontSize, number> = { small: 0, default: 1, large: 2 };

const isMigrating = computed(() => settings.migrationStatus === 'migrating');
const showMigrationProgress = computed(() => settings.migrationStatus !== 'idle');
const hasModelDirChanged = computed(() => settings.modelsDir !== settings.originalModelsDir);

const progressStatus = computed(() => {
  if (settings.migrationStatus === 'success') return 'success';
  if (settings.migrationStatus === 'error') return 'exception';
  return undefined;
});

const fontSizeValue = computed({
  get: () => reverseFontSizeMap[settings.fontSize],
  set: (v: number) => {
    settings.fontSize = fontSizeMap[v];
    settings.save();
  },
});

onMounted(async () => {
  // 加载应用信息
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    appInfo.value = await invoke<AppInfo>("get_app_info");
  } catch {
    appInfo.value = await mockGetAppInfo();
  }

  // 加载音频设备（真实或兜底）
  try {
    const devices = await listAudioDevices();
    inputDevices.value = devices.map((d) => ({ label: d, value: d }));
    outputDevices.value = devices.map((d) => ({ label: d, value: d }));
  } catch {
    // 兜底
    inputDevices.value = [
      { label: "系统默认", value: "default" },
      { label: "麦克风 (Realtek)", value: "mic_realtek" },
    ];
    outputDevices.value = [
      { label: "系统默认", value: "default" },
      { label: "扬声器 (Realtek)", value: "spk_realtek" },
    ];
  }
});

function handleSettingChange() {
  settings.save();
}

async function handleSelectDir() {
  const selected = await settings.selectModelDir();
  if (selected) {
    settings.resetMigrationStatus();
  }
}

async function handleApplyDir() {
  if (!hasModelDirChanged.value) return;
  
  if (settings.modelDirInfo && settings.modelDirInfo.modelCount > 0) {
    try {
      await ElMessageBox.confirm(
        '检测到当前目录已有模型文件。切换目录不会移动现有模型。建议使用"迁移模型"功能将模型复制到新目录。是否继续切换？',
        '确认切换',
        {
          confirmButtonText: '继续切换',
          cancelButtonText: '取消',
          type: 'warning',
        }
      );
    } catch {
      return;
    }
  }

  try {
    await settings.applyModelDir();
    ElMessage.success('模型存储位置已更新');
  } catch (e) {
    ElMessage.error('更新失败: ' + String(e));
  }
}

async function handleMigrate() {
  if (!hasModelDirChanged.value) {
    ElMessage.info('请先选择新的存储目录');
    return;
  }

  try {
    await ElMessageBox.confirm(
      `将把 ${settings.modelDirInfo?.modelCount || 0} 个模型从\n${settings.originalModelsDir}\n迁移到\n${settings.modelsDir}\n\n迁移完成后会自动删除原目录文件。是否继续？`,
      '确认模型迁移',
      {
        confirmButtonText: '开始迁移',
        cancelButtonText: '取消',
        type: 'info',
        confirmButtonClass: 'el-button--primary',
      }
    );
  } catch {
    return;
  }

  settings.resetMigrationStatus();
  await settings.startMigration();

  if (settings.migrationStatus === 'success') {
    ElMessage.success('模型迁移完成');
  } else if (settings.migrationStatus === 'error') {
    ElMessage.error('迁移失败: ' + settings.migrationError);
  }
}
</script>

<template>
  <div class="page-container settings-page">
    <h1 class="page-title">设置</h1>
    <p class="page-subtitle">应用偏好与隐私配置</p>

    <div class="settings-sections">
      <el-card class="settings-card" shadow="never">
        <template #header>通用</template>
        <el-form label-width="120px">
          <el-form-item label="语言">
            <el-select v-model="settings.language" style="width: 200px" @change="handleSettingChange">
              <el-option label="简体中文" value="zh-CN" />
              <el-option label="English" value="en-US" />
            </el-select>
          </el-form-item>
          <el-form-item label="开机启动">
            <el-switch v-model="settings.launchOnStartup" @change="handleSettingChange" />
          </el-form-item>
        </el-form>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>外观</template>
        <el-form label-width="120px">
          <el-form-item label="主题">
            <el-select v-model="settings.theme" style="width: 200px" @change="settings.setTheme(settings.theme)">
              <el-option label="跟随系统" value="system" />
              <el-option label="深色" value="dark" />
              <el-option label="浅色" value="light" />
            </el-select>
          </el-form-item>
          <el-form-item label="字体大小">
            <el-slider
              v-model="fontSizeValue"
              :min="0"
              :max="2"
              :step="1"
              :marks="fontSizeMarks"
              style="width: 240px"
            />
          </el-form-item>
          <el-form-item label="消息密度">
            <el-radio-group v-model="settings.messageDensity" @change="settings.save()">
              <el-radio-button value="compact">紧凑</el-radio-button>
              <el-radio-button value="default">默认</el-radio-button>
              <el-radio-button value="cozy">宽松</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="生成艺术">
            <el-switch v-model="settings.generativeArtEnabled" @change="settings.save()" />
            <div class="form-help">在界面背景中展示动态生成艺术效果</div>
          </el-form-item>
        </el-form>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>聊天</template>
        <el-form label-width="120px">
          <el-form-item label="语音消息优先">
            <el-switch v-model="settings.voiceMessagePreferred" @change="settings.save()" />
            <div class="form-help">麦克风按钮默认以语音消息形式发送，而非转文字</div>
          </el-form-item>
        </el-form>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>生成艺术</template>
        <div class="art-preview">
          <div class="art-disabled">生成艺术效果已关闭</div>
        </div>
        <p class="form-help">基于当前主题实时渲染的算法背景。相同种子可复现同一画面。</p>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>模型存储</template>
        <el-form label-width="120px">
          <el-form-item label="存储位置">
            <div class="model-dir-row">
              <el-input 
                v-model="settings.modelsDir" 
                readonly 
                placeholder="点击浏览选择目录"
                class="model-dir-input"
              />
              <el-button size="default" @click="handleSelectDir" :disabled="isMigrating">
                浏览
              </el-button>
            </div>
          </el-form-item>
          <el-form-item v-if="settings.modelDirInfo" label="当前统计">
            <div class="model-stats">
              <el-tag type="info" size="small">
                {{ settings.modelDirInfo.modelCount }} 个模型
              </el-tag>
              <el-tag type="info" size="small">
                占用 {{ settings.modelDirInfo.totalSizeFormatted }}
              </el-tag>
            </div>
          </el-form-item>
          <el-form-item v-if="showMigrationProgress" label="迁移进度">
            <div class="migration-progress">
              <el-progress 
                :percentage="settings.migrationProgress" 
                :stroke-width="8"
                :status="progressStatus"
              />
              <div v-if="isMigrating && settings.migrationCurrentModel" class="migration-current">
                正在迁移: {{ settings.migrationCurrentModel }}
              </div>
              <div v-else-if="settings.migrationStatus === 'success'" class="migration-success">
                ✓ 模型迁移完成，新目录已生效
              </div>
            </div>
          </el-form-item>
          <el-form-item v-if="settings.migrationStatus === 'error'" label="迁移失败">
            <el-alert type="error" :closable="false" show-icon>
              <template #title>{{ settings.migrationError }}</template>
            </el-alert>
          </el-form-item>
          <el-form-item v-if="hasModelDirChanged && !isMigrating" label="操作">
            <div class="model-dir-actions">
              <el-button type="primary" @click="handleApplyDir">
                仅切换目录
              </el-button>
              <el-button type="success" @click="handleMigrate">
                迁移模型到新目录
              </el-button>
            </div>
            <div class="form-help">
              "仅切换目录"会更改后续下载位置但不移动现有模型；"迁移模型"会将现有模型复制到新目录。
            </div>
          </el-form-item>
        </el-form>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>隐私</template>
        <el-form label-width="120px">
          <el-form-item label="联网权限">
            <el-switch v-model="settings.networkEnabled" @change="handleSettingChange" />
            <div class="form-help">允许 AI 访问网络来执行联网技能</div>
          </el-form-item>
          <el-form-item label="数据存储">
            <el-text type="info" size="small">{{ appInfo?.dataDir ?? '-' }}</el-text>
          </el-form-item>
        </el-form>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>音频设备</template>
        <el-form label-width="120px">
          <el-form-item label="输入设备">
            <el-select v-model="settings.inputDevice" style="width: 100%" @change="handleSettingChange">
              <el-option
                v-for="d in inputDevices"
                :key="d.value"
                :label="d.label"
                :value="d.value"
              />
            </el-select>
          </el-form-item>
          <el-form-item label="输出设备">
            <el-select v-model="settings.outputDevice" style="width: 100%" @change="handleSettingChange">
              <el-option
                v-for="d in outputDevices"
                :key="d.value"
                :label="d.label"
                :value="d.value"
              />
            </el-select>
          </el-form-item>
        </el-form>
      </el-card>

      <el-card class="settings-card" shadow="never">
        <template #header>关于</template>
        <div class="about-info">
          <div class="about-row">
            <span class="about-label">应用名称</span>
            <span>{{ appInfo?.name ?? 'AI 伴侣' }}</span>
          </div>
          <div class="about-row">
            <span class="about-label">版本</span>
            <span>v{{ appInfo?.version ?? '0.1.0' }}</span>
          </div>
          <div class="about-row">
            <span class="about-label">数据目录</span>
            <el-text type="info" size="small">{{ appInfo?.dataDir ?? '-' }}</el-text>
          </div>
          <div class="about-row">
            <span class="about-label">架构方案</span>
            <el-link
              href="https://github.com"
              type="primary"
              :underline="false"
              target="_blank"
            >
              查看文档
            </el-link>
          </div>
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-sections {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 800px;
}

.settings-card {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
  transition: all var(--transition);

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border-hover);
  }

  :deep(.el-card__header) {
    font-weight: 600;
    font-size: 15px;
    padding: 14px 20px;
    background: var(--glass-gradient);
    border-bottom: 1px solid var(--glass-border);
  }

  :deep(.el-card__body) {
    background: transparent;
  }
}

.form-help {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;
}

.art-disabled {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  color: var(--color-text-muted);
}

.about-info {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.about-row {
  display: flex;
  gap: 16px;
  font-size: 14px;
}

.about-label {
  min-width: 80px;
  color: var(--color-text-secondary);
}

.model-dir-row {
  display: flex;
  gap: 8px;
  width: 100%;
}

.model-dir-input {
  flex: 1;
}

.model-stats {
  display: flex;
  gap: 8px;
}

.migration-progress {
  width: 100%;
}

.migration-current {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;
}

.migration-success {
  font-size: 12px;
  color: var(--el-color-success);
  margin-top: 4px;
}

.model-dir-actions {
  display: flex;
  gap: 8px;
}

.art-preview {
  position: relative;
  height: 180px;
  border-radius: var(--radius-md);
  overflow: hidden;
  border: 1px solid var(--glass-border);
  background: var(--glass-bg-light);
}
</style>
