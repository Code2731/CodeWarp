// model/tabbyapi/mod.rs — TabbyAPI model helpers (model child module)
mod dir;

use dir::*;
use std::path::{Path, PathBuf};

pub(super) fn extract_bpw_hint(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for (idx, _) in lower.match_indices("bpw") {
        let mut start = idx;
        while start > 0 {
            let ch = bytes[start - 1];
            if ch.is_ascii_digit() || ch == b'.' {
                start -= 1;
            } else {
                break;
            }
        }
        if start < idx {
            return Some(lower[start..idx + 3].to_string());
        }
    }
    None
}

pub(super) fn resolve_tabbyapi_model_dir_with_hint(
    path: &Path,
    hint: Option<&str>,
) -> Option<PathBuf> {
    if is_valid_tabbyapi_model_dir_direct(path) {
        return Some(path.to_path_buf());
    }

    let candidates = tabbyapi_direct_model_children(path);
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return candidates.into_iter().next();
    }

    if let Some(bpw_hint) = hint.and_then(extract_bpw_hint) {
        let mut matched: Vec<PathBuf> = candidates
            .iter()
            .filter_map(|candidate| {
                let name = candidate.file_name().and_then(|n| n.to_str())?;
                if name.to_ascii_lowercase().contains(&bpw_hint) {
                    Some(candidate.clone())
                } else {
                    None
                }
            })
            .collect();
        if matched.len() == 1 {
            return matched.pop();
        }
    }

    None
}

pub(crate) fn resolve_tabbyapi_model_dir(path: &Path) -> Option<PathBuf> {
    resolve_tabbyapi_model_dir_with_hint(path, None)
}

pub(super) fn has_tabbyapi_model_dir(path: &Path) -> bool {
    is_valid_tabbyapi_model_dir_direct(path) || !tabbyapi_direct_model_children(path).is_empty()
}

pub(crate) fn resolve_tabbyapi_model_dir_for_folder(
    path: &Path,
    folder_name: &str,
) -> Option<PathBuf> {
    resolve_tabbyapi_model_dir_with_hint(path, Some(folder_name))
}

pub(super) fn is_downloaded_exl2_root(path: &Path) -> bool {
    has_tabbyapi_model_dir(path)
}

pub(super) fn is_downloaded_model_dir(path: &Path) -> bool {
    path.is_dir() && has_model_weight_file(path)
}

pub(crate) fn list_downloaded_models(dir: &Path) -> Vec<String> {
    use crate::util::resolve_user_path;
    let resolved_dir = resolve_user_path(&dir.to_string_lossy());
    if resolved_dir.as_os_str().is_empty() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(&resolved_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !is_downloaded_model_dir(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            out.push(name.to_string());
        }
    }
    out.sort_unstable();
    out
}

pub(crate) fn downloaded_model_path(dir: &str, folder_name: &str) -> PathBuf {
    use crate::util::resolve_user_path;
    resolve_user_path(dir).join(folder_name)
}

pub(super) fn exl2_repo_model_stem(repo_id: &str) -> Option<String> {
    let name = repo_id.rsplit('/').next()?.trim();
    if name.is_empty() {
        return None;
    }
    name.strip_suffix("-exl2")
        .or_else(|| name.strip_suffix("-EXL2"))
        .map(str::to_string)
}

pub(crate) fn downloaded_exl2_preset_folder(
    dir: &str,
    preset: &super::Exl2Preset,
) -> Option<String> {
    use crate::util::resolve_user_path;
    let root = resolve_user_path(dir);
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    let mut models: Vec<String> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && is_downloaded_exl2_root(p))
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect();
    models.sort_unstable();
    if let Some(exact) = models
        .iter()
        .find(|m| m.eq_ignore_ascii_case(preset.folder_name))
    {
        return Some(exact.clone());
    }

    let stem = exl2_repo_model_stem(preset.repo_id)?;
    let stem_prefix = format!("{}-", stem.to_ascii_lowercase());
    let mut matches: Vec<String> = models
        .into_iter()
        .filter(|m| {
            let lower = m.to_ascii_lowercase();
            lower.starts_with(&stem_prefix) && lower.contains("bpw")
        })
        .collect();
    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_model_extension ─────────────────────────────────────────────────────

    #[test]
    fn is_model_extension_accepts_bin() {
        assert!(is_model_extension("model.bin"));
    }

    #[test]
    fn is_model_extension_accepts_gguf() {
        assert!(is_model_extension("model.gguf"));
    }

    #[test]
    fn is_model_extension_accepts_pt() {
        assert!(is_model_extension("model.pt"));
    }

    #[test]
    fn is_model_extension_accepts_pth() {
        assert!(is_model_extension("model.pth"));
    }

    #[test]
    fn is_model_extension_rejects_uppercase() {
        // function is case-sensitive — extension must be lowercase
        assert!(!is_model_extension("MODEL.GGUF"));
    }

    #[test]
    fn is_model_extension_rejects_mixed_case() {
        assert!(!is_model_extension("Model.Bin"));
    }

    #[test]
    fn is_model_extension_accepts_multi_dot() {
        assert!(is_model_extension("model.backup.gguf"));
    }

    #[test]
    fn is_model_extension_rejects_other_ext() {
        assert!(!is_model_extension("model.txt"));
    }

    #[test]
    fn is_model_extension_rejects_safetensors() {
        assert!(!is_model_extension("model.safetensors"));
    }

    #[test]
    fn is_model_extension_rejects_substring_suffix() {
        assert!(!is_model_extension("model.bin2"));
    }

    #[test]
    fn is_model_extension_rejects_empty_string() {
        assert!(!is_model_extension(""));
    }

    #[test]
    fn is_model_extension_rejects_no_extension() {
        assert!(!is_model_extension("model"));
    }

    #[test]
    fn is_model_extension_rejects_just_dot() {
        assert!(!is_model_extension("model."));
    }

    #[test]
    fn is_model_extension_rejects_dot_only() {
        assert!(!is_model_extension("."));
    }

    // ── extract_bpw_hint ───────────────────────────────────────────────────────

    #[test]
    fn extract_bpw_hint_whole_string_4bpw() {
        assert_eq!(extract_bpw_hint("4bpw"), Some("4bpw".into()));
    }

    #[test]
    fn extract_bpw_hint_float_8dot0bpw() {
        assert_eq!(extract_bpw_hint("8.0bpw"), Some("8.0bpw".into()));
    }

    #[test]
    fn extract_bpw_hint_middle_of_text() {
        assert_eq!(
            extract_bpw_hint("llama-3.2-6.5bpw-h6"),
            Some("6.5bpw".into())
        );
    }

    #[test]
    fn extract_bpw_hint_uppercase() {
        assert_eq!(extract_bpw_hint("6.5BPW"), Some("6.5bpw".into()));
    }

    #[test]
    fn extract_bpw_hint_returns_first_match() {
        assert_eq!(extract_bpw_hint("4.0bpw-or-8.0bpw"), Some("4.0bpw".into()));
    }

    #[test]
    fn extract_bpw_hint_no_number_before() {
        assert_eq!(extract_bpw_hint("modelbpw"), None);
    }

    #[test]
    fn extract_bpw_hint_just_bpw_no_number() {
        assert_eq!(extract_bpw_hint("bpw"), None);
    }

    #[test]
    fn extract_bpw_hint_empty_string() {
        assert_eq!(extract_bpw_hint(""), None);
    }

    #[test]
    fn extract_bpw_hint_no_match() {
        assert_eq!(extract_bpw_hint("hello world"), None);
    }

    #[test]
    fn extract_bpw_hint_digits_only_before() {
        assert_eq!(extract_bpw_hint("42bpw"), Some("42bpw".into()));
    }

    #[test]
    fn extract_bpw_hint_leading_zero() {
        assert_eq!(extract_bpw_hint("0.5bpw"), Some("0.5bpw".into()));
    }

    #[test]
    fn extract_bpw_hint_trailing_dot() {
        assert_eq!(extract_bpw_hint("8.bpw"), Some("8.bpw".into()));
    }

    // ── exl2_repo_model_stem ───────────────────────────────────────────────────

    #[test]
    fn exl2_repo_model_stem_simple() {
        assert_eq!(
            exl2_repo_model_stem("author/ModelName-exl2"),
            Some("ModelName".into())
        );
    }

    #[test]
    fn exl2_repo_model_stem_uppercase_suffix() {
        assert_eq!(
            exl2_repo_model_stem("author/ModelName-EXL2"),
            Some("ModelName".into())
        );
    }

    #[test]
    fn exl2_repo_model_stem_no_author() {
        assert_eq!(exl2_repo_model_stem("model-exl2"), Some("model".into()));
    }

    #[test]
    fn exl2_repo_model_stem_no_suffix() {
        assert_eq!(exl2_repo_model_stem("author/ModelName"), None);
    }

    #[test]
    fn exl2_repo_model_stem_different_suffix() {
        assert_eq!(exl2_repo_model_stem("author/ModelName-other"), None);
    }

    #[test]
    fn exl2_repo_model_stem_empty_string() {
        assert_eq!(exl2_repo_model_stem(""), None);
    }

    #[test]
    fn exl2_repo_model_stem_trailing_slash() {
        assert_eq!(exl2_repo_model_stem("author/"), None);
    }

    #[test]
    fn exl2_repo_model_stem_only_suffix() {
        // "exl2" as the last component – "-exl2" suffix does NOT match
        assert_eq!(exl2_repo_model_stem("something/exl2"), None);
    }

    #[test]
    fn exl2_repo_model_stem_trimmed_whitespace_is_stripped() {
        assert_eq!(
            exl2_repo_model_stem("author/  model-exl2  "),
            Some("model".into())
        );
    }

    #[test]
    fn exl2_repo_model_stem_empty_after_trim() {
        assert_eq!(exl2_repo_model_stem("author/   "), None);
    }
}
