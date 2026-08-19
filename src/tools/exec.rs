use std::fmt::Write;
use std::fs;
use std::io::Read;
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

fn drain_command_stream(mut reader: impl Read) -> (Vec<u8>, usize) {
    let mut kept = Vec::with_capacity(MAX_CMD_OUTPUT);
    let mut discarded = 0_usize;
    let mut buffer = [0_u8; 8192];
    loop {
        let Ok(read) = reader.read(&mut buffer) else {
            break;
        };
        if read == 0 {
            break;
        }
        let remaining = MAX_CMD_OUTPUT.saturating_sub(kept.len());
        let keep = read.min(remaining);
        kept.extend_from_slice(&buffer[..keep]);
        discarded = discarded.saturating_add(read - keep);
    }
    (kept, discarded)
}

fn read_text_bounded(path: &Path, max_bytes: u64) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > max_bytes {
        return Err(format!("file exceeds {max_bytes} bytes"));
    }
    let capacity = usize::try_from(metadata.len()).map_err(|_| "file is too large".to_string())?;
    let mut bytes = Vec::with_capacity(capacity);
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("file exceeds {max_bytes} bytes"));
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
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
        let Ok(content) = read_text_bounded(entry.path(), MAX_READ_BYTES) else {
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

    run_cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match run_cmd.spawn() {
        Ok(child) => child,
        Err(e) => return format!("[error] 명령 실행 실패: {e}"),
    };
    let stdout_thread = child
        .stdout
        .take()
        .map(|stdout| std::thread::spawn(move || drain_command_stream(stdout)));
    let stderr_thread = child
        .stderr
        .take()
        .map(|stderr| std::thread::spawn(move || drain_command_stream(stderr)));
    let status = child.wait();
    let (stdout, stdout_discarded) = stdout_thread
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    let (stderr, stderr_discarded) = stderr_thread
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    let status = match status {
        Ok(status) => status,
        Err(e) => return format!("[error] 명령 실행 실패: {e}"),
    };

    let mut result = String::new();
    let code = status.code().unwrap_or(-1);
    let _ = writeln!(result, "$ {command}");
    let _ = writeln!(result, "exit code: {code}");

    let stdout = String::from_utf8_lossy(&stdout);
    if !stdout.trim().is_empty() {
        result.push_str("--- stdout ---\n");
        let (prefix, omitted) = bounded_prefix(&stdout, MAX_CMD_OUTPUT);
        let omitted = omitted.saturating_add(stdout_discarded);
        result.push_str(prefix);
        if omitted > 0 {
            let _ = write!(result, "\n…(stdout {omitted} bytes 잘림)\n");
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }
    let stderr = String::from_utf8_lossy(&stderr);
    if !stderr.trim().is_empty() {
        result.push_str("--- stderr ---\n");
        let (prefix, omitted) = bounded_prefix(&stderr, MAX_CMD_OUTPUT);
        let omitted = omitted.saturating_add(stderr_discarded);
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

pub(crate) fn read_file(cwd: &Path, rel_path: &str) -> Result<String, String> {
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
    read_text_bounded(&canonical, MAX_READ_BYTES)
}

#[cfg(test)]
mod tests {
    use super::{MAX_CMD_OUTPUT, bounded_prefix, run_command};

    #[test]
    fn bounded_prefix_never_splits_utf8() {
        let value = "😊한글".repeat(40_000);

        let (prefix, omitted) = bounded_prefix(&value, 100_000);

        assert!(prefix.len() <= 100_000);
        assert!(prefix.is_char_boundary(prefix.len()));
        assert_eq!(prefix.len() + omitted, value.len());
    }

    #[test]
    fn run_command_drains_large_output_with_bounded_result() {
        let command = if cfg!(windows) {
            "for /L %i in (1,1,200000) do @echo x"
        } else {
            "yes x | head -n 200000"
        };

        let result = run_command(std::path::Path::new("."), command);

        assert!(
            result.contains("잘림"),
            "large output must be marked: {result}"
        );
        assert!(result.len() <= MAX_CMD_OUTPUT + 256);
    }
}
