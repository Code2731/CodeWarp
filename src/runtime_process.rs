use super::Message;
use std::path::PathBuf;

/// Spawn an inference server and emit process metadata/stdout/stderr as app messages.
/// The first successful message includes `[pid:NNN]` so the app can track the child.
pub(crate) fn spawn_inference_stream(
    program: String,
    args: Vec<String>,
    work_dir: Option<PathBuf>,
) -> impl futures_util::Stream<Item = Message> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    async_stream::stream! {
        let mut cmd = Command::new(&program);
        if let Some(dir) = work_dir {
            cmd.current_dir(dir);
        }
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                yield Message::InferenceLogLine(format!(
                    "[spawn 실패] {}",
                    humanize_inference_spawn_error(&program, &e)
                ));
                yield Message::InferenceExited(-1);
                return;
            }
        };
        if let Some(pid) = child.id() {
            yield Message::InferenceLogLine(format!("[pid:{}] {} {}", pid, program, args.join(" ")));
        }
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        if let Some(out) = stdout {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(out).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        if let Some(err) = stderr {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(err).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(format!("[err] {}", line)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        let mut child_done = false;
        let mut exit_code: i32 = 0;
        loop {
            tokio::select! {
                line = rx.recv() => {
                    match line {
                        Some(l) => yield Message::InferenceLogLine(l),
                        None => {
                            if child_done { break; }
                        }
                    }
                }
                status = child.wait(), if !child_done => {
                    child_done = true;
                    exit_code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
                }
            }
        }
        yield Message::InferenceExited(exit_code);
    }
}

pub(crate) fn humanize_inference_spawn_error(program: &str, err: &std::io::Error) -> String {
    let raw = err.to_string();
    let program_name = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program)
        .to_ascii_lowercase();

    if matches!(
        program_name.as_str(),
        "xllm" | "xllm.exe" | "vllm" | "vllm.exe"
    ) && err.kind() == std::io::ErrorKind::NotFound
    {
        return format!(
            "{} binary was not found. Set Runtime > binary path to the executable (for example xllm.exe) or add it to PATH. Raw error: {}",
            program_name, raw
        );
    }

    if matches!(program_name.as_str(), "llama-server" | "llama-server.exe")
        && err.kind() == std::io::ErrorKind::NotFound
    {
        return format!(
            "{} binary was not found. Set Runtime > binary path to the executable or add it to PATH. Raw error: {}",
            program_name, raw
        );
    }

    if matches!(
        program_name.as_str(),
        "tabby" | "tabby.exe" | "tabby.cmd" | "tabby.bat"
    ) {
        let lower = raw.to_ascii_lowercase();
        if lower.contains("access is denied")
            || raw.contains("액세스가 거부")
            || raw.contains("응용 프로그램")
            || raw.contains("연결")
        {
            return format!(
                "Tabby executable could not be started. The tabby/tabby.exe/tabby.cmd/tabby.bat on PATH may not be a runnable TabbyML server CLI, or there may be a permission/alias issue: {}",
                raw
            );
        }
    }

    format!("{}: {}", program, raw)
}
