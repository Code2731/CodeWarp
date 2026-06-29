use super::ui::{
    FS_BODY, FS_LABEL, FS_SUBTITLE, SCROLL_GUTTER_PAD_X, app_vscrollbar, danger_btn,
    dark_scrollable, field_input, panel_style, primary_btn, secondary_btn, semibold_font,
};
use crate::{App, Message};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Font, Length};

impl App {
    pub(super) fn view_pty_panel(&self) -> Element<'_, Message> {
        // 헤더 행: 제목 + 버튼들
        let header = row![
            text("터미널").size(FS_SUBTITLE).font(semibold_font()),
            Space::new().width(Length::Fill),
            button(text("✕ Clear").size(FS_LABEL))
                .on_press(Message::PtyClear)
                .padding([2, 8])
                .style(secondary_btn),
            button(text("✕").size(FS_LABEL))
                .on_press(Message::PtyToggle)
                .padding([2, 8])
                .style(secondary_btn),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([4, 8]);

        // 출력 영역 (최근 줄이 아래)
        let mut out_col = column![].spacing(0);
        for line in &self.pty_output {
            out_col = out_col.push(
                text(line)
                    .size(FS_BODY)
                    .font(Font::with_name("JetBrains Mono")),
            );
        }
        let output_area = scrollable(container(out_col).padding([0, SCROLL_GUTTER_PAD_X]))
            .direction(Direction::Vertical(app_vscrollbar()))
            .style(dark_scrollable)
            .height(Length::Fixed(200.0))
            .width(Length::Fill);

        // 입력 행
        let session_active = self.pty_session.is_some();
        let input_row = row![
            text_input(
                if session_active {
                    "> 명령 입력…"
                } else {
                    "터미널 종료됨 (Ctrl+` 로 재시작)"
                },
                &self.pty_input
            )
            .on_input(Message::PtyInputChanged)
            .on_submit(Message::PtySend)
            .padding(6)
            .size(FS_LABEL)
            .style(field_input)
            .font(Font::with_name("JetBrains Mono"))
            .width(Length::Fill),
            button(text("전송").size(FS_LABEL))
                .on_press_maybe(if session_active {
                    Some(Message::PtySend)
                } else {
                    None
                })
                .padding([6, 10])
                .style(primary_btn),
            button(text("^C").size(FS_LABEL))
                .on_press_maybe(if session_active {
                    Some(Message::PtyCtrlC)
                } else {
                    None
                })
                .padding([6, 8])
                .style(danger_btn),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(column![header, output_area, container(input_row).padding([4, 8])].spacing(0))
            .width(Length::Fill)
            .style(panel_style)
            .into()
    }
}
