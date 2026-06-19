use super::types::StreamChunk;

pub(super) fn normalize_stream_payload_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let payload = trimmed
        .strip_prefix("data:")
        .map(str::trim)
        .unwrap_or(trimmed);
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}

pub(super) fn flush_pending_sse_data(pending_data: &mut String) -> Option<String> {
    if pending_data.is_empty() {
        return None;
    }
    let payload = pending_data.trim_end_matches('\n').to_string();
    pending_data.clear();
    if payload.trim().is_empty() {
        None
    } else {
        Some(payload)
    }
}

pub(super) fn consume_sse_line(line: &str, pending_data: &mut String) -> Option<String> {
    let trimmed = line.trim_end_matches('\r').trim();
    if trimmed.is_empty() {
        return flush_pending_sse_data(pending_data);
    }
    if let Some(data_part) = trimmed.strip_prefix("data:") {
        pending_data.push_str(data_part.trim_start());
        pending_data.push('\n');
        return None;
    }
    if trimmed == "data" {
        pending_data.push('\n');
        return None;
    }
    if trimmed.starts_with(':') {
        return None;
    }
    if !pending_data.is_empty() {
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            pending_data.push_str(trimmed);
            pending_data.push('\n');
            return None;
        }
        return None;
    }
    normalize_stream_payload_line(trimmed).map(str::to_string)
}

pub(super) fn parse_stream_chunks(payload: &str) -> Vec<StreamChunk> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(parsed) = serde_json::from_str::<StreamChunk>(trimmed) {
        return vec![parsed];
    }
    let mut stream_items = Vec::new();
    let mut had_error = false;
    for item in serde_json::Deserializer::from_str(trimmed).into_iter::<StreamChunk>() {
        match item {
            Ok(chunk) => stream_items.push(chunk),
            Err(_) => {
                had_error = true;
                break;
            }
        }
    }
    if !stream_items.is_empty() && !had_error {
        return stream_items;
    }
    let mut out = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<StreamChunk>(line) {
            out.push(parsed);
        }
    }
    out
}

pub(super) fn extract_plain_stream_token(payload: &str) -> Option<String> {
    let text = payload.trim();
    if text.is_empty() {
        return None;
    }
    if text.starts_with('{') || text.starts_with('[') {
        return None;
    }
    Some(text.to_string())
}
