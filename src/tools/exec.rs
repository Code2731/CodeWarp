use std::fs;
use std::path::{Path, PathBuf};

const MAX_READ_BYTES: u64 = 1_000_000;
const MAX_CMD_OUTPUT: usize = 100_000;

pub(super) fn glob_files(
    cwd: &Path,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<String>, String> {
    let glob = globset::Glob::new(pattern)
        .map_err(|e| format!("glob 패턴 오류: {}", e))?
        .compile_matcher();
    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(cwd).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let rel = match entry.path().strip_prefix(cwd) {
            Ok(p) => p,
            Err(_) => continue,
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
    let re = regex::Regex::new(pattern).map_err(|e| format!("정규식 오류: {}", e))?;
    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(cwd).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let rel = match entry.path().strip_prefix(cwd) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let content = match fs::read_to_string(entry.path()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let rel_str = rel.display().to_string().replace('\\', "/");
        for (lineno, line) in content.lines().enumerate() {
            if re.is_match(line) {
                let line_trimmed = if line.len() > 200 {
                    format!("{}…", &line[..200])
                } else {
                    line.to_string()
                };
                results.push(format!("{}:{}: {}", rel_str, lineno + 1, line_trimmed));
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

    let mut cmd;
    #[cfg(windows)]
    {
        cmd = Command::new("cmd");
        cmd.args(["/C", command]);
    }
    #[cfg(not(windows))]
    {
        cmd = Command::new("sh");
        cmd.args(["-c", command]);
    }
    cmd.current_dir(cwd);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => return format!("[error] 명령 실행 실패: {}", e),
    };

    let mut result = String::new();
    let code = output.status.code().unwrap_or(-1);
    result.push_str(&format!("$ {}\n", command));
    result.push_str(&format!("exit code: {}\n", code));

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        result.push_str("--- stdout ---\n");
        if stdout.len() > MAX_CMD_OUTPUT {
            result.push_str(&stdout[..MAX_CMD_OUTPUT]);
            result.push_str(&format!(
                "\n…(stdout {} bytes 잘림)\n",
                stdout.len() - MAX_CMD_OUTPUT
            ));
        } else {
            result.push_str(&stdout);
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        result.push_str("--- stderr ---\n");
        if stderr.len() > MAX_CMD_OUTPUT {
            result.push_str(&stderr[..MAX_CMD_OUTPUT]);
            result.push_str(&format!(
                "\n…(stderr {} bytes 잘림)",
                stderr.len() - MAX_CMD_OUTPUT
            ));
        } else {
            result.push_str(&stderr);
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
    let parent = joined
        .parent()
        .ok_or_else(|| "부모 경로 없음".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("부모 디렉토리 생성 실패: {}", e))?;
    let parent_canonical = parent
        .canonicalize()
        .map_err(|e| format!("부모 경로 해석 실패 ({}): {}", parent.display(), e))?;
    let cwd_canonical = cwd
        .canonicalize()
        .map_err(|e| format!("작업 디렉토리 해석 실패: {}", e))?;
    if !parent_canonical.starts_with(&cwd_canonical) {
        return Err(format!(
            "작업 디렉토리 밖 경로: {}",
            parent_canonical.display()
        ));
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
        .map_err(|e| format!("경로 해석 실패 ({}): {}", joined.display(), e))?;
    let cwd_canonical = cwd
        .canonicalize()
        .map_err(|e| format!("작업 디렉토리 해석 실패: {}", e))?;
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
