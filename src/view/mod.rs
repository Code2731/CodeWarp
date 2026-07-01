// view.rs — App 뷰 메서드 (main.rs child module)
use super::{App, Color, Message, hscrollbar};
mod chat;
mod pty;
mod rightpanel;
mod settings;
mod settings_view;
mod sidebar;
mod statusbar;
mod ui;
mod view_confirm;
mod view_palette;
mod view_topbar;
mod view_viewer;

use iced::widget::{button, column, container, row, stack, text};
use iced::{Alignment, Element, Font, Length, Padding, Shadow, Theme, Vector};
pub(crate) use ui::*;
use ui::{secondary_btn, toast_style};
use view_viewer::CodewarpViewer;

/// 두 텍스트의 line-by-line diff를 색상 표시된 Element로 변환.
/// 추가 라인은 녹색, 삭제 라인은 빨강, 동일 라인은 흐리게.
pub(crate) fn render_diff<'a>(old: &str, new: &str) -> Element<'a, Message> {
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
                text(format!("…(diff 라인 {MAX_LINES}+ 생략)"))
                    .size(FS_LABEL)
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
            format!("{sign} {}…", &raw[..200])
        } else {
            format!("{sign} {raw}")
        };
        col = col.push(
            text(line_text)
                .size(FS_LABEL)
                .font(Font::with_name("JetBrains Mono"))
                .color(color),
        );
    }
    container(col).padding(10).width(Length::Fill).into()
}

/// 모달 오버레이: 반투명 백드롭 + 가운데 정렬된 콘텐츠 박스.
/// `content`는 `view_settings`/`view_palette` 같은 기존 화면 함수의 결과.
fn modal_overlay(content: Element<'_, Message>) -> Element<'_, Message> {
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
                shadow: Shadow {
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                    offset: Vector { x: 0.0, y: 12.0 },
                    blur_radius: 32.0,
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
    fn right_panel_visible(&self) -> bool {
        self.window_width >= 1100.0
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let topbar = self.view_topbar();

        let mut main_row = row![
            self.view_sidebar(),
            container(self.view_stream())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(panel_style),
        ]
        .spacing(MAIN_ROW_SPACING)
        .height(Length::Fill);

        if self.right_panel_visible() {
            main_row = main_row.push(self.view_rightpanel());
        }

        let main_view: Element<Message> = container(main_row)
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
        let base = col.push(statusbar).width(Length::Fill).height(Length::Fill);

        if let Some(toast_text) = &self.toast {
            let toast = container(
                row![
                    text(toast_text).size(FS_LABEL),
                    button(text("✕").size(FS_MICRO))
                        .on_press(Message::DismissToast)
                        .padding([2, 6])
                        .style(secondary_btn),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .style(toast_style);
            let toast_layer = container(toast)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_bottom(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding::new(0.0).bottom(48.0));
            stack![base, toast_layer].into()
        } else {
            base.into()
        }
    }
}
