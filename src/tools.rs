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
        "write_file" => ToolKind::Mutating,
        _ => ToolKind::ReadOnly,
    }
}

#[derive(Debug, serde::Deserialize)]
struct ReadFileArgs {
    path: String,
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
        other => format!("[error] 알 수 없는 도구: {}", other),
    }
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
