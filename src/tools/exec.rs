use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_READ_BYTES: u64 = 1_000_000;
const MAX_CMD_OUTPUT: usize = 100_000;

fn bounded_prefix(value: &str, max_bytes: usize) -> (&str, usize) {
    if value.len() <= max_bytes {
        return (value, 0);
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    (&value[..end], value.len() - end)
}

pub(super) fn glob_files(
    cwd: &Path,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<String>, String> {
    let glob = globset::Glob::new(pattern)
        .map_err(|e| format!("glob 패턴 오류: {e}"))?
        .compile_matcher();
    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(cwd).build() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(cwd) else {
            continue;
        };
        if glob.is_match(rel) {
            results.push(rel.display().to_string().replace('\\', "/"));
            if results.len() >= max_results {
                break;
            }
        }
    }
    Ok(results)
}

pub(super) fn grep_files(
    cwd: &Path,
    pattern: &str,
    max_lines: usize,
) -> Result<Vec<String>, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("정규식 오류: {e}"))?;
    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(cwd).build() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(cwd) else {
            continue;
        };
        let Ok(content) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let rel_str = rel.display().to_string().replace('\\', "/");
        for (lineno, line) in content.lines().enumerate() {
            if re.is_match(line) {
                let line_trimmed = if line.len() > 200 {
                    format!("{}…", &line[..200])
                } else {
                    line.to_string()
                };
                results.push(format!("{rel_str}:{}: {line_trimmed}", lineno + 1));
                if results.len() >= max_lines {
                    return Ok(results);
                }
            }
        }
    }
    Ok(results)
}

pub(super) fn run_command(cwd: &Path, command: &str) -> String {
    use std::process::Command;

    let mut run_cmd;
    #[cfg(windows)]
    {
        run_cmd = Command::new("cmd");
        run_cmd.args(["/C", command]);
    }
    #[cfg(not(windows))]
    {
        run_cmd = Command::new("sh");
        run_cmd.args(["-c", command]);
    }
    run_cmd.current_dir(cwd);

    let output = match run_cmd.output() {
        Ok(o) => o,
        Err(e) => return format!("[error] 명령 실행 실패: {e}"),
    };

    let mut result = String::new();
    let code = output.status.code().unwrap_or(-1);
    let _ = writeln!(result, "$ {command}");
    let _ = writeln!(result, "exit code: {code}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        result.push_str("--- stdout ---\n");
        let (prefix, omitted) = bounded_prefix(&stdout, MAX_CMD_OUTPUT);
        result.push_str(prefix);
        if omitted > 0 {
            let _ = write!(result, "\n…(stdout {omitted} bytes 잘림)\n");
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        result.push_str("--- stderr ---\n");
        let (prefix, omitted) = bounded_prefix(&stderr, MAX_CMD_OUTPUT);
        result.push_str(prefix);
        if omitted > 0 {
            let _ = write!(result, "\n…(stderr {omitted} bytes 잘림)");
        }
    }
    result
}

pub(super) fn write_file(cwd: &Path, rel_path: &str, content: &str) -> Result<(), String> {
    let candidate = PathBuf::from(rel_path);
    if candidate.is_absolute() {
        return Err("절대 경로는 허용되지 않습니다".into());
    }
    let joined = cwd.join(&candidate);
    let parent = joined.parent().ok_or("부모 경로 없음".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("부모 디렉토리 생성 실패: {e}"))?;
    let parent_canonical = parent
        .canonicalize()
        .map_err(|e| format!("부모 경로 해석 실패 ({}): {e}", parent.display()))?;
    let cwd_canonical = cwd
        .canonicalize()
        .map_err(|e| format!("작업 디렉토리 해석 실패: {e}"))?;
    if !parent_canonical.starts_with(&cwd_canonical) {
        return Err(format!(
            "작업 디렉토리 밖 경로: {}",
            parent_canonical.display()
        ));
    }
    match fs::symlink_metadata(&joined) {
        Ok(_) => {
            let existing_canonical = joined
                .canonicalize()
                .map_err(|e| format!("기존 대상 경로 해석 실패 ({}): {e}", joined.display()))?;
            if !existing_canonical.starts_with(&cwd_canonical) {
                return Err(format!(
                    "작업 디렉토리 밖 대상 덮어쓰기 차단: {}",
                    existing_canonical.display()
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "기존 대상 경로 검사 실패 ({}): {error}",
                joined.display()
            ));
        }
    }
    fs::write(&joined, content).map_err(|e| e.to_string())
}

pub(super) fn read_file(cwd: &Path, rel_path: &str) -> Result<String, String> {
    let candidate = PathBuf::from(rel_path);
    if candidate.is_absolute() {
        return Err("절대 경로는 허용되지 않습니다".into());
    }
    let joined = cwd.join(&candidate);
    let canonical = joined
        .canonicalize()
        .map_err(|e| format!("경로 해석 실패 ({}): {e}", joined.display()))?;
    let cwd_canonical = cwd
        .canonicalize()
        .map_err(|e| format!("작업 디렉토리 해석 실패: {e}"))?;
    if !canonical.starts_with(&cwd_canonical) {
        return Err(format!(
            "작업 디렉토리 밖 접근 차단: {}",
            canonical.display()
        ));
    }
    let metadata = fs::metadata(&canonical).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("파일이 아닙니다".into());
    }
    if metadata.len() > MAX_READ_BYTES {
        return Err(format!(
            "파일 크기가 너무 큼 ({} bytes, 한도 {})",
            metadata.len(),
            MAX_READ_BYTES
        ));
    }
    fs::read_to_string(&canonical).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::bounded_prefix;

    #[test]
    fn bounded_prefix_never_splits_utf8() {
        let value = "😊한글".repeat(40_000);

        let (prefix, omitted) = bounded_prefix(&value, 100_000);

        assert!(prefix.len() <= 100_000);
        assert!(prefix.is_char_boundary(prefix.len()));
        assert_eq!(prefix.len() + omitted, value.len());
    }
}
