// CodeWarp — 채팅 블록 타입, 대화 헬퍼
// main.rs에서 추출: BlockBody, ViewMode, Block, PendingToolCall 등.

use iced::widget::markdown;
use iced::widget::text_editor;

use crate::session;

mod block_apply;
pub(crate) use block_apply::*;

// ── Block types ─────────────────────────────────────────────────────

/// 사용자 입력은 짧은 plain text, AI 응답은 read-only text_editor (부분 선택 + 복사 가능).
pub(crate) enum BlockBody {
    User(String),
    Assistant(text_editor::Content),
    /// 도구 호출 실행 결과 (휘발성 — 세션 저장 안 됨, 시각 알림용).
    ToolResult {
        name: String,
        summary: String,
        success: bool,
    },
}

impl BlockBody {
    pub(crate) fn role_label(&self) -> &'static str {
        match self {
            BlockBody::User(_) => "you",
            BlockBody::Assistant(_) => "ai",
            BlockBody::ToolResult { .. } => "tool",
        }
    }

    pub(crate) fn to_text(&self) -> String {
        match self {
            BlockBody::User(s) => s.clone(),
            BlockBody::Assistant(c) => c.text(),
            BlockBody::ToolResult { summary, .. } => summary.clone(),
        }
    }

    pub(crate) fn is_empty_for_history(&self) -> bool {
        match self {
            BlockBody::User(s) => s.trim().is_empty(),
            BlockBody::Assistant(c) => c.text().trim().is_empty(),
            BlockBody::ToolResult { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ViewMode {
    /// 마크다운으로 예쁘게 렌더. 코드 블록은 syntax highlight.
    Rendered,
    /// 원문(read-only text_editor). 기본 assistant 보기이며 부분 선택 + Ctrl+C 가능.
    Raw,
}

pub(crate) struct Block {
    pub(crate) id: u64,
    pub(crate) body: BlockBody,
    pub(crate) view_mode: ViewMode,
    /// assistant Rendered용 캐시. 토큰 도착 시마다 갱신.
    pub(crate) md_items: Vec<markdown::Item>,
    /// assistant 블록을 만든 모델 ID (user 블록은 None).
    pub(crate) model: Option<String>,
    /// 응답 끝난 후 추출된 Apply 가능한 변경사항 + 적용 여부.
    pub(crate) apply_candidates: Vec<(ApplyCandidate, bool)>,
}

// ── Pending tool call ───────────────────────────────────────────────

/// 도구 호출이 SSE delta로 부분씩 도착하는 동안 누적할 임시 구조.
#[derive(Default, Clone)]
pub(crate) struct PendingToolCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) arguments: String,
}

pub(crate) const MAX_TOOL_ROUNDS: u32 = 5;
pub(crate) const MAX_MID_STREAM_RETRIES: u32 = 1;

// ── Block / conversation helpers ────────────────────────────────────

pub(crate) fn persisted_to_block(pb: session::PersistedBlock) -> Block {
    let role = pb.role;
    let content = pb.content;
    let md_items = if role == "assistant" {
        markdown::parse(&content).collect()
    } else {
        Vec::new()
    };
    let body = if role == "user" {
        BlockBody::User(content)
    } else {
        BlockBody::Assistant(text_editor::Content::with_text(&content))
    };
    let model = if pb.model.is_empty() {
        None
    } else {
        Some(pb.model)
    };
    Block {
        id: pb.id,
        body,
        view_mode: if role == "assistant" {
            ViewMode::Raw
        } else {
            ViewMode::Rendered
        },
        md_items,
        model,
        apply_candidates: Vec::new(),
    }
}

/// 마지막 user 메시지 다음의 모든 메시지를 conversation에서 제거.
/// regenerate 또는 edit 직전 호출. user가 전혀 없으면 conversation을 비움.
pub(crate) fn truncate_after_last_user(conv: &mut Vec<crate::openrouter::ChatMessage>) {
    while let Some(last) = conv.last() {
        if last.role == "user" {
            return;
        }
        conv.pop();
    }
}

/// 가장 마지막 BlockBody::User 인덱스 (없으면 None).
pub(crate) fn last_user_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .rposition(|b| matches!(b.body, BlockBody::User(_)))
}

/// 가장 마지막 BlockBody::Assistant 인덱스 (없으면 None).
pub(crate) fn last_assistant_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .rposition(|b| matches!(b.body, BlockBody::Assistant(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ub(id: u64) -> Block {
        Block {
            id,
            body: BlockBody::User(format!("u{}", id)),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }
    fn ab(id: u64) -> Block {
        Block {
            id,
            body: BlockBody::Assistant(text_editor::Content::with_text(&format!("a{}", id))),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }
    fn tb(id: u64) -> Block {
        Block {
            id,
            body: BlockBody::ToolResult {
                name: "x".into(),
                summary: "y".into(),
                success: true,
            },
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }

    fn cm(role: &str, content: &str) -> crate::openrouter::ChatMessage {
        crate::openrouter::ChatMessage {
            role: role.into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    #[test]
    fn truncate_empty_conv() {
        let mut conv: Vec<crate::openrouter::ChatMessage> = Vec::new();
        truncate_after_last_user(&mut conv);
        assert!(conv.is_empty());
    }

    #[test]
    fn truncate_user_only() {
        let mut conv = vec![cm("user", "hi")];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 1);
        assert_eq!(conv[0].role, "user");
    }

    #[test]
    fn truncate_user_assistant() {
        let mut conv = vec![cm("user", "hi"), cm("assistant", "hello")];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 1);
        assert_eq!(conv[0].content.as_deref(), Some("hi"));
    }

    #[test]
    fn truncate_keeps_last_user_intact() {
        let mut conv = vec![
            cm("user", "first"),
            cm("assistant", "answer1"),
            cm("user", "second"),
        ];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 3);
        assert_eq!(conv[2].content.as_deref(), Some("second"));
    }

    #[test]
    fn truncate_user_tool_assistant_chain() {
        let mut conv = vec![
            cm("system", "sys"),
            cm("user", "hi"),
            cm("assistant", "let me check"),
            cm("tool", "result"),
            cm("assistant", "done"),
        ];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 2);
        assert_eq!(conv[0].role, "system");
        assert_eq!(conv[1].role, "user");
    }

    #[test]
    fn truncate_no_user_drops_all() {
        let mut conv = vec![cm("system", "sys"), cm("assistant", "lone")];
        truncate_after_last_user(&mut conv);
        assert!(conv.is_empty());
    }

    #[test]
    fn last_user_idx_empty() {
        assert_eq!(last_user_block_idx(&[]), None);
    }

    #[test]
    fn last_user_idx_only_user() {
        let blocks = vec![ub(1)];
        assert_eq!(last_user_block_idx(&blocks), Some(0));
    }

    #[test]
    fn last_user_idx_picks_last() {
        let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
        assert_eq!(last_user_block_idx(&blocks), Some(2));
    }

    #[test]
    fn last_user_idx_no_user() {
        let blocks = vec![ab(1), tb(2)];
        assert_eq!(last_user_block_idx(&blocks), None);
    }

    #[test]
    fn last_assistant_idx_picks_last_assistant() {
        let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
        assert_eq!(last_assistant_block_idx(&blocks), Some(3));
    }

    #[test]
    fn last_assistant_idx_no_assistant() {
        let blocks = vec![ub(1), ub(2)];
        assert_eq!(last_assistant_block_idx(&blocks), None);
    }

    #[test]
    fn persisted_assistant_blocks_default_to_raw_for_selection() {
        let block = persisted_to_block(session::PersistedBlock {
            id: 1,
            role: "assistant".into(),
            content: "selectable answer".into(),
            model: "local".into(),
        });

        assert_eq!(block.view_mode, ViewMode::Raw);
        assert_eq!(block.body.to_text(), "selectable answer");
    }

    #[test]
    fn persisted_user_blocks_keep_rendered_layout() {
        let block = persisted_to_block(session::PersistedBlock {
            id: 1,
            role: "user".into(),
            content: "hello".into(),
            model: String::new(),
        });

        assert_eq!(block.view_mode, ViewMode::Rendered);
    }
}
