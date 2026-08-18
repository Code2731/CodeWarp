// update_chat_stream_done.rs — Chat stream Done/Error handlers (main.rs child module)
use super::{
    App, Arc, BlockBody, ChatMessage, LlmProvider, MAX_MID_STREAM_RETRIES, MAX_TOOL_ROUNDS,
    Message, keystore, markdown, openrouter, parse_apply_candidates, tabby,
};
use iced::Task;
use iced::widget::text_editor;

impl App {
    pub(crate) fn handle_chat_done(
        &mut self,
        ai_id: u64,
        finish_reason: Option<&String>,
        generation_id: Option<String>,
    ) -> Task<Message> {
        let has_tools = !self.pending_tool_calls.is_empty()
            && (finish_reason.map(String::as_str) == Some("tool_calls") || finish_reason.is_none());

        if has_tools && self.tool_round < MAX_TOOL_ROUNDS {
            let assistant_text = self.streaming_raw.clone();
            return self.run_tool_round(assistant_text);
        }

        if self.tool_round >= MAX_TOOL_ROUNDS && !self.pending_tool_calls.is_empty() {
            self.status = format!("최대 도구 라운드 {MAX_TOOL_ROUNDS} 초과");
        } else {
            if let Some(started) = self.response_started_at.take() {
                self.last_response_time_ms = Some(started.elapsed().as_millis() as u64);
            }
            self.status = "준비됨".into();
        }

        let final_text = std::mem::take(&mut self.streaming_raw);
        if let Some(idx) = self.streaming_block_idx
            && idx < self.blocks.len()
            && self.blocks[idx].id == ai_id
            && let BlockBody::Assistant(content) = &mut self.blocks[idx].body
        {
            *content = text_editor::Content::with_text(&final_text);
            if !final_text.is_empty() {
                self.blocks[idx].md_items = markdown::parse(&final_text).collect();
            }
        }

        if final_text.is_empty() {
            self.status =
                "[WARN] 모델이 빈 응답을 반환했습니다. Provider/Runtime 로그를 확인해 주세요."
                    .into();
            self.toast = Some(self.status.clone());
            if let Some(idx) = self.streaming_block_idx
                && idx < self.blocks.len()
                && self.blocks[idx].id == ai_id
                && let BlockBody::Assistant(content) = &mut self.blocks[idx].body
                && content.text().trim().is_empty()
            {
                *content = text_editor::Content::with_text("[WARN] empty response");
            }
        } else {
            Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(final_text.clone()));
        }

        let candidates = parse_apply_candidates(&final_text);
        if !candidates.is_empty()
            && let Some(idx) = self.streaming_block_idx
            && idx < self.blocks.len()
            && self.blocks[idx].id == ai_id
        {
            self.blocks[idx].apply_candidates =
                candidates.into_iter().map(|c| (c, false)).collect();
        }
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.streaming_raw.clear();
        self.abort_handle = None;
        self.pending_tool_calls.clear();
        self.maybe_update_title();
        self.save_session();
        if self.selected_provider() == Some(LlmProvider::OpenRouter)
            && let Some(id) = generation_id
            && let Ok(api_key) = keystore::read_api_key()
        {
            self.generation_lookup_generation = self.generation_lookup_generation.saturating_add(1);
            let lookup_generation = self.generation_lookup_generation;
            return Task::perform(openrouter::get_generation(api_key, id), move |result| {
                Message::GenerationLoaded {
                    generation: lookup_generation,
                    result,
                }
            });
        }
        Task::none()
    }

    pub(crate) fn handle_chat_error(&mut self, ai_id: u64, error: &str) -> Task<Message> {
        if !self.streaming_raw.is_empty()
            && self.mid_stream_retries < MAX_MID_STREAM_RETRIES
            && !contains_http_status(error, 401)
            && !contains_http_status(error, 402)
            && !contains_http_status(error, 403)
        {
            self.mid_stream_retries += 1;
            self.streaming_raw.clear();
            if let Some(idx) = self.streaming_block_idx
                && idx < self.blocks.len()
                && self.blocks[idx].id == ai_id
            {
                if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                    *content = text_editor::Content::new();
                }
                self.blocks[idx].md_items.clear();
            }
            self.pending_tool_calls.clear();
            self.status = format!(
                "재시도 중… ({}/{})",
                self.mid_stream_retries, MAX_MID_STREAM_RETRIES
            );
            return self.kick_chat_stream();
        }

        if let Some(idx) = self.streaming_block_idx
            && idx < self.blocks.len()
            && self.blocks[idx].id == ai_id
            && let BlockBody::Assistant(content) = &mut self.blocks[idx].body
        {
            let prefix = if self.streaming_raw.is_empty() {
                ""
            } else {
                "\n\n"
            };
            let final_text = std::mem::take(&mut self.streaming_raw);
            let full = format!("{final_text}{prefix}[ERROR] {error}");
            *content = text_editor::Content::with_text(&full);
            self.blocks[idx].md_items = markdown::parse(&full).collect();
        }
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.streaming_raw.clear();
        self.abort_handle = None;
        self.pending_tool_calls.clear();
        self.response_started_at = None;
        let is_local_provider = self.selected_provider() == Some(LlmProvider::OpenAICompat);
        let humanized = if is_local_provider {
            tabby::humanize_error(error)
        } else {
            openrouter::humanize_error(error)
        };
        let credentials_warning = contains_http_status(error, 401)
            || contains_http_status(error, 403)
            || (!is_local_provider && contains_http_status(error, 402));
        if credentials_warning {
            self.status = if is_local_provider {
                format!("[WARN] {humanized} | Settings에서 local endpoint/token 확인")
            } else {
                format!("[WARN] {humanized} | Open Settings and check API key / credits")
            };
        } else {
            self.status = format!("[ERROR] {humanized}");
        }
        self.toast = Some(self.status.clone());
        Task::none()
    }
}

fn contains_http_status(raw: &str, status: u16) -> bool {
    raw.split(|ch: char| !ch.is_ascii_digit())
        .any(|token| token == status.to_string())
}

#[cfg(test)]
mod tests;
