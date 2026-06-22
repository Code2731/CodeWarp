// update_chat_session.rs — Session management update methods (main.rs child module)
use super::{persisted_to_block, session, App, Arc, BlockBody, InactiveSession, Message};
use iced::Task;

impl App {
    pub(crate) fn new_chat(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.snapshot_current_to_inactive();
        self.blocks.clear();
        Arc::make_mut(&mut self.conversation).clear();
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.show_write_confirm = false;
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.tool_round = 0;
        self.next_block_id = 0;
        self.input.clear();
        self.ui.pending_delete_session = None;
        self.current_session_id = self.allocate_session_id();
        self.current_session_title = "새 채팅".into();
        self.status = "새 채팅".into();
        self.save_session();
        Task::none()
    }
    pub(crate) fn delete_session(&mut self, target_id: u64) -> Task<Message> {
        self.ui.pending_delete_session = None;
        if target_id == self.current_session_id {
            self.blocks.clear();
            Arc::make_mut(&mut self.conversation).clear();
            self.next_block_id = 0;
            self.current_session_id = self.allocate_session_id();
            self.current_session_title = "새 채팅".into();
        } else {
            self.inactive_sessions.retain(|s| s.id != target_id);
        }
        self.save_session();
        Task::none()
    }
    pub(crate) fn switch_session(&mut self, target_id: u64) -> Task<Message> {
        if target_id == self.current_session_id {
            return Task::none();
        }
        let Some(idx) = self
            .inactive_sessions
            .iter()
            .position(|s| s.id == target_id)
        else {
            return Task::none();
        };
        self.abort_active_chat_stream(true);
        self.snapshot_current_to_inactive();
        let target = self.inactive_sessions.remove(idx);
        self.current_session_id = target.id;
        self.current_session_title = target.title;
        self.conversation = target.conversation;
        self.next_block_id = target.next_block_id;
        self.blocks = target.blocks.into_iter().map(persisted_to_block).collect();
        self.current_scroll_y = target.scroll_y;
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.show_write_confirm = false;
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.tool_round = 0;
        self.input.clear();
        self.ui.pending_delete_session = None;
        self.status = "세션 전환됨".into();
        self.save_session();
        iced::widget::operation::scroll_to(
            self.stream_id.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: target.scroll_y,
            },
        )
    }
    pub(crate) fn snapshot_current_to_inactive(&mut self) {
        if self.conversation.is_empty() && self.blocks.is_empty() {
            return; // 빈 세션은 보관 X
        }
        let sid = self.streaming_block_id;
        let raw = &self.streaming_raw;
        let blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .filter_map(|b| {
                let content = if sid == Some(b.id) {
                    raw.clone()
                } else {
                    b.body.to_text()
                };
                match &b.body {
                    BlockBody::User(_) | BlockBody::Assistant(_) => Some(session::PersistedBlock {
                        id: b.id,
                        role: if matches!(&b.body, BlockBody::User(_)) {
                            "user".into()
                        } else {
                            "assistant".into()
                        },
                        content,
                        model: b.model.clone().unwrap_or_default(),
                    }),
                    BlockBody::ToolResult { .. } => None,
                }
            })
            .collect();
        let snap = InactiveSession {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: self.conversation.clone(),
            blocks: blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        };
        if let Some(idx) = self.inactive_sessions.iter().position(|s| s.id == snap.id) {
            self.inactive_sessions[idx] = snap;
        } else {
            self.inactive_sessions.push(snap);
        }
    }
    pub(crate) fn save_session(&self) {
        let sid = self.streaming_block_id;
        let raw = &self.streaming_raw;
        let current_blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .filter_map(|b| {
                let content = if sid == Some(b.id) {
                    raw.clone()
                } else {
                    b.body.to_text()
                };
                match &b.body {
                    BlockBody::User(_) | BlockBody::Assistant(_) => Some(session::PersistedBlock {
                        id: b.id,
                        role: if matches!(&b.body, BlockBody::User(_)) {
                            "user".into()
                        } else {
                            "assistant".into()
                        },
                        content,
                        model: b.model.clone().unwrap_or_default(),
                    }),
                    BlockBody::ToolResult { .. } => None,
                }
            })
            .collect();

        let mut sessions: Vec<session::PersistedSessionData> = self
            .inactive_sessions
            .iter()
            .map(|s| session::PersistedSessionData {
                id: s.id,
                title: s.title.clone(),
                conversation: (*s.conversation).clone(),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();
        sessions.push(session::PersistedSessionData {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: (*self.conversation).clone(),
            blocks: current_blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        });

        let active_idx = sessions
            .iter()
            .position(|s| s.id == self.current_session_id)
            .unwrap_or(sessions.len() - 1);

        let p = session::PersistedAllSessions {
            sessions,
            active_idx,
        };
        let _ = session::save_all(&p);
    }
    pub(crate) fn maybe_update_title(&mut self) {
        if self.current_session_title.is_empty()
            || self.current_session_title.starts_with("새 채팅")
        {
            if let Some(first_user) = self
                .conversation
                .iter()
                .find(|m| m.role == "user")
                .and_then(|m| m.content.as_ref())
            {
                let snippet: String = first_user.chars().take(30).collect();
                self.current_session_title = snippet;
            }
        }
    }
    pub(crate) fn allocate_session_id(&mut self) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }
}
