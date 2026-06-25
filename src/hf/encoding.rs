// hf/encoding.rs — URL/path encoding helpers (hf child module)
use crate::hf::types::HF_BASE;
use std::fmt::Write;

pub(super) fn encode_path_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

pub(super) fn encode_repo_file_path(input: &str) -> String {
    input
        .split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn model_info_url(repo_id: &str, rev: &str) -> String {
    if rev == "main" {
        format!("{HF_BASE}/api/models/{repo_id}")
    } else {
        format!(
            "{}/api/models/{}/revision/{}",
            HF_BASE,
            repo_id,
            encode_path_segment(rev)
        )
    }
}

pub(super) fn model_tree_url(repo_id: &str, rev: &str) -> String {
    format!(
        "{}/api/models/{}/tree/{}?recursive=true",
        HF_BASE,
        repo_id,
        encode_path_segment(rev)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_path_segment_empty() {
        assert_eq!(encode_path_segment(""), "");
    }

    #[test]
    fn test_encode_path_segment_unreserved() {
        assert_eq!(encode_path_segment("abc123-._~"), "abc123-._~");
    }

    #[test]
    fn test_encode_path_segment_percent_encodes() {
        assert_eq!(encode_path_segment("a b"), "a%20b");
    }

    #[test]
    fn test_encode_path_segment_slash_is_encoded() {
        assert_eq!(encode_path_segment("a/b"), "a%2Fb");
    }

    #[test]
    fn test_encode_path_segment_special_chars() {
        assert_eq!(encode_path_segment("foo!@#"), "foo%21%40%23");
    }

    #[test]
    fn test_encode_path_segment_mixed() {
        assert_eq!(encode_path_segment("hello world-v2"), "hello%20world-v2");
    }

    #[test]
    fn test_encode_repo_file_path_empty() {
        assert_eq!(encode_repo_file_path(""), "");
    }

    #[test]
    fn test_encode_repo_file_path_simple() {
        assert_eq!(encode_repo_file_path("foo/bar"), "foo/bar");
    }

    #[test]
    fn test_encode_repo_file_path_encodes_segments() {
        assert_eq!(
            encode_repo_file_path("foo bar/baz qux"),
            "foo%20bar/baz%20qux"
        );
    }

    #[test]
    fn test_encode_repo_file_path_trailing_slash() {
        assert_eq!(encode_repo_file_path("a/b/"), "a/b/");
    }

    #[test]
    fn test_encode_repo_file_path_special_in_segments() {
        assert_eq!(encode_repo_file_path("a!b/c@d"), "a%21b/c%40d");
    }

    #[test]
    fn test_model_info_url_main_rev() {
        assert_eq!(
            model_info_url("org/model", "main"),
            "https://huggingface.co/api/models/org/model"
        );
    }

    #[test]
    fn test_model_info_url_non_main_rev() {
        assert_eq!(
            model_info_url("org/model", "v1.0"),
            "https://huggingface.co/api/models/org/model/revision/v1.0"
        );
    }

    #[test]
    fn test_model_info_url_encodes_rev() {
        assert_eq!(
            model_info_url("org/model", "my rev"),
            "https://huggingface.co/api/models/org/model/revision/my%20rev"
        );
    }

    #[test]
    fn test_model_tree_url_main_rev() {
        assert_eq!(
            model_tree_url("org/model", "main"),
            "https://huggingface.co/api/models/org/model/tree/main?recursive=true"
        );
    }

    #[test]
    fn test_model_tree_url_non_main_rev() {
        assert_eq!(
            model_tree_url("org/model", "v1.0"),
            "https://huggingface.co/api/models/org/model/tree/v1.0?recursive=true"
        );
    }

    #[test]
    fn test_model_tree_url_encodes_rev() {
        assert_eq!(
            model_tree_url("org/model", "rev 2"),
            "https://huggingface.co/api/models/org/model/tree/rev%202?recursive=true"
        );
    }
}
