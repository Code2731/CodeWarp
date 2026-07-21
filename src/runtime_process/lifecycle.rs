use std::fmt;
use std::io;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeDeadlines {
    pub(crate) startup: Duration,
    pub(crate) graceful: Duration,
    pub(crate) kill_reap: Duration,
}

impl RuntimeDeadlines {
    pub(crate) const fn new(startup: Duration, graceful: Duration, kill_reap: Duration) -> Self {
        Self {
            startup,
            graceful,
            kill_reap,
        }
    }
}

pub(crate) const PRODUCTION_DEADLINES: RuntimeDeadlines = RuntimeDeadlines::new(
    Duration::from_secs(10),
    Duration::from_secs(5),
    Duration::from_secs(2),
);

#[derive(Debug)]
pub(crate) struct RuntimeReceipt {
    pub(crate) pid: Option<u32>,
    pub(crate) forced: bool,
    pub(crate) status: ExitStatus,
}

#[derive(Debug)]
pub(crate) struct RuntimeFailure {
    pub(crate) message: String,
    pub(crate) receipt: Option<RuntimeReceipt>,
}

impl fmt::Display for RuntimeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(crate) struct RuntimeProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    deadlines: RuntimeDeadlines,
}

impl RuntimeProcess {
    pub(crate) fn spawn(
        mut command: Command,
        deadlines: RuntimeDeadlines,
    ) -> Result<(Self, ChildStdout, Option<ChildStderr>), io::Error> {
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let stdin = child.stdin.take();
        let mut process = Self {
            child: Some(child),
            stdin,
            deadlines,
        };
        if process.stdin.is_none() {
            return Err(io::Error::other("inference runtime stdin pipe unavailable"));
        }
        let stdout = process
            .child
            .as_mut()
            .and_then(|child| child.stdout.take())
            .ok_or_else(|| io::Error::other("inference runtime stdout pipe unavailable"))?;
        let stderr = process.child.as_mut().and_then(|child| child.stderr.take());
        Ok((process, stdout, stderr))
    }

    pub(crate) fn pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(Child::id)
    }

    pub(crate) fn close_stdin(&mut self) {
        drop(self.stdin.take());
    }

    pub(crate) async fn wait_until_healthy(
        &mut self,
        health_url: &str,
    ) -> Result<(), RuntimeFailure> {
        let request_timeout = self.deadlines.startup.min(Duration::from_secs(1));
        let client = match reqwest::Client::builder().timeout(request_timeout).build() {
            Ok(client) => client,
            Err(error) => {
                let receipt = self.shutdown().await.ok();
                return Err(RuntimeFailure {
                    message: format!("runtime health client creation failed: {error}"),
                    receipt,
                });
            }
        };
        let deadline = tokio::time::Instant::now() + self.deadlines.startup;

        loop {
            let child = self.child.as_mut().ok_or_else(|| RuntimeFailure {
                message: "runtime child was already reaped during startup".to_string(),
                receipt: None,
            })?;
            tokio::select! {
                status = child.wait() => {
                    let pid = child.id();
                    let receipt = status.map(|status| process_receipt(pid, status, false));
                    drop(self.child.take());
                    return Err(match receipt {
                        Ok(receipt) => RuntimeFailure {
                            message: format!("runtime exited before health response: {}", receipt.status),
                            receipt: Some(receipt),
                        },
                        Err(error) => RuntimeFailure {
                            message: format!("runtime wait failed during startup: {error}"),
                            receipt: None,
                        },
                    });
                }
                () = tokio::time::sleep_until(deadline) => {
                    let receipt = self.shutdown().await.ok();
                    return Err(RuntimeFailure {
                        message: format!(
                            "runtime startup deadline exceeded after {}s waiting for {health_url}",
                            self.deadlines.startup.as_secs_f64()
                        ),
                        receipt,
                    });
                }
                response = client.get(health_url).send() => {
                    if response.is_ok_and(|response| response.status().is_success()) {
                        return Ok(());
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub(crate) async fn wait_for_exit(&mut self) -> Result<RuntimeReceipt, String> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| "inference runtime child was already reaped".to_string())?;
        let pid = child.id();
        let status = child
            .wait()
            .await
            .map_err(|error| format!("inference runtime wait failed: {error}"))?;
        drop(self.child.take());
        Ok(process_receipt(pid, status, false))
    }

    pub(crate) async fn shutdown(&mut self) -> Result<RuntimeReceipt, String> {
        self.close_stdin();
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| "inference runtime child was already reaped".to_string())?;
        let receipt = terminate_child(child, self.deadlines).await?;
        drop(self.child.take());
        Ok(receipt)
    }
}

impl Drop for RuntimeProcess {
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
    deadlines: RuntimeDeadlines,
) -> Result<RuntimeReceipt, String> {
    let pid = child.id();
    match tokio::time::timeout(deadlines.graceful, child.wait()).await {
        Ok(Ok(status)) => Ok(process_receipt(pid, status, false)),
        Ok(Err(graceful_error)) => force_reap(child, deadlines, pid)
            .await
            .map_err(|error| format!("runtime graceful wait failed: {graceful_error}; {error}")),
        Err(_) => force_reap(child, deadlines, pid).await,
    }
}

async fn force_reap(
    child: &mut Child,
    deadlines: RuntimeDeadlines,
    pid: Option<u32>,
) -> Result<RuntimeReceipt, String> {
    let kill_error = child.start_kill().err();
    match tokio::time::timeout(deadlines.kill_reap, child.wait()).await {
        Ok(Ok(status)) => Ok(process_receipt(pid, status, true)),
        Ok(Err(error)) => Err(format!("runtime reap after force-kill failed: {error}")),
        Err(_) => Err(match kill_error {
            Some(error) => format!(
                "runtime force-kill failed: {error}; kill/reap deadline exceeded after {}s",
                deadlines.kill_reap.as_secs()
            ),
            None => format!(
                "runtime kill/reap deadline exceeded after {}s",
                deadlines.kill_reap.as_secs()
            ),
        }),
    }
}

fn process_receipt(pid: Option<u32>, status: ExitStatus, forced: bool) -> RuntimeReceipt {
    RuntimeReceipt {
        pid,
        forced,
        status,
    }
}
