<template>
  <div class="cloud-provider-manager">
    <div class="header">
      <h2>云服务商管理</h2>
      <el-button type="primary" @click="showAddDialog">
        <el-icon><Plus /></el-icon>
        添加云服务商
      </el-button>
    </div>

    <el-table :data="providers" style="width: 100%" v-loading="loading">
      <el-table-column prop="name" label="名称" width="180" />
      <el-table-column prop="apiBase" label="API 地址" />
      <el-table-column prop="models" label="模型数量" width="120">
        <template #default="{ row }">
          {{ row.models.length }} 个模型
        </template>
      </el-table-column>
      <el-table-column prop="isEnabled" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="row.isEnabled ? 'success' : 'info'">
            {{ row.isEnabled ? '已启用' : '已禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="280" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="handleVerify(row.id)">
            <el-icon><Check /></el-icon>
            验证
          </el-button>
          <el-button size="small" @click="handleSync(row.id)">
            <el-icon><Refresh /></el-icon>
            同步
          </el-button>
          <el-button size="small" type="primary" @click="showEditDialog(row)">
            <el-icon><Edit /></el-icon>
            编辑
          </el-button>
          <el-button size="small" type="danger" @click="handleDelete(row.id)">
            <el-icon><Delete /></el-icon>
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 添加/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑云服务商' : '添加云服务商'"
      width="500px"
    >
      <el-form :model="form" label-width="100px">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="例如：OpenAI、智谱AI" />
        </el-form-item>
        <el-form-item label="API 地址" required>
          <el-input v-model="form.apiBase" placeholder="例如：https://api.openai.com/v1" />
        </el-form-item>
        <el-form-item label="API Key" required>
          <el-input
            v-model="form.apiKey"
            type="password"
            show-password
            placeholder="sk-..."
          />
        </el-form-item>
        <el-form-item label="图标 URL">
          <el-input v-model="form.iconUrl" placeholder="可选，服务商图标地址" />
        </el-form-item>
        <el-form-item label="启用状态">
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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Check, Refresh, Edit, Delete } from '@element-plus/icons-vue';
import type { CloudProviderConfig } from '@/types';
import {
  listCloudProviders,
  createCloudProvider,
  updateCloudProvider,
  deleteCloudProvider,
  verifyCloudProvider,
  syncCloudModels,
} from '@/api/cloudProvider';

const providers = ref<CloudProviderConfig[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const isEdit = ref(false);
const submitting = ref(false);
const editingId = ref<string | null>(null);

const form = ref({
  name: '',
  providerType: 'openai',
  apiBase: '',
  apiKey: '',
  iconUrl: '',
  isEnabled: true,
});

const loadProviders = async () => {
  loading.value = true;
  try {
    providers.value = await listCloudProviders();
  } catch (error) {
    ElMessage.error('加载云服务商列表失败');
    console.error(error);
  } finally {
    loading.value = false;
  }
};

const showAddDialog = () => {
  isEdit.value = false;
  editingId.value = null;
  form.value = {
    name: '',
    providerType: 'openai',
    apiBase: '',
    apiKey: '',
    iconUrl: '',
    isEnabled: true,
  };
  dialogVisible.value = true;
};

const showEditDialog = (provider: CloudProviderConfig) => {
  isEdit.value = true;
  editingId.value = provider.id;
  form.value = {
    name: provider.name,
    providerType: provider.providerType,
    apiBase: provider.apiBase,
    apiKey: provider.apiKey,
    iconUrl: provider.iconUrl || '',
    isEnabled: provider.isEnabled,
  };
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!form.value.name || !form.value.apiBase || !form.value.apiKey) {
    ElMessage.warning('请填写必填项');
    return;
  }

  submitting.value = true;
  try {
    if (isEdit.value && editingId.value) {
      await updateCloudProvider(editingId.value, {
        name: form.value.name,
        apiBase: form.value.apiBase,
        apiKey: form.value.apiKey,
        iconUrl: form.value.iconUrl || undefined,
        isEnabled: form.value.isEnabled,
      });
      ElMessage.success('更新成功');
    } else {
      await createCloudProvider({
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
  } catch (error) {
    ElMessage.error(isEdit.value ? '更新失败' : '添加失败');
    console.error(error);
  } finally {
    submitting.value = false;
  }
};

const handleDelete = async (id: string) => {
  try {
    await ElMessageBox.confirm('确定要删除这个云服务商吗？', '确认', {
      type: 'warning',
    });
    await deleteCloudProvider(id);
    ElMessage.success('删除成功');
    await loadProviders();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败');
      console.error(error);
    }
  }
};

const handleVerify = async (id: string) => {
  try {
    const result = await verifyCloudProvider(id);
    if (result.success) {
      ElMessage.success(`验证成功！发现 ${result.models.length} 个模型`);
      await loadProviders();
    } else {
      ElMessage.error(`验证失败：${result.message}`);
    }
  } catch (error) {
    ElMessage.error('验证失败');
    console.error(error);
  }
};

const handleSync = async (providerId: string) => {
  try {
    const count = await syncCloudModels(providerId);
    ElMessage.success(`同步成功！新增 ${count} 个模型`);
    await loadProviders();
  } catch (error) {
    ElMessage.error('同步失败');
    console.error(error);
  }
};

onMounted(() => {
  loadProviders();
});
</script>

<style scoped lang="scss">
.cloud-provider-manager {
  padding: 20px;

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;

    h2 {
      margin: 0;
      font-size: 20px;
    }
  }
}
</style>
