use crate::view::ui::{
    FS_BODY, FS_LABEL, FS_SUBTITLE, FS_TITLE, LINE_HEIGHT_BODY, PAD_MD, PAD_XS, SPACE_SM, SPACE_XS,
    SPACE_XXS, bold_font, context_item_style, panel_style, secondary_btn, semibold_font,
};
use crate::{App, Message};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Color, Element, Font, Length, Theme};

impl App {
    pub(crate) fn view_empty_chat() -> Element<'static, Message> {
        const EXAMPLES: &[&str] = &[
            "이 프로젝트의 의존성을 알려줘",
            "src/main.rs의 첫 30줄을 요약해줘",
            "examples/hello.rs 만들어줘",
        ];
        let title = text("CodeWarp").size(FS_TITLE).font(bold_font());
        let subtitle =
            text("AI 코딩 데스크톱 — Plan으로 안전하게 둘러보고, Build로 변경 적용").size(FS_BODY);
        let about = column![
            text("CodeWarp란?")
                .size(FS_LABEL)
                .font(semibold_font())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().primary.base.color),
                }),
            text("Rust 네이티브 Iced 기반의 AI 코딩 데스크톱입니다. 프로젝트 컨텍스트, 도구 실행, 클라우드와 로컬 provider를 한 화면에서 다룹니다.")
                .size(FS_BODY)
                .line_height(LINE_HEIGHT_BODY),
        ]
        .spacing(SPACE_XXS);

        let mut examples_col = column![
            text("다음을 시도해보세요")
                .size(FS_LABEL)
                .font(semibold_font())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().primary.base.color),
                }),
        ]
        .spacing(SPACE_SM);
        for ex in EXAMPLES {
            examples_col = examples_col.push(
                button(text(ex.to_string()).size(FS_SUBTITLE))
                    .on_press(Message::InputChanged((*ex).to_string()))
                    .padding([7, 12])
                    .width(Length::Fill)
                    .style(secondary_btn),
            );
        }

        let modes = column![
            text("모드 (입력창 좌측 라벨 클릭 또는 슬래시)")
                .size(FS_LABEL)
                .font(semibold_font()),
            text("/plan   계획 먼저, 도구는 read-only").size(FS_BODY),
            text("/build  변경 적용 (write_file, run_command)").size(FS_BODY),
        ]
        .spacing(2);

        let shortcut_hint = |keys: &'static str, label: &'static str| {
            container(
                row![
                    text(keys)
                        .size(FS_LABEL)
                        .font(Font::with_name("JetBrains Mono")),
                    Space::new().width(Length::Fill),
                    text(label).size(FS_BODY),
                ]
                .spacing(SPACE_SM)
                .align_y(Alignment::Center),
            )
            .padding([PAD_XS, PAD_MD])
            .style(context_item_style)
        };
        let shortcuts = column![
            text("키보드 단축키").size(FS_LABEL).font(semibold_font()),
            shortcut_hint("Ctrl+K", "명령 팔레트"),
            shortcut_hint("Ctrl+N", "새 채팅"),
            shortcut_hint("Ctrl+,", "설정"),
            shortcut_hint("Ctrl+Shift+P / B", "Plan / Build 모드"),
        ]
        .spacing(SPACE_XS);

        let divider = || {
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(|_: &Theme| container::Style {
                    background: Some(Color::from_rgba8(0x1e, 0x29, 0x3b, 0.60).into()),
                    ..Default::default()
                })
        };

        container(
            column![
                title,
                subtitle,
                about,
                divider(),
                examples_col,
                divider(),
                modes,
                Space::new().height(Length::Fixed(SPACE_SM)),
                shortcuts,
            ]
            .spacing(SPACE_SM)
            .max_width(560),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(28)
        .style(panel_style)
        .into()
    }
}
