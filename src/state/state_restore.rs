use super::{
    App, Arc, InactiveSession, InferenceEngine, Message, TABBY_API_DEFAULT_PORT, Task,
    default_tabbyapi_runtime_dir, find_tabbyapi_launcher, keystore, mcp, persisted_to_block,
    session,
};

impl App {
    pub(super) fn auto_attach_tabbyapi(&mut self) {
        let should = self.openai_compat_label.eq_ignore_ascii_case("TabbyAPI")
            || self.tabby_url_input.contains(":5000");
        if should
            && self.inference_binary_path.trim().is_empty()
            && let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir())
        {
            self.inference_engine = InferenceEngine::TabbyApi;
            self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
            if self.tabby_url_input.trim().is_empty() {
                self.tabby_url_input = format!("http://localhost:{TABBY_API_DEFAULT_PORT}");
            }
            self.inference_binary_path = launcher.display().to_string();
            self.try_persist(
                keystore::write_inference_binary(&self.inference_binary_path),
                "Inference 바이너리 저장",
            );
        }
    }

    pub(super) fn restore_sessions(&mut self) -> Option<iced::widget::scrollable::AbsoluteOffset> {
        let loaded = session::load_all_with_notice();
        let marker = session::was_clean_shutdown();
        self.apply_restored_sessions(loaded, marker)
    }

    #[cfg(test)]
    fn restore_sessions_at(
        &mut self,
        storage_dir: &std::path::Path,
        marker_dir: &std::path::Path,
    ) -> Option<iced::widget::scrollable::AbsoluteOffset> {
        let loaded = session::load_all_with_notice_at(Some(storage_dir));
        let marker = session::was_clean_shutdown_at(marker_dir);
        self.apply_restored_sessions(loaded, marker)
    }

    fn apply_restored_sessions(
        &mut self,
        loaded: (
            session::PersistedAllSessions,
            Option<session::SessionLoadNotice>,
        ),
        marker: Result<bool, String>,
    ) -> Option<iced::widget::scrollable::AbsoluteOffset> {
        let (persisted, load_notice) = loaded;
        let active_idx = persisted
            .active_idx
            .min(persisted.sessions.len().saturating_sub(1));
        let active = persisted.sessions[active_idx].clone();
        let mut inactive: Vec<InactiveSession> =
            Vec::with_capacity(persisted.sessions.len().saturating_sub(1));
        for s in persisted
            .sessions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != active_idx)
            .map(|(_, s)| s)
        {
            inactive.push(InactiveSession {
                id: s.id,
                title: s.title.clone(),
                conversation: Arc::clone(&s.conversation),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            });
        }

        self.current_session_id = active.id;
        self.current_session_title = active.title;
        self.conversation = Arc::clone(&active.conversation);
        self.next_block_id = active.next_block_id;
        self.blocks = active.blocks.into_iter().map(persisted_to_block).collect();
        self.current_scroll_y = active.scroll_y;
        self.inactive_sessions = inactive;
        self.next_session_id = persisted.sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        let marker_applies = match load_notice {
            Some(session::SessionLoadNotice::BackupFallback { primary, backup }) => {
                self.status = format!(
                    "[세션 복구] {}을(를) 읽을 수 없어 {}에서 복구했습니다",
                    primary.display(),
                    backup.display()
                );
                true
            }
            Some(session::SessionLoadNotice::LegacyMigration) => false,
            None => true,
        };
        match marker {
            Ok(true) => {}
            Ok(false) => {
                if marker_applies && !self.blocks.is_empty() {
                    self.status = format!("[비정상 종료 복구] {}", self.status);
                }
            }
            Err(error) => {
                self.status = format!("{} | 복구 상태 확인 실패: {error}", self.status);
            }
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
        &mut self,
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
            let generation = self.next_mcp_tool_load_generation(&name);
            tasks.push(Task::perform(
                async move {
                    mcp::list_tools(&server)
                        .await
                        .map(|tools| (generation, name.clone(), tools))
                        .map_err(|e| (generation, name.clone(), format!("[{name}] {e}")))
                },
                |r| match r {
                    Ok((generation, server_name, tools)) => Message::McpToolsLoaded {
                        generation,
                        server_name,
                        tools,
                    },
                    Err((generation, server_name, message)) => Message::McpToolsFailed {
                        generation,
                        server_name,
                        message,
                    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn persisted(title: &str) -> session::PersistedAllSessions {
        session::PersistedAllSessions {
            sessions: vec![session::PersistedSessionData {
                id: 7,
                title: title.into(),
                conversation: Arc::new(Vec::new()),
                blocks: Vec::new(),
                next_block_id: 0,
                scroll_y: 0.0,
            }],
            active_idx: 0,
        }
    }

    #[test]
    fn backup_fallback_is_exposed_in_app_status_with_injected_roots() {
        // Given
        let storage = TempDir::new().unwrap();
        let marker = TempDir::new().unwrap();
        std::fs::write(storage.path().join("sessions.json"), b"{corrupt").unwrap();
        std::fs::write(
            storage.path().join("sessions.json.bak"),
            serde_json::to_vec(&persisted("backup session")).unwrap(),
        )
        .unwrap();
        let (mut app, _) = App::new();

        // When
        let _ = app.restore_sessions_at(storage.path(), marker.path());

        // Then
        assert!(app.status.contains("[세션 복구]"), "status: {}", app.status);
        assert!(
            app.status.contains("sessions.json.bak"),
            "status: {}",
            app.status
        );
    }

    #[test]
    fn marker_read_failure_is_exposed_in_app_status_with_injected_roots() {
        // Given
        let storage = TempDir::new().unwrap();
        let marker_root = TempDir::new().unwrap();
        std::fs::create_dir(marker_root.path().join(".clean_shutdown")).unwrap();
        let (mut app, _) = App::new();

        // When
        let _ = app.restore_sessions_at(storage.path(), marker_root.path());

        // Then
        assert!(
            app.status.contains("복구 상태 확인 실패"),
            "status: {}",
            app.status
        );
        assert!(
            app.status
                .contains(&marker_root.path().display().to_string()),
            "status: {}",
            app.status
        );
    }
}
