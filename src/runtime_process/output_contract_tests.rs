use super::output::{MAX_OUTPUT_LINE_BYTES, forward_output, read_bounded_line};
use std::time::Duration;
use tokio::io::BufReader;

#[tokio::test]
async fn saturated_startup_log_channel_does_not_block_reader() {
    // Given: startup output larger than the bounded channel while its receiver is idle.
    let input = (0..256)
        .map(|index| format!("startup line {index}\n"))
        .collect::<String>();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);

    // When: the output reader forwards every available line.
    tokio::time::timeout(
        Duration::from_millis(250),
        forward_output(BufReader::new(input.as_bytes()), sender, ""),
    )
    .await
    .expect("saturated output reader must not block")
    .expect("read startup output");

    // Then: memory remains bounded and the reader reaches EOF without receiver progress.
    assert!(receiver.try_recv().is_ok());
    assert!(receiver.try_recv().is_ok());
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected)
    ));
}

#[tokio::test]
async fn oversized_output_line_is_discarded_without_losing_next_line() {
    // Given: one line beyond the allocation limit followed by valid runtime output.
    let mut input = vec![b'x'; MAX_OUTPUT_LINE_BYTES + 1024];
    input.extend_from_slice(b"\nready\n");
    let mut reader = BufReader::new(input.as_slice());

    // When: both logical lines are read through the bounded decoder.
    let oversized = read_bounded_line(&mut reader)
        .await
        .expect("read oversized line")
        .expect("oversized line marker");
    let next = read_bounded_line(&mut reader)
        .await
        .expect("read following line")
        .expect("following output");

    // Then: the oversized payload is not retained and subsequent output remains usable.
    assert!(oversized.contains("exceeded"));
    assert!(oversized.len() < 128);
    assert_eq!(next, "ready");
}
