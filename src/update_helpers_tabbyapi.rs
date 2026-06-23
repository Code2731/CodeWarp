// update_helpers_tabbyapi.rs — TabbyAPI helper functions (main.rs child module)
use super::{resolve_user_path, PathBuf, TABBY_API_REPO_URL};

pub(crate) const TABBY_CONNECT_RETRIES_AFTER_START: u8 = 3;
pub(crate) const TABBY_CONNECT_RETRY_DELAY_SECS: u64 = 4;

fn tabbyapi_launcher_required_message() -> String {
    "TabbyAPI 런타임 스크립트가 비어 있습니다. EXL2 모델 폴더만으로는 서버가 실행되지 않습니다. TabbyAPI 프로젝트의 Start.bat/Start.cmd(Windows), start.sh(macOS/Linux), 또는 main.py 경로를 지정해 주세요. 해당 파일이 없다면 TabbyAPI를 먼저 설치해야 합니다."
        .into()
}

fn tabbyapi_reject_tabbyml_message() -> String {
    "지정한 tabby/tabby.exe/tabby.cmd/tabby.bat(tabby CLI)는 TabbyML CLI라 EXL2 모델을 실행할 수 없습니다. TabbyAPI 프로젝트의 Start.bat/Start.cmd, start.sh, 또는 main.py를 지정해 주세요."
        .into()
}

fn is_tabbyml_cli_launcher_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "tabby" | "tabby.exe" | "tabby.cmd" | "tabby.bat"
    )
}

fn tabbyapi_allowed_launcher_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "start.bat" | "start.cmd" | "start.sh" | "main.py"
    )
}

pub(crate) fn validate_tabbyapi_launcher_path(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(tabbyapi_launcher_required_message());
    }
    let launcher_path = std::path::Path::new(trimmed);
    let launcher_name = launcher_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if is_tabbyml_cli_launcher_name(&launcher_name) {
        return Err(tabbyapi_reject_tabbyml_message());
    }
    if launcher_path.is_dir() {
        return Err(format!(
            "지정한 TabbyAPI script 경로가 폴더입니다: {}. Start.bat/Start.cmd(Windows), start.sh(macOS/Linux), 또는 main.py 파일을 직접 지정해 주세요.",
            launcher_path.display()
        ));
    }
    if !launcher_path.is_file() {
        return Err(format!(
            "지정한 TabbyAPI script 파일을 찾을 수 없습니다: {}. Start.bat/Start.cmd(Windows), start.sh(macOS/Linux), 또는 main.py 경로를 다시 지정해 주세요.",
            launcher_path.display()
        ));
    }
    if !tabbyapi_allowed_launcher_name(&launcher_name) {
        return Err(format!(
            "TabbyAPI script 파일명이 올바르지 않습니다: {}. Start.bat/Start.cmd(Windows), start.sh(macOS/Linux), 또는 main.py를 지정해 주세요.",
            launcher_path.display()
        ));
    }
    Ok(())
}

pub(crate) fn is_tabbyapi_launcher_path(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    let p = std::path::Path::new(trimmed);
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    tabbyapi_allowed_launcher_name(&name)
}

pub(crate) fn default_tabbyapi_runtime_dir() -> PathBuf {
    if let Ok(path) = std::env::var("CODEWARP_TABBYAPI_RUNTIME_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return resolve_user_path(trimmed);
        }
    }
    if let Some(p) = dirs::data_local_dir() {
        return p.join("codewarp").join("runtimes").join("tabbyAPI");
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".codewarp").join("runtimes").join("tabbyAPI");
    }
    PathBuf::from("runtimes").join("tabbyAPI")
}

fn tabbyapi_launcher_candidates(runtime_dir: &std::path::Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        vec![
            runtime_dir.join("start.bat"),
            runtime_dir.join("Start.bat"),
            runtime_dir.join("start.cmd"),
            runtime_dir.join("Start.cmd"),
            runtime_dir.join("main.py"),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![runtime_dir.join("start.sh"), runtime_dir.join("main.py")]
    }
}

pub(crate) fn find_tabbyapi_launcher(runtime_dir: &std::path::Path) -> Option<PathBuf> {
    tabbyapi_launcher_candidates(runtime_dir)
        .into_iter()
        .find(|p| p.is_file())
}

fn yaml_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(crate) fn write_tabbyapi_config_for_launcher(
    launcher: &str,
    model_path: &str,
    port: u16,
) -> Result<PathBuf, String> {
    let launcher_path = std::path::Path::new(launcher);
    let runtime_dir = launcher_path
        .parent()
        .ok_or_else(|| "TabbyAPI script 상위 폴더를 확인할 수 없습니다.".to_string())?;
    let model_path = resolve_user_path(model_path);
    if !model_path.exists() {
        return Err(format!(
            "TabbyAPI 모델 폴더를 찾을 수 없습니다: {}",
            model_path.display()
        ));
    }
    let model_dir = model_path
        .parent()
        .ok_or_else(|| "TabbyAPI 모델 폴더의 상위 경로를 확인할 수 없습니다.".to_string())?;
    let model_name = model_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "TabbyAPI 모델 폴더 이름을 확인할 수 없습니다.".to_string())?;
    let config_path = runtime_dir.join("config.yml");
    let content = format!(
        "network:\n  host: 127.0.0.1\n  port: {}\n  disable_auth: true\nmodel:\n  model_dir: {}\n  model_name: {}\nsampling:\n  override_preset: safe_defaults\n",
        port,
        yaml_quote(&model_dir.display().to_string()),
        yaml_quote(model_name)
    );
    std::fs::write(&config_path, content)
        .map_err(|e| format!("TabbyAPI config.yml 작성 실패: {e}"))?;
    Ok(config_path)
}

pub(crate) async fn install_tabbyapi_runtime(runtime_dir: PathBuf) -> Result<PathBuf, String> {
    if let Some(launcher) = find_tabbyapi_launcher(&runtime_dir) {
        return Ok(launcher);
    }
    if runtime_dir.exists() {
        return Err(format!(
            "TabbyAPI 설치 폴더는 있지만 실행 스크립트를 찾지 못했습니다: {}",
            runtime_dir.display()
        ));
    }
    let parent = runtime_dir
        .parent()
        .ok_or_else(|| "TabbyAPI 설치 상위 폴더를 확인할 수 없습니다.".to_string())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| format!("TabbyAPI 설치 폴더 생성 실패: {e}"))?;

    let output = tokio::process::Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(TABBY_API_REPO_URL)
        .arg(&runtime_dir)
        .output()
        .await
        .map_err(|e| format!("git 실행 실패: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(format!("TabbyAPI git clone 실패: {detail}"));
    }

    find_tabbyapi_launcher(&runtime_dir).ok_or_else(|| {
        format!(
            "TabbyAPI 설치 후에도 실행 스크립트를 찾지 못했습니다: {}",
            runtime_dir.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
