use crate::model_bus::provider::{ChatMessage, ContentPart};
use crate::rag::retriever::RetrievedChunk;
use crate::memory::long_term::MemoryItem;

pub const MAX_CONTEXT_TOKENS: usize = 1_000_000;
const RESERVED_OUTPUT_TOKENS: usize = 8192;
const SYSTEM_PROMPT_RESERVE: usize = 4096;
const KNOWLEDGE_RESERVE: usize = 200_000;
const MEMORY_RESERVE: usize = 50_000;

pub fn estimate_tokens(text: &str) -> usize {
    let mut chars = text.chars();
    let mut count: f32 = 0.0;
    while let Some(c) = chars.next() {
        if c as u32 > 0x7F {
            count += 0.7;
        } else if c.is_ascii_whitespace() {
            count += 0.3;
        } else {
            count += 0.25;
        }
    }
    (count.ceil() as usize).max(1)
}

pub fn message_tokens(msg: &ChatMessage) -> usize {
    let mut total = 4;
    for part in &msg.content {
        match part {
            ContentPart::Text(t) => total += estimate_tokens(t),
            ContentPart::ImageBytes(_) => total += 1024,
            ContentPart::ImageUrl(_) => total += 256,
            ContentPart::AudioBytes { transcript, .. } => {
                if let Some(t) = transcript {
                    total += estimate_tokens(t);
                }
                total += 128;
            }
            ContentPart::VideoFrames(frames) => {
                total += frames.len() * 768;
            }
        }
    }
    if let Some(tc) = &msg.tool_calls {
        for call in tc {
            total += estimate_tokens(&call.name);
            total += estimate_tokens(&call.arguments.to_string());
        }
    }
    total
}

pub fn knowledge_context_tokens(chunks: &[RetrievedChunk]) -> usize {
    let mut total = 64;
    for chunk in chunks {
        total += estimate_tokens(&chunk.text) + 32;
        if let Some(title) = &chunk.doc_title {
            total += estimate_tokens(title) + 16;
        }
    }
    total
}

pub fn memory_context_tokens(memories: &[MemoryItem]) -> usize {
    let mut total = 64;
    for mem in memories {
        total += estimate_tokens(&mem.content) + estimate_tokens(&mem.typ) + 32;
    }
    total
}

pub fn build_knowledge_context(chunks: &[RetrievedChunk]) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    let mut parts = Vec::with_capacity(chunks.len() + 2);
    parts.push("以下是从知识库检索到的相关内容，请基于这些信息回答用户问题：".to_string());
    for (i, chunk) in chunks.iter().enumerate() {
        let title = chunk.doc_title.as_deref().unwrap_or("未知文档");
        parts.push(format!(
            "[片段{}] 来源：{}\n{}",
            i + 1,
            title,
            chunk.text.trim()
        ));
    }
    parts.push("（以上为参考资料，请结合对话上下文作答）".to_string());
    parts.join("\n\n")
}

pub fn build_memory_context(memories: &[MemoryItem]) -> String {
    if memories.is_empty() {
        return String::new();
    }
    let mut parts = Vec::with_capacity(memories.len() + 2);
    parts.push("以下是关于用户的长期记忆，请在对话中自然运用：".to_string());
    for (i, mem) in memories.iter().enumerate() {
        let type_label = match mem.typ.as_str() {
            "preference" => "用户偏好",
            "event" => "重要事件",
            "summary" => "对话摘要",
            _ => "记忆",
        };
        parts.push(format!(
            "[记忆{}] 类型：{} | 重要性：{:.1}\n{}",
            i + 1,
            type_label,
            mem.importance,
            mem.content.trim()
        ));
    }
    parts.join("\n\n")
}

pub fn build_augmented_system_prompt(
    base_system_prompt: &str,
    knowledge_chunks: &[RetrievedChunk],
    memories: &[MemoryItem],
    relationship_context: &str,
    core_memory_context: &str,
    history_tokens: usize,
) -> (String, usize) {
    let mut context_parts = Vec::new();

    let knowledge_text = build_knowledge_context(knowledge_chunks);
    let memory_text = build_memory_context(memories);

    let base_tokens = estimate_tokens(base_system_prompt);
    let rel_tokens = if relationship_context.is_empty() { 0 } else { estimate_tokens(relationship_context) + 32 };
    let core_tokens = if core_memory_context.is_empty() { 0 } else { estimate_tokens(core_memory_context) + 32 };
    let mut available_for_ctx = MAX_CONTEXT_TOKENS
        .saturating_sub(base_tokens)
        .saturating_sub(rel_tokens)
        .saturating_sub(core_tokens)
        .saturating_sub(SYSTEM_PROMPT_RESERVE)
        .saturating_sub(RESERVED_OUTPUT_TOKENS)
        .saturating_sub(history_tokens);

    if !relationship_context.is_empty() {
        context_parts.push(relationship_context.to_string());
    }
    if !core_memory_context.is_empty() {
        context_parts.push(core_memory_context.to_string());
    }

    if !knowledge_text.is_empty() {
        let k_tokens = estimate_tokens(&knowledge_text);
        let budget = available_for_ctx.min(KNOWLEDGE_RESERVE);
        if k_tokens <= budget {
            context_parts.push(knowledge_text);
            available_for_ctx = available_for_ctx.saturating_sub(k_tokens);
        } else if budget > 200 {
            let mut truncated = String::new();
            let mut used = 0;
            for line in knowledge_text.lines() {
                let lt = estimate_tokens(line);
                if used + lt > budget {
                    break;
                }
                truncated.push_str(line);
                truncated.push('\n');
                used += lt;
            }
            if !truncated.is_empty() {
                context_parts.push(truncated);
                available_for_ctx = available_for_ctx.saturating_sub(used);
            }
        }
    }

    if !memory_text.is_empty() {
        let m_tokens = estimate_tokens(&memory_text);
        let budget = available_for_ctx.min(MEMORY_RESERVE);
        if m_tokens <= budget {
            context_parts.push(memory_text);
        } else if budget > 100 {
            let mut truncated = String::new();
            let mut used = 0;
            for line in memory_text.lines() {
                let lt = estimate_tokens(line);
                if used + lt > budget {
                    break;
                }
                truncated.push_str(line);
                truncated.push('\n');
                used += lt;
            }
            if !truncated.is_empty() {
                context_parts.push(truncated);
            }
        }
    }

    if context_parts.is_empty() {
        return (base_system_prompt.to_string(), base_tokens);
    }

    let augmented = format!(
        "{}\n\n---\n{}\n---\n",
        base_system_prompt,
        context_parts.join("\n\n")
    );
    let total_tokens = estimate_tokens(&augmented);
    (augmented, total_tokens)
}

pub fn trim_history_to_budget(
    messages: &[ChatMessage],
    system_tokens: usize,
) -> Vec<ChatMessage> {
    if messages.is_empty() {
        return messages.to_vec();
    }

    let user_msg_idx = messages.len() - 1;
    let user_tokens = message_tokens(&messages[user_msg_idx]);

    let mut history_budget = MAX_CONTEXT_TOKENS
        .saturating_sub(system_tokens)
        .saturating_sub(user_tokens)
        .saturating_sub(RESERVED_OUTPUT_TOKENS)
        .saturating_sub(256);

    if history_budget < 1000 {
        history_budget = 8000;
    }

    let mut trimmed = Vec::new();
    let mut used = 0;
    let mut last_tool_pair: Vec<ChatMessage> = Vec::new();
    let mut tool_seq_started = false;

    for i in (0..user_msg_idx).rev() {
        let msg = &messages[i];
        let t = message_tokens(msg);

        let is_tool_seq = msg.role == "tool"
            || (msg.role == "assistant" && msg.tool_calls.is_some())
            || tool_seq_started;
        if is_tool_seq {
            tool_seq_started = true;
            last_tool_pair.insert(0, msg.clone());
            if i > 0 {
                let prev = &messages[i - 1];
                if prev.role != "tool" && prev.tool_calls.is_none() {
                    tool_seq_started = false;
                    let pair_tokens: usize = last_tool_pair.iter().map(message_tokens).sum();
                    if used + pair_tokens <= history_budget {
                        for m in last_tool_pair.drain(..) {
                            trimmed.insert(0, m);
                        }
                        used += pair_tokens;
                    } else {
                        last_tool_pair.clear();
                    }
                }
            }
            continue;
        }

        if used + t > history_budget {
            break;
        }
        trimmed.insert(0, msg.clone());
        used += t;
    }

    if !last_tool_pair.is_empty() {
        let pair_tokens: usize = last_tool_pair.iter().map(message_tokens).sum();
        if used + pair_tokens <= history_budget {
            for (idx, m) in last_tool_pair.into_iter().enumerate() {
                trimmed.insert(idx, m);
            }
        }
    }

    trimmed.push(messages[user_msg_idx].clone());
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_chinese() {
        let t = estimate_tokens("你好世界");
        assert!(t >= 2 && t <= 6);
    }

    #[test]
    fn test_estimate_tokens_english() {
        let t = estimate_tokens("hello world this is a test");
        assert!(t >= 3 && t <= 10);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 1);
    }
}
