// AI 에이전트가 호출하는 로컬 도구. Phase 2-5a는 read_file 만.
// 안전성: working directory 밖의 경로 접근 차단, 파일 크기 1MB 제한.

use std::fs;
use std::path::{Path, PathBuf};

const MAX_READ_BYTES: u64 = 1_000_000;

pub fn tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "작업 디렉토리 내의 텍스트 파일을 읽어 그 내용을 반환합니다. 절대 경로는 거부됩니다.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "작업 디렉토리 기준 상대 경로 (예: 'src/main.rs')"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "glob",
                "description": "작업 디렉토리에서 glob 패턴(예: '**/*.rs', 'src/**/*.toml')에 매칭되는 파일 경로 리스트를 반환합니다. .gitignore 자동 존중. 결과는 최대 200개.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "glob 패턴 (예: '**/*.rs')"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "grep",
                "description": "작업 디렉토리의 모든 파일에서 정규식 패턴을 검색하여 매칭되는 라인을 'path:lineno: line' 형식으로 반환합니다. .gitignore 자동 존중. 결과는 최대 300줄.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Rust regex 문법의 정규식 패턴 (예: 'fn\\s+main')"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "작업 디렉토리에서 셸 명령(Windows: cmd /C, Unix: sh -c)을 실행하고 stdout/stderr/exit code를 반환합니다. 부작용이 있을 수 있으므로 사용자 승인 후 실행됩니다.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "실행할 셸 명령 (예: 'cargo check', 'ls -la', 'git status')"
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "작업 디렉토리 내의 파일에 새 내용을 작성/덮어씁니다. 사용자 승인이 필요합니다. 절대 경로는 거부됩니다.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "작업 디렉토리 기준 상대 경로 (예: 'src/main.rs')"
                        },
                        "content": {
                            "type": "string",
                            "description": "파일에 쓸 전체 내용 (UTF-8 텍스트)"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }
    ])
}

/// 도구 종류 분류 — 부수효과가 있는 도구는 사용자 승인 후에만 실행.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// 부수효과 없음 (즉시 실행 OK)
    ReadOnly,
    /// 파일 시스템 변경 (사용자 승인 필요)
    Mutating,
}

pub fn tool_kind(name: &str) -> ToolKind {
    match name {
        "write_file" | "run_command" => ToolKind::Mutating,
        _ => ToolKind::ReadOnly,
    }
}

#[derive(Debug, serde::Deserialize)]
struct ReadFileArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct GlobArgs {
    pattern: String,
}

#[derive(Debug, serde::Deserialize)]
struct GrepArgs {
    pattern: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

impl WriteFileArgs {
    pub fn parse(arguments_json: &str) -> Result<Self, String> {
        serde_json::from_str(arguments_json).map_err(|e| e.to_string())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RunCommandArgs {
    pub command: String,
}

impl RunCommandArgs {
    pub fn parse(arguments_json: &str) -> Result<Self, String> {
        serde_json::from_str(arguments_json).map_err(|e| e.to_string())
    }
}

/// 도구 호출 결과를 OpenRouter `role: "tool"` 메시지의 content로 그대로 사용 가능한 문자열.
pub fn dispatch(name: &str, arguments_json: &str, cwd: &Path) -> String {
    match name {
        "read_file" => match serde_json::from_str::<ReadFileArgs>(arguments_json) {
            Ok(args) => match read_file(cwd, &args.path) {
                Ok(content) => content,
                Err(e) => format!("[error] {}", e),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {}", e),
        },
        "write_file" => match WriteFileArgs::parse(arguments_json) {
            Ok(args) => match write_file(cwd, &args.path, &args.content) {
                Ok(()) => format!("[ok] {} 에 {} bytes 작성", args.path, args.content.len()),
                Err(e) => format!("[error] {}", e),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {}", e),
        },
        "run_command" => match RunCommandArgs::parse(arguments_json) {
            Ok(args) => run_command(cwd, &args.command),
            Err(e) => format!("[error] arguments JSON 파싱 실패: {}", e),
        },
        "glob" => match serde_json::from_str::<GlobArgs>(arguments_json) {
            Ok(args) => match glob_files(cwd, &args.pattern, 200) {
                Ok(paths) => {
                    if paths.is_empty() {
                        "0 matches".to_string()
                    } else {
                        format!("{} matches:\n{}", paths.len(), paths.join("\n"))
                    }
                }
                Err(e) => format!("[error] {}", e),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {}", e),
        },
        "grep" => match serde_json::from_str::<GrepArgs>(arguments_json) {
            Ok(args) => match grep_files(cwd, &args.pattern, 300) {
                Ok(lines) => {
                    if lines.is_empty() {
                        "0 matches".to_string()
                    } else {
                        format!("{} matches:\n{}", lines.len(), lines.join("\n"))
                    }
                }
                Err(e) => format!("[error] {}", e),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {}", e),
        },
        other => format!("[error] 알 수 없는 도구: {}", other),
    }
}

fn glob_files(cwd: &Path, pattern: &str, max_results: usize) -> Result<Vec<String>, String> {
    let glob = globset::Glob::new(pattern)
        .map_err(|e| format!("glob 패턴 오류: {}", e))?
        .compile_matcher();
    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(cwd).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
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

fn grep_files(cwd: &Path, pattern: &str, max_lines: usize) -> Result<Vec<String>, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("정규식 오류: {}", e))?;
    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(cwd).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let rel = match entry.path().strip_prefix(cwd) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let content = match fs::read_to_string(entry.path()) {
            Ok(s) => s,
            Err(_) => continue, // 바이너리/권한 문제 등 스킵
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

const MAX_CMD_OUTPUT: usize = 100_000;

fn run_command(cwd: &Path, command: &str) -> String {
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

fn write_file(cwd: &Path, rel_path: &str, content: &str) -> Result<(), String> {
    let candidate = PathBuf::from(rel_path);
    if candidate.is_absolute() {
        return Err("절대 경로는 허용되지 않습니다".into());
    }
    let joined = cwd.join(&candidate);
    // 새 파일도 허용해야 하므로 부모 디렉토리만 canonicalize 비교.
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

fn read_file(cwd: &Path, rel_path: &str) -> Result<String, String> {
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
