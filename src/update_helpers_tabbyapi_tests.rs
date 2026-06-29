use super::update_helpers_tabbyapi::{
    is_tabbyml_cli_launcher_name, tabbyapi_allowed_launcher_name, yaml_quote,
};

#[test]
fn test_yaml_quote_normal() {
    assert_eq!(yaml_quote("normal"), "'normal'");
}

#[test]
fn test_yaml_quote_with_single_quote() {
    assert_eq!(yaml_quote("it's"), "'it''s'");
}

#[test]
fn test_yaml_quote_empty() {
    assert_eq!(yaml_quote(""), "''");
}

#[test]
fn test_yaml_quote_multiple_quotes() {
    assert_eq!(yaml_quote("a'b'c"), "'a''b''c'");
}

#[test]
fn test_yaml_quote_only_quote() {
    assert_eq!(yaml_quote("'"), "''''");
}

#[test]
fn test_is_tabbyml_cli_launcher_name_matches_exact() {
    assert!(is_tabbyml_cli_launcher_name("tabby"));
    assert!(is_tabbyml_cli_launcher_name("tabby.exe"));
    assert!(is_tabbyml_cli_launcher_name("tabby.cmd"));
    assert!(is_tabbyml_cli_launcher_name("tabby.bat"));
}

#[test]
fn test_is_tabbyml_cli_launcher_name_case_insensitive() {
    assert!(is_tabbyml_cli_launcher_name("TABBY"));
    assert!(is_tabbyml_cli_launcher_name("TABBY.EXE"));
    assert!(is_tabbyml_cli_launcher_name("Tabby.Cmd"));
    assert!(is_tabbyml_cli_launcher_name("TABBY.BAT"));
}

#[test]
fn test_is_tabbyml_cli_launcher_name_rejects_non_tabby() {
    assert!(!is_tabbyml_cli_launcher_name("start.bat"));
    assert!(!is_tabbyml_cli_launcher_name("main.py"));
    assert!(!is_tabbyml_cli_launcher_name(""));
    assert!(!is_tabbyml_cli_launcher_name("tabbyXYZ"));
}

#[test]
fn test_tabbyapi_allowed_launcher_name_matches_exact() {
    assert!(tabbyapi_allowed_launcher_name("start.bat"));
    assert!(tabbyapi_allowed_launcher_name("start.cmd"));
    assert!(tabbyapi_allowed_launcher_name("start.sh"));
    assert!(tabbyapi_allowed_launcher_name("main.py"));
}

#[test]
fn test_tabbyapi_allowed_launcher_name_case_insensitive() {
    assert!(tabbyapi_allowed_launcher_name("Start.bat"));
    assert!(tabbyapi_allowed_launcher_name("START.CMD"));
    assert!(tabbyapi_allowed_launcher_name("Start.Sh"));
    assert!(tabbyapi_allowed_launcher_name("MAIN.PY"));
}

#[test]
fn test_tabbyapi_allowed_launcher_name_rejects_others() {
    assert!(!tabbyapi_allowed_launcher_name("tabby"));
    assert!(!tabbyapi_allowed_launcher_name("start.exe"));
    assert!(!tabbyapi_allowed_launcher_name(""));
    assert!(!tabbyapi_allowed_launcher_name("launcher.sh"));
}
