<script setup lang="ts">
import { ref } from 'vue';
import { ElMessage, ElTooltip } from 'element-plus';
import { marked } from 'marked';
import hljs from 'highlight.js';
import type { MessageSegment, MessageSegmentSource } from '@/types';

interface Props {
  segments: MessageSegment[];
  streaming?: boolean;
}

withDefaults(defineProps<Props>(), {
  streaming: false,
});

const expandedThink = ref<Record<number, boolean>>({});

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function imageSrc(segment: MessageSegment & { type: 'image' }): string {
  if (segment.imageUrl) return segment.imageUrl;
  if (segment.imageBytes) {
    return `data:image/png;base64,${bytesToBase64(segment.imageBytes)}`;
  }
  return '';
}

function segmentContent(segment: MessageSegment): string {
  switch (segment.type) {
    case 'text':
    case 'code':
    case 'think':
    case 'tool_result':
      return segment.content ?? '';
    case 'image':
      return segment.imageUrl ?? '(image)';
    default:
      return '';
  }
}

async function copySegment(segment: MessageSegment) {
  const text = segmentContent(segment);
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    ElMessage.success('已复制到剪贴板');
  } catch {
    ElMessage.error('复制失败');
  }
}

function sourceLabel(source: MessageSegmentSource): string {
  return source.name;
}

function sourceTooltipContent(source: MessageSegmentSource): string {
  const parts = [source.name];
  if (source.url) parts.push(source.url);
  return parts.join('\n');
}

function renderMarkdown(text: string): string {
  try {
    const result = marked.parseInline(text);
    return typeof result === 'string' ? result : text;
  } catch {
    return text;
  }
}

function highlightedCode(code: string, lang?: string): string {
  const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
  try {
    return language !== 'plaintext'
      ? hljs.highlight(code, { language }).value
      : hljs.highlightAuto(code).value;
  } catch {
    return code;
  }
}

function toggleThink(index: number) {
  expandedThink.value[index] = !expandedThink.value[index];
}
</script>

<template>
  <div class="segmented-message">
    <div
      v-for="(segment, index) in segments"
      :key="`${segment.type}-${index}`"
      class="segment"
      :class="`segment-${segment.type}`"
    >
      <!-- 文本段 -->
      <template v-if="segment.type === 'text'">
        <div class="segment-text" v-html="renderMarkdown(segment.content)"></div>
      </template>

      <!-- 代码段 -->
      <template v-else-if="segment.type === 'code'">
        <div class="code-block">
          <div class="code-header">
            <span class="code-lang">{{ segment.language || 'plaintext' }}</span>
            <button class="copy-btn" title="复制代码" @click="copySegment(segment)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span>复制</span>
            </button>
          </div>
          <pre><code class="hljs" :class="`language-${segment.language || 'plaintext'}`" v-html="highlightedCode(segment.content, segment.language)"></code></pre>
        </div>
      </template>

      <!-- 图片段 -->
      <template v-else-if="segment.type === 'image'">
        <img
          v-if="imageSrc(segment)"
          :src="imageSrc(segment)"
          class="segment-image"
          alt="segment image"
        />
        <div v-else class="segment-placeholder">图片加载失败</div>
      </template>

      <!-- 思考段 -->
      <template v-else-if="segment.type === 'think'">
        <div class="think-block" :class="{ expanded: expandedThink[index] }">
          <button class="think-header" @click="toggleThink(index)">
            <svg
              class="think-chevron"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              width="14"
              height="14"
            >
              <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
            <span class="think-label">思考过程</span>
            <span class="think-hint">{{ expandedThink[index] ? '收起' : '展开' }}</span>
          </button>
          <div v-show="expandedThink[index]" class="think-content">
            <pre>{{ segment.content }}</pre>
          </div>
        </div>
      </template>

      <!-- 工具结果段 -->
      <template v-else-if="segment.type === 'tool_result'">
        <div class="tool-result-block">
          <div class="tool-result-header">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
              <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>
            </svg>
            <span>工具调用结果</span>
            <span v-if="segment.toolCallId" class="tool-call-id">{{ segment.toolCallId }}</span>
          </div>
          <pre class="tool-result-content">{{ segment.content }}</pre>
        </div>
      </template>

      <!-- 操作栏：复制 + 来源 -->
      <div class="segment-actions">
        <button class="action-btn copy-action" title="复制该段" @click="copySegment(segment)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>

        <ElTooltip
          v-if="segment.source"
          placement="top"
          :content="sourceTooltipContent(segment.source)"
          effect="dark"
        >
          <a
            v-if="segment.source.url"
            class="source-badge"
            :href="segment.source.url"
            target="_blank"
            rel="noopener noreferrer"
          >
            {{ sourceLabel(segment.source) }}
          </a>
          <span v-else class="source-badge">{{ sourceLabel(segment.source) }}</span>
        </ElTooltip>
      </div>
    </div>

    <span v-if="streaming" class="streaming-cursor"></span>
  </div>
</template>

<style scoped lang="scss">
.segmented-message {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  line-height: 1.6;
  color: var(--color-text-primary);
}

.segment {
  position: relative;
  padding: var(--spacing-sm) 0;

  & + .segment {
    border-top: 1px solid var(--glass-border);
  }

  &:hover .segment-actions {
    opacity: 1;
  }
}

.segment-text {
  word-break: break-word;

  :deep(p) {
    margin: var(--spacing-sm) 0;

    &:first-child {
      margin-top: 0;
    }

    &:last-child {
      margin-bottom: 0;
    }
  }

  :deep(code) {
    padding: 2px 6px;
    font-family: var(--font-family-code);
    font-size: 0.9em;
    color: var(--color-accent);
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    word-break: break-word;
  }

  :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color var(--transition-base);

    &:hover {
      border-bottom-color: var(--color-primary);
    }
  }
}

.code-block {
  margin: var(--spacing-sm) 0;
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border);
  box-shadow: var(--shadow-sm);

  .code-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--color-bg-elevated);
    border-bottom: 1px solid var(--color-border);

    .code-lang {
      font-size: var(--font-size-xs);
      color: var(--color-text-secondary);
      text-transform: uppercase;
      font-weight: 600;
      letter-spacing: 0.05em;
    }
  }

  pre {
    margin: 0;
    padding: var(--spacing-md);
    overflow-x: auto;

    code {
      display: block;
      font-family: var(--font-family-code);
      font-size: var(--font-size-sm);
      line-height: 1.7;
      background: transparent;
    }
  }
}

.copy-btn,
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-base);

  &:hover {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-text-inverse);
  }

  &:active {
    transform: scale(0.96);
  }
}

.segment-image {
  max-width: 100%;
  max-height: 320px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
}

.segment-placeholder {
  padding: var(--spacing-md);
  color: var(--color-text-secondary);
  background: var(--color-bg-surface);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
  text-align: center;
  font-size: var(--font-size-sm);
}

.think-block {
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
  background: var(--glass-bg-light);
  overflow: hidden;

  &.expanded .think-chevron {
    transform: rotate(90deg);
  }
}

.think-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  width: 100%;
  padding: var(--spacing-sm) var(--spacing-md);
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: background var(--transition-base);

  &:hover {
    background: var(--color-bg-hover);
  }
}

.think-chevron {
  transition: transform var(--transition-base);
}

.think-label {
  font-weight: 600;
}

.think-hint {
  margin-left: auto;
  font-size: var(--font-size-xs);
  opacity: 0.7;
}

.think-content {
  padding: var(--spacing-sm) var(--spacing-md);
  border-top: 1px dashed var(--color-border);

  pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--font-family-code);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    line-height: 1.6;
  }
}

.tool-result-block {
  border: 1px solid var(--tool-border);
  border-radius: var(--radius-md);
  background: var(--tool-bg);
  overflow: hidden;
}

.tool-result-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--tool-icon-bg);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  font-weight: 600;
}

.tool-call-id {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--color-text-muted);
  font-weight: 400;
  font-family: var(--font-family-code);
}

.tool-result-content {
  margin: 0;
  padding: var(--spacing-md);
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-family-code);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  background: var(--tool-content-bg);
  border-top: 1px solid var(--tool-content-border);
}

.segment-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-sm);
  opacity: 0;
  transition: opacity var(--transition-base);
}

.action-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  justify-content: center;
}

.source-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  font-size: var(--font-size-xs);
  color: var(--color-primary-light);
  background: var(--color-primary-subtle);
  border: 1px solid var(--color-primary-subtle);
  border-radius: var(--radius-sm);
  text-decoration: none;
  cursor: pointer;
  transition: all var(--transition-base);

  &:hover {
    background: var(--color-primary);
    color: var(--color-text-inverse);
  }
}

.streaming-cursor {
  display: inline-block;
  width: 2px;
  height: 1em;
  background: var(--color-primary);
  animation: blink 0.8s infinite;
  margin-left: 2px;
  vertical-align: text-bottom;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
</style>
