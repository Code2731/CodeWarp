// block_apply.rs — Apply candidate parsing (block child module)

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
