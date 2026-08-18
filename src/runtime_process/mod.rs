use super::Message;
use std::path::PathBuf;

mod lifecycle;
mod output;
#[cfg(test)]
pub(crate) use lifecycle::RuntimeDeadlines;
use lifecycle::{PRODUCTION_DEADLINES, RuntimeFailure, RuntimeProcess, RuntimeReceipt};
use output::forward_output;

#[derive(Clone, Debug)]
pub(crate) struct RuntimeStopHandle {
    sender: tokio::sync::mpsc::Sender<()>,
}

impl RuntimeStopHandle {
    pub(crate) fn request_stop(&self) -> bool {
        match self.sender.try_send(()) {
            Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Full(())) => true,
            Err(tokio::sync::mpsc::error::TrySendError::Closed(())) => false,
        }
    }

    pub(crate) async fn stop_and_wait(&self) -> Result<(), String> {
        if !self.request_stop() && !self.sender.is_closed() {
            return Err("inference runtime stop request failed".to_string());
        }
        self.sender.closed().await;
        Ok(())
    }
}

pub(crate) struct InferenceLaunch {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) work_dir: Option<PathBuf>,
    pub(crate) health_url: String,
}

pub(crate) fn spawn_inference_stream(
    launch: InferenceLaunch,
    generation: u64,
) -> (impl futures_util::Stream<Item = Message>, RuntimeStopHandle) {
    use tokio::io::BufReader;
    use tokio::process::Command;

    let InferenceLaunch {
        program,
        args,
        work_dir,
        health_url,
    } = launch;
    let (stop_sender, mut stop_receiver) = tokio::sync::mpsc::channel(1);
    let stop_handle = RuntimeStopHandle {
        sender: stop_sender,
    };
    let stream = async_stream::stream! {
        let mut cmd = Command::new(&program);
        if let Some(dir) = work_dir {
            cmd.current_dir(dir);
        }
        cmd.args(&args);
        let (mut process, stdout, stderr) = match RuntimeProcess::spawn(cmd, PRODUCTION_DEADLINES) {
            Ok(parts) => parts,
            Err(e) => {
                yield Message::InferenceLogLine {
                    generation,
                    line: format!(
                        "[spawn 실패] {}",
                        humanize_inference_spawn_error(&program, &e)
                    ),
                };
                yield Message::InferenceExited { generation, code: -1 };
                return;
            }
        };
        if let Some(pid) = process.pid() {
            yield Message::InferenceLogLine {
                generation,
                line: format!("[pid:{pid}] {program} {}", args.join(" ")),
            };
        }
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(128);
        let stdout_tx = tx.clone();
        tokio::spawn(async move {
            let _read_result = forward_output(BufReader::new(stdout), stdout_tx, "").await;
        });
        if let Some(err) = stderr {
            let tx = tx.clone();
            tokio::spawn(async move {
                let _read_result = forward_output(BufReader::new(err), tx, "[err] ").await;
            });
        }
        drop(tx);

        let startup = tokio::select! {
            result = process.wait_until_healthy(&health_url) => result,
            _ = stop_receiver.recv() => {
                match process.shutdown().await {
                    Ok(receipt) => Err(RuntimeFailure {
                        message: "runtime startup cancelled".to_string(),
                        receipt: Some(receipt),
                    }),
                    Err(error) => Err(RuntimeFailure {
                        message: format!("runtime startup cancellation cleanup failed: {error}"),
                        receipt: None,
                    }),
                }
            }
        };
        if let Err(failure) = startup {
            yield Message::InferenceLogLine {
                generation,
                line: format!("[startup 실패] {}", failure.message),
            };
            let code = failure
                .receipt
                .as_ref()
                .map_or(-1, exit_code);
            yield Message::InferenceExited { generation, code };
            return;
        }
        yield Message::InferenceLogLine {
            generation,
            line: format!("[ready] {health_url}"),
        };
        yield Message::FetchTabbyModelsForInference(generation);

        let mut output_open = true;
        loop {
            tokio::select! {
                line = rx.recv(), if output_open => {
                    match line {
                        Some(l) => yield Message::InferenceLogLine { generation, line: l },
                        None => output_open = false,
                    }
                }
                _ = stop_receiver.recv() => {
                    let receipt = process.shutdown().await;
                    match receipt {
                        Ok(receipt) => {
                            yield Message::InferenceLogLine {
                                generation,
                                line: format!(
                                    "[stopped] pid={} forced={}",
                                    receipt.pid.map_or_else(|| "unknown".to_string(), |pid| pid.to_string()),
                                    receipt.forced,
                                ),
                            };
                            yield Message::InferenceExited {
                                generation,
                                code: exit_code(&receipt),
                            };
                        }
                        Err(error) => {
                            yield Message::InferenceLogLine {
                                generation,
                                line: format!("[cleanup 실패] {error}"),
                            };
                            yield Message::InferenceExited { generation, code: -1 };
                        }
                    }
                    return;
                }
                receipt = process.wait_for_exit() => {
                    match receipt {
                        Ok(receipt) => yield Message::InferenceExited {
                            generation,
                            code: exit_code(&receipt),
                        },
                        Err(error) => {
                            yield Message::InferenceLogLine {
                                generation,
                                line: format!("[cleanup 실패] {error}"),
                            };
                            yield Message::InferenceExited { generation, code: -1 };
                        }
                    }
                    return;
                }
            }
        }
    };
    (stream, stop_handle)
}

fn exit_code(receipt: &RuntimeReceipt) -> i32 {
    receipt.status.code().unwrap_or(-1)
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
            "{program_name} binary was not found. Set Runtime > binary path to the executable (for example xllm.exe) or add it to PATH. Raw error: {raw}"
        );
    }

    if matches!(program_name.as_str(), "llama-server" | "llama-server.exe")
        && err.kind() == std::io::ErrorKind::NotFound
    {
        return format!(
            "{program_name} binary was not found. Set Runtime > binary path to the executable or add it to PATH. Raw error: {raw}"
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
                "Tabby executable could not be started. The tabby/tabby.exe/tabby.cmd/tabby.bat on PATH may not be a runnable TabbyML server CLI, or there may be a permission/alias issue: {raw}"
            );
        }
    }

    format!("{program}: {raw}")
}

#[cfg(test)]
mod lifecycle_contract_tests;
#[cfg(test)]
mod output_contract_tests;
#[cfg(test)]
mod tests;
