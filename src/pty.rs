// PTY 세션 관리 — portable-pty (ConPTY on Windows, POSIX PTY on Unix).
// 라인 입력 모드: 명령 입력창 → PTY stdin, PTY stdout → line stream.

use std::path::Path;
use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

mod lifecycle;
mod output;
mod session;
#[cfg(windows)]
mod windows;

pub(crate) use lifecycle::{PRODUCTION_DEADLINES, PtyDeadlines, PtyReceipt};
pub(crate) use session::{PtySession, PtyShutdownFailure};

/// PTY 이벤트 — Iced `Task::run`의 Item 타입.
#[derive(Debug, Clone)]
pub(crate) enum PtyEvent {
    /// 한 줄 출력 (ANSI 포함 raw line)
    Line(String),
    /// PTY 프로세스 종료
    Exited,
}

enum PtySignal {
    Line(String),
    ChildExited,
    OutputDrained,
}

/// PTY 세션을 spawn하고 (session 핸들, 출력 line stream)을 반환.
/// stream은 `Task::run`에 직접 전달 가능.
pub(crate) fn spawn_pty(
    cwd: &Path,
) -> Result<
    (
        PtySession,
        impl futures_util::Stream<Item = PtyEvent> + use<>,
    ),
    String,
> {
    spawn_pty_command(cwd, default_shell(), PRODUCTION_DEADLINES)
}

pub(crate) fn spawn_pty_command(
    cwd: &Path,
    mut command: CommandBuilder,
    deadlines: PtyDeadlines,
) -> Result<
    (
        PtySession,
        impl futures_util::Stream<Item = PtyEvent> + use<>,
    ),
    String,
> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 220,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("PTY 열기 실패: {e}"))?;

    command.cwd(cwd);

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|e| format!("셸 시작 실패: {e}"))?;

    let writer = match pair.master.take_writer() {
        Ok(writer) => Arc::new(Mutex::new(Some(writer))),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("writer 취득 실패: {error}"));
        }
    };
    let reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            if let Ok(mut writer) = writer.lock() {
                drop(writer.take());
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("reader 취득 실패: {error}"));
        }
    };

    let session = PtySession {
        writer,
        master: Arc::new(Mutex::new(Some(pair.master))),
        child: Arc::new(Mutex::new(Some(child))),
        deadlines,
    };

    // spawn_blocking으로 blocking reader를 tokio channel로 브릿지
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PtySignal>(512);
    let exit_child = Arc::clone(&session.child);
    let exit_writer = Arc::clone(&session.writer);
    let exit_master = Arc::clone(&session.master);
    let exit_tx = tx.clone();
    drop(std::thread::spawn(move || {
        loop {
            let exited = match exit_child.lock() {
                Ok(mut child) => match child.as_mut() {
                    Some(child) => child.try_wait().map_or(true, |status| status.is_some()),
                    None => return,
                },
                Err(_) => true,
            };
            if exited {
                if let Ok(mut writer) = exit_writer.lock() {
                    drop(writer.take());
                }
                if let Ok(mut master) = exit_master.lock() {
                    drop(master.take());
                }
                let _ = exit_tx.blocking_send(PtySignal::ChildExited);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }));
    let output_writer = Arc::clone(&session.writer);
    tokio::task::spawn_blocking(move || {
        output::forward(reader, output_writer, &tx);
    });

    let stream = async_stream::stream! {
        let mut child_exited = false;
        let mut output_drained = false;
        while let Some(signal) = rx.recv().await {
            match signal {
                PtySignal::Line(line) => yield PtyEvent::Line(line),
                PtySignal::ChildExited => child_exited = true,
                PtySignal::OutputDrained => output_drained = true,
            }
            if child_exited && output_drained {
                yield PtyEvent::Exited;
                break;
            }
        }
    };

    Ok((session, stream))
}

/// ANSI escape를 제거해 plain text로 변환.
pub(crate) fn strip_ansi(raw: &str) -> String {
    strip_ansi_escapes::strip_str(raw)
}

#[cfg(test)]
pub(crate) async fn test_lifecycle_lock() -> tokio::sync::OwnedMutexGuard<()> {
    static LOCK: std::sync::OnceLock<Arc<tokio::sync::Mutex<()>>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
        .lock_owned()
        .await
}

fn default_shell() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("cmd.exe")
    }
    #[cfg(not(windows))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        CommandBuilder::new(shell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_codes() {
        let raw = "\x1b[32mhello\x1b[0m world";
        assert_eq!(strip_ansi(raw), "hello world");
    }

    #[test]
    fn strip_ansi_plain_text_unchanged() {
        let raw = "cargo build --release";
        assert_eq!(strip_ansi(raw), raw);
    }

    #[test]
    fn strip_ansi_cursor_codes() {
        // 커서 이동 등도 제거
        let raw = "\x1b[2J\x1b[H$ prompt";
        assert_eq!(strip_ansi(raw), "$ prompt");
    }
}

#[cfg(test)]
mod lifecycle_tests;
