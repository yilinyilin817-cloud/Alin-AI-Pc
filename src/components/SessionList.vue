<script setup lang="ts">
import { useRouter, useRoute } from "vue-router";
import { useChatStore } from "@/stores/chat";
import { usePersonaStore } from "@/stores/persona";
import { ElMessageBox, ElMessage } from "element-plus";
import { Delete } from "@element-plus/icons-vue";

const router = useRouter();
const route = useRoute();
const chatStore = useChatStore();
const personaStore = usePersonaStore();

function selectSession(id: string) {
  chatStore.selectSession(id);
  router.push(`/chat/${id}`).catch((e) => console.warn("selectSession:", e));
}

async function handleDelete(e: MouseEvent, id: string) {
  e.stopPropagation();
  try {
    await ElMessageBox.confirm(
      "确定要删除该会话吗？删除后会话中的所有消息将无法恢复。",
      "删除会话",
      {
        confirmButtonText: "确定删除",
        cancelButtonText: "取消",
        type: "warning",
      }
    );
    await chatStore.deleteSession(id);
    ElMessage.success("会话已删除");
    if (route.params.sessionId === id) {
      if (chatStore.sessions.length > 0) {
        const nextId = chatStore.sessions[0].id;
        chatStore.selectSession(nextId);
        router.replace(`/chat/${nextId}`).catch(() => {});
      } else {
        const pid = personaStore.currentPersonaId ?? personaStore.personas[0]?.id;
        if (pid) {
          const session = await chatStore.createSession(pid);
          router.replace(`/chat/${session.id}`).catch(() => {});
        } else {
          router.replace("/chat").catch(() => {});
        }
      }
    }
  } catch (e: any) {
    if (e !== "cancel") {
      ElMessage.error(`删除失败: ${e?.message ?? e}`);
    }
  }
}
</script>

<template>
  <el-scrollbar class="session-list">
    <div
      v-for="session in chatStore.sessions"
      :key="session.id"
      class="session-item"
      :class="{ active: session.id === chatStore.currentSessionId }"
      @click="selectSession(session.id)"
    >
      <span v-if="session.isPinned" class="pin">📌</span>
      <span class="session-title">{{ session.title }}</span>
      <el-button
        class="delete-btn"
        :icon="Delete"
        size="small"
        text
        circle
        @click="handleDelete($event, session.id)"
      />
    </div>
  </el-scrollbar>
</template>

<style scoped lang="scss">
.session-list {
  width: 100%;
  min-height: 0;
}

.session-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-secondary);
  transition: all var(--transition);
  background: transparent;
  border: 1px solid transparent;
  margin-bottom: 2px;
  position: relative;

  &:hover {
    background: var(--glass-hover-bg);
    border-color: var(--glass-border);

    .delete-btn {
      opacity: 1;
    }
  }

  &.active {
    background: var(--glass-active-bg);
    border-color: var(--glass-border-hover);
    color: var(--color-primary);
    box-shadow: var(--glass-glow);
  }
}

.session-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.pin {
  font-size: 10px;
  flex-shrink: 0;
}

.delete-btn {
  opacity: 0;
  flex-shrink: 0;
  padding: 2px;
  color: var(--color-text-muted);
  transition: opacity 0.2s, color 0.2s;

  &:hover {
    color: var(--color-danger) !important;
  }
}
</style>
