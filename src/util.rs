// CodeWarp — 일반 유틸리티 함수
//
// main.rs에서 추출: fmt_bytes, fmt_context_length, resolve_user_path,
// extract_mention_query, fuzzy_match_paths, build_file_context,
// collect_mention_candidates, kill_pid, hscrollbar, summarize_tool_result.

use std::path::PathBuf;

use iced::widget::scrollable::Scrollbar;

use crate::tools;

// ── Formatting ──────────────────────────────────────────────────────

/// 바이트 수를 KB/MB/GB 단위로 표시 (1024 진법).
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
        format!("{} B", n)
    }
}

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
    if trimmed == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
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

/// PathBuf 목록을 query로 fuzzy filter (대소문자 무시, 부분 포함).
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

/// cwd 기준으로 파일 목록을 비동기 수집 (최대 200개, max_depth=5).
pub(crate) async fn collect_mention_candidates(cwd: PathBuf) -> Vec<PathBuf> {
    tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for entry in ignore::WalkBuilder::new(&cwd).max_depth(Some(5)).build() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
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
    Scrollbar::new().width(8).scroller_width(8).margin(2)
}

// ── Tool result summarizer ──────────────────────────────────────────

/// 도구 호출 결과를 ToolResult 칩에 표시할 한 줄 요약 + 성공 여부로 변환.
pub(crate) fn summarize_tool_result(name: &str, args_json: &str, result: &str) -> (String, bool) {
    let lower = result.to_ascii_lowercase();
    let success =
        !(result.starts_with("Error") || lower.contains("[err]") || lower.starts_with("error"));
    let summary = match name {
        "write_file" => tools::WriteFileArgs::parse(args_json)
            .map(|a| format!("{} ({} bytes)", a.path, a.content.len()))
            .unwrap_or_else(|_| "?".into()),
        "run_command" => tools::RunCommandArgs::parse(args_json)
            .map(|a| format!("$ {}", a.command.chars().take(60).collect::<String>()))
            .unwrap_or_else(|_| "?".into()),
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

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── fmt_bytes ───────────────────────────────────────────────────

    #[test]
    fn fmt_bytes_units() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1024), "1 KB");
        assert_eq!(fmt_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(fmt_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn fmt_bytes_large_gb() {
        let n = 5_500_000_000u64;
        let s = fmt_bytes(n);
        assert!(s.ends_with(" GB"), "got: {}", s);
        assert!(s.starts_with("5.1"), "got: {}", s);
    }

    // ── fmt_context_length ──────────────────────────────────────────

    #[test]
    fn fmt_context_length_units() {
        assert_eq!(fmt_context_length(500), "500");
        assert_eq!(fmt_context_length(8000), "8k");
        assert_eq!(fmt_context_length(128_000), "128k");
        assert_eq!(fmt_context_length(1_000_000), "1.0M");
        assert_eq!(fmt_context_length(2_500_000), "2.5M");
    }

    // ── resolve_user_path ───────────────────────────────────────────

    #[test]
    fn resolve_user_path_expands_tilde() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(resolve_user_path("~"), home);
            assert_eq!(resolve_user_path("~/models"), home.join("models"));
            assert_eq!(resolve_user_path("~\\models"), home.join("models"));
        }
    }

    // ── extract_mention_query ───────────────────────────────────────

    #[test]
    fn mention_query_basic() {
        assert_eq!(extract_mention_query("fix @main"), Some("main"));
        assert_eq!(extract_mention_query("fix @main "), None);
        assert_eq!(extract_mention_query("@src/lib"), Some("src/lib"));
        assert_eq!(extract_mention_query("no at sign"), None);
        assert_eq!(extract_mention_query("@"), Some(""));
    }

    #[test]
    fn mention_query_last_at_wins() {
        assert_eq!(extract_mention_query("@foo @bar"), Some("bar"));
        assert_eq!(extract_mention_query("@foo @bar "), None);
        assert_eq!(extract_mention_query("email@ex.com @file"), Some("file"));
    }

    // ── fuzzy_match_paths ───────────────────────────────────────────

    #[test]
    fn fuzzy_match_empty_query_returns_all() {
        let paths: Vec<PathBuf> = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/tools.rs")];
        let result = fuzzy_match_paths(&paths, "", 10);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn fuzzy_match_filters_by_query() {
        let paths: Vec<PathBuf> = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/tools.rs"),
            PathBuf::from("Cargo.toml"),
        ];
        let result = fuzzy_match_paths(&paths, "tool", 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], PathBuf::from("src/tools.rs"));
    }

    #[test]
    fn fuzzy_match_respects_max_results() {
        let paths: Vec<PathBuf> = (0..20)
            .map(|i| PathBuf::from(format!("file{i}.rs")))
            .collect();
        let result = fuzzy_match_paths(&paths, "file", 5);
        assert_eq!(result.len(), 5);
    }

    // ── build_file_context ──────────────────────────────────────────

    #[test]
    fn build_file_context_single() {
        let files = vec![(PathBuf::from("src/main.rs"), "fn main() {}".to_string())];
        let ctx = build_file_context(&files);
        assert!(ctx.contains("src/main.rs"));
        assert!(ctx.contains("fn main() {}"));
        assert!(ctx.starts_with("```"));
    }

    #[test]
    fn build_file_context_multi_separator() {
        let files = vec![
            (PathBuf::from("a.rs"), "aaa".to_string()),
            (PathBuf::from("b.rs"), "bbb".to_string()),
        ];
        let ctx = build_file_context(&files);
        assert!(ctx.contains("\n\n"));
        assert!(ctx.contains("aaa"));
        assert!(ctx.contains("bbb"));
    }

    // ── summarize_tool_result ───────────────────────────────────────

    #[test]
    fn summarize_write_file_success() {
        let args = r#"{"path":"src/foo.rs","content":"hello"}"#;
        let (summary, success) = summarize_tool_result("write_file", args, "OK: wrote 5 bytes");
        assert!(summary.contains("src/foo.rs"));
        assert!(summary.contains("5 bytes"));
        assert!(success);
    }

    #[test]
    fn summarize_write_file_error_result() {
        let args = r#"{"path":"src/foo.rs","content":"x"}"#;
        let (_, success) = summarize_tool_result("write_file", args, "Error: permission denied");
        assert!(!success);
    }

    #[test]
    fn summarize_run_command_truncates_long_command() {
        let long = "x".repeat(200);
        let args = format!(r#"{{"command":"{}"}}"#, long);
        let (summary, _) = summarize_tool_result("run_command", &args, "OK");
        assert!(summary.starts_with("$ "));
        assert!(summary.len() <= 62);
    }

    #[test]
    fn summarize_unknown_tool() {
        let (summary, success) = summarize_tool_result("foo", "{}", "first line\nsecond line");
        assert_eq!(summary, "first line");
        assert!(success);
    }

    #[test]
    fn summarize_err_marker() {
        let (_, success) = summarize_tool_result("foo", "{}", "[err] something broke");
        assert!(!success);
    }
}
