use crate::view::ui::*;
use crate::*;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Font, Length};

impl App {
    pub(crate) fn view_empty_chat(&self) -> Element<'_, Message> {
        const EXAMPLES: &[&str] = &[
            "이 프로젝트의 의존성을 알려줘",
            "src/main.rs의 첫 30줄을 요약해줘",
            "examples/hello.rs 만들어줘",
        ];
        let title = text("CodeWarp").size(FS_TITLE).font(bold_font());
        let subtitle =
            text("AI 코딩 데스크톱 — Plan으로 안전하게 둘러보고, Build로 변경 적용").size(FS_BODY);
        let about = column![
            text("CodeWarp란?").size(FS_LABEL).font(semibold_font()),
            text("Rust 네이티브 Iced 기반의 AI 코딩 데스크톱입니다. 프로젝트 컨텍스트, 도구 실행, 클라우드와 로컬 provider를 한 화면에서 다룹니다.")
                .size(FS_BODY)
                .line_height(1.35),
        ]
        .spacing(SPACE_XXS);

        let mut examples_col = column![text("다음을 시도해보세요")
            .size(FS_LABEL)
            .font(semibold_font())]
        .spacing(SPACE_SM);
        for ex in EXAMPLES {
            examples_col = examples_col.push(
                button(text(format!("▸ {}", ex)).size(FS_SUBTITLE))
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

        container(
            column![
                title,
                subtitle,
                about,
                Space::new().height(Length::Fixed(PANEL_SECTION_GAP_LG)),
                examples_col,
                Space::new().height(Length::Fixed(PANEL_SECTION_GAP_LG)),
                modes,
                Space::new().height(Length::Fixed(SPACE_SM)),
                shortcuts,
            ]
            .spacing(SPACE_SM)
            .max_width(560),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(panel_style)
        .into()
    }
}
