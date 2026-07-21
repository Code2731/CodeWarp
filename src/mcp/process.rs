use std::fmt;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

#[derive(Clone, Copy, Debug)]
pub(super) struct McpDeadlines {
    pub(super) response: Duration,
    pub(super) graceful: Duration,
    pub(super) kill_reap: Duration,
}

impl McpDeadlines {
    pub(super) const fn new(response: Duration, graceful: Duration, kill_reap: Duration) -> Self {
        Self {
            response,
            graceful,
            kill_reap,
        }
    }
}

pub(super) const PRODUCTION_DEADLINES: McpDeadlines = McpDeadlines::new(
    Duration::from_secs(10),
    Duration::from_secs(5),
    Duration::from_secs(2),
);

#[derive(Debug)]
pub(super) struct ProcessReceipt {
    #[cfg(test)]
    pub(super) pid: Option<u32>,
    #[cfg(test)]
    pub(super) forced: bool,
    #[cfg(test)]
    pub(super) status: ExitStatus,
}

#[derive(Debug)]
pub(super) struct RpcFailure {
    pub(super) message: String,
    #[cfg(test)]
    pub(super) receipt: Option<ProcessReceipt>,
}

impl RpcFailure {
    pub(super) fn new(message: String, _receipt: Option<ProcessReceipt>) -> Self {
        Self {
            message,
            #[cfg(test)]
            receipt: _receipt,
        }
    }
}

impl fmt::Display for RpcFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(super) struct McpProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    deadlines: McpDeadlines,
}

impl McpProcess {
    pub(super) fn spawn(
        mut command: Command,
        deadlines: McpDeadlines,
    ) -> Result<(Self, ChildStdout), String> {
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let child = command
            .spawn()
            .map_err(|error| format!("MCP 서버 시작 실패: {error}"))?;
        let mut process = Self {
            child: Some(child),
            stdin: None,
            deadlines,
        };
        process.stdin = process.child.as_mut().and_then(|child| child.stdin.take());
        if process.stdin.is_none() {
            return Err("MCP stdin pipe 열기 실패".to_string());
        }
        let stdout = process
            .child
            .as_mut()
            .and_then(|child| child.stdout.take())
            .ok_or_else(|| "MCP stdout pipe 열기 실패".to_string())?;
        Ok((process, stdout))
    }

    pub(super) fn stdin_mut(&mut self) -> Result<&mut ChildStdin, String> {
        self.stdin
            .as_mut()
            .ok_or_else(|| "MCP stdin pipe가 닫힘".to_string())
    }

    pub(super) async fn shutdown(&mut self) -> Result<ProcessReceipt, String> {
        drop(self.stdin.take());
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| "MCP child가 이미 정리됨".to_string())?;
        let result = terminate_child(child, self.deadlines).await;
        if result.is_ok() {
            drop(self.child.take());
        }
        result
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let Some(mut child) = self.child.take() else {
            return;
        };
        let deadlines = self.deadlines;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            drop(handle.spawn(async move {
                let _ = terminate_child(&mut child, deadlines).await;
            }));
        } else {
            let _ = child.start_kill();
        }
    }
}

async fn terminate_child(
    child: &mut Child,
    deadlines: McpDeadlines,
) -> Result<ProcessReceipt, String> {
    let pid = child.id();
    match tokio::time::timeout(deadlines.graceful, child.wait()).await {
        Ok(Ok(status)) => Ok(process_receipt(pid, status, false)),
        Ok(Err(graceful_error)) => force_reap(child, deadlines, pid)
            .await
            .map_err(|error| format!("MCP graceful wait 실패: {graceful_error}; {error}")),
        Err(_) => force_reap(child, deadlines, pid).await,
    }
}

async fn force_reap(
    child: &mut Child,
    deadlines: McpDeadlines,
    pid: Option<u32>,
) -> Result<ProcessReceipt, String> {
    let kill_error = child.start_kill().err();
    match tokio::time::timeout(deadlines.kill_reap, child.wait()).await {
        Ok(Ok(status)) => Ok(process_receipt(pid, status, true)),
        Ok(Err(error)) => Err(format!("MCP kill 후 reap 실패: {error}")),
        Err(_) => Err(match kill_error {
            Some(error) => format!(
                "MCP 강제 종료 실패: {error}; kill/reap deadline exceeded after {}s",
                deadlines.kill_reap.as_secs()
            ),
            None => format!(
                "MCP kill/reap deadline exceeded after {}s",
                deadlines.kill_reap.as_secs()
            ),
        }),
    }
}

fn process_receipt(_pid: Option<u32>, _status: ExitStatus, _forced: bool) -> ProcessReceipt {
    ProcessReceipt {
        #[cfg(test)]
        pid: _pid,
        #[cfg(test)]
        forced: _forced,
        #[cfg(test)]
        status: _status,
    }
}
