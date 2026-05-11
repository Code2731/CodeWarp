// PTY 세션 관리 — portable-pty (ConPTY on Windows, POSIX PTY on Unix).
// 라인 입력 모드: 명령 입력창 → PTY stdin, PTY stdout → line stream.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

/// PTY 이벤트 — Iced `Task::run`의 Item 타입.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// 한 줄 출력 (ANSI 포함 raw line)
    Line(String),
    /// PTY 프로세스 종료
    Exited,
}

/// PTY 세션 write 핸들. Clone 가능 (Arc<Mutex> 내부).
#[derive(Clone)]
pub struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl PtySession {
    pub fn write_line(&self, line: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = writeln!(w, "{}", line);
        }
    }

    pub fn write_bytes(&self, bytes: &[u8]) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(bytes);
        }
    }

    /// Ctrl+C (ETX)
    pub fn ctrl_c(&self) {
        self.write_bytes(&[0x03]);
    }
}

/// PTY 세션을 spawn하고 (session 핸들, 출력 line stream)을 반환.
/// stream은 `Task::run`에 직접 전달 가능.
pub fn spawn_pty(
    cwd: &Path,
) -> Result<(PtySession, impl futures_util::Stream<Item = PtyEvent>), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 220,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("PTY 열기 실패: {e}"))?;

    let mut cmd = default_shell();
    cmd.cwd(cwd);

    pair.slave
        .spawn_command(cmd)
        .map_err(|e| format!("셸 시작 실패: {e}"))?;

    let writer = Arc::new(Mutex::new(
        pair.master
            .take_writer()
            .map_err(|e| format!("writer 취득 실패: {e}"))?,
    ));

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("reader 취득 실패: {e}"))?;

    let session = PtySession { writer };

    // spawn_blocking으로 blocking reader를 tokio channel로 브릿지
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PtyEvent>(512);
    tokio::task::spawn_blocking(move || {
        let buf = BufReader::new(reader);
        for line in buf.lines() {
            match line {
                Ok(l) => {
                    if tx.blocking_send(PtyEvent::Line(l)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.blocking_send(PtyEvent::Exited);
                    break;
                }
            }
        }
    });

    let stream = async_stream::stream! {
        while let Some(event) = rx.recv().await {
            let done = matches!(event, PtyEvent::Exited);
            yield event;
            if done { break; }
        }
    };

    Ok((session, stream))
}

/// ANSI escape를 제거해 plain text로 변환.
pub fn strip_ansi(raw: &str) -> String {
    strip_ansi_escapes::strip_str(raw)
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
