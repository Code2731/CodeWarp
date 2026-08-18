use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use super::PtySignal;

#[cfg(windows)]
const CURSOR_QUERY: &[u8] = b"\x1b[6n";

#[cfg(windows)]
fn forward_utf8_chunk(
    pending: &mut Vec<u8>,
    chunk: &[u8],
    sender: &tokio::sync::mpsc::Sender<PtySignal>,
) -> bool {
    pending.extend_from_slice(chunk);
    match std::str::from_utf8(pending) {
        Ok(text) => {
            let text = text.to_owned();
            pending.clear();
            text.is_empty() || sender.blocking_send(PtySignal::Line(text)).is_ok()
        }
        Err(error) if error.error_len().is_none() => {
            let valid_up_to = error.valid_up_to();
            if valid_up_to == 0 {
                return true;
            }
            let text = match std::str::from_utf8(&pending[..valid_up_to]) {
                Ok(text) => text.to_owned(),
                Err(_) => return false,
            };
            pending.drain(..valid_up_to);
            text.is_empty() || sender.blocking_send(PtySignal::Line(text)).is_ok()
        }
        Err(_) => {
            let text = String::from_utf8_lossy(pending).into_owned();
            pending.clear();
            text.is_empty() || sender.blocking_send(PtySignal::Line(text)).is_ok()
        }
    }
}

pub(super) fn forward(
    mut reader: Box<dyn Read + Send>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    sender: &tokio::sync::mpsc::Sender<PtySignal>,
) {
    #[cfg(not(windows))]
    let _ = &writer;
    let mut pending = Vec::new();
    let mut buffer = [0_u8; 4096];
    #[cfg(windows)]
    let mut cursor_ready = false;
    #[cfg(windows)]
    let mut pending_utf8 = Vec::new();
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                pending.extend_from_slice(&buffer[..read]);
                #[cfg(windows)]
                {
                    if !cursor_ready {
                        cursor_ready = answer_cursor_query(&mut pending, &writer);
                    }
                    if !cursor_ready {
                        let retained = (1..CURSOR_QUERY.len())
                            .rev()
                            .find(|length| pending.ends_with(&CURSOR_QUERY[..*length]))
                            .unwrap_or(0);
                        let safe_len = pending.len() - retained;
                        if safe_len > 0 {
                            if !forward_utf8_chunk(&mut pending_utf8, &pending[..safe_len], sender)
                            {
                                return;
                            }
                            pending.drain(..safe_len);
                        }
                        continue;
                    }
                }
                #[cfg(windows)]
                if !pending.is_empty() {
                    if !forward_utf8_chunk(&mut pending_utf8, &pending, sender) {
                        return;
                    }
                    pending.clear();
                }
                #[cfg(not(windows))]
                while let Some(newline) = pending.iter().position(|byte| *byte == b'\n') {
                    let mut line = pending.drain(..=newline).collect::<Vec<_>>();
                    while line
                        .last()
                        .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
                    {
                        line.pop();
                    }
                    if sender
                        .blocking_send(PtySignal::Line(String::from_utf8_lossy(&line).into_owned()))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
    }
    #[cfg(windows)]
    {
        if !pending.is_empty() && !forward_utf8_chunk(&mut pending_utf8, &pending, sender) {
            return;
        }
        if !pending_utf8.is_empty() {
            let text = String::from_utf8_lossy(&pending_utf8).into_owned();
            if sender.blocking_send(PtySignal::Line(text)).is_err() {
                return;
            }
        }
    }
    #[cfg(not(windows))]
    if !pending.is_empty() {
        let _ = sender.blocking_send(PtySignal::Line(
            String::from_utf8_lossy(&pending).into_owned(),
        ));
    }
    let _ = sender.blocking_send(PtySignal::OutputDrained);
}

#[cfg(windows)]
fn answer_cursor_query(
    pending: &mut Vec<u8>,
    writer: &Arc<Mutex<Option<Box<dyn Write + Send>>>>,
) -> bool {
    let Some(index) = pending
        .windows(CURSOR_QUERY.len())
        .position(|window| window == CURSOR_QUERY)
    else {
        return false;
    };
    pending.drain(index..index + CURSOR_QUERY.len());
    if let Ok(mut writer) = writer.lock()
        && let Some(writer) = writer.as_mut()
    {
        let _ = writer.write_all(b"\x1b[1;1R");
        let _ = writer.flush();
    }
    true
}

#[cfg(all(test, windows))]
mod tests {
    use std::io;
    use std::sync::mpsc::{Receiver, SyncSender};
    use std::thread;

    use super::*;

    struct SplitQueryReader {
        read_index: u8,
        waiting: SyncSender<()>,
        release: Receiver<()>,
    }

    impl Read for SplitQueryReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            match self.read_index {
                0 => {
                    self.read_index = 1;
                    let chunk = b"ordinary\x1b[";
                    buffer[..chunk.len()].copy_from_slice(chunk);
                    Ok(chunk.len())
                }
                1 => {
                    self.read_index = 2;
                    self.waiting
                        .send(())
                        .map_err(|_| io::Error::other("query gate receiver dropped"))?;
                    self.release
                        .recv()
                        .map_err(|_| io::Error::other("query release sender dropped"))?;
                    let chunk = b"6ntrailing";
                    buffer[..chunk.len()].copy_from_slice(chunk);
                    Ok(chunk.len())
                }
                _ => Ok(0),
            }
        }
    }

    struct RecordingWriter(Arc<Mutex<Vec<u8>>>);

    struct ChunkReader {
        chunks: Vec<Vec<u8>>,
        read_index: usize,
    }

    impl Read for ChunkReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let Some(chunk) = self.chunks.get(self.read_index) else {
                return Ok(0);
            };
            self.read_index += 1;
            buffer[..chunk.len()].copy_from_slice(chunk);
            Ok(chunk.len())
        }
    }

    impl Write for RecordingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .map_err(|_| io::Error::other("recording writer lock poisoned"))?
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn forward_streams_safe_prefix_while_split_cursor_query_is_pending() {
        // Given: a reader that blocks before releasing the cursor-query suffix.
        let (waiting_tx, waiting_rx) = std::sync::mpsc::sync_channel(0);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(0);
        let reader = SplitQueryReader {
            read_index: 0,
            waiting: waiting_tx,
            release: release_rx,
        };
        let written = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Option<Box<dyn Write + Send>>>> = Arc::new(Mutex::new(Some(
            Box::new(RecordingWriter(Arc::clone(&written))),
        )));
        let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
        let worker = thread::spawn(move || forward(Box::new(reader), writer, &sender));

        // When: forward has processed the ordinary bytes and requests the suffix.
        waiting_rx.recv().expect("reader must reach the query gate");
        let streamed_before_query = match receiver.try_recv() {
            Ok(PtySignal::Line(line)) => Some(line),
            Ok(PtySignal::ChildExited | PtySignal::OutputDrained)
            | Err(tokio::sync::mpsc::error::TryRecvError::Empty)
            | Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => None,
        };
        release_tx
            .send(())
            .expect("reader must accept the query suffix release");
        worker.join().expect("forward worker must finish");
        let mut remaining = Vec::new();
        while let Some(signal) = receiver.blocking_recv() {
            remaining.push(signal);
        }

        // Then: ordinary output preceded the consumed query, response, tail, and drain signal.
        assert_eq!(streamed_before_query.as_deref(), Some("ordinary"));
        assert_eq!(
            written.lock().expect("recording writer lock").as_slice(),
            b"\x1b[1;1R"
        );
        let mut remaining = remaining.into_iter();
        match remaining.next() {
            Some(PtySignal::Line(line)) => assert_eq!(line, "trailing"),
            Some(PtySignal::ChildExited | PtySignal::OutputDrained) | None => {
                panic!("trailing output must follow the cursor response")
            }
        }
        assert!(matches!(remaining.next(), Some(PtySignal::OutputDrained)));
        assert!(remaining.next().is_none());
    }

    #[test]
    fn forward_preserves_utf8_split_across_reads() {
        let bytes = "한글".as_bytes();
        let reader = ChunkReader {
            chunks: vec![
                bytes[..1].to_vec(),
                bytes[1..4].to_vec(),
                bytes[4..].to_vec(),
            ],
            read_index: 0,
        };
        let writer: Arc<Mutex<Option<Box<dyn Write + Send>>>> = Arc::new(Mutex::new(None));
        let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
        let worker = thread::spawn(move || forward(Box::new(reader), writer, &sender));

        worker.join().expect("forward worker must finish");
        let mut output = String::new();
        while let Some(signal) = receiver.blocking_recv() {
            match signal {
                PtySignal::Line(line) => output.push_str(&line),
                PtySignal::OutputDrained => break,
                PtySignal::ChildExited => panic!("child exit is not part of forwarding"),
            }
        }

        assert_eq!(output, "한글");
    }
}
