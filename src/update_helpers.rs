use super::*;

const HF_HINT_MARKERS: [&str; 3] = [
    "fallback retry from",
    "fallback lookup failed:",
    "requested revision:",
];

fn starts_with_ascii_case_insensitive(text: &str, prefix: &str) -> bool {
    text.to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

fn find_hint_boundary(tail: &str) -> Option<usize> {
    for sep in [") (", ")("] {
        let mut offset = 0usize;
        while let Some(rel) = tail[offset..].find(sep) {
            let pos = offset + rel;
            let after = tail[pos + sep.len()..].trim_start();
            if HF_HINT_MARKERS
                .iter()
                .any(|m| starts_with_ascii_case_insensitive(after, m))
            {
                return Some(pos);
            }
            offset = pos + sep.len();
        }
    }
    None
}

fn extract_hf_error_hint(raw: &str, marker: &str) -> Option<String> {
    let raw_lc = raw.to_ascii_lowercase();
    let marker_lc = marker.to_ascii_lowercase();
    let idx = raw_lc.find(&marker_lc)?;
    let tail = &raw[idx..];
    let cut = find_hint_boundary(tail);
    let head = cut.map(|i| &tail[..i]).unwrap_or(tail);
    let head = head.strip_suffix(')').unwrap_or(head).trim();
    if head.is_empty() {
        None
    } else {
        Some(head.to_string())
    }
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn merge_hint(hints: &mut Vec<String>, candidate: String) {
    if hints.iter().any(|existing| {
        existing == &candidate || contains_ascii_case_insensitive(existing, &candidate)
    }) {
        return;
    }
    hints.retain(|existing| !contains_ascii_case_insensitive(&candidate, existing));
    hints.push(candidate);
}

pub(crate) fn compose_hf_download_error(raw: &str) -> String {
    let humanized = hf::humanize_error(raw);
    let mut hints: Vec<String> = Vec::new();
    for marker in HF_HINT_MARKERS {
        if let Some(h) = extract_hf_error_hint(raw, marker) {
            merge_hint(&mut hints, h);
        }
    }
    if hints.is_empty() {
        return humanized;
    }
    let missing: Vec<String> = hints
        .into_iter()
        .filter(|h| !contains_ascii_case_insensitive(&humanized, h))
        .collect();
    if missing.is_empty() {
        humanized
    } else {
        format!("{humanized} ({})", missing.join(" | "))
    }
}

pub(crate) fn is_loopback_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("[::1]")
}

pub(crate) fn extract_loopback_port(url: &str) -> Option<u16> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let no_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let authority = no_scheme.split('/').next()?.trim();
    if authority.is_empty() {
        return None;
    }
    if authority.starts_with('[') {
        let closing = authority.find(']')?;
        let host = &authority[..=closing];
        if !host.eq_ignore_ascii_case("[::1]") {
            return None;
        }
        let rest = &authority[closing + 1..];
        let port = rest.strip_prefix(':')?;
        return port.parse::<u16>().ok();
    }
    let (host, port) = authority.rsplit_once(':')?;
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" {
        port.parse::<u16>().ok()
    } else {
        None
    }
}

pub(crate) fn tabby_connection_error_looks_unreachable(raw: &str, actionable: &str) -> bool {
    let raw_lower = raw.to_ascii_lowercase();
    raw_lower.contains("refused")
        || raw_lower.contains("os error 10061")
        || raw_lower.contains("timeout")
        || raw_lower.contains("timed out")
        || contains_ascii_case_insensitive(actionable, "응답 없음")
        || contains_ascii_case_insensitive(actionable, "시간 초과")
        || contains_ascii_case_insensitive(actionable, "응답하지")
}

pub(crate) fn runtime_command_exists(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }
    let candidate = std::path::Path::new(trimmed);
    if candidate.is_absolute()
        || trimmed.contains(std::path::MAIN_SEPARATOR)
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return candidate.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    let path_dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();

    #[cfg(windows)]
    {
        let has_ext = candidate.extension().is_some();
        let extensions: Vec<String> = if has_ext {
            vec![String::new()]
        } else {
            std::env::var_os("PATHEXT")
                .and_then(|v| v.into_string().ok())
                .map(|v| {
                    v.split(';')
                        .map(|e| e.trim())
                        .filter(|e| !e.is_empty())
                        .map(|e| e.to_ascii_lowercase())
                        .collect::<Vec<_>>()
                })
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        ".com".to_string(),
                        ".exe".to_string(),
                        ".bat".to_string(),
                        ".cmd".to_string(),
                    ]
                })
        };

        for dir in path_dirs {
            for ext in &extensions {
                let full = if ext.is_empty() {
                    dir.join(trimmed)
                } else {
                    dir.join(format!("{trimmed}{ext}"))
                };
                if full.is_file() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        for dir in path_dirs {
            if dir.join(trimmed).is_file() {
                return true;
            }
        }
        false
    }
}

pub(crate) fn resolve_binary_from_dir(dir: &std::path::Path, program: &str) -> Option<PathBuf> {
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);

    #[cfg(windows)]
    {
        let has_ext = std::path::Path::new(base).extension().is_some();
        let mut candidates = vec![dir.join(base)];
        if !has_ext {
            candidates.push(dir.join(format!("{base}.exe")));
            candidates.push(dir.join(format!("{base}.cmd")));
            candidates.push(dir.join(format!("{base}.bat")));
            candidates.push(dir.join(format!("{base}.com")));
        }
        candidates.into_iter().find(|c| c.is_file())
    }

    #[cfg(not(windows))]
    {
        let candidate = dir.join(base);
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    }
}

pub(crate) fn expected_binary_name(program: &str) -> String {
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    #[cfg(windows)]
    {
        if std::path::Path::new(base).extension().is_some() {
            base.to_string()
        } else {
            format!("{base}.exe")
        }
    }
    #[cfg(not(windows))]
    {
        base.to_string()
    }
}

pub(crate) fn default_models_dir() -> String {
    if let Some(p) = dirs::data_local_dir() {
        return p.join("codewarp").join("models").display().to_string();
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".codewarp").join("models").display().to_string();
    }
    "models".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        compose_hf_download_error, contains_ascii_case_insensitive, default_models_dir,
        extract_hf_error_hint, find_hint_boundary, merge_hint, runtime_command_exists,
        starts_with_ascii_case_insensitive,
    };
    use crate::humanize_inference_spawn_error;

    #[test]
    fn extract_hf_error_hint_parses_requested_revision_tail() {
        let raw =
            "HF 404: revision not found (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("requested revision: '4bpw'; available branches: main, 4.0bpw")
        );
    }

    #[test]
    fn extract_hf_error_hint_parses_fallback_retry() {
        let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "fallback retry from").as_deref(),
            Some("fallback retry from '4bpw' to '4.0bpw'")
        );
    }

    #[test]
    fn compose_hf_download_error_appends_revision_hint() {
        let raw =
            "HF 404: revision not found (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        let msg = compose_hf_download_error(raw);
        assert!(msg.contains("requested revision: '4bpw'"));
        assert!(msg.contains("available branches: main, 4.0bpw"));
    }

    #[test]
    fn compose_hf_download_error_appends_fallback_and_revision_hints() {
        let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        let msg = compose_hf_download_error(raw);
        assert!(msg.contains("fallback retry from '4bpw' to '4.0bpw'"));
        assert!(msg.contains("requested revision: '4bpw'"));
    }

    #[test]
    fn compose_hf_download_error_appends_fallback_lookup_failure_hint() {
        let raw = "HF 404: revision not found (fallback lookup failed: branch refs unavailable; requested revision: '4bpw')";
        let msg = compose_hf_download_error(raw);
        assert!(msg.contains("fallback lookup failed: branch refs unavailable"));
        assert!(msg.contains("requested revision: '4bpw'"));
    }

    #[test]
    fn extract_hf_error_hint_keeps_branch_names_with_parentheses() {
        let raw = "HF 404: revision not found (requested revision: '4bpw'; available branches: exl2(legacy), main)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("requested revision: '4bpw'; available branches: exl2(legacy), main")
        );
    }

    #[test]
    fn extract_hf_error_hint_parses_no_space_parenthesis_separator() {
        let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw')(requested revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "fallback retry from").as_deref(),
            Some("fallback retry from '4bpw' to '4.0bpw'")
        );
    }

    #[test]
    fn merge_hint_prefers_more_specific_hint() {
        let mut hints = vec!["requested revision: '4bpw'".to_string()];
        merge_hint(
            &mut hints,
            "fallback lookup failed: branch refs unavailable; requested revision: '4bpw'"
                .to_string(),
        );
        assert_eq!(hints.len(), 1);
        assert!(hints[0].starts_with("fallback lookup failed:"));
    }

    #[test]
    fn compose_hf_download_error_avoids_overlapping_hint_duplicates() {
        let raw = "HF 404: revision not found (fallback lookup failed: branch refs unavailable; requested revision: '4bpw')";
        let msg = compose_hf_download_error(raw);
        assert_eq!(msg.matches("requested revision: '4bpw'").count(), 1);
    }

    #[test]
    fn extract_hf_error_hint_is_case_insensitive_for_marker() {
        let raw = "HF 404: revision not found (Requested Revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("Requested Revision: '4bpw'; available branches: main, 4.0bpw")
        );
    }

    #[test]
    fn contains_ascii_case_insensitive_matches_mixed_case() {
        assert!(contains_ascii_case_insensitive(
            "Requested Revision: '4bpw'",
            "requested revision:"
        ));
    }

    #[test]
    fn merge_hint_deduplicates_case_insensitive_overlap() {
        let mut hints = vec!["Requested Revision: '4bpw'".to_string()];
        merge_hint(&mut hints, "requested revision: '4bpw'".to_string());
        assert_eq!(hints.len(), 1);
    }

    #[test]
    fn starts_with_ascii_case_insensitive_matches_mixed_case_prefix() {
        assert!(starts_with_ascii_case_insensitive(
            "Requested Revision: '4bpw'",
            "requested revision:"
        ));
    }

    #[test]
    fn find_hint_boundary_detects_next_marker_separator() {
        let tail = "fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw')";
        assert_eq!(find_hint_boundary(tail), Some(38));
    }

    #[test]
    fn extract_hf_error_hint_keeps_internal_paren_separator_not_followed_by_marker() {
        let raw = "HF 404: revision not found (requested revision: '4bpw'; available branches: weird)(branch), main)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("requested revision: '4bpw'; available branches: weird)(branch), main")
        );
    }

    #[test]
    fn default_models_dir_returns_non_empty_path() {
        assert!(!default_models_dir().trim().is_empty());
    }

    #[test]
    fn runtime_command_exists_accepts_current_exe_absolute_path() {
        let current = std::env::current_exe().unwrap();
        assert!(runtime_command_exists(&current.to_string_lossy()));
    }

    #[test]
    fn runtime_command_exists_rejects_missing_absolute_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing = tmp.path().join("missing-runtime-binary.exe");
        assert!(!runtime_command_exists(&missing.to_string_lossy()));
    }

    #[test]
    fn humanize_inference_spawn_error_explains_missing_xllm_binary() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let msg = humanize_inference_spawn_error("xllm", &err);
        assert!(msg.contains("xllm"), "got: {}", msg);
        assert!(msg.to_ascii_lowercase().contains("path"), "got: {}", msg);
        assert!(
            msg.to_ascii_lowercase().contains("binary path"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn humanize_inference_spawn_error_falls_back_for_other_errors() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let msg = humanize_inference_spawn_error("xllm", &err);
        assert!(msg.starts_with("xllm: "), "got: {}", msg);
    }

    #[test]
    fn humanize_inference_spawn_error_handles_tabby_cmd_alias() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access is denied");
        let msg = humanize_inference_spawn_error("tabby.cmd", &err);
        assert!(
            msg.contains("Tabby executable could not be started"),
            "got: {}",
            msg
        );
        assert!(msg.contains("tabby.cmd"), "got: {}", msg);
    }

    #[test]
    fn humanize_inference_spawn_error_vllm_not_found() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let msg = humanize_inference_spawn_error("vllm", &err);
        assert!(msg.contains("vllm"), "got: {}", msg);
        assert!(
            msg.to_ascii_lowercase().contains("binary path"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn humanize_inference_spawn_error_llama_server_not_found() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let msg = humanize_inference_spawn_error("llama-server", &err);
        assert!(msg.contains("llama-server"), "got: {}", msg);
        assert!(
            msg.to_ascii_lowercase().contains("binary path"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn humanize_inference_spawn_error_tabby_not_found_falls_back() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let msg = humanize_inference_spawn_error("tabby.exe", &err);
        assert!(msg.starts_with("tabby.exe:"), "got: {}", msg);
    }

    #[test]
    fn humanize_inference_spawn_error_tabby_korean_access_denied() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "액세스가 거부됨");
        let msg = humanize_inference_spawn_error("tabby.bat", &err);
        assert!(
            msg.contains("Tabby executable could not be started"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn humanize_inference_spawn_error_generic_fallback() {
        let err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let msg = humanize_inference_spawn_error("my-tool", &err);
        assert_eq!(msg, "my-tool: connection refused");
    }
}
