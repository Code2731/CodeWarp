// update_chat_send — Send/input update methods (main.rs child module)
use super::{
    AgentMode, App, Block, BlockBody, ChatMessage, MAX_ATTACH_BYTES, Message, ViewMode,
    build_file_context, collect_mention_candidates, extract_mention_query, fmt_bytes,
    last_user_block_idx, openrouter, snap_to_end, truncate_after_last_user,
};
use iced::Task;

mod send;
#[cfg(test)]
mod tests;

impl App {
    pub(crate) fn on_file_read_done(
        &mut self,
        path: std::path::PathBuf,
        content: String,
    ) -> Task<Message> {
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
        } else {
            self.attached_files.push((path, content));
            let current_total = self.total_attached_bytes();
            self.status = format!(
                "Attached ({} files): {}/{}",
                self.attached_files.len(),
                fmt_bytes(current_total),
                fmt_bytes(MAX_ATTACH_BYTES)
            );
        }
        Task::none()
    }
    pub(crate) fn on_input_changed(&mut self, value: String) -> Task<Message> {
        self.input = value;
        match extract_mention_query(&self.input) {
            Some(q) => {
                self.mention_query = q.to_string();
                self.mention_selected = 0;
                if !self.show_mention {
                    self.show_mention = true;
                    let cwd = self.cwd.clone();
                    return Task::perform(
                        collect_mention_candidates(cwd),
                        Message::MentionCandidatesLoaded,
                    );
                }
            }
            None => {
                if self.show_mention {
                    self.close_mention();
                }
            }
        }
        Task::none()
    }
    pub(crate) fn edit_last_user(&mut self) -> Task<Message> {
        if self.streaming_block_id.is_some() {
            return Task::none();
        }
        let Some(idx) = last_user_block_idx(&self.blocks) else {
            return Task::none();
        };
        let user_text = match &self.blocks[idx].body {
            BlockBody::User(s) => s.clone(),
            _ => return Task::none(),
        };
        self.blocks.truncate(idx);
        truncate_after_last_user(std::sync::Arc::make_mut(&mut self.conversation));
        std::sync::Arc::make_mut(&mut self.conversation).pop();
        self.tool_round = 0;
        self.pending_tool_calls.clear();
        self.input = user_text;
        self.status = "편집 모드 — 수정 후 Enter".into();
        Task::none()
    }
}
