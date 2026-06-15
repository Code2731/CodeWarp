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
}
