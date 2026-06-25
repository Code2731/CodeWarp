use super::ui::{
    FS_BODY, FS_LABEL, PAD_LG, PANEL_SECTION_GAP_LG, RIGHT_PANEL_WIDTH, SCROLL_GUTTER_PAD_X,
    SPACE_SM, app_vscrollbar, panel_style, semibold_font,
};
use crate::{App, BlockBody, MAX_TOOL_ROUNDS, Message};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, column, container, scrollable, text};
use iced::{Element, Font, Length, Theme};

impl App {
    pub(super) fn view_rightpanel(&self) -> Element<'_, Message> {
        // 세션 통계 — blocks/conversation에서 derive
        let user_msg_count = self
            .conversation
            .iter()
            .filter(|m| m.role == "user")
            .count();
        let tool_results: Vec<(&str, &str, bool)> = self
            .blocks
            .iter()
            .filter_map(|b| match &b.body {
                BlockBody::ToolResult {
                    name,
                    summary,
                    success,
                } => Some((name.as_str(), summary.as_str(), *success)),
                _ => None,
            })
            .collect();
        let tool_count = tool_results.len();
        let success_count = tool_results.iter().filter(|(_, _, s)| *s).count();
        let fail_count = tool_count - success_count;

        let stats = column![
            text("세션 통계").size(FS_LABEL).font(semibold_font()),
            text(format!("· 메시지: {user_msg_count}")).size(FS_BODY),
            text(format!(
                "· 도구 호출: {tool_count} (✓{success_count} ✗{fail_count})"
            ))
            .size(FS_BODY),
            text(format!("· 모드: {}", self.agent_mode.label())).size(FS_BODY),
        ]
        .spacing(2);

        // 도구 호출 로그 (역순 — 최근이 위)
        let mut log_col =
            column![text("도구 호출 로그").size(FS_LABEL).font(semibold_font())].spacing(2);
        if tool_results.is_empty() {
            log_col = log_col.push(text("// 도구 호출 시 여기 누적").size(FS_LABEL));
        } else {
            for (name, summary, success) in tool_results.iter().rev() {
                let icon = if *success { "✓" } else { "✗" };
                let line = text(format!("{icon} {name} → {summary}"))
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono"));
                log_col = log_col.push(line);
            }
        }

        // 도구 라운드 진행 표시 (streaming 중일 때만)
        let round_indicator: Element<Message> =
            if self.streaming_block_id.is_some() && self.tool_round > 0 {
                text(format!(
                    "▶ 도구 라운드 {}/{}",
                    self.tool_round, MAX_TOOL_ROUNDS
                ))
                .size(FS_LABEL)
                .font(semibold_font())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().primary.base.color),
                })
                .into()
            } else {
                Space::new()
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .into()
            };

        let body = column![
            stats,
            Space::new().height(Length::Fixed(PANEL_SECTION_GAP_LG)),
            round_indicator,
            Space::new().height(Length::Fixed(SPACE_SM)),
            log_col,
        ]
        .spacing(SPACE_SM);

        container(
            scrollable(container(body).padding([0, SCROLL_GUTTER_PAD_X]))
                .direction(Direction::Vertical(app_vscrollbar()))
                .height(Length::Fill),
        )
        .width(Length::Fixed(RIGHT_PANEL_WIDTH))
        .height(Length::Fill)
        .padding(PAD_LG)
        .style(panel_style)
        .into()
    }
}
