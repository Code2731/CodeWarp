use super::{
    default_tabbyapi_runtime_dir, find_tabbyapi_launcher, keystore, mcp, persisted_to_block,
    session, App, Arc, InactiveSession, InferenceEngine, Message, Task, TABBY_API_DEFAULT_PORT,
};

impl App {
    pub(super) fn auto_attach_tabbyapi(&mut self) {
        let should = self.openai_compat_label.eq_ignore_ascii_case("TabbyAPI")
            || self.tabby_url_input.contains(":5000");
        if should && self.inference_binary_path.trim().is_empty() {
            if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir()) {
                self.inference_engine = InferenceEngine::TabbyApi;
                self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                if self.tabby_url_input.trim().is_empty() {
                    self.tabby_url_input = format!("http://localhost:{TABBY_API_DEFAULT_PORT}");
                }
                self.inference_binary_path = launcher.display().to_string();
                let _ = keystore::write_inference_binary(&self.inference_binary_path);
            }
        }
    }

    pub(super) fn restore_sessions(&mut self) -> Option<iced::widget::scrollable::AbsoluteOffset> {
        let mut persisted = session::load_all();
        if persisted.sessions.is_empty() {
            persisted = session::load_all();
        }
        let active_idx = persisted
            .active_idx
            .min(persisted.sessions.len().saturating_sub(1));
        let active = persisted.sessions[active_idx].clone();
        let inactive: Vec<InactiveSession> = persisted
            .sessions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != active_idx)
            .map(|(_, s)| InactiveSession {
                id: s.id,
                title: s.title.clone(),
                conversation: Arc::new(s.conversation.clone()),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();

        self.current_session_id = active.id;
        self.current_session_title = active.title;
        self.conversation = Arc::new(active.conversation);
        self.next_block_id = active.next_block_id;
        self.blocks = active.blocks.into_iter().map(persisted_to_block).collect();
        self.current_scroll_y = active.scroll_y;
        self.inactive_sessions = inactive;
        self.next_session_id = persisted.sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        if !session::was_clean_shutdown() && !self.blocks.is_empty() {
            self.status = format!("[복구됨] {}", self.status);
        }

        if self.current_scroll_y > 0.0 {
            Some(iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: self.current_scroll_y,
            })
        } else {
            None
        }
    }

    pub(super) fn build_startup_tasks(
        &self,
        scroll_restore: Option<iced::widget::scrollable::AbsoluteOffset>,
    ) -> Task<Message> {
        let mut tasks: Vec<Task<Message>> = Vec::new();
        if self.has_key {
            tasks.push(Task::done(Message::FetchModels));
            tasks.push(Task::done(Message::FetchAccount));
        }
        if !self.tabby_url_input.trim().is_empty() {
            tasks.push(Task::done(Message::FetchTabbyModels));
        }
        if !self.inference_command_input.trim().is_empty() {
            tasks.push(Task::done(Message::StartInference));
        }
        for server in self.mcp_servers.clone() {
            let name = server.name.clone();
            tasks.push(Task::perform(
                async move {
                    mcp::list_tools(&server)
                        .await
                        .map(|tools| (name.clone(), tools))
                        .map_err(|e| format!("[{name}] {e}"))
                },
                |r| match r {
                    Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                    Err(msg) => Message::McpToolsFailed(msg),
                },
            ));
        }
        if let Some(off) = scroll_restore {
            tasks.push(iced::widget::operation::scroll_to(
                self.stream_id.clone(),
                off,
            ));
        }
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }
}
