mod args;
mod dispatch;
mod exec;

#[cfg(test)]
mod tests;

pub(crate) use args::{RunCommandArgs, WriteFileArgs};
pub(crate) use dispatch::dispatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolKind {
    ReadOnly,
    Mutating,
}

pub(crate) fn tool_kind(name: &str) -> ToolKind {
    match name {
        "write_file" | "run_command" => ToolKind::Mutating,
        _ => ToolKind::ReadOnly,
    }
}

pub(crate) fn tool_definitions(allow_mutating: bool) -> serde_json::Value {
    let mut tools = read_only_tools();
    if allow_mutating
        && let serde_json::Value::Array(arr) = &mut tools
        && let serde_json::Value::Array(muts) = mutating_tools()
    {
        arr.extend(muts);
    }
    tools
}

fn read_only_tools() -> serde_json::Value {
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
        }
    ])
}

fn mutating_tools() -> serde_json::Value {
    serde_json::json!([
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
