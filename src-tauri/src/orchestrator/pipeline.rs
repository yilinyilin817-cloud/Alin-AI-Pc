use crate::model_bus::provider::{
    ChatChunk, ChatMessage, ChatRequest, ContentPart, ToolCall, ToolSchema,
};
use crate::orchestrator::workflow_engine::{self, WorkflowContext, WorkflowEvent};
use crate::skill::registry::SkillRegistry;
use crate::state::AppState;
use crate::storage::models::{EmotionTag, Message, MessageSegment, SegmentType};
use crate::storage::repo;
use crate::storage::Database;
use crate::context::{build_augmented_system_prompt, trim_history_to_budget, message_tokens};

use chrono::{Datelike, Timelike, Utc};
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

// ─── 流式事件 ─────────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatChunkEvent {
    pub session_id: String,
    pub message_id: String,
    pub chunk: String,
    pub done: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSegmentEvent {
    pub session_id: String,
    pub message_id: String,
    pub segment: MessageSegment,
    pub segment_index: usize,
    pub done: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionEvent {
    pub emotion: String,
    pub valence: f64,
    pub arousal: f64,
}

// ─── Mock 降级 ─────────────────────────────────────────

struct FallbackLlm;

impl FallbackLlm {
    fn generate(persona_name: &str, persona_prompt: &str, user_message: &str) -> String {
        let lower = user_message.to_lowercase();
        if lower.contains("天气") {
            return format!(
                "{persona_name}：让我帮你查一下～今天天气晴朗，气温 28°C，适合出门散步哦！"
            );
        }
        if lower.contains("你好") || lower.contains("嗨") {
            return format!("{persona_name}：你好呀～很高兴见到你！");
        }
        let templates = [
            format!("{persona_name}：嗯，我理解你的意思～让我想想怎么回答你最好。"),
            format!("{persona_name}：谢谢你跟我分享这些，我很开心能陪你聊天。"),
            format!("{persona_name}：这个问题很有趣呢！我觉得可以从几个角度来看……"),
        ];
        let idx = user_message.len() % templates.len();
        templates[idx].clone()
    }

    fn random_emotion() -> EmotionTag {
        let emotions = ["happy", "neutral", "sad", "surprised"];
        let idx = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_millis() as usize
            % emotions.len();
        EmotionTag {
            emotion: emotions[idx].into(),
            valence: 0.3,
            arousal: 0.5,
        }
    }
}

// ─── 主 Pipeline ──────────────────────────────────────

pub async fn handle_send_message(
    app: AppHandle,
    state: &AppState,
    session_id: String,
    content: String,
    parts: Option<Vec<ContentPart>>,
    user_message_id: String,
) -> Result<Message, String> {
    let session = state
        .db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT persona_id FROM session WHERE id = ?1",
                rusqlite::params![session_id],
                |row| row.get::<_, String>(0),
            )
        })
        .map_err(|e| e.to_string())?;

    let persona_row = repo::get_persona(&state.db, &session)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Persona not found".to_string())?;
    let persona = persona_row.definition;

    // 1. 保存用户消息
    let now = Utc::now().to_rfc3339();
    let parts_json = parts.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let user_msg = Message {
        id: user_message_id,
        session_id: session_id.clone(),
        role: "user".into(),
        content: content.clone(),
        content_parts: parts_json,
        segments: None,
        emotion_tag: None,
        tool_calls: None,
        created_at: now.clone(),
    };
    repo::insert_message(&state.db, &user_msg).map_err(|e| e.to_string())?;

    if content.len() <= 20 {
        let _ = repo::update_session_title(&state.db, &session_id, &content);
    }
    let _ = repo::touch_session(&state.db, &session_id);

    // 2. 工作流触发与执行（失败不影响主流程）
    let mut workflow_ctx = WorkflowContext::new(
        session_id.clone(),
        persona.id.clone(),
        content.clone(),
    );

    match repo::list_workflows_by_persona(&state.db, &persona.id) {
        Ok(workflows) => {
            let event = WorkflowEvent::Message { content: content.clone() };
            for workflow in workflows {
                if workflow.enabled && workflow_engine::match_trigger(&workflow, &event) {
                    if let Err(e) = workflow_engine::execute_workflow(state, &workflow, &mut workflow_ctx).await {
                        log::warn!("Workflow {} execution failed: {}", workflow.id, e);
                    }
                }
            }
        }
        Err(e) => log::warn!("Failed to load workflows for persona {}: {}", persona.id, e),
    }

    // 3. 构造聊天请求
    let recent_msgs = repo::list_messages(&state.db, &session_id).map_err(|e| e.to_string())?;
    let base_system_prompt = if workflow_ctx.variables.is_empty() {
        build_system_prompt(&persona)
    } else {
        workflow_engine::inject_workflow_context(&build_system_prompt(&persona), &workflow_ctx)
    };

    // 加载关系状态和核心记忆
    let mut rel_state = crate::relationship::get_or_create(&state.db, &persona.id);
    let core_mem = crate::memory::core_memory::get_or_create(&state.db, &persona.id);
    let relationship_context = crate::relationship::build_relationship_context(&rel_state);
    let core_memory_context = crate::memory::core_memory::build_core_memory_context(&core_mem);

    // 收集所有历史消息（不再固定限制条数，后续由上下文管理器动态裁剪）
    let mut history_messages: Vec<ChatMessage> = Vec::with_capacity(recent_msgs.len() + 8);
    for msg in &recent_msgs {
        match msg.role.as_str() {
            "user" => {
                let mut parts = vec![ContentPart::text(&msg.content)];
                if let Some(ref pjson) = msg.content_parts {
                    if let Ok(extra) = serde_json::from_str::<Vec<ContentPart>>(pjson) {
                        parts.extend(extra);
                    }
                }
                history_messages.push(ChatMessage {
                    role: "user".into(),
                    content: parts,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            "assistant" => {
                let tool_calls = msg
                    .tool_calls
                    .as_ref()
                    .and_then(|s| serde_json::from_str::<Vec<ToolCall>>(s).ok());
                if let Some(tc) = tool_calls {
                    history_messages.push(ChatMessage::assistant_with_tools(
                        &msg.content,
                        tc,
                    ));
                } else {
                    history_messages.push(ChatMessage::assistant(&msg.content));
                }
            }
            "tool" => {
                history_messages.push(ChatMessage::tool("", &msg.content));
            }
            _ => {}
        }
    }

    // 添加本次用户消息
    let user_parts = if let Some(p) = parts {
        let mut p = p;
        p.insert(0, ContentPart::text(&content));
        p
    } else {
        vec![ContentPart::text(&content)]
    };
    history_messages.push(ChatMessage {
        role: "user".into(),
        content: user_parts,
        tool_calls: None,
        tool_call_id: None,
    });

    // 4. 注入技能 tool schema
    let mut skill_registry = SkillRegistry::new();
    {
        let plugin_registry = state
            .plugin_registry
            .read()
            .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
        let _ = skill_registry.load_all(&state.db, &*plugin_registry);
    }
    let tools: Vec<ToolSchema> = if persona.skills.is_empty() {
        skill_registry.enabled_schemas(&skill_registry.list_names())
    } else {
        skill_registry.enabled_schemas(&persona.skills)
    };

    // 5. 检索记忆 + RAG（并行，增加检索数量以充分利用 1M 上下文）
    let (memories, knowledge_chunks) = tokio::join!(
        crate::memory::long_term::recall(&state.db, &*state.vector_store, &persona.id, &content, 15),
        crate::rag::retriever::search(&state.db, &*state.vector_store, &content, None, 20),
    );
    let memories = memories.unwrap_or_default();
    let knowledge_chunks = knowledge_chunks.unwrap_or_default();

    // 计算历史消息预估 token 数
    let history_tokens: usize = history_messages.iter().map(message_tokens).sum();

    // 构建增强的系统提示（包含时间、角色设定、关系状态、核心记忆、知识检索结果、长期记忆）
    let (system_prompt, system_tokens) = build_augmented_system_prompt(
        &base_system_prompt,
        &knowledge_chunks,
        &memories,
        &relationship_context,
        &core_memory_context,
        history_tokens,
    );

    // 智能裁剪历史消息，确保总上下文不超过 1M tokens
    let trimmed_history = trim_history_to_budget(&history_messages, system_tokens);
    let mut chat_messages = vec![ChatMessage::system(&system_prompt)];
    chat_messages.extend(trimmed_history);

    // 判断互动事件类型（用于更新亲密度）
    let hour = Utc::now().hour();
    let is_late_night = hour >= 23 || hour < 5;
    let turn_count = recent_msgs.len() as u32 / 2;

    let mut event = crate::relationship::InteractionEvent::Normal;
    let content_lower = content.to_lowercase();
    if recent_msgs.is_empty() && rel_state.conversation_count <= 1 {
        event = crate::relationship::InteractionEvent::FirstGreeting;
    } else if turn_count >= 20 {
        event = crate::relationship::InteractionEvent::LongConversation(turn_count);
    } else if is_late_night {
        event = crate::relationship::InteractionEvent::LateNightChat;
    } else if content_lower.contains("喜欢") || content_lower.contains("爱") || content_lower.contains("想你") || content_lower.contains("宝贝") || content_lower.contains("亲爱的") {
        event = crate::relationship::InteractionEvent::UserAffectionate;
    } else if content_lower.contains("难过") || content_lower.contains("伤心") || content_lower.contains("不开心") || content_lower.contains("哭") || content_lower.contains("痛苦") {
        event = crate::relationship::InteractionEvent::UserEmotionNegative;
    } else if content_lower.contains("嗯") || content_lower.contains("哦") || content_lower.contains("...") || content.len() <= 2 {
        event = crate::relationship::InteractionEvent::UserCold;
    } else if content_lower.contains("心事") || content_lower.contains("秘密") || content_lower.contains("告诉你") || content_lower.contains("我觉得") {
        event = crate::relationship::InteractionEvent::UserSharedFeelings;
    }

    // 更新关系状态（记录互动）
    let new_milestones = crate::relationship::record_interaction(&state.db, &mut rel_state, event);
    for ms in &new_milestones {
        let _ = app.emit("milestone-achieved", serde_json::json!({
            "personaId": persona.id,
            "milestone": ms,
        }));
    }

    log::info!(
        "Context prepared: intimacy {:.0}, style {:?}, system ~{} tokens, history {} messages, knowledge {} chunks, memories {} items, core_mem {}b, total estimated ~{} tokens",
        rel_state.intimacy,
        rel_state.response_style,
        system_tokens,
        chat_messages.len() - 1,
        knowledge_chunks.len(),
        memories.len(),
        core_memory_context.len(),
        system_tokens + chat_messages.iter().skip(1).map(message_tokens).sum::<usize>()
    );

    // 6. 尝试真实 LLM：优先角色偏好模型，回退全局活跃
    let persona_provider = persona
        .llm
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let scheduler = state.model_scheduler.clone();
    let mut provider = scheduler.get(persona_provider).await;
    if provider.is_none() {
        provider = scheduler.active().await;
    }
    let use_fallback = provider.is_none();

    if use_fallback {
        log::info!("No active LLM provider (角色偏好={persona_provider}), using fallback mock");
    }

    let temperature = persona
        .llm
        .get("temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.7) as f32;

    let mut full_content = String::new();
    let mut final_assistant_id = String::new();

    if let Some(ref provider) = provider {
        let req = ChatRequest {
            messages: chat_messages.clone(),
            tools: tools.clone(),
            temperature,
            max_tokens: 8192,
            ..Default::default()
        };

        // 第一次请求：收集 assistant 的 tool_calls
        let (first_content, first_tool_calls) = collect_assistant_turn(provider.chat_stream(req)).await;

        if !first_tool_calls.is_empty() {
            // 保存本次工具调用消息
            let tool_turn_id = format!("msg_{}", Uuid::new_v4());
            let tool_turn_msg = Message {
                id: tool_turn_id.clone(),
                session_id: session_id.clone(),
                role: "assistant".into(),
                content: first_content.clone(),
                content_parts: None,
                segments: None,
                emotion_tag: None,
                tool_calls: Some(serde_json::to_string(&first_tool_calls).unwrap_or_default()),
                created_at: Utc::now().to_rfc3339(),
            };
            repo::insert_message(&state.db, &tool_turn_msg).map_err(|e| e.to_string())?;

            let _ = app.emit(
                "tool-call-start",
                serde_json::json!({
                    "sessionId": session_id,
                    "messageId": tool_turn_id,
                    "toolCalls": first_tool_calls,
                }),
            );

            // 执行工具并构造 tool 消息
            let mut tool_messages = Vec::new();
            for tc in &first_tool_calls {
                let mut status = "success";
                let result = crate::skill::executor::execute_skill(
                    &skill_registry,
                    &state.db,
                    &session_id,
                    &tc.name,
                    tc.arguments.clone(),
                )
                .await
                .unwrap_or_else(|e| {
                    status = "error";
                    format!("Error: {e}")
                });

                let _ = app.emit(
                    "tool-result",
                    serde_json::json!({
                        "sessionId": session_id,
                        "toolCallId": tc.id,
                        "status": status,
                        "result": result,
                    }),
                );

                tool_messages.push(ChatMessage::tool(&tc.id, &result));

                let tool_msg = Message {
                    id: format!("msg_{}", Uuid::new_v4()),
                    session_id: session_id.clone(),
                    role: "tool".into(),
                    content: result,
                    content_parts: None,
                    segments: None,
                    emotion_tag: None,
                    tool_calls: None,
                    created_at: Utc::now().to_rfc3339(),
                };
                repo::insert_message(&state.db, &tool_msg).map_err(|e| e.to_string())?;
            }

            // 回灌：assistant tool_call 消息 + tool 结果消息
            chat_messages.push(ChatMessage::assistant_with_tools(
                &first_content,
                first_tool_calls,
            ));
            chat_messages.extend(tool_messages);

            // 第二次请求：获取最终回复（重新裁剪以适应上下文窗口）
            let sys_msg = chat_messages[0].clone();
            let msgs_after_sys: Vec<ChatMessage> = chat_messages[1..].to_vec();
            let sys_tok = message_tokens(&sys_msg);
            let trimmed = trim_history_to_budget(&msgs_after_sys, sys_tok);
            let mut final_messages = vec![sys_msg];
            final_messages.extend(trimmed);

            let final_req = ChatRequest {
                messages: final_messages,
                tools: vec![],
                temperature,
                max_tokens: 8192,
                ..Default::default()
            };

            final_assistant_id = format!("msg_{}", Uuid::new_v4());
            let placeholder = Message {
                id: final_assistant_id.clone(),
                session_id: session_id.clone(),
                role: "assistant".into(),
                content: String::new(),
                content_parts: None,
                segments: None,
                emotion_tag: None,
                tool_calls: None,
                created_at: Utc::now().to_rfc3339(),
            };
            repo::insert_message(&state.db, &placeholder).map_err(|e| e.to_string())?;

            full_content = stream_content_to_message(
                provider.chat_stream(final_req),
                &app,
                &session_id,
                &final_assistant_id,
            )
            .await;

            let segments = split_content_to_segments(&full_content);
            emit_segments_and_save(&app, &state.db, &session_id, &final_assistant_id, &segments).await;
        } else {
            // 无工具调用：直接流式输出第一次回复
            final_assistant_id = format!("msg_{}", Uuid::new_v4());
            let placeholder = Message {
                id: final_assistant_id.clone(),
                session_id: session_id.clone(),
                role: "assistant".into(),
                content: String::new(),
                content_parts: None,
                segments: None,
                emotion_tag: None,
                tool_calls: None,
                created_at: Utc::now().to_rfc3339(),
            };
            repo::insert_message(&state.db, &placeholder).map_err(|e| e.to_string())?;

            full_content = first_content.clone();
            for chunk in mock_fake_stream(&first_content, 4, 8).await {
                let _ = app.emit(
                    "chat-chunk",
                    ChatChunkEvent {
                        session_id: session_id.clone(),
                        message_id: final_assistant_id.clone(),
                        chunk,
                        done: false,
                    },
                );
            }

            let segments = split_content_to_segments(&full_content);
            emit_segments_and_save(&app, &state.db, &session_id, &final_assistant_id, &segments).await;
        }
    }

    // 降级：Mock 回复
    if full_content.is_empty() {
        final_assistant_id = format!("msg_{}", Uuid::new_v4());
        let placeholder = Message {
            id: final_assistant_id.clone(),
            session_id: session_id.clone(),
            role: "assistant".into(),
            content: String::new(),
            content_parts: None,
            segments: None,
            emotion_tag: None,
            tool_calls: None,
            created_at: Utc::now().to_rfc3339(),
        };
        repo::insert_message(&state.db, &placeholder).map_err(|e| e.to_string())?;

        let mock_reply = FallbackLlm::generate(&persona.name, &persona.system_prompt, &content);
        let chunks = mock_fake_stream(&mock_reply, 2, 30).await;
        for chunk in chunks {
            full_content.push_str(&chunk);
            let _ = app.emit(
                "chat-chunk",
                ChatChunkEvent {
                    session_id: session_id.clone(),
                    message_id: final_assistant_id.clone(),
                    chunk,
                    done: false,
                },
            );
        }

        let segments = split_content_to_segments(&full_content);
        emit_segments_and_save(&app, &state.db, &session_id, &final_assistant_id, &segments).await;
    }

    // 8. 保存回复内容
    repo::update_message_content(&state.db, &final_assistant_id, &full_content)
        .map_err(|e| e.to_string())?;

    // 9. 情绪识别
    let emotion = if !use_fallback {
        crate::emotion::text_emotion::from_text(state.model_scheduler.as_ref(), &full_content)
            .await
            .unwrap_or_else(|_| FallbackLlm::random_emotion())
    } else {
        FallbackLlm::random_emotion()
    };
    repo::update_message_emotion(&state.db, &final_assistant_id, &emotion)
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "emotion-update",
        EmotionEvent {
            emotion: emotion.emotion.clone(),
            valence: emotion.valence,
            arousal: emotion.arousal,
        },
    );

    // 9.5 提交后台记忆提取任务
    state.memory_extractor.submit(persona.id.clone(), &content, &full_content);

    // 10. 完成信号
    let _ = app.emit(
        "chat-chunk",
        ChatChunkEvent {
            session_id: session_id.clone(),
            message_id: final_assistant_id.clone(),
            chunk: String::new(),
            done: true,
        },
    );

    Ok(Message {
        id: final_assistant_id,
        session_id,
        role: "assistant".into(),
        content: full_content,
        content_parts: None,
        segments: None,
        emotion_tag: Some(emotion),
        tool_calls: None,
        created_at: Utc::now().to_rfc3339(),
    })
}

/// 收集 assistant 首次回复中的文本与 tool_calls
async fn collect_assistant_turn(
    mut stream: futures_util::stream::BoxStream<'static, ChatChunk>,
) -> (String, Vec<ToolCall>) {
    let mut content = String::new();
    let mut tool_calls = Vec::new();

    while let Some(chunk) = stream.next().await {
        content.push_str(&chunk.content);
        if let Some(calls) = chunk.tool_calls {
            tool_calls.extend(calls);
        }
        if chunk.done {
            break;
        }
    }

    (content, tool_calls)
}

/// 将 provider 的流式输出转发到指定 message_id，并返回完整文本
async fn stream_content_to_message(
    mut stream: futures_util::stream::BoxStream<'static, ChatChunk>,
    app: &AppHandle,
    session_id: &str,
    message_id: &str,
) -> String {
    let mut full = String::new();

    while let Some(chunk) = stream.next().await {
        if !chunk.content.is_empty() {
            full.push_str(&chunk.content);
            let _ = app.emit(
                "chat-chunk",
                ChatChunkEvent {
                    session_id: session_id.to_string(),
                    message_id: message_id.to_string(),
                    chunk: chunk.content,
                    done: false,
                },
            );
        }
        if chunk.done {
            break;
        }
    }

    full
}

/// 将完整回复拆分为 MessageSegment，优先识别代码块与 Markdown 图片
fn split_content_to_segments(content: &str) -> Vec<MessageSegment> {
    let mut segments = Vec::new();
    let mut text_buf = String::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_code {
                // 结束代码块
                segments.push(MessageSegment {
                    segment_type: SegmentType::Code,
                    content: Some(code_buf.trim_end_matches('\n').to_string()),
                    language: if code_lang.is_empty() { None } else { Some(code_lang.clone()) },
                    image_url: None,
                    image_bytes: None,
                    tool_call_id: None,
                    source: None,
                    collapsed: None,
                });
                code_buf.clear();
                code_lang.clear();
                in_code = false;
            } else {
                // 开始代码块，先刷新已有文本
                if !text_buf.trim().is_empty() {
                    segments.extend(split_text_to_plain_and_images(&text_buf));
                    text_buf.clear();
                }
                code_lang = trimmed.trim_start_matches("```").trim().to_string();
                in_code = true;
            }
        } else if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
        } else {
            text_buf.push_str(line);
            text_buf.push('\n');
        }
    }

    if !text_buf.trim().is_empty() {
        segments.extend(split_text_to_plain_and_images(&text_buf));
    }

    segments
}

/// 将普通文本按 Markdown 图片语法拆分为 text/image 片段
fn split_text_to_plain_and_images(text: &str) -> Vec<MessageSegment> {
    let re = regex::Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();
    let mut out = Vec::new();
    let mut last_end = 0;

    for cap in re.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let before = &text[last_end..m.start()];
        if !before.trim().is_empty() {
            out.push(MessageSegment {
                segment_type: SegmentType::Text,
                content: Some(before.trim().to_string()),
                language: None,
                image_url: None,
                image_bytes: None,
                tool_call_id: None,
                source: None,
                collapsed: None,
            });
        }
        out.push(MessageSegment {
            segment_type: SegmentType::Image,
            content: Some(cap[1].to_string()),
            language: None,
            image_url: Some(cap[2].to_string()),
            image_bytes: None,
            tool_call_id: None,
            source: None,
            collapsed: None,
        });
        last_end = m.end();
    }

    let after = &text[last_end..];
    if !after.trim().is_empty() {
        out.push(MessageSegment {
            segment_type: SegmentType::Text,
            content: Some(after.trim().to_string()),
            language: None,
            image_url: None,
            image_bytes: None,
            tool_call_id: None,
            source: None,
            collapsed: None,
        });
    }

    out
}

/// 将 segments 通过 chat-segment 事件逐个发送到前端，并持久化到数据库
async fn emit_segments_and_save(
    app: &AppHandle,
    db: &Database,
    session_id: &str,
    message_id: &str,
    segments: &[MessageSegment],
) {
    let count = segments.len();
    for (idx, segment) in segments.iter().enumerate() {
        let _ = app.emit(
            "chat-segment",
            ChatSegmentEvent {
                session_id: session_id.to_string(),
                message_id: message_id.to_string(),
                segment: segment.clone(),
                segment_index: idx,
                done: idx == count.saturating_sub(1),
            },
        );
    }
    if !segments.is_empty() {
        let _ = repo::update_message_segments(db, message_id, segments);
    }
}

pub fn build_system_prompt(persona: &crate::storage::models::PersonaDefinition) -> String {
    let now = chrono::Local::now();

    // 基础时间信息
    let weekday = match now.weekday().num_days_from_monday() {
        0 => "一", 1 => "二", 2 => "三", 3 => "四",
        4 => "五", 5 => "六", _ => "日",
    };

    // 季节
    let season = match now.month() {
        3 | 4 | 5 => "春",
        6 | 7 | 8 => "夏",
        9 | 10 | 11 => "秋",
        _ => "冬",
    };

    // 时段
    let time_of_day = match now.hour() {
        0..=5 => "凌晨",
        6..=8 => "早晨",
        9..=11 => "上午",
        12..=13 => "中午",
        14..=17 => "下午",
        18..=19 => "傍晚",
        20..=22 => "晚上",
        _ => "深夜",
    };

    let week_num = now.iso_week().week();

    // 简易农历日期（仅月日，使用查表法估算）
    let lunar_info = simple_lunar_date(now.month(), now.day());

    // 临近节日提示
    let holiday_hint = upcoming_holiday(now.month(), now.day());

    let time_info = format!(
        "当前时间：{}年{}月{}日 {}:{:02}，星期{}（{}季，{}，第{}周）{}。{}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        weekday,
        season,
        time_of_day,
        week_num,
        lunar_info,
        holiday_hint,
    );

    format!(
        "{}\n\n{}\n\n性格特征：{}\n你可以使用以下工具来帮助用户。\
         \n当用户情绪明显波动时，请体现在回复风格中。\
         \n注意：当前是{}，请根据时段调整语气和回复内容。",
        time_info,
        persona.system_prompt,
        persona.personality.join("、"),
        time_of_day,
    )
}

/// 简易农历日期提示（近似计算，非精确农历）
fn simple_lunar_date(month: u32, day: u32) -> String {
    // 农历节日与特殊日期映射（近似对应公历日期，±1天误差）
    let lunar_hints: &[(u32, u32, &str)] = &[
        (1, 1, "春节/农历正月初一"),
        (1, 15, "元宵节附近"),
        (2, 14, "情人节"),
        (3, 8, "妇女节"),
        (4, 5, "清明节附近"),
        (5, 1, "劳动节"),
        (5, 4, "青年节"),
        (6, 1, "儿童节"),
        (7, 1, "建党节"),
        (8, 1, "建军节"),
        (9, 10, "教师节"),
        (10, 1, "国庆节"),
        (12, 25, "圣诞节"),
        (12, 31, "元旦前夕"),
    ];

    for (m, d, name) in lunar_hints {
        if *m == month && (*d == day || (day > 0 && *d == day + 1) || (day > 1 && *d == day - 1)) {
            return format!("临近节日：{}", name);
        }
    }

    // 默认显示农历月信息
    let lunar_month_name = match month {
        1 => "正月", 2 => "二月", 3 => "三月", 4 => "四月",
        5 => "五月", 6 => "六月", 7 => "七月", 8 => "八月",
        9 => "九月", 10 => "十月", 11 => "冬月", 12 => "腊月",
        _ => "",
    };
    format!("农历{}", lunar_month_name)
}

/// 检测即将到来的节日（30天内）
fn upcoming_holiday(month: u32, day: u32) -> String {
    let holidays: &[(u32, u32, &str)] = &[
        (1, 1, "元旦"),
        (2, 14, "情人节"),
        (3, 8, "妇女节"),
        (4, 1, "愚人节"),
        (5, 1, "劳动节"),
        (6, 1, "儿童节"),
        (10, 1, "国庆节"),
        (12, 25, "圣诞节"),
        (12, 31, "元旦"),
    ];

    let mut upcoming = Vec::new();
    for (hm, hd, name) in holidays {
        let days_ahead = days_until(*hm, *hd, month, day);
        if days_ahead >= 0 && days_ahead <= 30 {
            upcoming.push((days_ahead, *name));
        }
    }
    upcoming.sort_by_key(|(d, _)| *d);

    if let Some((days, name)) = upcoming.first() {
        if *days == 0 {
            return format!("今天是{}！", name);
        } else {
            return format!("距离{}还有{}天", name, days);
        }
    }
    String::new()
}

fn days_until(target_month: u32, target_day: u32, current_month: u32, current_day: u32) -> i64 {
    use chrono::Datelike;
    let now = chrono::Local::now();
    let current_year = now.year();
    let target = chrono::NaiveDate::from_ymd_opt(current_year, target_month, target_day);
    let today = chrono::NaiveDate::from_ymd_opt(current_year, current_month, current_day);

    if let (Some(t), Some(now_d)) = (target, today) {
        let diff = (t - now_d).num_days();
        if diff >= 0 { diff } else {
            // 今年已过，看明年
            let next_year_target = chrono::NaiveDate::from_ymd_opt(current_year + 1, target_month, target_day);
            if let Some(nt) = next_year_target {
                (nt - now_d).num_days()
            } else {
                -1
            }
        }
    } else {
        -1
    }
}

/// 模拟流式输出（降级用）
async fn mock_fake_stream(text: &str, _chunk_size: usize, delay_ms: u64) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    for chunk in chars.chunks(2) {
        chunks.push(chunk.iter().collect());
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }
    chunks
}

/// 微信自动回复专用：用指定角色 + 上下文调用 LLM，流式返回文本块
/// 不做 tool calling、RAG — 简单可靠，适合微信流式回复
pub async fn reply_via_llm_stream(
    scheduler: &crate::model_bus::scheduler::ModelScheduler,
    persona: &crate::storage::models::PersonaDefinition,
    peer_context: &[(String, String)],
    new_message: &str,
    mut on_chunk: impl FnMut(&str),
) -> Result<String, String> {
    use crate::model_bus::provider::{ChatMessage, ChatRequest};
    use crate::context::{trim_history_to_budget, message_tokens};
    use futures_util::StreamExt;

    let temperature = persona
        .llm
        .get("temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.7) as f32;

    let system = build_system_prompt(persona);
    let sys_msg = ChatMessage::system(&system);
    let sys_tokens = message_tokens(&sys_msg);

    let mut messages: Vec<ChatMessage> = vec![sys_msg];

    let mut history: Vec<ChatMessage> = Vec::with_capacity(peer_context.len() + 1);
    for (role, content) in peer_context.iter().rev().take(100).rev() {
        match role.as_str() {
            "user" => history.push(ChatMessage::user(content)),
            "assistant" => history.push(ChatMessage::assistant(content)),
            _ => {}
        }
    }
    history.push(ChatMessage::user(new_message));

    let trimmed = trim_history_to_budget(&history, sys_tokens);
    messages.extend(trimmed);

    let total_tokens: usize = messages.iter().map(message_tokens).sum();
    log::info!("WeChat reply context: {} messages, ~{} tokens", messages.len(), total_tokens);

    let req = ChatRequest {
        messages,
        temperature,
        max_tokens: 4096,
        ..Default::default()
    };

    // 尝试角色的偏好模型，fallback 到全局活跃模型
    let persona_provider = persona
        .llm
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 先尝试角色指定的 provider，再回退全局活跃，最后任选已注册 provider
    let mut provider = if persona_provider.is_empty() {
        None
    } else {
        scheduler.get(persona_provider).await
    };
    if provider.is_none() {
        provider = scheduler.active().await;
    }
    if provider.is_none() {
        let ids = scheduler.list().await;
        for id in &ids {
            provider = scheduler.get(id).await;
            if provider.is_some() {
                log::info!("reply_via_llm_stream: 未指定/未找到偏好模型，回退到 {id}");
                break;
            }
        }
    }

    if let Some(provider) = provider {
        log::info!(
            "reply_via_llm_stream: 模型 {:?} (角色偏好={}) 温度={}",
            provider.id(),
            persona_provider,
            temperature
        );
        let mut full = String::new();
        let mut stream = provider.chat_stream(req);
        while let Some(chunk) = stream.next().await {
            if !chunk.content.is_empty() {
                full.push_str(&chunk.content);
                on_chunk(&chunk.content);
            }
        }
        if !full.is_empty() {
            if full.contains("正在思考中，请稍后重试") {
                return Err("LLM 服务暂不可用，请检查 Ollama 是否启动".to_string());
            }
            log::info!("reply_via_llm_stream: 回复 {} 字符", full.len());
            return Ok(full);
        }
        log::warn!("reply_via_llm_stream: LLM 返回空内容");
        return Err("LLM 返回空内容，请检查模型服务".to_string());
    } else {
        log::warn!("reply_via_llm_stream: 无活跃 LLM provider");
    }

    Err(format!(
        "模型服务暂不可用（{}），请检查 Ollama 是否已启动并拉取模型",
        if persona_provider.is_empty() { "默认模型".to_string() } else { persona_provider.to_string() }
    ))
}
