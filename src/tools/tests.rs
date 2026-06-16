use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn tool_kind_classification() {
    assert_eq!(tool_kind("read_file"), ToolKind::ReadOnly);
    assert_eq!(tool_kind("glob"), ToolKind::ReadOnly);
    assert_eq!(tool_kind("grep"), ToolKind::ReadOnly);
    assert_eq!(tool_kind("write_file"), ToolKind::Mutating);
    assert_eq!(tool_kind("run_command"), ToolKind::Mutating);
    assert_eq!(tool_kind("unknown"), ToolKind::ReadOnly);
}

#[test]
fn read_file_absolute_path_rejected() {
    let tmp = TempDir::new().unwrap();
    let abs_path = if cfg!(windows) {
        "C:\\Windows\\system.ini"
    } else {
        "/etc/passwd"
    };
    let result = dispatch(
        "read_file",
        &format!(r#"{{"path":"{}"}}"#, abs_path.replace('\\', "\\\\")),
        tmp.path(),
    );
    assert!(result.contains("[error]"), "got: {}", result);
    assert!(result.contains("절대 경로"), "got: {}", result);
}

#[test]
fn read_file_traversal_rejected() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path().join("inner");
    fs::create_dir_all(&cwd).unwrap();
    fs::write(tmp.path().join("outside.txt"), "secret").unwrap();
    let result = dispatch("read_file", r#"{"path":"../outside.txt"}"#, &cwd);
    assert!(result.contains("[error]"), "got: {}", result);
    assert!(
        result.contains("작업 디렉토리 밖") || result.contains("경로 해석 실패"),
        "got: {}",
        result
    );
}

#[test]
fn read_file_ok_for_relative_path() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("hello.txt"), "world").unwrap();
    let result = dispatch("read_file", r#"{"path":"hello.txt"}"#, tmp.path());
    assert_eq!(result, "world");
}

#[test]
fn read_file_directory_rejected() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("sub")).unwrap();
    let result = dispatch("read_file", r#"{"path":"sub"}"#, tmp.path());
    assert!(result.contains("파일이 아닙니다"));
}

#[test]
fn read_file_size_limit() {
    let tmp = TempDir::new().unwrap();
    let big = vec![b'x'; 1_000_001];
    fs::write(tmp.path().join("big.bin"), &big).unwrap();
    let result = dispatch("read_file", r#"{"path":"big.bin"}"#, tmp.path());
    assert!(result.contains("파일 크기가 너무 큼"), "got: {}", result);
}

#[test]
fn write_file_absolute_path_rejected() {
    let tmp = TempDir::new().unwrap();
    let abs_path = if cfg!(windows) {
        "C:\\evil.txt"
    } else {
        "/tmp/evil.txt"
    };
    let escaped = abs_path.replace('\\', "\\\\");
    let result = dispatch(
        "write_file",
        &format!(r#"{{"path":"{}","content":"x"}}"#, escaped),
        tmp.path(),
    );
    assert!(result.contains("[error]"), "got: {}", result);
    assert!(result.contains("절대 경로"), "got: {}", result);
}

#[test]
fn write_file_traversal_rejected() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path().join("inner");
    fs::create_dir_all(&cwd).unwrap();
    let result = dispatch(
        "write_file",
        r#"{"path":"../escaped.txt","content":"bad"}"#,
        &cwd,
    );
    assert!(
        result.contains("[error]") && result.contains("작업 디렉토리 밖"),
        "got: {}",
        result
    );
}

#[test]
fn write_file_creates_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let result = dispatch(
        "write_file",
        r#"{"path":"sub/nested/file.rs","content":"hello"}"#,
        tmp.path(),
    );
    assert!(result.contains("[ok]"), "got: {}", result);
    let written = fs::read_to_string(tmp.path().join("sub/nested/file.rs")).unwrap();
    assert_eq!(written, "hello");
}

#[test]
fn write_file_overwrites_existing() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.txt"), "old").unwrap();
    let result = dispatch(
        "write_file",
        r#"{"path":"a.txt","content":"new"}"#,
        tmp.path(),
    );
    assert!(result.contains("[ok]"));
    assert_eq!(fs::read_to_string(tmp.path().join("a.txt")).unwrap(), "new");
}

#[test]
fn glob_finds_files() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.rs"), "").unwrap();
    fs::write(tmp.path().join("b.rs"), "").unwrap();
    fs::write(tmp.path().join("c.txt"), "").unwrap();
    let result = dispatch("glob", r#"{"pattern":"*.rs"}"#, tmp.path());
    assert!(result.contains("matches"), "got: {}", result);
    assert!(result.contains("a.rs"));
    assert!(result.contains("b.rs"));
    assert!(!result.contains("c.txt"));
}

#[test]
fn glob_no_match() {
    let tmp = TempDir::new().unwrap();
    let result = dispatch("glob", r#"{"pattern":"*.xyz"}"#, tmp.path());
    assert!(result.contains("0 matches"), "got: {}", result);
}

#[test]
fn grep_finds_pattern() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.rs"), "fn main() {}\nfn helper() {}\n").unwrap();
    let result = dispatch("grep", r#"{"pattern":"fn main"}"#, tmp.path());
    assert!(result.contains("a.rs"), "got: {}", result);
    assert!(result.contains("fn main"));
    assert!(!result.contains("fn helper"));
}

#[test]
fn dispatch_unknown_tool() {
    let tmp = TempDir::new().unwrap();
    let result = dispatch("foo", "{}", tmp.path());
    assert!(result.contains("알 수 없는 도구"));
}
