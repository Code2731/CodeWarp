// model/tabbyapi.rs — TabbyAPI model directory helpers (model child module)
use super::Exl2Preset;
use std::path::{Path, PathBuf};

/// 모델 매니저 다운로드 폴더 안의 받은 모델(서브폴더) 리스트.
/// 빈 폴더는 모델 아님 — skip.
pub(crate) fn has_model_weight_file(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if has_model_weight_file(&path) {
                return true;
            }
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let file_name = file_name.to_ascii_lowercase();
        if file_name.ends_with(".safetensors")
            || file_name.ends_with(".bin")
            || file_name.ends_with(".gguf")
            || file_name.ends_with(".pt")
            || file_name.ends_with(".pth")
        {
            return true;
        }
    }

    false
}

pub(crate) fn is_valid_tabbyapi_model_dir_direct(path: &Path) -> bool {
    path.is_dir() && path.join("config.json").is_file() && has_model_weight_file(path)
}

pub(crate) fn tabbyapi_direct_model_children(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_valid_tabbyapi_model_dir_direct(p))
        .collect()
}

pub(crate) fn extract_bpw_hint(text: &str) -> Option<String> {
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

pub(crate) fn resolve_tabbyapi_model_dir_with_hint(
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

pub(crate) fn has_tabbyapi_model_dir(path: &Path) -> bool {
    is_valid_tabbyapi_model_dir_direct(path) || !tabbyapi_direct_model_children(path).is_empty()
}

pub(crate) fn resolve_tabbyapi_model_dir_for_folder(
    path: &Path,
    folder_name: &str,
) -> Option<PathBuf> {
    resolve_tabbyapi_model_dir_with_hint(path, Some(folder_name))
}

pub(crate) fn is_downloaded_exl2_root(path: &Path) -> bool {
    has_tabbyapi_model_dir(path)
}

pub(crate) fn is_downloaded_model_dir(path: &Path) -> bool {
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

pub(crate) fn exl2_repo_model_stem(repo_id: &str) -> Option<String> {
    let name = repo_id.rsplit('/').next()?.trim();
    if name.is_empty() {
        return None;
    }
    name.strip_suffix("-exl2")
        .or_else(|| name.strip_suffix("-EXL2"))
        .map(str::to_string)
}

pub(crate) fn downloaded_exl2_preset_folder(dir: &str, preset: &Exl2Preset) -> Option<String> {
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
