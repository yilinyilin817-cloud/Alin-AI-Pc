<template>
  <div ref="rendererRef" class="message-renderer" @click="handleRendererClick">
    <div v-html="renderedContent"></div>
    <VoiceMessageBubble
      v-for="(part, i) in audioParts"
      :key="i"
      :part="part"
      class="renderer-audio-part"
    />
    <span v-if="streaming" class="streaming-cursor"></span>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { marked } from 'marked';
import hljs from 'highlight.js';
import { ElMessage } from 'element-plus';
import type { ContentPart } from '@/types';
import VoiceMessageBubble from './VoiceMessageBubble.vue';

interface Props {
  content: string;
  streaming?: boolean;
  parts?: ContentPart[];
}

const props = withDefaults(defineProps<Props>(), {
  streaming: false,
  parts: () => [],
});

const audioParts = computed(() =>
  props.parts.filter((p): p is ContentPart & { type: 'audio_bytes' } => p.type === 'audio_bytes'),
);

const rendererRef = ref<HTMLElement | null>(null);

// 配置 marked
marked.setOptions({
  breaks: true,
  gfm: true,
});

// 自定义渲染器
const renderer = new marked.Renderer();

// 将高亮后的代码按行包裹，便于显示行号
function wrapLines(highlighted: string): string {
  const lines = highlighted.split('\n');
  // 去掉末尾空行（highlight.js 有时会在最后保留一个换行）
  if (lines.length > 0 && lines[lines.length - 1].trim() === '') {
    lines.pop();
  }
  return lines
    .map((line) => `<div class="code-line">${line || ' '}</div>`)
    .join('');
}

// 代码块渲染
renderer.code = function({ text, lang }: { text: string; lang?: string }) {
  const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
  let highlighted: string;
  try {
    highlighted = language && language !== 'plaintext'
      ? hljs.highlight(text, { language }).value
      : hljs.highlightAuto(text).value;
  } catch {
    highlighted = text;
  }
  const wrapped = wrapLines(highlighted);
  // 对原始代码做 HTML 转义后存入 data-code，用于复制
  const escapedCode = text
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  return `
<div class="code-block">
  <div class="code-header">
    <span class="code-lang">${language}</span>
    <button class="copy-btn" data-code="${escapedCode}" title="复制代码">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
      </svg>
      <span>复制</span>
    </button>
  </div>
  <pre><code class="hljs language-${language}">${wrapped}</code></pre>
</div>`;
};

// 链接渲染（新窗口打开）
renderer.link = function({ href, title, tokens }: { href: string; title?: string | null; tokens: any[] }) {
  const text = this.parser.parseInline(tokens);
  const titleAttr = title ? ` title="${title}"` : '';
  return `<a href="${href}"${titleAttr} target="_blank" rel="noopener noreferrer">${text}</a>`;
};

// 标题渲染：添加锚点 id
renderer.heading = function({ text, depth, raw }: { text: string; depth: number; raw: string }) {
  const id = raw
    .toLowerCase()
    .replace(/<[^>]+>/g, '')
    .replace(/[^\w\u4e00-\u9fa5]+/g, '-')
    .replace(/^-|-$/g, '');
  return `<h${depth} id="${id}" class="md-heading">${text}<a href="#${id}" class="heading-anchor" aria-hidden="true">#</a></h${depth}>`;
};

marked.use({ renderer });

const renderedContent = computed(() => {
  try {
    return marked(props.content);
  } catch (error) {
    console.error('Markdown render error:', error);
    return props.content;
  }
});

// 事件委托处理代码块复制按钮
function handleRendererClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  const btn = target.closest('.copy-btn') as HTMLElement | null;
  if (!btn || !rendererRef.value) return;

  const rawCode = btn.getAttribute('data-code');
  if (rawCode === null) return;

  // 还原转义后的代码文本
  const code = rawCode
    .replace(/&gt;/g, '>')
    .replace(/&lt;/g, '<')
    .replace(/&quot;/g, '"')
    .replace(/&amp;/g, '&');

  navigator.clipboard.writeText(code).then(() => {
    ElMessage.success('已复制到剪贴板');
  }).catch(() => {
    ElMessage.error('复制失败，请手动复制');
  });
}
</script>

<style scoped lang="scss">
.message-renderer {
  :deep(.code-block) {
    margin: var(--spacing-md) 0;
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

      .copy-btn {
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
    }

    pre {
      margin: 0;
      padding: var(--spacing-md) 0;
      overflow-x: auto;
      counter-reset: code-line;

      code {
        display: block;
        font-family: var(--font-family-code);
        font-size: var(--font-size-sm);
        line-height: 1.7;

        .code-line {
          display: flex;
          counter-increment: code-line;

          &::before {
            content: counter(code-line);
            flex-shrink: 0;
            width: 40px;
            padding-right: var(--spacing-md);
            text-align: right;
            color: var(--color-text-muted);
            font-size: var(--font-size-xs);
            line-height: inherit;
            user-select: none;
            border-right: 1px solid var(--color-border-light);
            margin-right: var(--spacing-md);
          }
        }
      }
    }
  }

  :deep(code):not(pre code) {
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

  :deep(blockquote) {
    margin: var(--spacing-md) 0;
    padding: var(--spacing-sm) var(--spacing-md);
    border-left: 3px solid var(--color-primary);
    background: var(--glass-bg-light);
    color: var(--color-text-secondary);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }

  :deep(table) {
    width: 100%;
    margin: var(--spacing-md) 0;
    border-collapse: collapse;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;

    th, td {
      padding: var(--spacing-sm) var(--spacing-md);
      border: 1px solid var(--color-border);
      text-align: left;
    }

    th {
      background: var(--color-bg-elevated);
      font-weight: 600;
      color: var(--color-text-primary);
    }

    tr:nth-child(even) {
      background: var(--glass-bg-light);
    }
  }

  :deep(ul), :deep(ol) {
    margin: var(--spacing-sm) 0;
    padding-left: var(--spacing-lg);
  }

  :deep(li) {
    margin: var(--spacing-xs) 0;
  }

  :deep(p) {
    margin: var(--spacing-sm) 0;

    &:first-child {
      margin-top: 0;
    }

    &:last-child {
      margin-bottom: 0;
    }
  }

  :deep(.md-heading) {
    position: relative;
    margin: var(--spacing-xl) 0 var(--spacing-sm);
    color: var(--color-text-primary);
    font-weight: 600;
    line-height: 1.4;

    &:first-child {
      margin-top: 0;
    }

    .heading-anchor {
      margin-left: var(--spacing-xs);
      color: var(--color-text-muted);
      text-decoration: none;
      opacity: 0;
      transition: opacity var(--transition-base);
    }

    &:hover .heading-anchor {
      opacity: 1;
    }
  }

  :deep(h1.md-heading) { font-size: var(--font-size-2xl); }
  :deep(h2.md-heading) { font-size: var(--font-size-xl); }
  :deep(h3.md-heading) { font-size: var(--font-size-lg); }
  :deep(h4.md-heading) { font-size: var(--font-size-base); }
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

.renderer-audio-part {
  margin-top: var(--spacing-sm);
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
</style>
