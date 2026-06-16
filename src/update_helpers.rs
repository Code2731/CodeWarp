use super::*;

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
    use super::{default_models_dir, runtime_command_exists};

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
}
