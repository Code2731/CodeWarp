use super::{App, MAX_ATTACH_BYTES, Message, fmt_bytes, fuzzy_match_paths};
use iced::Task;

async fn read_and_validate_attachment(
    read_path: std::path::PathBuf,
    display_path: std::path::PathBuf,
    existing_total: u64,
) -> Result<(std::path::PathBuf, String), String> {
    let content = tokio::fs::read_to_string(&read_path)
        .await
        .map_err(|e| format!("File read failed: {e}"))?;
    #[allow(clippy::cast_possible_truncation)]
    if content.len() > MAX_ATTACH_BYTES as usize {
        return Err(format!(
            "Attachment too large (max {}): {}",
            fmt_bytes(MAX_ATTACH_BYTES),
            display_path.display()
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
    Ok((display_path, content))
}

fn attach_result_mapper(r: Result<(std::path::PathBuf, String), String>) -> Message {
    match r {
        Ok((p, s)) => Message::FileReadDone(p, s),
        Err(msg) => Message::FileAttachError(msg),
    }
}

impl App {
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
        let read_path = self.cwd.join(&chosen);
        let display_path = chosen;
        let existing_total = self.total_attached_bytes();
        Task::perform(
            read_and_validate_attachment(read_path, display_path, existing_total),
            attach_result_mapper,
        )
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
            read_and_validate_attachment(path.clone(), path, existing_total),
            attach_result_mapper,
        )
    }
    pub(crate) fn on_file_dropped(&mut self, path: std::path::PathBuf) -> Task<Message> {
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            read_and_validate_attachment(path.clone(), path, existing_total),
            attach_result_mapper,
        )
    }
}
