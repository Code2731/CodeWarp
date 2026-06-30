use super::super::ui::{
    FS_BODY, PAD_XS, SCROLL_GUTTER_PAD_X, app_vscrollbar, dark_scrollable, secondary_btn,
};
use crate::util::file_tree::FileTreeItem;
use crate::{App, Message};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};
use std::collections::HashSet;
use std::path::PathBuf;

impl App {
    pub(super) fn view_file_tree(&self) -> Element<'static, Message> {
        let visible = visible_items(&self.file_tree_items, &self.file_tree_expanded);
        let mut col = column![].spacing(0);
        for item in &visible {
            let indent = item.depth as f32 * 16.0;
            let prefix = if item.is_dir {
                if self.file_tree_expanded.contains(&item.path) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            let label = format!("{}{}", prefix, item.name);
            let btn = button(text(label).size(FS_BODY))
                .width(Length::Fill)
                .padding([PAD_XS, 6])
                .style(secondary_btn);
            let btn = if item.is_dir {
                btn.on_press(Message::FileTreeToggle(item.path.clone()))
            } else {
                btn
            };
            let row = container(row![Space::new().width(Length::Fixed(indent)), btn,].spacing(0))
                .padding([0, SCROLL_GUTTER_PAD_X]);
            col = col.push(row);
        }
        if visible.is_empty() {
            col = col
                .push(container(text("(empty)").size(FS_BODY)).padding([0, SCROLL_GUTTER_PAD_X]));
        }
        scrollable(col)
            .direction(Direction::Vertical(app_vscrollbar()))
            .style(dark_scrollable)
            .height(Length::Fixed(200.0))
            .into()
    }
}

fn visible_items<'a>(
    items: &'a [FileTreeItem],
    expanded: &HashSet<PathBuf>,
) -> Vec<&'a FileTreeItem> {
    let mut result = Vec::new();
    for item in items {
        if item.depth == 0 {
            result.push(item);
        } else if let Some(parent) = item.path.parent()
            && expanded.contains(parent)
        {
            result.push(item);
        }
    }
    result
}
