// CodeWarp — 채팅 블록 타입, Apply 후보, 대화 헬퍼
//
// main.rs에서 추출: BlockBody, ViewMode, Block, ApplyCandidate, PendingToolCall,
// parse_apply_candidates, persisted_to_block, truncate_after_last_user 등.

use iced::widget::markdown;
use iced::widget::text_editor;

use crate::session;

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

// ── Apply candidate ─────────────────────────────────────────────────

/// AI 응답의 fenced code block 첫 줄에서 `// path: ...` 또는 `# path: ...`를
/// 검사해 적용 후보를 추출. 닫는 fence가 없거나 path가 첫 줄이면 skip.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApplyCandidate {
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) content: String,
}

pub(crate) fn extract_path_from_comment(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    for prefix in ["//", "#", "--"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.trim_start();
            if let Some(p) = rest.strip_prefix("path:") {
                let path = p.trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}

pub(crate) fn parse_apply_candidates(markdown: &str) -> Vec<ApplyCandidate> {
    let mut out = Vec::new();
    let mut lines = markdown.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("```") else {
            continue;
        };
        let language = rest.split_whitespace().next().unwrap_or("").to_string();
        let Some(first) = lines.next() else { break };
        let Some(path) = extract_path_from_comment(first) else {
            for inner in lines.by_ref() {
                if inner.trim_start().starts_with("```") {
                    break;
                }
            }
            continue;
        };
        let mut content = String::new();
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                closed = true;
                break;
            }
            content.push_str(inner);
            content.push('\n');
        }
        if closed && !content.is_empty() {
            out.push(ApplyCandidate {
                path,
                language,
                content,
            });
        }
    }
    out
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

// ── Tests ───────────────────────────────────────────────────────────

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

    // ── truncate_after_last_user ────────────────────────────────────

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

    // ── last_user_block_idx / last_assistant_block_idx ─────────────

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

    // ── parse_apply_candidates ──────────────────────────────────────

    #[test]
    fn apply_empty_markdown() {
        assert!(parse_apply_candidates("").is_empty());
        assert!(parse_apply_candidates("just plain text\nno code blocks").is_empty());
    }

    #[test]
    fn apply_no_path_comment_skipped() {
        let md = "```rust\nfn main() {}\n```\n";
        assert!(parse_apply_candidates(md).is_empty());
    }

    #[test]
    fn apply_rust_path_comment() {
        let md = "```rust\n// path: src/foo.rs\nfn main() {}\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "src/foo.rs");
        assert_eq!(candidates[0].language, "rust");
        assert_eq!(candidates[0].content, "fn main() {}\n");
    }

    #[test]
    fn apply_python_hash_comment() {
        let md = "```python\n# path: scripts/build.py\nprint('hi')\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "scripts/build.py");
        assert_eq!(candidates[0].language, "python");
    }

    #[test]
    fn apply_multiple_blocks_filters_no_path() {
        let md = "intro\n\
                  ```rust\n// path: a.rs\nA\n```\n\
                  some text\n\
                  ```rust\nB without path\n```\n\
                  ```python\n# path: b.py\nB\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].path, "a.rs");
        assert_eq!(candidates[1].path, "b.py");
    }

    #[test]
    fn apply_path_comment_with_extra_spaces() {
        let md = "```rust\n//    path:    src/x.rs   \nbody\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "src/x.rs");
    }

    #[test]
    fn apply_unclosed_fence_ignored() {
        let md = "```rust\n// path: a.rs\nbody (no closing)\n";
        assert!(parse_apply_candidates(md).is_empty());
    }

    #[test]
    fn apply_no_language_still_works() {
        let md = "```\n// path: x.txt\nhello\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "x.txt");
        assert_eq!(candidates[0].language, "");
    }

    #[test]
    fn apply_first_line_must_be_path() {
        let md = "```rust\nfn main() {}\n// path: a.rs\n```\n";
        assert!(parse_apply_candidates(md).is_empty());
    }

    // ── persisted_to_block ──────────────────────────────────────────

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
