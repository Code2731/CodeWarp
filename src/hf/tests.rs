use super::{
    annotate_revision_not_found_error, choose_revision_fallback, contains_status,
    encode_path_segment, encode_repo_file_path, extract_bpw_value, format_branch_suggestions,
    humanize_error, model_info_url, model_tree_url, normalize_revision_name,
};

#[test]
fn humanize_auth() {
    let msg = humanize_error("HF 401: unauthorized");
    assert!(msg.contains("토큰"));
}

#[test]
fn humanize_auth_lowercase_prefix() {
    let msg = humanize_error("hf 403: forbidden");
    assert!(msg.contains("권한"));
}

#[test]
fn humanize_auth_status_text() {
    let msg = humanize_error("request failed with status 401 Unauthorized");
    assert!(msg.contains("401/403"));
}

#[test]
fn humanize_404_revision() {
    let msg = humanize_error("HF 404: revision not found");
    assert!(msg.contains("리비전"));
}

#[test]
fn humanize_timeout() {
    let msg = humanize_error("operation timed out");
    assert!(msg.contains("시간 초과"));
}

#[test]
fn contains_status_matches_standalone_code() {
    assert!(contains_status("status=404 not found", 404));
    assert!(!contains_status("file size 1404 bytes", 404));
}

#[test]
fn normalize_revision_name_compacts_symbols() {
    assert_eq!(normalize_revision_name("4.0-bpw"), "40bpw");
    assert_eq!(normalize_revision_name(" 4_0 BPW "), "40bpw");
}

#[test]
fn extract_bpw_value_parses_number() {
    assert_eq!(extract_bpw_value("4.0bpw"), Some(4.0));
    assert_eq!(extract_bpw_value("exl2-6.5bpw"), Some(6.5));
    assert_eq!(extract_bpw_value("main"), None);
}

#[test]
fn choose_revision_fallback_prefers_closest_bpw() {
    let branches = vec![
        "3.0bpw".to_string(),
        "4.5bpw".to_string(),
        "6.0bpw".to_string(),
    ];
    assert_eq!(
        choose_revision_fallback("4.0bpw", &branches).as_deref(),
        Some("4.5bpw")
    );
}

#[test]
fn choose_revision_fallback_uses_main_when_no_bpw_match() {
    let branches = vec!["dev".to_string(), "main".to_string()];
    assert_eq!(
        choose_revision_fallback("unknown-branch", &branches).as_deref(),
        Some("main")
    );
}

#[test]
fn format_branch_suggestions_limits_and_counts() {
    let branches = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
    ];
    assert_eq!(format_branch_suggestions(&branches, 2), "a, b ... +2 more");
}

#[test]
fn annotate_revision_not_found_error_appends_requested_and_candidates() {
    let branches = vec!["main".to_string(), "4.0bpw".to_string()];
    let text = annotate_revision_not_found_error("HF 404: revision not found", "4bpw", &branches);
    assert!(text.contains("requested revision: '4bpw'"));
    assert!(text.contains("available branches: main, 4.0bpw"));
}

#[test]
fn encode_path_segment_keeps_safe_revision_names() {
    assert_eq!(encode_path_segment("4.0bpw"), "4.0bpw");
    assert_eq!(encode_path_segment("main"), "main");
}

#[test]
fn encode_path_segment_escapes_reserved_chars() {
    assert_eq!(
        encode_path_segment("branch with/slash"),
        "branch%20with%2Fslash"
    );
}

#[test]
fn encode_repo_file_path_keeps_slashes_and_escapes_segments() {
    assert_eq!(
        encode_repo_file_path("tokenizer/my file @v1.json"),
        "tokenizer/my%20file%20%40v1.json"
    );
}

#[test]
fn encode_repo_file_path_escapes_each_nested_segment() {
    assert_eq!(
        encode_repo_file_path("a b/c+d/model-00001.safetensors"),
        "a%20b/c%2Bd/model-00001.safetensors"
    );
}

#[test]
fn model_info_url_uses_revision_path_endpoint() {
    assert_eq!(
        model_info_url("owner/repo", "4.0bpw"),
        "https://huggingface.co/api/models/owner/repo/revision/4.0bpw"
    );
}

#[test]
fn model_tree_url_uses_revision_tree_endpoint() {
    assert_eq!(
        model_tree_url("owner/repo", "4.0bpw"),
        "https://huggingface.co/api/models/owner/repo/tree/4.0bpw?recursive=true"
    );
}

#[test]
fn model_info_url_keeps_main_on_base_model_endpoint() {
    assert_eq!(
        model_info_url("owner/repo", "main"),
        "https://huggingface.co/api/models/owner/repo"
    );
}
