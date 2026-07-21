use std::time::{Duration, Instant};

use portable_pty::{Child, ExitStatus};

#[derive(Clone, Copy, Debug)]
pub(crate) struct PtyDeadlines {
    pub(super) graceful: Duration,
    pub(super) kill_reap: Duration,
}

impl PtyDeadlines {
    pub(crate) const fn new(graceful: Duration, kill_reap: Duration) -> Self {
        Self {
            graceful,
            kill_reap,
        }
    }
}

pub(crate) const PRODUCTION_DEADLINES: PtyDeadlines =
    PtyDeadlines::new(Duration::from_secs(5), Duration::from_secs(2));

#[derive(Clone, Debug)]
pub(crate) struct PtyReceipt {
    pub(crate) pid: u32,
    pub(crate) forced: bool,
    pub(crate) status: ExitStatus,
    pub(crate) elapsed: Duration,
}

pub(super) struct PtyReapFailure {
    pub(super) message: String,
    pub(super) child: Box<dyn Child + Send>,
}

pub(super) fn terminate_child(
    mut child: Box<dyn Child + Send>,
    deadlines: PtyDeadlines,
) -> Result<PtyReceipt, PtyReapFailure> {
    let Some(pid) = child.process_id() else {
        return Err(failure("PTY child PID unavailable".to_string(), child));
    };
    let started = Instant::now();
    let graceful_deadline = started + deadlines.graceful;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(receipt(pid, false, status, started.elapsed())),
            Ok(None) if Instant::now() < graceful_deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => break,
            Err(error) => {
                return Err(failure(format!("PTY graceful wait failed: {error}"), child));
            }
        }
    }

    if let Err(error) = child.kill() {
        return Err(failure(format!("PTY force-kill failed: {error}"), child));
    }
    let reap_deadline = Instant::now() + deadlines.kill_reap;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(receipt(pid, true, status, started.elapsed())),
            Ok(None) if Instant::now() < reap_deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                return Err(failure(
                    format!(
                        "PTY kill/reap deadline exceeded after {}s",
                        deadlines.kill_reap.as_secs_f64()
                    ),
                    child,
                ));
            }
            Err(error) => {
                return Err(failure(
                    format!("PTY reap after force-kill failed: {error}"),
                    child,
                ));
            }
        }
    }
}

fn failure(message: String, child: Box<dyn Child + Send>) -> PtyReapFailure {
    PtyReapFailure { message, child }
}

fn receipt(pid: u32, forced: bool, status: ExitStatus, elapsed: Duration) -> PtyReceipt {
    PtyReceipt {
        pid,
        forced,
        status,
        elapsed,
    }
}
