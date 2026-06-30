use crate::Message;
use crate::update_tldr::TldrFileEntry;
use iced::widget::{column, row, text};
use iced::{Color, Element};

const ADDED: Color = Color::from_rgba(0.29, 0.76, 0.49, 1.0);
const REMOVED: Color = Color::from_rgba(0.94, 0.36, 0.44, 1.0);
const NEW_FILE: &str = "+";
const MODIFIED: &str = "~";

pub(crate) fn view_tldr_summary<'a>(
    _block_id: u64,
    entries: &'a [TldrFileEntry],
) -> Element<'a, Message> {
    let mut col = column![].spacing(4);
    for entry in entries {
        let icon = if entry.is_new_file {
            NEW_FILE
        } else {
            MODIFIED
        };
        let proposed = entry.proposed_lines;
        let diff = proposed as isize - entry.existing_lines as isize;
        let stat = if entry.is_new_file {
            format!("+{proposed}")
        } else if diff >= 0 {
            format!("+{diff}")
        } else {
            format!("{diff}")
        };
        let stat_color = if entry.is_new_file || diff > 0 {
            ADDED
        } else if diff < 0 {
            REMOVED
        } else {
            Color::from_rgba(0.6, 0.6, 0.6, 1.0)
        };
        col = col.push(
            row![
                text(icon).size(12).color(stat_color),
                text(&entry.path)
                    .size(12)
                    .color(Color::from_rgba(0.8, 0.8, 0.8, 1.0)),
                text(stat).size(12).color(stat_color),
            ]
            .spacing(6),
        );
    }
    col.into()
}
