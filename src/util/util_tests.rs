use super::*;

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

#[test]
fn fmt_context_length_units() {
    assert_eq!(fmt_context_length(500), "500");
    assert_eq!(fmt_context_length(8000), "8k");
    assert_eq!(fmt_context_length(128_000), "128k");
    assert_eq!(fmt_context_length(1_000_000), "1.0M");
    assert_eq!(fmt_context_length(2_500_000), "2.5M");
}

#[test]
fn resolve_user_path_expands_tilde() {
    if let Some(home) = dirs::home_dir() {
        assert_eq!(resolve_user_path("~"), home);
        assert_eq!(resolve_user_path("~/models"), home.join("models"));
        assert_eq!(resolve_user_path("~\\models"), home.join("models"));
    }
}

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
