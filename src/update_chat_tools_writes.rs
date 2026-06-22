// update_chat_tools_writes.rs — Tool write-approval methods (main.rs child module)
use super::{
    summarize_tool_result, tools, App, Arc, Block, BlockBody, ChatMessage, Message, ViewMode,
    MAX_TOOL_ROUNDS,
};
use iced::Task;

impl App {
    pub(crate) fn approve_pending_writes(&mut self) -> Task<Message> {
        self.ui.expanded_confirm_idx = None;
        self.continue_after_writes(true)
    }
    pub(crate) fn deny_pending_writes(&mut self) -> Task<Message> {
        self.ui.expanded_confirm_idx = None;
        self.continue_after_writes(false)
    }
    pub(crate) fn discard_write_call(&mut self, idx: usize) -> Task<Message> {
        if idx >= self.pending_write_calls.len() {
            return Task::none();
        }
        let tc = self.pending_write_calls.remove(idx);
        self.push_tool_result_block(tc.name.clone(), "discarded".into(), false);
        Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(
            &tc.id,
            "[denied] 사용자가 이 도구 호출을 제외했습니다.",
        ));
        self.ui.expanded_confirm_idx = match self.ui.expanded_confirm_idx {
            Some(e) if e == idx => None,
            Some(e) if e > idx => Some(e - 1),
            other => other,
        };
        if self.pending_write_calls.is_empty() {
            return self.continue_after_writes(true);
        }
        Task::none()
    }
    pub(crate) fn push_tool_result_block(&mut self, name: String, summary: String, success: bool) {
        let id = self.next_id();
        self.blocks.push(Block {
            id,
            body: BlockBody::ToolResult {
                name,
                summary,
                success,
            },
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
    }
    pub(crate) fn continue_after_writes(&mut self, approved: bool) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_write_calls);
        self.show_write_confirm = false;

        if approved {
            let mut names: Vec<String> = Vec::new();
            for tc in &calls {
                names.push(tc.name.clone());
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                let (summary, success) = summarize_tool_result(&tc.name, &tc.arguments, &result);
                self.push_tool_result_block(tc.name.clone(), summary, success);
                Arc::make_mut(&mut self.conversation)
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            self.status = format!("실행 완료: {}", names.join(", "));
        } else {
            for tc in &calls {
                self.push_tool_result_block(tc.name.clone(), "denied".into(), false);
                Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 파일 쓰기를 거부했습니다.",
                ));
            }
            self.status = "사용자가 파일 쓰기를 거부했습니다".into();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }
    pub(crate) fn apply_change(&mut self, block_id: u64, idx: usize) -> Task<Message> {
        let snapshot = self
            .blocks
            .iter()
            .find(|b| b.id == block_id)
            .and_then(|b| b.apply_candidates.get(idx))
            .filter(|(_, applied)| !*applied)
            .map(|(c, _)| (c.path.clone(), c.content.clone()));
        let Some((path, content)) = snapshot else {
            return Task::none();
        };
        let args_json = serde_json::json!({
            "path": path,
            "content": content,
        })
        .to_string();
        let result = tools::dispatch("write_file", &args_json, &self.cwd);
        let success = !result.contains("[error]");
        if success {
            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id) {
                if let Some((_, applied)) = b.apply_candidates.get_mut(idx) {
                    *applied = true;
                }
            }
        }
        let summary = if success {
            format!("{path} ({} bytes)", content.len())
        } else {
            format!("실패: {path}")
        };
        self.push_tool_result_block("apply".into(), summary, success);
        self.status = if success {
            format!("적용됨: {path}")
        } else {
            result
        };
        Task::none()
    }
}
