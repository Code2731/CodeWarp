use super::*;

#[test]
fn parse_command_splits_basic_tokens() {
    let parsed = parse_command("npx -y @modelcontextprotocol/server-filesystem /tmp").unwrap();
    assert_eq!(
        parsed,
        vec![
            "npx".to_string(),
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/tmp".to_string()
        ]
    );
}

#[test]
fn parse_command_preserves_quoted_segments() {
    let parsed = parse_command(r#"npx -y server-filesystem "C:\Work Dir\project root""#).unwrap();
    assert_eq!(
        parsed,
        vec![
            "npx".to_string(),
            "-y".to_string(),
            "server-filesystem".to_string(),
            r#"C:\Work Dir\project root"#.to_string()
        ]
    );
}

#[test]
fn parse_command_unescapes_escaped_quote_in_double_quotes() {
    let parsed = parse_command("python -c \"print(\\\"hi\\\")\"").unwrap();
    assert_eq!(
        parsed,
        vec![
            "python".to_string(),
            "-c".to_string(),
            "print(\"hi\")".to_string()
        ]
    );
}

#[test]
fn parse_command_preserves_windows_backslashes() {
    let parsed = parse_command(r#"cmd /c "C:\Tools\server.exe --port 8080""#).unwrap();
    assert_eq!(
        parsed,
        vec![
            "cmd".to_string(),
            "/c".to_string(),
            r#"C:\Tools\server.exe --port 8080"#.to_string()
        ]
    );
}

#[test]
fn parse_command_supports_escaped_space_outside_quotes() {
    let parsed = parse_command(r#"tool C:\Work\My\ Folder"#).unwrap();
    assert_eq!(
        parsed,
        vec!["tool".to_string(), r#"C:\Work\My Folder"#.to_string()]
    );
}

#[test]
fn parse_command_rejects_unclosed_quote() {
    let err = parse_command(r#"npx -y "server-filesystem"#).unwrap_err();
    assert!(err.contains("따옴표"));
}

#[tokio::test]
async fn silent_server_reports_response_deadline_without_outer_watchdog() {
    use crate::test_support::process_fixture::{ProcessFixtureMode, command_line};
    use std::time::Duration;

    // Given: a repository-owned MCP process that never sends an initialize response.
    let server = McpServer {
        name: "silent-fixture".into(),
        command: command_line(ProcessFixtureMode::McpSilent),
    };

    // When: the production response deadline expires inside a larger safety watchdog.
    let started = tokio::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(12), list_tools(&server)).await;

    // Then: the MCP client, not the outer watchdog, must report its response deadline.
    let error = result
        .expect("MCP response wait exceeded its 12-second safety bound")
        .unwrap_err();
    eprintln!(
        "mode=mcp-silent production_elapsed_ms={} result={error}",
        started.elapsed().as_millis()
    );
    assert!(error.contains("deadline"));
    assert!(started.elapsed() >= Duration::from_secs(10));
    assert!(started.elapsed() < Duration::from_secs(12));
}
