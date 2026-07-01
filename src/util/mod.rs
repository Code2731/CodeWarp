// CodeWarp — 일반 유틸리티 함수
//
// main.rs에서 추출: fmt_bytes, fmt_context_length, resolve_user_path,
// extract_mention_query, fuzzy_match_paths, build_file_context,
// collect_mention_candidates, kill_pid, hscrollbar, summarize_tool_result.

pub(crate) mod file_tree;

use std::path::PathBuf;

use iced::widget::scrollable::Scrollbar;

use crate::tools;

// ── Formatting ──────────────────────────────────────────────────────

/// 바이트 수를 KB/MB/GB 단위로 표시 (1024 진법).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn fmt_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.0} KB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn fmt_context_length(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

// ── Path helpers ────────────────────────────────────────────────────

/// `~`, `~/...`, `~\...` 경로를 사용자 홈 기준 절대 경로로 확장.
/// 그 외 입력은 그대로 반환.
pub(crate) fn resolve_user_path(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(trimmed)
}

// ── Mention / fuzzy ─────────────────────────────────────────────────

/// input 문자열에서 마지막 '@' 이후 mention query 추출.
/// 공백·개행이 포함되면 None (이미 완성된 멘션이므로 팝업 불필요).
pub(crate) fn extract_mention_query(input: &str) -> Option<&str> {
    let at_pos = input.rfind('@')?;
    let rest = &input[at_pos + 1..];
    if rest.bytes().any(|b| matches!(b, b' ' | b'\n' | b'\t')) {
        return None;
    }
    Some(rest)
}

/// `PathBuf` 목록을 query로 fuzzy filter (대소문자 무시, 부분 포함).
pub(crate) fn fuzzy_match_paths(
    candidates: &[PathBuf],
    query: &str,
    max_results: usize,
) -> Vec<PathBuf> {
    if query.is_empty() {
        return candidates.iter().take(max_results).cloned().collect();
    }
    let q = query.to_lowercase();
    candidates
        .iter()
        .filter(|p| p.to_string_lossy().to_lowercase().contains(&q))
        .take(max_results)
        .cloned()
        .collect()
}

/// 첨부 파일 목록을 코드 펜스 블록 컨텍스트 문자열로 변환.
pub(crate) fn build_file_context(files: &[(PathBuf, String)]) -> String {
    files
        .iter()
        .map(|(path, content)| {
            let name = path.display();
            format!("```{name}\n{content}\n```")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// cwd 기준으로 파일 목록을 비동기 수집 (최대 200개, `max_depth=5`).
pub(crate) async fn collect_mention_candidates(cwd: PathBuf) -> Vec<PathBuf> {
    tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for entry in ignore::WalkBuilder::new(&cwd).max_depth(Some(5)).build() {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            if let Ok(rel) = entry.path().strip_prefix(&cwd) {
                results.push(rel.to_path_buf());
            }
            if results.len() >= 200 {
                break;
            }
        }
        results
    })
    .await
    .unwrap_or_default()
}

// ── Process management ──────────────────────────────────────────────

/// 윈도우는 taskkill /T /F (자식 트리 포함), 그 외는 kill SIGTERM.
pub(crate) fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status();
    }
}

// ── Constants ───────────────────────────────────────────────────────

/// 첨부 파일 크기 상한 (512 KB 초과 시 거부)
pub(crate) const MAX_ATTACH_BYTES: u64 = 512 * 1024;

/// PTY 출력 버퍼 최대 줄 수 (FIFO)
pub(crate) const PTY_MAX_LINES: usize = 500;

// ── Scrollbar ───────────────────────────────────────────────────────

pub(crate) fn hscrollbar() -> Scrollbar {
    Scrollbar::new().width(10).scroller_width(10).margin(2)
}

// ── Tool result summarizer ──────────────────────────────────────────

/// 도구 호출 결과를 `ToolResult` 칩에 표시할 한 줄 요약 + 성공 여부로 변환.
pub(crate) fn summarize_tool_result(name: &str, args_json: &str, result: &str) -> (String, bool) {
    let lower = result.to_ascii_lowercase();
    let success =
        !(result.starts_with("Error") || lower.contains("[err]") || lower.starts_with("error"));
    let summary = match name {
        "write_file" => tools::WriteFileArgs::parse(args_json).map_or_else(
            |_| "?".into(),
            |a| format!("{} ({} bytes)", a.path, a.content.len()),
        ),
        "run_command" => tools::RunCommandArgs::parse(args_json).map_or_else(
            |_| "?".into(),
            |a| format!("$ {}", a.command.chars().take(60).collect::<String>()),
        ),
        _ => result
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect(),
    };
    (summary, success)
}

#[cfg(test)]
mod util_tests;
