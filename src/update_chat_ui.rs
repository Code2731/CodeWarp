// update_chat_ui.rs — Chat UI interaction update methods (main.rs child module)
use super::*;
use iced::{widget::markdown, Task};

impl App {
    pub(crate) fn on_stream_scrolled(
        &mut self,
        viewport: &iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        let rel = viewport.relative_offset();
        self.follow_bottom = rel.y > 0.95;
        self.current_scroll_y = viewport.absolute_offset().y;
        Task::none()
    }
    pub(crate) fn on_editor_action(
        &mut self,
        id: u64,
        action: iced::widget::text_editor::Action,
    ) -> Task<Message> {
        if action.is_edit() {
            return Task::none();
        }
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            if let BlockBody::Assistant(content) = &mut b.body {
                content.perform(action);
            }
        }
        Task::none()
    }
    pub(crate) fn toggle_block_view(&mut self, id: u64) -> Task<Message> {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            b.view_mode = match b.view_mode {
                ViewMode::Rendered => ViewMode::Raw,
                ViewMode::Raw => {
                    if let BlockBody::Assistant(content) = &b.body {
                        b.md_items = markdown::parse(&content.text()).collect();
                    }
                    ViewMode::Rendered
                }
            };
        }
        Task::none()
    }
    pub(crate) fn on_link_clicked(&mut self, uri: &markdown::Uri) -> Task<Message> {
        let url = uri.to_string();
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("javascript:") {
            self.status = "차단된 링크 스킴입니다.".into();
            return Task::none();
        }
        match webbrowser::open(&url) {
            Ok(_) => {
                self.status = format!("브라우저에서 열기: {}", url);
            }
            Err(e) => {
                self.status = format!("링크 열기 실패: {}", e);
            }
        }
        Task::none()
    }
    pub(crate) fn copy_block(&self, id: u64) -> Task<Message> {
        if self.streaming_block_id == Some(id) && !self.streaming_raw.is_empty() {
            return iced::clipboard::write(self.streaming_raw.clone());
        }
        if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
            return iced::clipboard::write(b.body.to_text());
        }
        Task::none()
    }
}
