use std::io;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub(super) const MAX_OUTPUT_LINE_BYTES: usize = 64 * 1024;

pub(super) async fn read_bounded_line<R>(reader: &mut R) -> io::Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    let mut bytes = Vec::with_capacity(1024);
    let mut exceeded = false;

    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if bytes.is_empty() && !exceeded {
                return Ok(None);
            }
            break;
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        let content_len = newline.map_or(consumed, |_| consumed - 1);
        if !exceeded {
            let remaining = MAX_OUTPUT_LINE_BYTES.saturating_sub(bytes.len());
            let copied = content_len.min(remaining);
            bytes.extend_from_slice(&available[..copied]);
            exceeded = content_len > remaining;
        }
        reader.consume(consumed);
        if newline.is_some() {
            break;
        }
    }

    if exceeded {
        return Ok(Some(format!(
            "[output line exceeded {MAX_OUTPUT_LINE_BYTES} bytes and was discarded]"
        )));
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub(super) async fn forward_output<R>(
    mut reader: R,
    sender: tokio::sync::mpsc::Sender<String>,
    prefix: &str,
) -> io::Result<()>
where
    R: AsyncBufRead + Unpin,
{
    while let Some(line) = read_bounded_line(&mut reader).await? {
        let line = format!("{prefix}{line}");
        match sender.try_send(line) {
            Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {}
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => break,
        }
    }
    Ok(())
}
