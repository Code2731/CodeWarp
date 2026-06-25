use crate::view::ui::{SCROLL_GUTTER_PAD_X, app_vscrollbar};
use crate::{App, Message, last_assistant_block_idx, last_user_block_idx};
use iced::widget::scrollable::Direction;
use iced::widget::{column, container, scrollable};
use iced::{Element, Length};

impl App {
    pub(crate) fn view_blocks(&self) -> Element<'_, Message> {
        if self.blocks.is_empty() {
            Self::view_empty_chat()
        } else {
            let last_user_idx = last_user_block_idx(&self.blocks);
            let last_asst_idx = last_assistant_block_idx(&self.blocks);
            let streaming = self.streaming_block_id.is_some();
            let mut col = column![].spacing(10).width(Length::Fill);
            for (i, b) in self.blocks.iter().enumerate() {
                col = col.push(self.view_block_item(b, i, last_user_idx, last_asst_idx, streaming));
            }
            scrollable(container(col).padding([0, SCROLL_GUTTER_PAD_X]))
                .id(self.stream_id.clone())
                .on_scroll(Message::StreamScrolled)
                .direction(Direction::Vertical(app_vscrollbar()))
                .height(Length::Fill)
                .into()
        }
    }
}
