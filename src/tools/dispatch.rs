use std::path::Path;

use super::args::{GlobArgs, GrepArgs, ReadFileArgs, RunCommandArgs, WriteFileArgs};
use super::exec;

pub(crate) fn dispatch(name: &str, arguments_json: &str, cwd: &Path) -> String {
    match name {
        "read_file" => match serde_json::from_str::<ReadFileArgs>(arguments_json) {
            Ok(args) => match exec::read_file(cwd, &args.path) {
                Ok(content) => content,
                Err(e) => format!("[error] {e}"),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {e}"),
        },
        "write_file" => match WriteFileArgs::parse(arguments_json) {
            Ok(args) => match exec::write_file(cwd, &args.path, &args.content) {
                Ok(()) => format!("[ok] {} 에 {} bytes 작성", args.path, args.content.len()),
                Err(e) => format!("[error] {e}"),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {e}"),
        },
        "run_command" => match RunCommandArgs::parse(arguments_json) {
            Ok(args) => exec::run_command(cwd, &args.command),
            Err(e) => format!("[error] arguments JSON 파싱 실패: {e}"),
        },
        "glob" => match serde_json::from_str::<GlobArgs>(arguments_json) {
            Ok(args) => match exec::glob_files(cwd, &args.pattern, 200) {
                Ok(paths) => {
                    if paths.is_empty() {
                        "0 matches".to_string()
                    } else {
                        format!("{} matches:\n{}", paths.len(), paths.join("\n"))
                    }
                }
                Err(e) => format!("[error] {e}"),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {e}"),
        },
        "grep" => match serde_json::from_str::<GrepArgs>(arguments_json) {
            Ok(args) => match exec::grep_files(cwd, &args.pattern, 300) {
                Ok(lines) => {
                    if lines.is_empty() {
                        "0 matches".to_string()
                    } else {
                        format!("{} matches:\n{}", lines.len(), lines.join("\n"))
                    }
                }
                Err(e) => format!("[error] {e}"),
            },
            Err(e) => format!("[error] arguments JSON 파싱 실패: {e}"),
        },
        other => format!("[error] 알 수 없는 도구: {other}"),
    }
}
