// CodeWarp — 채팅 블록 타입, 대화 헬퍼
// main.rs에서 추출: BlockBody, ViewMode, Block, PendingToolCall 등.

use iced::widget::markdown;
use iced::widget::text_editor;

use crate::session;

mod block_apply;
pub(crate) use block_apply::*;

// ── Block types ─────────────────────────────────────────────────────

/// 사용자 입력은 짧은 plain text, AI 응답은 read-only `text_editor` (부분 선택 + 복사 가능).
#[derive(Debug)]
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
    /// 원문(read-only `text_editor`). 기본 assistant 보기이며 부분 선택 + Ctrl+C 가능.
    Raw,
}

#[derive(Debug)]
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
#[derive(Debug, Default, Clone)]
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

/// 가장 마지막 `BlockBody::User` 인덱스 (없으면 None).
pub(crate) fn last_user_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .rposition(|b| matches!(b.body, BlockBody::User(_)))
}

/// 가장 마지막 `BlockBody::Assistant` 인덱스 (없으면 None).
pub(crate) fn last_assistant_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .rposition(|b| matches!(b.body, BlockBody::Assistant(_)))
}

#[cfg(test)]
mod tests;
