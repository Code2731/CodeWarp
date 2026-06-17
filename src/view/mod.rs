// view.rs — App 뷰 메서드 (main.rs child module)
use super::*;
mod chat;
mod chat_block;
mod chat_block_item;
mod chat_block_style;
mod chat_empty;
mod pty;
mod rightpanel;
mod settings;
mod settings_endpoint;
mod settings_health;
mod settings_health_panel;
mod settings_health_panel_badge;
mod settings_health_tab;
mod settings_mcp;
mod settings_models;
mod settings_models_exl2;
mod settings_models_presets;
mod settings_provider;
mod settings_runtime;
mod settings_runtime_actions;
mod settings_runtime_binary;
mod settings_runtime_model;
mod sidebar;
mod statusbar;
mod ui;
mod view_confirm;
mod view_confirm_full;
mod view_topbar;
mod view_viewer;

use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, stack, text, text_input, Space};
use iced::{Element, Font, Length, Theme};
pub(crate) use ui::*;
pub(crate) use view_viewer::CodewarpViewer;

/// 두 텍스트의 line-by-line diff를 색상 표시된 Element로 변환.
/// 추가 라인은 녹색, 삭제 라인은 빨강, 동일 라인은 흐리게.
fn render_diff<'a>(old: &str, new: &str) -> Element<'a, Message> {
    use similar::{ChangeTag, TextDiff};

    const MAX_LINES: usize = 400;
    let added = Color::from_rgb(0.55, 0.85, 0.55);
    let removed = Color::from_rgb(0.95, 0.45, 0.45);
    let equal = Color::from_rgb(0.5, 0.5, 0.55);

    let diff = TextDiff::from_lines(old, new);
    let mut col = column![].spacing(0);
    for (count, change) in diff.iter_all_changes().enumerate() {
        if count >= MAX_LINES {
            col = col.push(
                text(format!("…(diff 라인 {}+ 생략)", MAX_LINES))
                    .size(11)
                    .color(equal),
            );
            break;
        }
        let (sign, color) = match change.tag() {
            ChangeTag::Delete => ("-", removed),
            ChangeTag::Insert => ("+", added),
            ChangeTag::Equal => (" ", equal),
        };
        let raw = change.value().trim_end_matches('\n');
        let line_text = if raw.len() > 200 {
            format!("{} {}…", sign, &raw[..200])
        } else {
            format!("{} {}", sign, raw)
        };
        col = col.push(
            text(line_text)
                .size(11)
                .font(Font::with_name("JetBrains Mono"))
                .color(color),
        );
    }
    container(col).padding(10).width(Length::Fill).into()
}

/// 모달 오버레이: 반투명 백드롭 + 가운데 정렬된 콘텐츠 박스.
/// content는 view_settings/view_write_confirm 같은 기존 화면 함수의 결과.
fn modal_overlay<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    let modal_box = container(content)
        .padding(0)
        .width(Length::Shrink)
        .max_width(720.0)
        .max_height(720.0)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.base.color.into()),
                border: iced::Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        });

    container(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55).into()),
            ..Default::default()
        })
        .into()
}

impl App {
    pub(crate) fn view(&self) -> Element<'_, Message> {
        let topbar = self.view_topbar();

        let main_view: Element<Message> = row![
            self.view_sidebar(),
            container(self.view_stream())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(panel_style),
            self.view_rightpanel(),
        ]
        .spacing(MAIN_ROW_SPACING)
        .padding([MAIN_PAD_Y, MAIN_PAD_X])
        .height(Length::Fill)
        .into();

        // overlay가 필요하면 stack으로 메인 위에 띄움 (backdrop + 가운데 모달 박스)
        let middle: Element<Message> = if self.ui.show_command_palette {
            stack![main_view, modal_overlay(self.view_command_palette())].into()
        } else if self.ui.show_settings {
            stack![main_view, modal_overlay(self.view_settings())].into()
        } else {
            // write_confirm은 입력창 위 인라인 패널(view_stream 안에서 처리)
            main_view
        };

        let statusbar = self.view_statusbar();

        let mut col = column![topbar, middle];
        if self.pty_visible {
            col = col.push(self.view_pty_panel());
        }
        col.push(statusbar)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_command_palette(&self) -> Element<'_, Message> {
        let header = text("명령 팔레트").size(18).font(bold_font());
        let hint = column![
            text("탐색  Esc 닫기 · Ctrl+K 토글").size(FS_LABEL),
            text("작업  Ctrl+N 새 채팅 · Ctrl+, 설정").size(FS_LABEL),
            text("모드  Ctrl+Shift+P 계획 · Ctrl+Shift+B 빌드").size(FS_LABEL),
        ]
        .spacing(2);
        let input = text_input("명령 검색…", &self.ui.command_palette_input)
            .on_input(Message::CommandPaletteChanged)
            .on_submit(Message::ExecuteCommand(0))
            .padding(10)
            .size(FS_BODY)
            .style(field_input);

        let filtered = self.filtered_palette_commands();
        let mut list = column![].spacing(4);
        if filtered.is_empty() {
            list = list.push(text("(매칭 없음)").size(FS_BODY));
        } else {
            for (i, cmd) in filtered.iter().enumerate() {
                list = list.push(
                    button(
                        column![
                            text(cmd.label).size(FS_SUBTITLE).font(semibold_font()),
                            text(cmd.hint).size(FS_LABEL),
                        ]
                        .spacing(2),
                    )
                    .on_press(Message::ExecuteCommand(i))
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(secondary_btn),
                );
            }
        }

        let body = column![
            header,
            hint,
            Space::new().height(Length::Fixed(8.0)),
            input,
            Space::new().height(Length::Fixed(8.0)),
            scrollable(list)
                .direction(Direction::Vertical(app_vscrollbar(),))
                .height(Length::Fixed(320.0)),
            Space::new().height(Length::Fixed(8.0)),
            row![
                Space::new().width(Length::Fill),
                button(text("닫기").size(FS_BODY))
                    .on_press(Message::CloseCommandPalette)
                    .padding([4, 12])
                    .style(secondary_btn),
            ],
        ]
        .spacing(4);

        container(body)
            .padding(20)
            .width(Length::Fixed(560.0))
            .style(panel_style)
            .into()
    }
}
