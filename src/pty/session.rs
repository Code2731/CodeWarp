use std::fmt;
use std::io::Write;
use std::sync::{Arc, Mutex};

use portable_pty::{Child, MasterPty};

use super::lifecycle;
use super::{PtyDeadlines, PtyReceipt};

#[derive(Clone)]
pub(crate) struct PtySession {
    pub(super) writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    pub(super) master: Arc<Mutex<Option<Box<dyn MasterPty + Send>>>>,
    pub(super) child: Arc<Mutex<Option<Box<dyn Child + Send>>>>,
    pub(super) deadlines: PtyDeadlines,
}

impl fmt::Debug for PtySession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PtySession")
            .field("pid", &self.pid())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PtyShutdownFailure {
    pub(crate) message: String,
    pub(crate) session: PtySession,
}

impl PtySession {
    pub(crate) fn write_line(&self, line: &str) {
        if let Ok(mut writer) = self.writer.lock()
            && let Some(writer) = writer.as_mut()
        {
            let _ = writer.write_all(line.as_bytes());
            #[cfg(windows)]
            let _ = writer.write_all(b"\r");
            #[cfg(not(windows))]
            let _ = writer.write_all(b"\n");
            let _ = writer.flush();
        }
    }

    pub(crate) fn write_bytes(&self, bytes: &[u8]) {
        if let Ok(mut writer) = self.writer.lock()
            && let Some(writer) = writer.as_mut()
        {
            let _ = writer.write_all(bytes);
            let _ = writer.flush();
        }
    }

    pub(crate) fn ctrl_c(&self) {
        #[cfg(windows)]
        self.write_bytes(b"\x1b[67;46;3;1;8;1_");
        #[cfg(not(windows))]
        self.write_bytes(&[0x03]);
    }

    pub(crate) fn pid(&self) -> Option<u32> {
        self.child
            .lock()
            .ok()
            .and_then(|child| child.as_ref().and_then(|child| child.process_id()))
    }

    pub(crate) async fn shutdown(self) -> Result<PtyReceipt, PtyShutdownFailure> {
        self.write_line("exit");
        self.reap().await
    }

    pub(crate) async fn wait_for_exit(self) -> Result<PtyReceipt, PtyShutdownFailure> {
        self.reap().await
    }

    async fn reap(self) -> Result<PtyReceipt, PtyShutdownFailure> {
        let child = match self.child.lock() {
            Ok(mut child) => Ok(child.take()),
            Err(_) => Err("PTY child lock poisoned"),
        };
        let child = match child {
            Ok(Some(child)) => child,
            Ok(None) => return Err(self.shutdown_failure("PTY child was already reaped")),
            Err(message) => return Err(self.shutdown_failure(message)),
        };
        let deadlines = self.deadlines;
        let result =
            match tokio::task::spawn_blocking(move || lifecycle::terminate_child(child, deadlines))
                .await
            {
                Ok(Ok(receipt)) => receipt,
                Ok(Err(failure)) => {
                    if let Ok(mut child) = self.child.lock() {
                        *child = Some(failure.child);
                    }
                    return Err(self.shutdown_failure(&failure.message));
                }
                Err(error) => {
                    return Err(self.shutdown_failure(&format!("PTY reap worker failed: {error}")));
                }
            };
        if let Ok(mut writer) = self.writer.lock() {
            drop(writer.take());
        }
        if let Ok(mut master) = self.master.lock() {
            drop(master.take());
        }
        Ok(result)
    }

    fn shutdown_failure(self, message: &str) -> PtyShutdownFailure {
        PtyShutdownFailure {
            message: message.to_string(),
            session: self,
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if Arc::strong_count(&self.child) != 1 {
            return;
        }
        if let Ok(mut writer) = self.writer.lock() {
            drop(writer.take());
        }
        if let Ok(mut master) = self.master.lock() {
            drop(master.take());
        }
        let child = self.child.lock().ok().and_then(|mut child| child.take());
        if let Some(child) = child {
            let deadlines = self.deadlines;
            drop(std::thread::spawn(move || {
                if let Err(mut failure) = lifecycle::terminate_child(child, deadlines) {
                    let _ = failure.child.kill();
                    let _ = failure.child.wait();
                }
            }));
        }
    }
}
