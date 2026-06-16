// update_chat_compare.rs — Compare mode update methods (main.rs child module)
use super::*;
use futures_util::StreamExt;
use iced::widget::text_editor;
use iced::Task;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ChatRoute {
    pub(crate) label: String,
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
    pub(crate) model: String,
}

impl App {
    pub(crate) fn on_compare_responses_loaded(
        &mut self,
        openrouter_block_id: u64,
        tabby_block_id: u64,
        openrouter_result: Result<String, String>,
        tabby_result: Result<String, String>,
    ) -> Task<Message> {
        if !self.compare_pending {
            return Task::none();
        }
        let openrouter_text = match openrouter_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[OpenRouter] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", openrouter::humanize_error(&e)),
        };
        let tabby_text = match tabby_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[Tabby] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", tabby::humanize_error(&e)),
        };
        self.fill_assistant_block(openrouter_block_id, openrouter_text.clone());
        self.fill_assistant_block(tabby_block_id, tabby_text.clone());
        Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(format!(
            "[OpenRouter]\n{}\n\n[Tabby]\n{}",
            openrouter_text, tabby_text
        )));
        self.compare_pending = false;
        self.status = "Compare 응답 완료".into();
        self.maybe_update_title();
        self.save_session();
        if self.follow_bottom {
            snap_to_end(self.stream_id.clone())
        } else {
            Task::none()
        }
    }
    pub(crate) fn compare_send_message(&mut self, text: String) -> Task<Message> {
        let (openrouter_route, tabby_route) = match self.compare_routes() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };

        self.ensure_system_message();
        let user_msg = if !self.attached_files.is_empty() {
            let ctx = build_file_context(&self.attached_files);
            format!("{ctx}\n\n{text}")
        } else {
            text.clone()
        };
        Arc::make_mut(&mut self.conversation).push(ChatMessage::user(user_msg));
        self.attached_files.clear();
        self.close_mention();
        self.pending_tool_calls.clear();
        self.tool_round = 0;
        self.mid_stream_retries = 0;
        let messages = self.conversation.clone();

        let user_id = self.next_id();
        self.blocks.push(Block {
            id: user_id,
            body: BlockBody::User(text),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        let openrouter_block_id = self.next_id();
        self.blocks.push(Block {
            id: openrouter_block_id,
            body: BlockBody::Assistant(text_editor::Content::with_text("OpenRouter 응답 대기 중…")),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: Some(format!(
                "{}: {}",
                openrouter_route.label, openrouter_route.model
            )),
            apply_candidates: Vec::new(),
        });
        let tabby_block_id = self.next_id();
        self.blocks.push(Block {
            id: tabby_block_id,
            body: BlockBody::Assistant(text_editor::Content::with_text("Tabby 응답 대기 중…")),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: Some(format!("{}: {}", tabby_route.label, tabby_route.model)),
            apply_candidates: Vec::new(),
        });

        self.input.clear();
        self.compare_pending = true;
        self.status = "Compare 응답 생성 중…".into();
        self.follow_bottom = true;

        let openrouter_messages = messages.clone();
        let tabby_messages = messages;
        let task = Task::perform(
            async move {
                let openrouter = collect_chat_text(
                    openrouter_route.base_url,
                    openrouter_route.api_key,
                    openrouter_route.model,
                    openrouter_messages,
                );
                let tabby = collect_chat_text(
                    tabby_route.base_url,
                    tabby_route.api_key,
                    tabby_route.model,
                    tabby_messages,
                );
                tokio::join!(openrouter, tabby)
            },
            move |(openrouter_result, tabby_result)| Message::CompareResponsesLoaded {
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            },
        );
        Task::batch(vec![snap_to_end(self.stream_id.clone()), task])
    }

    pub(crate) fn compare_routes(&self) -> Result<(ChatRoute, ChatRoute), String> {
        let selected = self.selected_option();
        let openrouter_model = selected
            .filter(|o| o.provider == LlmProvider::OpenRouter)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenRouter)
            })
            .ok_or_else(|| "Compare 모드: OpenRouter 모델이 없습니다. OpenRouter 키/모델 목록을 먼저 불러와 주세요.".to_string())?;
        let tabby_model = selected
            .filter(|o| o.provider == LlmProvider::OpenAICompat)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenAICompat)
            })
            .ok_or_else(|| "Compare 모드: Tabby 모델이 없습니다. Provider 연결 테스트로 Tabby 모델을 먼저 불러와 주세요.".to_string())?;

        let openrouter_key = keystore::read_api_key()?;
        let tabby_base = if self.tabby_url_input.trim().is_empty() {
            keystore::read_tabby_base_url()
        } else {
            Some(self.tabby_url_input.clone())
        }
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Compare 모드: Tabby URL 미설정".to_string())?;
        let tabby_token = if self.tabby_token_input.trim().is_empty() {
            keystore::read_tabby_token()
        } else {
            Some(self.tabby_token_input.clone())
        }
        .filter(|s| !s.trim().is_empty());

        Ok((
            ChatRoute {
                label: "OpenRouter".into(),
                base_url: openrouter::BASE_URL.to_string(),
                api_key: Some(openrouter_key),
                model: openrouter_model.id.clone(),
            },
            ChatRoute {
                label: if tabby_model.provider_label.trim().is_empty() {
                    "Local".into()
                } else {
                    tabby_model.provider_label.trim().to_string()
                },
                base_url: tabby::chat_base(&tabby_base),
                api_key: tabby_token,
                model: tabby_model.id.clone(),
            },
        ))
    }
}

pub(crate) async fn collect_chat_text(
    base_url: String,
    api_key: Option<String>,
    model: String,
    messages: Arc<Vec<ChatMessage>>,
) -> Result<String, String> {
    let stream = openrouter::chat_stream(base_url, api_key, model, messages, None);
    futures_util::pin_mut!(stream);
    let mut out = String::new();
    while let Some(event) = stream.next().await {
        match event {
            ChatEvent::Token(t) => out.push_str(&t),
            ChatEvent::Done { .. } => return Ok(out),
            ChatEvent::Error(e) => return Err(e),
            ChatEvent::ToolCallDelta { .. } => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_mode_send_requires_registered_providers() {
        let (mut app, _) = App::new();
        app.compare_both = true;
        app.input = "compare this".into();
        app.selected_model = None;
        app.model_options.clear();
        let before_blocks = app.blocks.len();

        let _ = app.update(Message::Send);

        assert!(
            app.status.contains("Compare 모드: OpenRouter 모델"),
            "got: {}",
            app.status
        );
        assert_eq!(app.blocks.len(), before_blocks);
    }
}
