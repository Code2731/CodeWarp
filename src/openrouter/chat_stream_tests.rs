use std::sync::Arc;
use std::time::Duration;

use futures_util::{StreamExt, stream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::api_types::provider_label;
use super::chat_stream::{chat_stream as openai_chat_stream, decode_utf8_chunk, next_stream_item};
use super::{ChatEvent, ChatMessage};

#[test]
fn utf8_decoder_keeps_codepoint_split_across_chunks() {
    let mut pending = Vec::new();
    let bytes = "한글".as_bytes();

    assert_eq!(decode_utf8_chunk(&mut pending, &bytes[..1]), Ok(None));
    assert_eq!(
        decode_utf8_chunk(&mut pending, &bytes[1..4]),
        Ok(Some("한".into()))
    );
    assert_eq!(
        decode_utf8_chunk(&mut pending, &bytes[4..]),
        Ok(Some("글".into()))
    );
    assert!(pending.is_empty());
}

#[test]
fn utf8_decoder_rejects_invalid_bytes() {
    let mut pending = Vec::new();

    assert_eq!(decode_utf8_chunk(&mut pending, &[0xff]), Err(()));
}

#[test]
fn provider_label_distinguishes_openrouter_and_local_endpoints() {
    assert_eq!(provider_label("https://openrouter.ai/api/v1"), "OpenRouter");
    assert_eq!(
        provider_label("http://localhost:11434/v1"),
        "OpenAI-compatible provider"
    );
}

#[tokio::test]
async fn stream_wait_returns_item_before_deadline() {
    let mut input = stream::iter([42_u8]);

    assert_eq!(
        next_stream_item(&mut input, Duration::from_secs(1)).await,
        Ok(Some(42))
    );
}

#[tokio::test]
async fn stream_wait_returns_timeout_for_silent_stream() {
    let mut input = stream::pending::<u8>();

    assert_eq!(
        next_stream_item(&mut input, Duration::from_millis(5)).await,
        Err(())
    );
}

#[tokio::test]
async fn chat_stream_round_trips_through_openai_compatible_sse_server() {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind loopback mock server");
    let address = listener.local_addr().expect("mock server address");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept chat request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = socket.read(&mut buffer).await.expect("read chat request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
        assert!(request.contains("/v1/chat/completions"));
        assert!(request.contains("authorization: bearer local-token"));
        assert!(request.contains("x-api-key: local-token"));
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write mock response headers");
        for frame in [
            "data: {\"id\":\"mock-1\",\"choices\":[{\"delta\":{\"content\":\"안녕\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"mock-1\",\"choices\":[{\"delta\":{\"content\":\" CodeWarp\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ] {
            socket
                .write_all(frame.as_bytes())
                .await
                .expect("write mock SSE frame");
        }
    });

    let messages = Arc::new(vec![ChatMessage::user("ping")]);
    let mut events = Box::pin(openai_chat_stream(
        format!("http://{address}/v1"),
        Some("local-token".into()),
        "mock-model".into(),
        messages,
        None,
    ));
    let mut output = String::new();
    let mut done = false;
    while let Some(event) = events.next().await {
        match event {
            ChatEvent::Token(token) => output.push_str(&token),
            ChatEvent::Done { .. } => {
                done = true;
                break;
            }
            ChatEvent::ToolCallDelta { .. } => panic!("mock response should not contain tools"),
            ChatEvent::Error(error) => panic!("unexpected chat stream error: {error}"),
        }
    }

    server.await.expect("mock server must finish");
    assert_eq!(output, "안녕 CodeWarp");
    assert!(done, "chat stream must emit Done after [DONE]");
}

#[tokio::test]
async fn chat_stream_labels_local_http_errors_as_openai_compatible() {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind local error server");
    let address = listener.local_addr().expect("local error server address");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept error request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = socket.read(&mut buffer).await.expect("read error request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        socket
            .write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write local error response");
    });

    let messages = Arc::new(vec![ChatMessage::user("ping")]);
    let mut events = Box::pin(openai_chat_stream(
        format!("http://{address}/v1"),
        Some("local-token".into()),
        "local-model".into(),
        messages,
        None,
    ));
    let event = events.next().await.expect("local error event");
    server.await.expect("local error server must finish");

    match event {
        ChatEvent::Error(error) => {
            assert!(
                error.contains("OpenAI-compatible provider 401"),
                "error: {error}"
            );
            assert!(!error.contains("OpenRouter"), "error: {error}");
        }
        other => panic!("expected local HTTP error, got {other:?}"),
    }
}
