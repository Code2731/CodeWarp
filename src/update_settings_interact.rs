// update_settings_interact.rs — PTY/attachment/mention interaction methods
use super::*;
use iced::Task;

impl App {
    pub(crate) fn toggle_pty(&mut self) -> Task<Message> {
        self.pty_visible = !self.pty_visible;
        if self.pty_visible && self.pty_session.is_none() {
            return Task::done(Message::PtyStart);
        }
        Task::none()
    }
    pub(crate) fn send_pty_input(&mut self) -> Task<Message> {
        let line = self.pty_input.trim_end().to_string();
        if let Some(s) = &self.pty_session {
            s.write_line(&line);
        }
        self.pty_input.clear();
        Task::none()
    }
    pub(crate) fn remove_attachment(&mut self, idx: usize) -> Task<Message> {
        if idx < self.attached_files.len() {
            let removed = self.attached_files.remove(idx);
            let removed_name = removed
                .0
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| removed.0.display().to_string());
            self.status = format!(
                "Removed attachment: {} ({} left)",
                removed_name,
                self.attached_files.len()
            );
        }
        Task::none()
    }
    pub(crate) fn clear_attachments(&mut self) -> Task<Message> {
        if !self.attached_files.is_empty() {
            let removed_count = self.attached_files.len();
            let removed_bytes: u64 = self
                .attached_files
                .iter()
                .map(|(_, content)| content.len() as u64)
                .sum();
            self.attached_files.clear();
            self.status = format!(
                "Cleared attachments: {} files ({})",
                removed_count,
                fmt_bytes(removed_bytes)
            );
        }
        Task::none()
    }
    pub(crate) fn move_mention_selection(&mut self, delta: i32) -> Task<Message> {
        if !self.show_mention || self.mention_candidates.is_empty() {
            return Task::none();
        }
        let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
        let n = filtered.len();
        if n == 0 {
            return Task::none();
        }
        self.mention_selected =
            (self.mention_selected as i64 + delta as i64).rem_euclid(n as i64) as usize;
        Task::none()
    }
    pub(crate) fn confirm_mention(&mut self) -> Task<Message> {
        if !self.show_mention {
            return Task::none();
        }
        let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
        let Some(chosen) = filtered.into_iter().nth(self.mention_selected) else {
            return Task::none();
        };
        if let Some(at_pos) = self.input.rfind('@') {
            self.input.truncate(at_pos);
        }
        self.close_mention();
        if self.is_already_attached(&chosen) {
            self.status = format!("Already attached: {}", chosen.display());
            return Task::none();
        }
        let full_path = self.cwd.join(&chosen);
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&full_path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        chosen.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((chosen, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }
    pub(crate) fn load_mention_candidates(
        &mut self,
        paths: Vec<std::path::PathBuf>,
    ) -> Task<Message> {
        self.mention_candidates = paths;
        Task::none()
    }
    pub(crate) fn close_mention(&mut self) {
        self.show_mention = false;
        self.mention_query.clear();
        self.mention_selected = 0;
    }
    pub(crate) fn normalized_attachment_path(&self, path: &std::path::Path) -> std::path::PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }
    pub(crate) fn is_already_attached(&self, path: &std::path::Path) -> bool {
        let needle = self.normalized_attachment_path(path);
        self.attached_files
            .iter()
            .any(|(p, _)| self.normalized_attachment_path(p) == needle)
    }
    pub(crate) fn total_attached_bytes(&self) -> u64 {
        self.attached_files
            .iter()
            .map(|(_, content)| content.len() as u64)
            .sum()
    }
    pub(crate) fn push_pty_line(&mut self, line: String) {
        self.pty_output.push_back(line);
        if self.pty_output.len() > PTY_MAX_LINES {
            self.pty_output.pop_front();
        }
    }
    pub(crate) fn pty_start(&mut self) -> Task<Message> {
        match pty::spawn_pty(&self.cwd) {
            Ok((session, stream)) => {
                self.pty_session = Some(session);
                self.pty_output.clear();
                self.status = "터미널 시작됨".into();
                Task::run(stream, |event| match event {
                    pty::PtyEvent::Line(l) => Message::PtyLine(l),
                    pty::PtyEvent::Exited => Message::PtyExited,
                })
            }
            Err(e) => {
                self.status = format!("터미널 시작 실패: {e}");
                Task::none()
            }
        }
    }
    pub(crate) fn on_pty_line(&mut self, line: String) -> Task<Message> {
        let clean = pty::strip_ansi(&line);
        if !clean.trim().is_empty() {
            self.push_pty_line(clean);
        }
        Task::none()
    }
    pub(crate) fn on_pty_exited(&mut self) -> Task<Message> {
        self.pty_session = None;
        self.push_pty_line("-- 셸 종료 --".into());
        self.status = "터미널 종료됨".into();
        Task::none()
    }
    pub(crate) fn pick_attachment(&self) -> Task<Message> {
        let cwd = self.cwd.clone();
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("첨부 파일 선택")
                    .set_directory(cwd)
                    .pick_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::AttachmentPicked,
        )
    }
    pub(crate) fn on_attachment_picked(
        &mut self,
        maybe_path: Option<std::path::PathBuf>,
    ) -> Task<Message> {
        let Some(path) = maybe_path else {
            return Task::none();
        };
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        path.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((path, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }
    pub(crate) fn on_file_dropped(&mut self, path: std::path::PathBuf) -> Task<Message> {
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        path.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((path, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }
}
