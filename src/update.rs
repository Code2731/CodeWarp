// update.rs — App update + 헬퍼 메서드 (main.rs child module)
use super::*;
use iced::widget::text_editor;
use iced::{Subscription, Task};

const HF_HINT_MARKERS: [&str; 3] = [
    "fallback retry from",
    "fallback lookup failed:",
    "requested revision:",
];
const TABBY_CONNECT_RETRIES_AFTER_START: u8 = 3;
const TABBY_CONNECT_RETRY_DELAY_SECS: u64 = 4;

fn starts_with_ascii_case_insensitive(text: &str, prefix: &str) -> bool {
    text.to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

fn find_hint_boundary(tail: &str) -> Option<usize> {
    for sep in [") (", ")("] {
        let mut offset = 0usize;
        while let Some(rel) = tail[offset..].find(sep) {
            let pos = offset + rel;
            let after = tail[pos + sep.len()..].trim_start();
            if HF_HINT_MARKERS
                .iter()
                .any(|m| starts_with_ascii_case_insensitive(after, m))
            {
                return Some(pos);
            }
            offset = pos + sep.len();
        }
    }
    None
}

fn extract_hf_error_hint(raw: &str, marker: &str) -> Option<String> {
    let raw_lc = raw.to_ascii_lowercase();
    let marker_lc = marker.to_ascii_lowercase();
    let idx = raw_lc.find(&marker_lc)?;
    let tail = &raw[idx..];
    let cut = find_hint_boundary(tail);
    let head = cut.map(|i| &tail[..i]).unwrap_or(tail);
    let head = head.strip_suffix(')').unwrap_or(head).trim();
    if head.is_empty() {
        None
    } else {
        Some(head.to_string())
    }
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn merge_hint(hints: &mut Vec<String>, candidate: String) {
    if hints.iter().any(|existing| {
        existing == &candidate || contains_ascii_case_insensitive(existing, &candidate)
    }) {
        return;
    }
    hints.retain(|existing| !contains_ascii_case_insensitive(&candidate, existing));
    hints.push(candidate);
}

fn compose_hf_download_error(raw: &str) -> String {
    let humanized = hf::humanize_error(raw);
    let mut hints: Vec<String> = Vec::new();
    for marker in HF_HINT_MARKERS {
        if let Some(h) = extract_hf_error_hint(raw, marker) {
            merge_hint(&mut hints, h);
        }
    }
    if hints.is_empty() {
        return humanized;
    }
    let missing: Vec<String> = hints
        .into_iter()
        .filter(|h| !contains_ascii_case_insensitive(&humanized, h))
        .collect();
    if missing.is_empty() {
        humanized
    } else {
        format!("{humanized} ({})", missing.join(" | "))
    }
}

fn is_loopback_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("[::1]")
}

fn extract_loopback_port(url: &str) -> Option<u16> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let no_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let authority = no_scheme.split('/').next()?.trim();
    if authority.is_empty() {
        return None;
    }
    if authority.starts_with('[') {
        let closing = authority.find(']')?;
        let host = &authority[..=closing];
        if !host.eq_ignore_ascii_case("[::1]") {
            return None;
        }
        let rest = &authority[closing + 1..];
        let port = rest.strip_prefix(':')?;
        return port.parse::<u16>().ok();
    }
    let (host, port) = authority.rsplit_once(':')?;
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" {
        port.parse::<u16>().ok()
    } else {
        None
    }
}

fn tabby_connection_error_looks_unreachable(raw: &str, actionable: &str) -> bool {
    let raw_lower = raw.to_ascii_lowercase();
    raw_lower.contains("refused")
        || raw_lower.contains("os error 10061")
        || raw_lower.contains("timeout")
        || raw_lower.contains("timed out")
        || contains_ascii_case_insensitive(actionable, "응답 없음")
        || contains_ascii_case_insensitive(actionable, "시간 초과")
        || contains_ascii_case_insensitive(actionable, "응답하지")
}

fn tabbyapi_launcher_required_message() -> String {
    "TabbyAPI 런타임 스크립트가 비어 있습니다. EXL2 모델 폴더만으로는 서버가 실행되지 않습니다. TabbyAPI 프로젝트의 Start.bat/Start.cmd(Windows), start.sh(macOS/Linux), 또는 main.py 경로를 지정해 주세요. 해당 파일이 없다면 TabbyAPI를 먼저 설치해야 합니다."
        .into()
}

fn tabbyapi_reject_tabbyml_message() -> String {
    "지정한 tabby/tabby.exe/tabby.cmd/tabby.bat(tabby CLI)는 TabbyML CLI라 EXL2 모델을 실행할 수 없습니다. TabbyAPI 프로젝트의 Start.bat/Start.cmd, start.sh, 또는 main.py를 지정해 주세요."
        .into()
}

fn is_tabbyml_cli_launcher_name(name: &str) -> bool {
    matches!(name, "tabby" | "tabby.exe" | "tabby.cmd" | "tabby.bat")
}

fn tabbyapi_allowed_launcher_name(name: &str) -> bool {
    matches!(name, "start.bat" | "start.cmd" | "start.sh" | "main.py")
}

fn validate_tabbyapi_launcher_path(path: &str) -> Result<(), String> {
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

fn is_tabbyapi_launcher_path(path: &str) -> bool {
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
    if !matches!(name.as_str(), "start.bat" | "start.sh" | "main.py") {
        return false;
    }
    let parent = p
        .parent()
        .map(|d| d.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    parent.contains("tabbyapi")
}

fn runtime_command_exists(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }
    let candidate = std::path::Path::new(trimmed);
    if candidate.is_absolute()
        || trimmed.contains(std::path::MAIN_SEPARATOR)
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return candidate.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    let path_dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();

    #[cfg(windows)]
    {
        let has_ext = candidate.extension().is_some();
        let extensions: Vec<String> = if has_ext {
            vec![String::new()]
        } else {
            std::env::var_os("PATHEXT")
                .and_then(|v| v.into_string().ok())
                .map(|v| {
                    v.split(';')
                        .map(|e| e.trim())
                        .filter(|e| !e.is_empty())
                        .map(|e| e.to_ascii_lowercase())
                        .collect::<Vec<_>>()
                })
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        ".com".to_string(),
                        ".exe".to_string(),
                        ".bat".to_string(),
                        ".cmd".to_string(),
                    ]
                })
        };

        for dir in path_dirs {
            for ext in &extensions {
                let full = if ext.is_empty() {
                    dir.join(trimmed)
                } else {
                    dir.join(format!("{trimmed}{ext}"))
                };
                if full.is_file() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        for dir in path_dirs {
            if dir.join(trimmed).is_file() {
                return true;
            }
        }
        false
    }
}

fn resolve_binary_from_dir(dir: &std::path::Path, program: &str) -> Option<PathBuf> {
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);

    #[cfg(windows)]
    {
        let has_ext = std::path::Path::new(base).extension().is_some();
        let mut candidates = vec![dir.join(base)];
        if !has_ext {
            candidates.push(dir.join(format!("{base}.exe")));
            candidates.push(dir.join(format!("{base}.cmd")));
            candidates.push(dir.join(format!("{base}.bat")));
            candidates.push(dir.join(format!("{base}.com")));
        }
        return candidates.into_iter().find(|c| c.is_file());
    }

    #[cfg(not(windows))]
    {
        let candidate = dir.join(base);
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    }
}

fn expected_binary_name(program: &str) -> String {
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    #[cfg(windows)]
    {
        if std::path::Path::new(base).extension().is_some() {
            base.to_string()
        } else {
            format!("{base}.exe")
        }
    }
    #[cfg(not(windows))]
    {
        base.to_string()
    }
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

#[derive(Clone)]
struct ChatRoute {
    label: String,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

async fn collect_chat_text(
    base_url: String,
    api_key: Option<String>,
    model: String,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    let stream = openrouter::chat_stream(base_url, api_key, model, messages, None);
    futures_util::pin_mut!(stream);
    let mut out = String::new();
    while let Some(event) = stream.next().await {
        match event {
            ChatEvent::Token(t) => out.push_str(&t),
            ChatEvent::Done { .. } => return Ok(out),
            ChatEvent::Error(e) => return Err(e),
            ChatEvent::ToolCallDelta { .. } => {}
        }
    }
    Ok(out)
}

async fn install_tabbyapi_runtime(runtime_dir: PathBuf) -> Result<PathBuf, String> {
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

fn default_models_dir() -> String {
    if let Some(p) = dirs::data_local_dir() {
        return p.join("codewarp").join("models").display().to_string();
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".codewarp").join("models").display().to_string();
    }
    "models".to_string()
}

impl App {
    fn has_selected_local_model_available(&self) -> bool {
        let selected = self.inference_selected_model.trim();
        if selected.is_empty() {
            return false;
        }
        list_downloaded_models(std::path::Path::new(&self.model_dir_input))
            .iter()
            .any(|m| m == selected)
    }

    fn sync_selected_local_model_for_model_dir(&mut self) {
        if !matches!(
            self.inference_engine,
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer
        ) {
            return;
        }
        if !self.has_selected_local_model_available() {
            self.inference_selected_model.clear();
        }
    }

    pub(crate) fn can_start_inference(&self) -> bool {
        match self.inference_engine {
            InferenceEngine::Custom => !self.inference_command_input.trim().is_empty(),
            InferenceEngine::Ollama => true,
            InferenceEngine::TabbyMl => !self.inference_selected_model.trim().is_empty(),
            InferenceEngine::TabbyApi => !self.inference_binary_path.trim().is_empty(),
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer => {
                self.has_selected_local_model_available()
            }
        }
    }

    pub(crate) fn can_attempt_start_inference(&self) -> bool {
        match self.inference_engine {
            InferenceEngine::TabbyApi => true,
            _ => self.can_start_inference(),
        }
    }

    pub(crate) fn resolve_runtime_spawn_command(
        &self,
        program: String,
        args: Vec<String>,
    ) -> (String, Vec<String>, Option<PathBuf>) {
        let override_path = self.inference_binary_path.trim();
        if !matches!(self.inference_engine, InferenceEngine::TabbyApi) {
            let final_program = if override_path.is_empty() {
                program
            } else if is_tabbyapi_launcher_path(override_path) {
                program
            } else if std::path::Path::new(override_path).is_dir() {
                resolve_binary_from_dir(std::path::Path::new(override_path), &program)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| override_path.to_string())
            } else {
                override_path.to_string()
            };
            return (final_program, args, None);
        }

        let script = if override_path.is_empty() {
            program
        } else {
            override_path.to_string()
        };
        let script_path = std::path::Path::new(&script);
        let work_dir = script_path.parent().map(|p| p.to_path_buf());
        let file_name = script_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&script)
            .to_string();
        let ext = script_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        #[cfg(windows)]
        {
            if ext == "bat" || ext == "cmd" {
                let mut final_args = vec!["/C".into(), file_name];
                final_args.extend(args);
                return ("cmd.exe".into(), final_args, work_dir);
            }
            if ext == "py" {
                let mut final_args = vec![file_name];
                final_args.extend(args);
                return ("python".into(), final_args, work_dir);
            }
        }

        #[cfg(not(windows))]
        {
            if ext == "sh" {
                return (format!("./{}", file_name), args, work_dir);
            }
            if ext == "py" {
                let mut final_args = vec![file_name];
                final_args.extend(args);
                return ("python3".into(), final_args, work_dir);
            }
        }

        (script, args, work_dir)
    }

    pub(crate) fn compose_tabby_connection_error(&self, raw: &str) -> String {
        let actionable = tabby::humanize_error(raw);
        if self.inference_pid.is_some()
            || !is_loopback_url(&self.tabby_url_input)
            || !tabby_connection_error_looks_unreachable(raw, &actionable)
        {
            return actionable;
        }

        if matches!(self.inference_engine, InferenceEngine::TabbyApi) {
            if self.inference_binary_path.trim().is_empty() {
                return "TabbyAPI 서버가 아직 실행되지 않았습니다. Runtime 탭에서 TabbyAPI script에 Start.bat/start.sh/main.py 경로를 지정하고 시작한 뒤 연결 테스트해 주세요."
                    .into();
            }
            if let Ok(port) = self.inference_port_input.trim().parse::<u16>() {
                let expected_base = format!("http://localhost:{}", port);
                let normalized_current = self
                    .tabby_url_input
                    .trim()
                    .trim_end_matches('/')
                    .trim_end_matches("/v1")
                    .trim_end_matches('/')
                    .to_string();
                if !normalized_current.is_empty()
                    && !normalized_current.eq_ignore_ascii_case(&expected_base)
                {
                    return format!(
                        "Provider URL과 Runtime 포트가 다릅니다. Runtime 포트가 {} 이므로 Provider URL을 {} 로 맞춘 뒤 연결 테스트해 주세요.",
                        port, expected_base
                    );
                }
            }
            return format!(
                "TabbyAPI 서버가 아직 응답하지 않습니다. Runtime 탭의 시작 상태와 로그를 확인한 뒤 {} 로 연결 테스트해 주세요.",
                self.tabby_url_input.trim()
            );
        }

        if !list_downloaded_models(std::path::Path::new(&self.model_dir_input)).is_empty() {
            return "서버가 아직 실행 중이 아닙니다. Runtime 탭에서 현재 엔진을 시작한 뒤 연결 테스트를 다시 실행해 주세요."
                .into();
        }

        actionable
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenSettings => self.open_settings_overlay(),
            Message::CloseSettings => self.close_settings_overlay(),
            Message::SetSettingsTab(tab) => self.set_settings_tab(tab),
            Message::KeyInputChanged(v) => self.set_key_input(v),
            Message::SaveKey => self.save_api_key(),
            Message::KeySaved(r) => self.on_key_saved(r),
            Message::ClearKey => self.clear_api_key(),
            Message::KeyCleared(r) => self.on_key_cleared(r),
            Message::TabbyUrlChanged(v) => self.set_tabby_url(v),
            Message::TabbyTokenChanged(v) => self.set_tabby_token(v),
            Message::ToggleTabbyTokenVisible => self.toggle_tabby_token_visible(),
            Message::InferenceCommandChanged(v) => self.set_inference_command(v),
            Message::SelectInferenceEngine(e) => self.select_inference_engine(e),
            Message::SelectInferenceModel(m) => self.set_inference_model(m),
            Message::InferencePortChanged(v) => self.set_inference_port(v),
            Message::InferenceBinaryChanged(v) => self.set_inference_binary(v),
            Message::PickInferenceBinary => self.pick_inference_binary(),
            Message::InferenceBinaryPicked(maybe) => self.on_inference_binary_picked(maybe),
            Message::InstallTabbyApiRuntime => self.install_tabbyapi_runtime_cmd(),
            Message::TabbyApiRuntimeInstalled(result) => self.on_tabbyapi_runtime_installed(result),
            Message::StartInference => {
                if self.inference_pid.is_some() {
                    self.status = "이미 실행 중".into();
                    return Task::none();
                }
                // 포트 parse
                let port: u16 = self
                    .inference_port_input
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| self.inference_engine.default_port());
                // 엔진별 명령 합성 + URL 자동 등록
                let (program, args) = match self.inference_engine {
                    InferenceEngine::Custom => {
                        let cmd_str = self.inference_command_input.trim();
                        if cmd_str.is_empty() {
                            self.status = "시작 명령 비어있음".into();
                            return Task::none();
                        }
                        let parts = match mcp::parse_command(cmd_str) {
                            Ok(v) => v,
                            Err(e) => {
                                self.status = format!("시작 명령 파싱 실패: {}", e);
                                return Task::none();
                            }
                        };
                        let Some(p) = parts.first().cloned() else {
                            return Task::none();
                        };
                        (p, parts.into_iter().skip(1).collect::<Vec<_>>())
                    }
                    InferenceEngine::Ollama => {
                        // spawn 안 함 — endpoint만 자동 등록 + ping
                        self.tabby_url_input = format!("http://localhost:{}", port);
                        let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                        if self.openai_compat_label.trim().is_empty() {
                            self.openai_compat_label = "Ollama".into();
                            let _ = keystore::write_openai_compat_label("Ollama");
                        }
                        self.status = "Ollama daemon endpoint 등록 — 연결 테스트".into();
                        return Task::done(Message::FetchTabbyModels);
                    }
                    eng => {
                        let model = self.inference_selected_model.trim().to_string();
                        if model.is_empty() && !matches!(eng, InferenceEngine::TabbyApi) {
                            let msg = if matches!(eng, InferenceEngine::TabbyApi) {
                                "TabbyAPI 모델 경로가 비어 있습니다. Models 탭에서 다운로드된 모델을 선택하거나 Runtime의 EXL2 model folder path에 모델 폴더를 지정해 주세요."
                            } else {
                                "모델 선택 안 됨"
                            }
                            .to_string();
                            self.status = msg.clone();
                            if matches!(eng, InferenceEngine::TabbyApi) {
                                self.tabby_status = Some(Err(msg));
                            }
                            return Task::none();
                        }
                        if matches!(eng, InferenceEngine::TabbyApi) {
                            let launcher = self.inference_binary_path.trim();
                            if let Err(msg) = validate_tabbyapi_launcher_path(launcher) {
                                self.status = msg.clone();
                                self.tabby_status = Some(Err(msg));
                                return Task::none();
                            }
                            if !model.is_empty() {
                                let model_path = std::path::Path::new(&model);
                                let Some(resolved_model_path) =
                                    resolve_tabbyapi_model_dir(model_path)
                                else {
                                    let msg = format!(
                                        "TabbyAPI 모델 폴더가 완전하지 않습니다: {} (config.json과 실제 모델 가중치 파일이 필요합니다.)",
                                        model_path.display()
                                    );
                                    self.status = msg.clone();
                                    self.tabby_status = Some(Err(msg));
                                    return Task::none();
                                };
                                let resolved_model = resolved_model_path.display().to_string();
                                if let Err(e) = write_tabbyapi_config_for_launcher(
                                    launcher,
                                    &resolved_model,
                                    port,
                                ) {
                                    self.status = e.clone();
                                    self.tabby_status = Some(Err(e));
                                    return Task::none();
                                }
                                self.inference_selected_model = resolved_model;
                            }
                        }
                        if matches!(eng, InferenceEngine::TabbyMl)
                            && std::path::Path::new(&model).exists()
                        {
                            let msg = format!(
                                "EXL2 로컬 폴더는 TabbyAPI용입니다. TabbyAPI(Start.bat 또는 python main.py)를 실행한 뒤 Provider URL을 http://localhost:{} 로 연결 테스트해 주세요.",
                                TABBY_API_DEFAULT_PORT
                            );
                            self.status = msg.clone();
                            self.tabby_status = Some(Err(msg));
                            return Task::none();
                        }
                        if matches!(
                            eng,
                            InferenceEngine::XLlm
                                | InferenceEngine::VLlm
                                | InferenceEngine::LlamaServer
                        ) && !self.has_selected_local_model_available()
                        {
                            self.status = "Selected local model was not found in the current model directory. Verify Models > download status and Runtime > model directory/path, then try Start again.".into();
                            return Task::none();
                        }
                        // xLLM/vLLM/llama-server는 받은 폴더를 absolute path로
                        let abs_model = if matches!(
                            eng,
                            InferenceEngine::TabbyMl | InferenceEngine::TabbyApi
                        ) {
                            // Tabby는 카탈로그 ID 그대로
                            model.clone()
                        } else {
                            resolve_user_path(&self.model_dir_input)
                                .join(&model)
                                .display()
                                .to_string()
                        };
                        let Some(cmd) = eng.compose_command(&abs_model, port) else {
                            return Task::none();
                        };
                        let mut iter = cmd.into_iter();
                        let p = iter.next().unwrap_or_default();
                        (p, iter.collect::<Vec<_>>())
                    }
                };

                // URL/라벨 자동 등록 (시작 시점)
                self.tabby_url_input = format!("http://localhost:{}", port);
                let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                if self.openai_compat_label.trim().is_empty() {
                    let label = self
                        .inference_engine
                        .label()
                        .split_whitespace()
                        .next()
                        .unwrap_or("Local")
                        .to_string();
                    self.openai_compat_label = label.clone();
                    let _ = keystore::write_openai_compat_label(&label);
                }

                // 바이너리 경로가 명시되어 있으면 PATH 의존 안 하고 절대 경로 사용
                let program_hint = program.clone();
                let (final_program, args, work_dir) =
                    self.resolve_runtime_spawn_command(program, args);
                if matches!(
                    self.inference_engine,
                    InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer
                ) && !runtime_command_exists(&final_program)
                {
                    let override_path = self.inference_binary_path.trim();
                    if !override_path.is_empty()
                        && std::path::Path::new(override_path).is_dir()
                        && std::path::Path::new(&final_program)
                            == std::path::Path::new(override_path)
                    {
                        let expected = expected_binary_name(&program_hint);
                        self.status = format!(
                            "Runtime binary path '{}' is a directory, but '{}' was not found inside it. Select the executable file directly or place '{}' in that folder.",
                            override_path, expected, expected
                        );
                        return Task::none();
                    }
                    self.status = if matches!(self.inference_engine, InferenceEngine::XLlm) {
                        "xLLM binary was not found on this machine. Set Runtime > binary path to a host xllm executable, or use Engine=Custom and run xLLM through Docker.".into()
                    } else {
                        format!(
                            "{} binary was not found. Set Runtime > binary path to the executable or install/add it to PATH.",
                            self.inference_engine.label()
                        )
                    };
                    return Task::none();
                }
                self.inference_log.clear();
                self.tabby_connect_retry_left = TABBY_CONNECT_RETRIES_AFTER_START;
                self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
                self.status = format!("실행 시작: {} {}", final_program, args.join(" "));
                Task::batch(vec![
                    Task::run(
                        spawn_inference_stream(final_program, args, work_dir),
                        |ev| ev,
                    ),
                    Task::perform(
                        async {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        },
                        {
                            let generation = self.tabby_retry_generation;
                            move |_| Message::FetchTabbyModelsRetry(generation)
                        },
                    ),
                ])
            }
            Message::StopInference => self.stop_inference(),
            Message::InferenceLogLine(line) => self.on_inference_log_line(line),
            Message::InferenceExited(code) => self.on_inference_exited(code),
            Message::OpenAICompatLabelChanged(v) => self.set_openai_compat_label(v),
            Message::SaveTabby => self.save_tabby_settings(),
            Message::TabbySaved(r) => self.on_tabby_saved(r),
            Message::ClearTabby => self.clear_tabby_settings(),
            Message::FetchTabbyModels => self.fetch_tabby_models(),
            Message::FetchTabbyModelsRetry(generation) => self.retry_fetch_tabby_models(generation),
            Message::TabbyModelsLoaded(r) => self.on_tabby_models_loaded(r),
            // ── HF 모델 매니저 ────────────────────────────────────
            Message::HfTokenChanged(v) => self.set_hf_token_input(v),
            Message::ToggleHfTokenVisible => self.toggle_hf_token_visible(),
            Message::SaveHfToken => self.save_hf_token(),
            Message::HfTokenSaved(r) => self.on_hf_token_saved(r),
            Message::ModelDirChanged(v) => self.set_model_dir(v),
            Message::PickModelDir => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::ModelDirPicked,
            ),
            Message::ModelDirPicked(maybe) => self.on_model_dir_picked(maybe),
            Message::HfRepoChanged(v) => self.set_hf_repo_input(v),
            Message::UsePreset(idx) => self.apply_model_preset(idx),
            Message::DownloadExl2Preset(idx) => self.prepare_exl2_preset_download(idx),
            Message::SelectDownloadedModel(folder_name) => {
                self.select_downloaded_model(folder_name)
            }
            Message::StartHfDownload => self.start_hf_download(),
            Message::HfDownloadEvent(ev) => self.on_hf_download_event(ev),
            Message::CancelHfDownload => self.cancel_hf_download(),
            Message::RegenerateLast => self.regenerate_last(),
            Message::ApplyChange(block_id, idx) => self.apply_change(block_id, idx),
            Message::EditLastUser => self.edit_last_user(),

            // ── 파일 컨텍스트 첨부 ────────────────────────────────
            Message::FileDropped(path) => self.on_file_dropped(path),
            Message::FileDragHover => self.file_drag_hover(),
            Message::FileReadDone(path, content) => self.on_file_read_done(path, content),
            Message::FileAttachError(msg) => self.file_attach_error(msg),

            // ── MCP ───────────────────────────────────────────────────
            Message::McpNameChanged(v) => self.update_mcp_name_input(v),
            Message::McpCommandChanged(v) => self.update_mcp_command_input(v),
            Message::AddMcpServer => self.add_mcp_server(),
            Message::RemoveMcpServer(idx) => self.remove_mcp_server(idx),
            Message::McpToolsLoaded(server_name, tools) => {
                self.on_mcp_tools_loaded(server_name, tools)
            }
            Message::McpToolsFailed(msg) => self.on_mcp_tools_failed(msg),
            Message::McpToolResult(tool_call_id, result) => {
                self.on_mcp_tool_result(tool_call_id, result)
            }

            // ── PTY 터미널 ─────────────────────────────────────────
            Message::PtyToggle => self.toggle_pty(),
            Message::PtyStart => self.pty_start(),
            Message::PtyLine(line) => self.on_pty_line(line),
            Message::PtyExited => self.on_pty_exited(),
            Message::PtyInputChanged(v) => self.set_pty_input(v),
            Message::PtySend => self.send_pty_input(),
            Message::PtyCtrlC => self.pty_ctrl_c(),
            Message::PtyClear => self.pty_clear(),
            Message::RemoveAttachment(idx) => self.remove_attachment(idx),
            Message::ClearAttachments => self.clear_attachments(),

            // ── @-mention ─────────────────────────────────────────
            Message::MentionMove(delta) => self.move_mention_selection(delta),
            Message::MentionConfirm => self.confirm_mention(),
            Message::MentionCandidatesLoaded(paths) => self.load_mention_candidates(paths),

            Message::FetchModels => self.fetch_models_cmd(),
            Message::ModelsLoaded(r) => self.on_models_loaded(r),
            Message::SelectModel(opt) => self.select_model(opt),
            Message::FetchAccount => self.fetch_account_cmd(),
            Message::AccountLoaded(r) => self.on_account_loaded(r),
            Message::InputChanged(v) => self.on_input_changed(v),
            Message::Send => {
                let text = self.input.trim().to_string();
                if text.is_empty() {
                    return Task::none();
                }
                // 슬래시 커맨드 처리
                match text.as_str() {
                    "/plan" => {
                        self.agent_mode = AgentMode::Plan;
                        self.input.clear();
                        self.status = format!("{} 모드", AgentMode::Plan.label());
                        return Task::none();
                    }
                    "/build" => {
                        self.agent_mode = AgentMode::Build;
                        self.input.clear();
                        self.status = format!("{} 모드", AgentMode::Build.label());
                        return Task::none();
                    }
                    s if s.starts_with('/') => {
                        self.status = format!("알 수 없는 슬래시 명령: {}", s);
                        return Task::none();
                    }
                    _ => {}
                }
                if self.streaming_block_id.is_some() || self.compare_pending {
                    return Task::none();
                }
                if self.compare_both {
                    let (openrouter_route, tabby_route) = match self.compare_routes() {
                        Ok(v) => v,
                        Err(e) => {
                            self.status = e;
                            return Task::none();
                        }
                    };

                    self.ensure_system_message();
                    let user_msg = if !self.attached_files.is_empty() {
                        let ctx = build_file_context(&self.attached_files);
                        format!("{ctx}\n\n{text}")
                    } else {
                        text.clone()
                    };
                    self.conversation.push(ChatMessage::user(user_msg));
                    self.attached_files.clear();
                    self.close_mention();
                    self.pending_tool_calls.clear();
                    self.tool_round = 0;
                    let messages = self.conversation.clone();

                    let user_id = self.next_id();
                    self.blocks.push(Block {
                        id: user_id,
                        body: BlockBody::User(text),
                        view_mode: ViewMode::Rendered,
                        md_items: Vec::new(),
                        model: None,
                        apply_candidates: Vec::new(),
                    });
                    let openrouter_block_id = self.next_id();
                    self.blocks.push(Block {
                        id: openrouter_block_id,
                        body: BlockBody::Assistant(text_editor::Content::with_text(
                            "OpenRouter 응답 대기 중…",
                        )),
                        view_mode: ViewMode::Raw,
                        md_items: Vec::new(),
                        model: Some(format!(
                            "{}: {}",
                            openrouter_route.label, openrouter_route.model
                        )),
                        apply_candidates: Vec::new(),
                    });
                    let tabby_block_id = self.next_id();
                    self.blocks.push(Block {
                        id: tabby_block_id,
                        body: BlockBody::Assistant(text_editor::Content::with_text(
                            "Tabby 응답 대기 중…",
                        )),
                        view_mode: ViewMode::Raw,
                        md_items: Vec::new(),
                        model: Some(format!("{}: {}", tabby_route.label, tabby_route.model)),
                        apply_candidates: Vec::new(),
                    });

                    self.input.clear();
                    self.compare_pending = true;
                    self.status = "Compare 응답 생성 중…".into();
                    self.follow_bottom = true;

                    let openrouter_messages = messages.clone();
                    let tabby_messages = messages;
                    let task = Task::perform(
                        async move {
                            let openrouter = collect_chat_text(
                                openrouter_route.base_url,
                                openrouter_route.api_key,
                                openrouter_route.model,
                                openrouter_messages,
                            );
                            let tabby = collect_chat_text(
                                tabby_route.base_url,
                                tabby_route.api_key,
                                tabby_route.model,
                                tabby_messages,
                            );
                            tokio::join!(openrouter, tabby)
                        },
                        move |(openrouter_result, tabby_result)| Message::CompareResponsesLoaded {
                            openrouter_block_id,
                            tabby_block_id,
                            openrouter_result,
                            tabby_result,
                        },
                    );
                    return Task::batch(vec![snap_to_end(self.stream_id.clone()), task]);
                }
                let (base_url, api_key) = match self.resolve_provider() {
                    Ok(v) => v,
                    Err(e) => {
                        self.status = e;
                        return Task::none();
                    }
                };
                let Some(model) = self.selected_model.clone() else {
                    self.status = "모델을 먼저 선택해주세요.".into();
                    return Task::none();
                };

                // 새 turn 시작: system 메시지(cwd 안내) 보장 → user 메시지 push.
                self.ensure_system_message();
                // 첨부 파일이 있으면 user 메시지 앞에 파일 컨텍스트를 붙임
                let user_msg = if !self.attached_files.is_empty() {
                    let ctx = build_file_context(&self.attached_files);
                    format!("{ctx}\n\n{text}")
                } else {
                    text.clone()
                };
                self.conversation.push(ChatMessage::user(user_msg));
                self.attached_files.clear();
                self.close_mention();
                self.pending_tool_calls.clear();
                self.tool_round = 0;
                let messages = self.conversation.clone();

                let user_id = self.next_id();
                self.blocks.push(Block {
                    id: user_id,
                    body: BlockBody::User(text),
                    view_mode: ViewMode::Rendered,
                    md_items: Vec::new(),
                    model: None,
                    apply_candidates: Vec::new(),
                });
                let ai_id = self.next_id();
                self.blocks.push(Block {
                    id: ai_id,
                    body: BlockBody::Assistant(text_editor::Content::new()),
                    view_mode: ViewMode::Raw,
                    md_items: Vec::new(),
                    model: self.selected_model.clone(),
                    apply_candidates: Vec::new(),
                });
                self.streaming_block_id = Some(ai_id);
                self.input.clear();
                self.status = "응답 생성 중…".into();
                self.follow_bottom = true; // 새 메시지 전송 시 follow ON

                let (chat_task, handle) = Task::run(
                    openrouter::chat_stream(
                        base_url,
                        api_key,
                        model,
                        messages,
                        self.tool_definitions_for_selected_model(),
                    ),
                    Message::ChatChunk,
                )
                .abortable();
                self.abort_handle = Some(handle);
                Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
            }
            Message::StopStream => self.stop_stream(),
            Message::CopyBlock(id) => self.copy_block(id),
            Message::CopyText(text) => iced::clipboard::write(text),
            Message::CompareResponsesLoaded {
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            } => self.on_compare_responses_loaded(
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            ),
            Message::ChatChunk(event) => {
                let Some(ai_id) = self.streaming_block_id else {
                    return Task::none();
                };
                match event {
                    ChatEvent::Token(t) => {
                        self.append_assistant_block_text(ai_id, &t);
                    }
                    ChatEvent::ToolCallDelta {
                        index,
                        id,
                        name,
                        arguments,
                    } => {
                        let i = index as usize;
                        while self.pending_tool_calls.len() <= i {
                            self.pending_tool_calls.push(PendingToolCall::default());
                        }
                        let tc = &mut self.pending_tool_calls[i];
                        if let Some(id) = id {
                            tc.id = id;
                        }
                        if let Some(name) = name {
                            tc.name = name;
                        }
                        if let Some(args) = arguments {
                            tc.arguments.push_str(&args);
                        }
                    }
                    ChatEvent::Done {
                        finish_reason,
                        generation_id,
                    } => {
                        // 현재 assistant block에 누적된 텍스트
                        let assistant_text = self
                            .blocks
                            .iter()
                            .find(|b| b.id == ai_id)
                            .and_then(|b| match &b.body {
                                BlockBody::Assistant(c) => Some(c.text()),
                                _ => None,
                            })
                            .unwrap_or_default();

                        let has_tools = !self.pending_tool_calls.is_empty()
                            && (finish_reason.as_deref() == Some("tool_calls")
                                || finish_reason.is_none());

                        if has_tools && self.tool_round < MAX_TOOL_ROUNDS {
                            return self.run_tool_round(assistant_text);
                        }

                        // 정상 종료 (또는 라운드 한도 초과)
                        if self.tool_round >= MAX_TOOL_ROUNDS && !self.pending_tool_calls.is_empty()
                        {
                            self.status = format!("최대 도구 라운드 {} 초과", MAX_TOOL_ROUNDS);
                        } else {
                            self.status = "준비됨".into();
                        }
                        if !assistant_text.is_empty() {
                            self.conversation
                                .push(ChatMessage::assistant(assistant_text.clone()));
                        } else {
                            self.status =
                                "[WARN] 모델이 빈 응답을 반환했습니다. Provider/Runtime 로그를 확인해 주세요.".into();
                            if let Some(b) = self.blocks.iter().find(|b| b.id == ai_id) {
                                if b.body.to_text().trim().is_empty() {
                                    self.append_assistant_block_text(
                                        ai_id,
                                        "[WARN] empty response",
                                    );
                                }
                            }
                        }
                        // Apply 후보 추출
                        let candidates = parse_apply_candidates(&assistant_text);
                        if !candidates.is_empty() {
                            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == ai_id) {
                                b.apply_candidates =
                                    candidates.into_iter().map(|c| (c, false)).collect();
                            }
                        }
                        self.streaming_block_id = None;
                        self.abort_handle = None;
                        self.pending_tool_calls.clear();
                        self.maybe_update_title();
                        self.save_session();
                        if let Some(id) = generation_id {
                            if let Ok(api_key) = keystore::read_api_key() {
                                return Task::perform(
                                    openrouter::get_generation(api_key, id),
                                    Message::GenerationLoaded,
                                );
                            }
                        }
                    }
                    ChatEvent::Error(e) => {
                        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == ai_id) {
                            if let BlockBody::Assistant(content) = &mut b.body {
                                let prefix = if content.text().is_empty() {
                                    ""
                                } else {
                                    "\n\n"
                                };
                                let msg = format!("{}[ERROR] {}", prefix, e);
                                let mut raw = content.text();
                                raw.push_str(&msg);
                                *content = text_editor::Content::with_text(&raw);
                                b.md_items = markdown::parse(&raw).collect();
                            }
                        }
                        self.streaming_block_id = None;
                        self.abort_handle = None;
                        self.pending_tool_calls.clear();
                        let humanized = openrouter::humanize_error(&e);
                        if e.contains("OpenRouter 401") || e.contains("OpenRouter 402") {
                            self.status = format!(
                                "[WARN] {} | Open Settings and check API key / credits",
                                humanized
                            );
                        } else {
                            self.status = format!("[ERROR] {}", humanized);
                        }
                    }
                }
                if self.follow_bottom {
                    snap_to_end(self.stream_id.clone())
                } else {
                    Task::none()
                }
            }
            Message::StreamScrolled(viewport) => self.on_stream_scrolled(&viewport),
            Message::EditorAction(id, action) => self.on_editor_action(id, action),
            Message::ToggleBlockView(id) => self.toggle_block_view(id),
            Message::LinkClicked(uri) => self.on_link_clicked(&uri),
            Message::PickCwd => self.pick_cwd(),
            Message::PickAttachment => self.pick_attachment(),
            Message::AttachmentPicked(maybe_path) => self.on_attachment_picked(maybe_path),
            Message::ApproveWrites => self.approve_pending_writes(),
            Message::DenyWrites => self.deny_pending_writes(),
            Message::ToggleConfirmExpand(idx) => self.toggle_write_confirm_expand(idx),
            Message::DiscardWriteCall(idx) => self.discard_write_call(idx),
            Message::ToggleFilterCoding(v) => self.set_filter_coding(v),
            Message::ToggleFilterReasoning(v) => self.set_filter_reasoning(v),
            Message::ToggleFilterGeneral(v) => self.set_filter_general(v),
            Message::ToggleFilterFavorites(v) => self.set_filter_favorites_only(v),
            Message::ToggleCompareBoth(v) => self.set_compare_both(v),
            Message::CycleSortMode => self.cycle_model_sort_mode(),
            Message::SetAgentMode(mode) => self.set_agent_mode(mode),
            Message::ToggleAgentMode => self.toggle_agent_mode(),
            Message::NewChat => self.new_chat(),
            Message::SwitchSession(target_id) => self.switch_session(target_id),
            Message::OpenCommandPalette => self.open_command_palette(),
            Message::CloseCommandPalette => self.close_command_palette(),
            Message::CloseAllOverlays => self.close_all_overlays(),
            Message::CommandPaletteChanged(v) => self.update_command_palette_input(v),
            Message::ExecuteCommand(idx) => self.execute_palette_command(idx),
            Message::GenerationLoaded(r) => self.on_generation_loaded(r),
            Message::AskDeleteSession(id) => self.ask_delete_session(id),
            Message::CancelDeleteSession => self.cancel_delete_session(),
            Message::DeleteSession(target_id) => self.delete_session(target_id),
            Message::ToggleFavorite => self.toggle_favorite(),
            Message::CwdPicked(maybe_path) => self.apply_picked_cwd(maybe_path),
        }
    }

    fn open_settings_overlay(&mut self) -> Task<Message> {
        self.ui.show_settings = true;
        self.ui.settings_tab = SettingsTab::Provider;
        Task::none()
    }

    fn close_settings_overlay(&mut self) -> Task<Message> {
        self.ui.show_settings = false;
        Task::none()
    }

    fn set_settings_tab(&mut self, tab: SettingsTab) -> Task<Message> {
        self.ui.settings_tab = tab;
        Task::none()
    }

    fn update_mcp_name_input(&mut self, value: String) -> Task<Message> {
        self.mcp_input.name_input = value;
        Task::none()
    }

    fn update_mcp_command_input(&mut self, value: String) -> Task<Message> {
        self.mcp_input.command_input = value;
        Task::none()
    }

    fn toggle_write_confirm_expand(&mut self, idx: usize) -> Task<Message> {
        self.ui.expanded_confirm_idx = if self.ui.expanded_confirm_idx == Some(idx) {
            None
        } else {
            Some(idx)
        };
        Task::none()
    }

    fn set_filter_coding(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_coding = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    fn set_filter_reasoning(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_reasoning = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    fn set_filter_general(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_general = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    fn set_filter_favorites_only(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_favorites_only = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    fn cycle_model_sort_mode(&mut self) -> Task<Message> {
        self.model_filter.sort_mode = self.model_filter.sort_mode.cycle();
        self.refresh_model_combo();
        Task::none()
    }

    fn open_command_palette(&mut self) -> Task<Message> {
        self.ui.show_command_palette = true;
        self.ui.command_palette_input.clear();
        Task::none()
    }

    fn close_command_palette(&mut self) -> Task<Message> {
        self.ui.show_command_palette = false;
        Task::none()
    }

    fn update_command_palette_input(&mut self, value: String) -> Task<Message> {
        self.ui.command_palette_input = value;
        Task::none()
    }

    fn ask_delete_session(&mut self, id: u64) -> Task<Message> {
        self.ui.pending_delete_session = if self.ui.pending_delete_session == Some(id) {
            None
        } else {
            Some(id)
        };
        Task::none()
    }

    fn cancel_delete_session(&mut self) -> Task<Message> {
        self.ui.pending_delete_session = None;
        Task::none()
    }

    fn toggle_favorite(&mut self) -> Task<Message> {
        if let Some(id) = &self.selected_model {
            if self.model_filter.favorites.contains(id) {
                self.model_filter.favorites.remove(id);
            } else {
                self.model_filter.favorites.insert(id.clone());
            }
            let favs: Vec<String> = self.model_filter.favorites.iter().cloned().collect();
            let _ = session::write_favorites(&favs);
            self.refresh_model_combo();
        }
        Task::none()
    }

    fn set_compare_both(&mut self, enabled: bool) -> Task<Message> {
        self.compare_both = enabled;
        self.status = if enabled {
            "Compare 모드 — OpenRouter와 Tabby가 각각 답변합니다.".into()
        } else {
            "Single 모드 — 선택한 모델 하나만 답변합니다.".into()
        };
        Task::none()
    }

    fn set_agent_mode(&mut self, mode: AgentMode) -> Task<Message> {
        self.agent_mode = mode;
        self.status = format!("{} 모드", mode.label());
        Task::none()
    }

    fn toggle_agent_mode(&mut self) -> Task<Message> {
        self.agent_mode = match self.agent_mode {
            AgentMode::Plan => AgentMode::Build,
            AgentMode::Build => AgentMode::Plan,
        };
        self.status = format!("{} 모드", self.agent_mode.label());
        Task::none()
    }

    fn close_all_overlays(&mut self) -> Task<Message> {
        self.ui.show_command_palette = false;
        self.ui.show_settings = false;
        self.show_write_confirm = false;
        self.close_mention();
        Task::none()
    }

    fn execute_palette_command(&mut self, idx: usize) -> Task<Message> {
        let filtered = self.filtered_palette_commands();
        let Some(cmd) = filtered.get(idx) else {
            return Task::none();
        };
        let action = cmd.action;
        self.ui.show_command_palette = false;
        self.ui.command_palette_input.clear();
        match action {
            PaletteAction::NewChat => Task::done(Message::NewChat),
            PaletteAction::PlanMode => Task::done(Message::SetAgentMode(AgentMode::Plan)),
            PaletteAction::BuildMode => Task::done(Message::SetAgentMode(AgentMode::Build)),
            PaletteAction::OpenSettings => Task::done(Message::OpenSettings),
            PaletteAction::PickCwd => Task::done(Message::PickCwd),
            PaletteAction::CycleSort => Task::done(Message::CycleSortMode),
            PaletteAction::ToggleFavorite => Task::done(Message::ToggleFavorite),
        }
    }

    fn apply_picked_cwd(&mut self, maybe_path: Option<std::path::PathBuf>) -> Task<Message> {
        if let Some(path) = maybe_path {
            self.cwd = path.clone();
            let _ = keystore::write_cwd(&path.display().to_string());
            self.status = format!("작업 폴더: {}", path.display());
            self.ensure_system_message();
        }
        Task::none()
    }

    fn set_key_input(&mut self, value: String) -> Task<Message> {
        self.key_input = value;
        Task::none()
    }

    fn set_tabby_url(&mut self, value: String) -> Task<Message> {
        self.tabby_url_input = value;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }

    fn set_tabby_token(&mut self, value: String) -> Task<Message> {
        self.tabby_token_input = value;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }

    fn toggle_tabby_token_visible(&mut self) -> Task<Message> {
        self.show_tabby_token = !self.show_tabby_token;
        Task::none()
    }

    fn set_inference_command(&mut self, value: String) -> Task<Message> {
        self.inference_command_input = value.clone();
        let _ = keystore::write_inference_command(&value);
        Task::none()
    }

    fn set_inference_model(&mut self, value: String) -> Task<Message> {
        self.inference_selected_model = value;
        Task::none()
    }

    fn set_hf_token_input(&mut self, value: String) -> Task<Message> {
        self.hf_token_input = value;
        Task::none()
    }

    fn toggle_hf_token_visible(&mut self) -> Task<Message> {
        self.show_hf_token = !self.show_hf_token;
        Task::none()
    }

    fn set_hf_repo_input(&mut self, value: String) -> Task<Message> {
        self.hf_repo_input = value;
        Task::none()
    }

    fn set_pty_input(&mut self, value: String) -> Task<Message> {
        self.pty_input = value;
        Task::none()
    }

    fn pty_ctrl_c(&mut self) -> Task<Message> {
        if let Some(s) = &self.pty_session {
            s.ctrl_c();
        }
        Task::none()
    }

    fn pty_clear(&mut self) -> Task<Message> {
        self.pty_output.clear();
        Task::none()
    }

    fn file_drag_hover(&mut self) -> Task<Message> {
        Task::none()
    }

    fn file_attach_error(&mut self, msg: String) -> Task<Message> {
        self.status = msg;
        Task::none()
    }

    // ── Key persistence helpers ──────────────────────────────────

    fn save_api_key(&mut self) -> Task<Message> {
        let key = self.key_input.clone();
        self.busy = true;
        self.status = "키 저장 중…".into();
        Task::perform(
            async move { keystore::write_api_key(&key) },
            Message::KeySaved,
        )
    }

    fn on_key_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.has_key = true;
                self.key_input.clear();
                self.ui.show_settings = false;
                self.status = "키 저장됨".into();
                Task::done(Message::FetchModels)
            }
            Err(e) => {
                self.status = format!("저장 실패: {}", e);
                Task::none()
            }
        }
    }

    fn clear_api_key(&mut self) -> Task<Message> {
        self.busy = true;
        Task::perform(async { keystore::delete_api_key() }, Message::KeyCleared)
    }

    fn on_key_cleared(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.has_key = false;
                self.models.clear();
                self.model_ids.clear();
                self.selected_model = None;
                self.selected_model_provider = None;
                let _ = keystore::clear_selected_model();
                self.status = "키 삭제됨".into();
            }
            Err(e) => self.status = format!("삭제 실패: {}", e),
        }
        Task::none()
    }

    // ── Tabby connection helpers ──────────────────────────────────

    fn set_openai_compat_label(&mut self, value: String) -> Task<Message> {
        self.openai_compat_label = value;
        let _ = keystore::write_openai_compat_label(&self.openai_compat_label);
        let new_label = self.openai_compat_label.clone();
        for opt in &mut self.model_options {
            if opt.provider == LlmProvider::OpenAICompat {
                opt.provider_label = new_label.clone();
            }
        }
        self.refresh_model_combo();
        Task::none()
    }

    fn save_tabby_settings(&mut self) -> Task<Message> {
        let url = self.tabby_url_input.clone();
        let token = self.tabby_token_input.clone();
        self.busy = true;
        self.status = "Tabby 설정 저장 중…".into();
        Task::perform(
            async move {
                keystore::write_tabby_base_url(&url)?;
                keystore::write_tabby_token(&token)?;
                Ok(())
            },
            Message::TabbySaved,
        )
    }

    fn on_tabby_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.status = "Tabby 설정 저장됨".into();
                if !self.tabby_url_input.trim().is_empty() {
                    return Task::done(Message::FetchTabbyModels);
                }
            }
            Err(e) => self.status = format!("Tabby 저장 실패: {}", e),
        }
        Task::none()
    }

    fn clear_tabby_settings(&mut self) -> Task<Message> {
        let _ = keystore::clear_tabby_base_url();
        let _ = keystore::clear_tabby_token();
        self.tabby_url_input.clear();
        self.tabby_token_input.clear();
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        self.tabby_status = None;
        self.status = "Tabby 설정 삭제됨".into();
        self.model_options
            .retain(|o| o.provider != LlmProvider::OpenAICompat);
        self.refresh_model_combo();
        if let Some(sel) = self.selected_model.clone() {
            if !self.model_options.iter().any(|o| o.id == sel) {
                if let Some(first) = self.model_options.first() {
                    self.selected_model = Some(first.id.clone());
                    self.selected_model_provider = Some(first.provider);
                } else {
                    self.selected_model = None;
                    self.selected_model_provider = None;
                }
                if let Some(id) = &self.selected_model {
                    let _ = keystore::write_selected_model(id);
                }
            }
        }
        Task::none()
    }

    // ── Inference/Model dir helpers ───────────────────────────────

    fn set_inference_binary(&mut self, value: String) -> Task<Message> {
        self.inference_binary_path = value.clone();
        let _ = keystore::write_inference_binary(&value);
        Task::none()
    }

    fn set_model_dir(&mut self, value: String) -> Task<Message> {
        self.model_dir_input = value.clone();
        let _ = keystore::write_model_dir(&value);
        self.sync_selected_local_model_for_model_dir();
        Task::none()
    }

    // ── PTY helpers ───────────────────────────────────────────────

    fn toggle_pty(&mut self) -> Task<Message> {
        self.pty_visible = !self.pty_visible;
        if self.pty_visible && self.pty_session.is_none() {
            return Task::done(Message::PtyStart);
        }
        Task::none()
    }

    fn send_pty_input(&mut self) -> Task<Message> {
        let line = self.pty_input.trim_end().to_string();
        if let Some(s) = &self.pty_session {
            s.write_line(&line);
        }
        self.pty_input.clear();
        Task::none()
    }

    // ── Attachment helpers ────────────────────────────────────────

    fn remove_attachment(&mut self, idx: usize) -> Task<Message> {
        if idx < self.attached_files.len() {
            let removed = self.attached_files.remove(idx);
            let removed_name = removed
                .0
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| removed.0.display().to_string());
            self.status = format!(
                "Removed attachment: {} ({} left)",
                removed_name,
                self.attached_files.len()
            );
        }
        Task::none()
    }

    fn clear_attachments(&mut self) -> Task<Message> {
        if !self.attached_files.is_empty() {
            let removed_count = self.attached_files.len();
            let removed_bytes: u64 = self
                .attached_files
                .iter()
                .map(|(_, content)| content.len() as u64)
                .sum();
            self.attached_files.clear();
            self.status = format!(
                "Cleared attachments: {} files ({})",
                removed_count,
                fmt_bytes(removed_bytes)
            );
        }
        Task::none()
    }

    // ── Mention helpers ───────────────────────────────────────────

    fn move_mention_selection(&mut self, delta: i32) -> Task<Message> {
        if !self.show_mention || self.mention_candidates.is_empty() {
            return Task::none();
        }
        let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
        let n = filtered.len();
        if n == 0 {
            return Task::none();
        }
        self.mention_selected =
            (self.mention_selected as i64 + delta as i64).rem_euclid(n as i64) as usize;
        Task::none()
    }

    fn confirm_mention(&mut self) -> Task<Message> {
        if !self.show_mention {
            return Task::none();
        }
        let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
        let Some(chosen) = filtered.into_iter().nth(self.mention_selected) else {
            return Task::none();
        };
        if let Some(at_pos) = self.input.rfind('@') {
            self.input.truncate(at_pos);
        }
        self.close_mention();
        if self.is_already_attached(&chosen) {
            self.status = format!("Already attached: {}", chosen.display());
            return Task::none();
        }
        let full_path = self.cwd.join(&chosen);
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&full_path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        chosen.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((chosen, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }

    fn load_mention_candidates(&mut self, paths: Vec<std::path::PathBuf>) -> Task<Message> {
        self.mention_candidates = paths;
        Task::none()
    }

    // ── Write confirm helpers ────────────────────────────────────

    fn approve_pending_writes(&mut self) -> Task<Message> {
        self.ui.expanded_confirm_idx = None;
        self.continue_after_writes(true)
    }

    fn deny_pending_writes(&mut self) -> Task<Message> {
        self.ui.expanded_confirm_idx = None;
        self.continue_after_writes(false)
    }

    // ── HF token helpers ─────────────────────────────────────────

    fn save_hf_token(&mut self) -> Task<Message> {
        let t = self.hf_token_input.clone();
        Task::perform(
            async move { keystore::write_hf_token(&t) },
            Message::HfTokenSaved,
        )
    }

    fn on_hf_token_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => self.status = "HF 토큰 저장됨".into(),
            Err(e) => self.status = format!("HF 토큰 저장 실패: {}", e),
        }
        Task::none()
    }

    // ── HF preset helpers ────────────────────────────────────────

    fn apply_model_preset(&mut self, idx: usize) -> Task<Message> {
        if let Some(p) = MODEL_PRESETS.get(idx) {
            self.hf_repo_input = p.repo_id.into();
            self.hf_revision = None;
            self.hf_folder_name = None;
        }
        Task::none()
    }

    fn prepare_exl2_preset_download(&mut self, idx: usize) -> Task<Message> {
        if let Some(p) = EXL2_PRESETS.get(idx) {
            self.hf_repo_input = p.repo_id.into();
            self.hf_revision = Some(p.revision.into());
            self.hf_folder_name = Some(p.folder_name.into());
            self.status = format!(
                "프리셋 다운로드 시작 준비: {} ({} @ {})",
                p.label, p.repo_id, p.revision
            );
            return Task::done(Message::StartHfDownload);
        }
        self.status = format!("잘못된 프리셋 인덱스: {}", idx);
        Task::none()
    }

    // ── File attachment result helpers ────────────────────────────

    fn on_file_read_done(&mut self, path: std::path::PathBuf, content: String) -> Task<Message> {
        if !self.is_already_attached(&path) {
            self.attached_files.push((path, content));
            let current_total = self.total_attached_bytes();
            self.status = format!(
                "Attached ({} files): {}/{}",
                self.attached_files.len(),
                fmt_bytes(current_total),
                fmt_bytes(MAX_ATTACH_BYTES)
            );
        } else {
            self.status = format!("Already attached: {}", path.display());
        }
        Task::none()
    }

    // ── Model select / account helpers ────────────────────────────

    fn select_model(&mut self, opt: ModelOption) -> Task<Message> {
        let _ = keystore::write_selected_model(&opt.id);
        self.selected_model_provider = Some(opt.provider);
        self.selected_model = Some(opt.id);
        Task::none()
    }

    fn on_account_loaded(
        &mut self,
        result: Result<openrouter::AuthKeyData, String>,
    ) -> Task<Message> {
        if let Ok(data) = result {
            self.account = Some(data);
        }
        Task::none()
    }

    fn fetch_models_cmd(&mut self) -> Task<Message> {
        let key = match keystore::read_api_key() {
            Ok(k) => k,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };
        self.busy = true;
        self.status = "모델 리스트 가져오는 중…".into();
        Task::perform(openrouter::list_models(key), Message::ModelsLoaded)
    }

    fn on_models_loaded(
        &mut self,
        result: Result<Vec<openrouter::OpenRouterModel>, String>,
    ) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(models) => {
                let n = models.len();
                self.model_ids = models.iter().map(|m| m.id.clone()).collect();
                self.model_options
                    .retain(|o| o.provider != LlmProvider::OpenRouter);
                self.model_options.extend(models.iter().map(|m| {
                    let id = m.id.clone();
                    let ko_friendly = is_korean_friendly(&id);
                    let favorite = self.model_filter.favorites.contains(&id);
                    ModelOption {
                        id,
                        provider: LlmProvider::OpenRouter,
                        provider_label: String::new(),
                        ko_friendly,
                        favorite,
                        context_length: m.context_length,
                        prompt_per_million: parse_price_per_million(
                            m.pricing.as_ref().and_then(|p| p.prompt.as_deref()),
                        ),
                        completion_per_million: parse_price_per_million(
                            m.pricing.as_ref().and_then(|p| p.completion.as_deref()),
                        ),
                    }
                }));
                self.refresh_model_combo();
                let saved_in_list = self.selected_model_exists_in_options();
                if !saved_in_list && self.tabby_url_input.trim().is_empty() {
                    self.selected_model = self.model_ids.first().cloned();
                    self.selected_model_provider = self
                        .selected_model
                        .as_ref()
                        .map(|_| LlmProvider::OpenRouter);
                    if let Some(id) = &self.selected_model {
                        let _ = keystore::write_selected_model(id);
                    }
                }
                self.models = models;
                self.status = format!("모델 {} 로드됨", n);
            }
            Err(e) => self.status = format!("페치 실패: {}", openrouter::humanize_error(&e)),
        }
        Task::none()
    }

    fn on_input_changed(&mut self, value: String) -> Task<Message> {
        self.input = value;
        match extract_mention_query(&self.input) {
            Some(q) => {
                self.mention_query = q.to_string();
                self.mention_selected = 0;
                if !self.show_mention {
                    self.show_mention = true;
                    let cwd = self.cwd.clone();
                    return Task::perform(
                        collect_mention_candidates(cwd),
                        Message::MentionCandidatesLoaded,
                    );
                }
            }
            None => {
                if self.show_mention {
                    self.close_mention();
                }
            }
        }
        Task::none()
    }

    // ── Session lifecycle helpers ──────────────────────────────────

    fn new_chat(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.snapshot_current_to_inactive();
        self.blocks.clear();
        self.conversation.clear();
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.show_write_confirm = false;
        self.streaming_block_id = None;
        self.tool_round = 0;
        self.next_block_id = 0;
        self.input.clear();
        self.ui.pending_delete_session = None;
        self.current_session_id = self.allocate_session_id();
        self.current_session_title = "새 채팅".into();
        self.status = "새 채팅".into();
        self.save_session();
        Task::none()
    }

    fn delete_session(&mut self, target_id: u64) -> Task<Message> {
        self.ui.pending_delete_session = None;
        if target_id == self.current_session_id {
            self.blocks.clear();
            self.conversation.clear();
            self.next_block_id = 0;
            self.current_session_id = self.allocate_session_id();
            self.current_session_title = "새 채팅".into();
        } else {
            self.inactive_sessions.retain(|s| s.id != target_id);
        }
        self.save_session();
        Task::none()
    }

    fn switch_session(&mut self, target_id: u64) -> Task<Message> {
        if target_id == self.current_session_id {
            return Task::none();
        }
        let Some(idx) = self
            .inactive_sessions
            .iter()
            .position(|s| s.id == target_id)
        else {
            return Task::none();
        };
        self.abort_active_chat_stream(true);
        self.snapshot_current_to_inactive();
        let target = self.inactive_sessions.remove(idx);
        self.current_session_id = target.id;
        self.current_session_title = target.title;
        self.conversation = target.conversation;
        self.next_block_id = target.next_block_id;
        self.blocks = target.blocks.into_iter().map(persisted_to_block).collect();
        self.current_scroll_y = target.scroll_y;
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.show_write_confirm = false;
        self.streaming_block_id = None;
        self.tool_round = 0;
        self.input.clear();
        self.ui.pending_delete_session = None;
        self.status = "세션 전환됨".into();
        self.save_session();
        iced::widget::operation::scroll_to(
            self.stream_id.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: target.scroll_y,
            },
        )
    }

    fn fetch_account_cmd(&mut self) -> Task<Message> {
        let key = match keystore::read_api_key() {
            Ok(k) => k,
            Err(_) => return Task::none(),
        };
        Task::perform(openrouter::get_account_info(key), Message::AccountLoaded)
    }

    fn on_stream_scrolled(
        &mut self,
        viewport: &iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        let rel = viewport.relative_offset();
        self.follow_bottom = rel.y > 0.95;
        self.current_scroll_y = viewport.absolute_offset().y;
        Task::none()
    }

    fn on_editor_action(
        &mut self,
        id: u64,
        action: iced::widget::text_editor::Action,
    ) -> Task<Message> {
        if action.is_edit() {
            return Task::none();
        }
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            if let BlockBody::Assistant(content) = &mut b.body {
                content.perform(action);
            }
        }
        Task::none()
    }

    fn toggle_block_view(&mut self, id: u64) -> Task<Message> {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            b.view_mode = match b.view_mode {
                ViewMode::Rendered => ViewMode::Raw,
                ViewMode::Raw => ViewMode::Rendered,
            };
        }
        Task::none()
    }

    fn on_link_clicked(&mut self, uri: &markdown::Uri) -> Task<Message> {
        let url = uri.to_string();
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("javascript:") {
            self.status = "차단된 링크 스킴입니다.".into();
            return Task::none();
        }
        match webbrowser::open(&url) {
            Ok(_) => {
                self.status = format!("브라우저에서 열기: {}", url);
            }
            Err(e) => {
                self.status = format!("링크 열기 실패: {}", e);
            }
        }
        Task::none()
    }

    fn pick_cwd(&self) -> Task<Message> {
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("작업 폴더 선택")
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::CwdPicked,
        )
    }

    fn pick_attachment(&self) -> Task<Message> {
        let cwd = self.cwd.clone();
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("첨부 파일 선택")
                    .set_directory(cwd)
                    .pick_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::AttachmentPicked,
        )
    }

    fn on_attachment_picked(&mut self, maybe_path: Option<std::path::PathBuf>) -> Task<Message> {
        let Some(path) = maybe_path else {
            return Task::none();
        };
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        path.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((path, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }

    fn stop_stream(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.compare_pending = false;
        self.status = "중지됨".into();
        self.maybe_update_title();
        self.save_session();
        Task::none()
    }

    fn copy_block(&self, id: u64) -> Task<Message> {
        if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
            return iced::clipboard::write(b.body.to_text());
        }
        Task::none()
    }

    fn edit_last_user(&mut self) -> Task<Message> {
        if self.streaming_block_id.is_some() {
            return Task::none();
        }
        let Some(idx) = last_user_block_idx(&self.blocks) else {
            return Task::none();
        };
        let user_text = match &self.blocks[idx].body {
            BlockBody::User(s) => s.clone(),
            _ => return Task::none(),
        };
        self.blocks.truncate(idx);
        truncate_after_last_user(&mut self.conversation);
        self.conversation.pop();
        self.tool_round = 0;
        self.pending_tool_calls.clear();
        self.input = user_text;
        self.status = "편집 모드 — 수정 후 Enter".into();
        Task::none()
    }

    fn on_compare_responses_loaded(
        &mut self,
        openrouter_block_id: u64,
        tabby_block_id: u64,
        openrouter_result: Result<String, String>,
        tabby_result: Result<String, String>,
    ) -> Task<Message> {
        if !self.compare_pending {
            return Task::none();
        }
        let openrouter_text = match openrouter_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[OpenRouter] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", openrouter::humanize_error(&e)),
        };
        let tabby_text = match tabby_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[Tabby] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", tabby::humanize_error(&e)),
        };
        self.fill_assistant_block(openrouter_block_id, openrouter_text.clone());
        self.fill_assistant_block(tabby_block_id, tabby_text.clone());
        self.conversation.push(ChatMessage::assistant(format!(
            "[OpenRouter]\n{}\n\n[Tabby]\n{}",
            openrouter_text, tabby_text
        )));
        self.compare_pending = false;
        self.status = "Compare 응답 완료".into();
        self.maybe_update_title();
        self.save_session();
        if self.follow_bottom {
            snap_to_end(self.stream_id.clone())
        } else {
            Task::none()
        }
    }

    fn on_file_dropped(&mut self, path: std::path::PathBuf) -> Task<Message> {
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        path.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((path, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }

    fn add_mcp_server(&mut self) -> Task<Message> {
        let name = self.mcp_input.name_input.trim().to_string();
        let command = self.mcp_input.command_input.trim().to_string();
        if name.is_empty() || command.is_empty() {
            self.status = "MCP 서버 이름과 명령을 모두 입력하세요.".into();
            return Task::none();
        }
        let server = mcp::McpServer {
            name: name.clone(),
            command,
        };
        self.mcp_servers.push(server.clone());
        self.mcp_input.name_input.clear();
        self.mcp_input.command_input.clear();
        if let Err(e) = mcp::save_servers(&self.mcp_servers) {
            self.status = format!("MCP 저장 실패: {e}");
            return Task::none();
        }
        self.status = format!("MCP 서버 추가됨: {name} — tool 목록 로드 중…");
        Task::perform(
            async move {
                mcp::list_tools(&server)
                    .await
                    .map(|tools| (name.clone(), tools))
                    .map_err(|e| format!("[{name}] {e}"))
            },
            |r| match r {
                Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                Err(msg) => Message::McpToolsFailed(msg),
            },
        )
    }

    fn pty_start(&mut self) -> Task<Message> {
        match pty::spawn_pty(&self.cwd) {
            Ok((session, stream)) => {
                self.pty_session = Some(session);
                self.pty_output.clear();
                self.status = "터미널 시작됨".into();
                Task::run(stream, |event| match event {
                    pty::PtyEvent::Line(l) => Message::PtyLine(l),
                    pty::PtyEvent::Exited => Message::PtyExited,
                })
            }
            Err(e) => {
                self.status = format!("터미널 시작 실패: {e}");
                Task::none()
            }
        }
    }

    fn on_pty_line(&mut self, line: String) -> Task<Message> {
        let clean = pty::strip_ansi(&line);
        if !clean.trim().is_empty() {
            self.push_pty_line(clean);
        }
        Task::none()
    }

    fn on_pty_exited(&mut self) -> Task<Message> {
        self.pty_session = None;
        self.push_pty_line("-- 셸 종료 --".into());
        self.status = "터미널 종료됨".into();
        Task::none()
    }

    fn start_hf_download(&mut self) -> Task<Message> {
        if self.hf_dl.is_some() {
            self.status = "이미 다운로드가 진행 중입니다.".into();
            return Task::none();
        }
        if let Some(h) = self.hf_abort_handle.take() {
            h.abort();
        }
        let repo = self.hf_repo_input.trim().to_string();
        if repo.is_empty() {
            self.status = "HF repo ID 비어있음".into();
            return Task::none();
        }
        let mut dir = self.model_dir_input.trim().to_string();
        if dir.is_empty() {
            dir = default_models_dir();
            self.status = format!("다운로드 경로 자동 설정: {}", dir);
        }
        let resolved_dir = resolve_user_path(&dir);
        dir = resolved_dir.display().to_string();
        self.model_dir_input = dir.clone();
        if let Err(e) = std::fs::create_dir_all(&resolved_dir) {
            self.status = format!("다운로드 경로 생성 실패 ({}): {}", dir, e);
            return Task::none();
        }
        let _ = keystore::write_model_dir(&dir);
        let token = if self.hf_token_input.trim().is_empty() {
            keystore::read_hf_token()
        } else {
            Some(self.hf_token_input.trim().to_string())
        };
        let download_folder_name = self
            .hf_folder_name
            .take()
            .unwrap_or_else(|| repo.replace('/', "--"));
        let revision = self.hf_revision.take();
        self.hf_dl = Some(HfDownload {
            folder_name: download_folder_name.clone(),
            total_files: 0,
            file_idx: 0,
            file_name: String::new(),
            file_bytes_done: 0,
            file_bytes_total: None,
        });
        self.status = format!("다운로드 시작: {}", repo);
        let (task, handle) = Task::run(
            hf::download_repo(
                repo,
                resolved_dir,
                token,
                revision,
                Some(download_folder_name),
            ),
            Message::HfDownloadEvent,
        )
        .abortable();
        self.hf_abort_handle = Some(handle);
        task
    }

    fn on_hf_download_event(&mut self, ev: hf::DownloadEvent) -> Task<Message> {
        if let Some(dl) = self.hf_dl.as_mut() {
            match &ev {
                hf::DownloadEvent::Started { total_files } => {
                    dl.total_files = *total_files;
                }
                hf::DownloadEvent::FileStart { idx, name, size } => {
                    dl.file_idx = *idx;
                    dl.file_name = name.clone();
                    dl.file_bytes_done = 0;
                    dl.file_bytes_total = *size;
                }
                hf::DownloadEvent::FileProgress {
                    idx,
                    bytes_done,
                    bytes_total,
                } => {
                    dl.file_idx = *idx;
                    dl.file_bytes_done = *bytes_done;
                    dl.file_bytes_total = *bytes_total;
                }
                hf::DownloadEvent::FileDone => {}
                hf::DownloadEvent::AllDone => {
                    let folder_name = dl.folder_name.clone();
                    let model_path = downloaded_model_path(&self.model_dir_input, &folder_name);
                    let Some(resolved_model_path) =
                        resolve_tabbyapi_model_dir_for_folder(&model_path, &folder_name)
                    else {
                        self.status = format!(
                            "다운로드 결과에서 TabbyAPI 모델 경로를 확정할 수 없습니다: {} (config.json+가중치 파일이 필요하며, 여러 하위 모델이면 폴더 이름에 bpw 힌트가 필요합니다.)",
                            model_path.display()
                        );
                        self.tabby_status = Some(Err(self.status.clone()));
                        self.hf_dl = None;
                        self.hf_abort_handle = None;
                        return Task::none();
                    };
                    self.inference_engine = InferenceEngine::TabbyApi;
                    self.inference_selected_model = resolved_model_path.display().to_string();
                    self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                    if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir())
                    {
                        self.inference_binary_path = launcher.display().to_string();
                        let _ = keystore::write_inference_binary(&self.inference_binary_path);
                    } else {
                        self.inference_binary_path.clear();
                        let _ = keystore::clear_inference_binary();
                    }
                    self.tabby_url_input =
                        format!("http://localhost:{}", self.inference_port_input);
                    let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                    if self.openai_compat_label.trim().is_empty() {
                        self.openai_compat_label = "TabbyAPI".into();
                        let _ = keystore::write_openai_compat_label("TabbyAPI");
                    }
                    self.status = format!(
                        "다운로드 완료: {} — Runtime에서 시작을 누른 뒤 연결 테스트",
                        folder_name
                    );
                    self.hf_dl = None;
                    self.hf_abort_handle = None;
                }
                hf::DownloadEvent::Error(e) => {
                    self.status = format!("다운로드 실패: {}", compose_hf_download_error(e));
                    self.hf_dl = None;
                    self.hf_abort_handle = None;
                }
            }
        }
        Task::none()
    }

    fn cancel_hf_download(&mut self) -> Task<Message> {
        if let Some(h) = self.hf_abort_handle.take() {
            h.abort();
        }
        self.hf_dl = None;
        self.status = "다운로드 취소됨".into();
        Task::none()
    }

    fn regenerate_last(&mut self) -> Task<Message> {
        if self.streaming_block_id.is_some() {
            return Task::none();
        }
        if !self.conversation.iter().any(|m| m.role == "user") {
            return Task::none();
        }
        truncate_after_last_user(&mut self.conversation);
        let Some(idx) = last_user_block_idx(&self.blocks) else {
            return Task::none();
        };
        self.blocks.truncate(idx + 1);
        self.tool_round = 0;
        self.pending_tool_calls.clear();

        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };
        let model = self.selected_model.clone().unwrap_or_default();
        let messages = self.conversation.clone();

        let ai_id = self.next_id();
        self.blocks.push(Block {
            id: ai_id,
            body: BlockBody::Assistant(text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: self.selected_model.clone(),
            apply_candidates: Vec::new(),
        });
        self.streaming_block_id = Some(ai_id);
        self.status = "응답 다시 생성 중…".into();
        self.follow_bottom = true;

        let (chat_task, handle) = Task::run(
            openrouter::chat_stream(
                base_url,
                api_key,
                model,
                messages,
                self.tool_definitions_for_selected_model(),
            ),
            Message::ChatChunk,
        )
        .abortable();
        self.abort_handle = Some(handle);
        Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
    }

    fn apply_change(&mut self, block_id: u64, idx: usize) -> Task<Message> {
        let snapshot = self
            .blocks
            .iter()
            .find(|b| b.id == block_id)
            .and_then(|b| b.apply_candidates.get(idx))
            .filter(|(_, applied)| !*applied)
            .map(|(c, _)| (c.path.clone(), c.content.clone()));
        let Some((path, content)) = snapshot else {
            return Task::none();
        };
        let args_json = serde_json::json!({
            "path": path,
            "content": content,
        })
        .to_string();
        let result = tools::dispatch("write_file", &args_json, &self.cwd);
        let success = !result.contains("[error]");
        if success {
            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id) {
                if let Some((_, applied)) = b.apply_candidates.get_mut(idx) {
                    *applied = true;
                }
            }
        }
        let summary = if success {
            format!("{} ({} bytes)", path, content.len())
        } else {
            format!("실패: {}", path)
        };
        self.push_tool_result_block("apply".into(), summary, success);
        self.status = if success {
            format!("적용됨: {}", path)
        } else {
            result
        };
        Task::none()
    }

    // ── MCP server helpers ────────────────────────────────────────

    fn remove_mcp_server(&mut self, idx: usize) -> Task<Message> {
        if idx < self.mcp_servers.len() {
            let removed = self.mcp_servers.remove(idx);
            self.mcp_tools.retain(|t| t.server_name != removed.name);
            let _ = mcp::save_servers(&self.mcp_servers);
            self.status = format!("MCP 서버 제거됨: {}", removed.name);
        }
        Task::none()
    }

    fn on_mcp_tools_loaded(
        &mut self,
        server_name: String,
        tools: Vec<mcp::McpTool>,
    ) -> Task<Message> {
        self.mcp_tools.retain(|t| t.server_name != server_name);
        let count = tools.len();
        self.mcp_tools.extend(tools);
        self.status = format!("MCP [{server_name}] tool {count}개 로드 완료");
        Task::none()
    }

    fn on_mcp_tools_failed(&mut self, msg: String) -> Task<Message> {
        self.status = format!("MCP tool 로드 실패: {msg}");
        Task::none()
    }

    fn on_mcp_tool_result(&mut self, tool_call_id: String, result: String) -> Task<Message> {
        self.conversation
            .push(ChatMessage::tool_result(&tool_call_id, result));
        self.tool_round += 1;
        self.kick_chat_stream()
    }

    // ── Inference lifecycle helpers ────────────────────────────────

    fn stop_inference(&mut self) -> Task<Message> {
        if let Some(pid) = self.inference_pid.take() {
            kill_pid(pid);
            self.status = format!("inference 서버 중지 (pid {})", pid);
            self.push_inference_log(format!("[stopped] pid {}", pid));
        }
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }

    fn on_inference_log_line(&mut self, line: String) -> Task<Message> {
        if line.starts_with("[pid:") {
            if let Some(pid) = line
                .strip_prefix("[pid:")
                .and_then(|r| r.split(']').next())
                .and_then(|s| s.trim().parse::<u32>().ok())
            {
                self.inference_pid = Some(pid);
            }
        }
        if let Some(detail) = line.strip_prefix("[spawn 실패] ") {
            self.status = detail.to_string();
            self.tabby_status = Some(Err(detail.to_string()));
        }
        self.push_inference_log(line);
        Task::none()
    }

    fn on_inference_exited(&mut self, code: i32) -> Task<Message> {
        let last_error = self
            .inference_log
            .iter()
            .rev()
            .find(|line| line.starts_with("[spawn 실패]") || line.starts_with("[err]"))
            .cloned();
        self.push_inference_log(format!("[exited] code {}", code));
        self.inference_pid = None;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        self.status = format!("inference 서버 종료 (exit {})", code);
        self.tabby_status = Some(Err("inference 서버 종료됨".into()));
        let status = if code == -1 {
            last_error
                .and_then(|line| line.strip_prefix("[spawn 실패] ").map(str::to_string))
                .unwrap_or_else(|| "inference 서버 시작 실패".into())
        } else if code == 0 {
            format!("inference 서버 종료 (exit {})", code)
        } else if let Some(line) = last_error {
            format!("inference 서버 종료 (exit {}) — {}", code, line)
        } else {
            format!("inference 서버 종료 (exit {})", code)
        };
        self.status = status.clone();
        self.tabby_status = Some(Err(status));
        self.model_options
            .retain(|o| o.provider != LlmProvider::OpenAICompat);
        self.refresh_model_combo();
        Task::none()
    }

    // ── Model dir / HF model helpers ──────────────────────────────

    fn on_model_dir_picked(&mut self, maybe_path: Option<std::path::PathBuf>) -> Task<Message> {
        if let Some(path) = maybe_path {
            let s = path.display().to_string();
            let _ = keystore::write_model_dir(&s);
            self.model_dir_input = s;
            self.sync_selected_local_model_for_model_dir();
            self.status = "모델 다운로드 경로 저장됨".into();
        }
        Task::none()
    }

    fn select_downloaded_model(&mut self, folder_name: String) -> Task<Message> {
        let model_path = downloaded_model_path(&self.model_dir_input, &folder_name);
        let Some(resolved_model_path) =
            resolve_tabbyapi_model_dir_for_folder(&model_path, &folder_name)
        else {
            let msg = format!(
                "TabbyAPI 모델 폴더를 확정할 수 없습니다: {} (config.json+가중치 파일이 필요하며, 여러 하위 모델이면 폴더 이름에 bpw 힌트가 포함되어야 합니다.)",
                model_path.display()
            );
            self.status = msg.clone();
            self.tabby_status = Some(Err(msg));
            return Task::none();
        };
        self.inference_engine = InferenceEngine::TabbyApi;
        self.inference_selected_model = resolved_model_path.display().to_string();
        self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
        if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir()) {
            self.inference_binary_path = launcher.display().to_string();
            let _ = keystore::write_inference_binary(&self.inference_binary_path);
        } else {
            self.inference_binary_path.clear();
            let _ = keystore::clear_inference_binary();
        }
        self.tabby_url_input = format!("http://localhost:{}", self.inference_port_input);
        let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
        if self.openai_compat_label.trim().is_empty() {
            self.openai_compat_label = "TabbyAPI".into();
            let _ = keystore::write_openai_compat_label("TabbyAPI");
        }
        self.ui.settings_tab = SettingsTab::Runtime;
        self.status = format!(
            "다운로드된 모델 선택됨: {} — Runtime에서 시작 후 연결 테스트",
            folder_name
        );
        Task::none()
    }

    // ── Usage / write confirm helpers ──────────────────────────────

    fn on_generation_loaded(
        &mut self,
        result: Result<openrouter::GenerationData, String>,
    ) -> Task<Message> {
        if let Ok(data) = result {
            let cost = data.total_cost.unwrap_or(0.0);
            self.last_response_cost = Some(cost);
            let model_id = data.model.clone().unwrap_or_default();
            if !model_id.is_empty() {
                let entry = self.usage.by_model.entry(model_id).or_default();
                entry.total_cost += cost;
                entry.prompt_tokens += data.native_tokens_prompt.unwrap_or(0);
                entry.completion_tokens += data.native_tokens_completion.unwrap_or(0);
                entry.call_count += 1;
            }
            let _ = session::save_usage(&self.usage);
            return Task::done(Message::FetchAccount);
        }
        Task::none()
    }

    fn discard_write_call(&mut self, idx: usize) -> Task<Message> {
        if idx >= self.pending_write_calls.len() {
            return Task::none();
        }
        let tc = self.pending_write_calls.remove(idx);
        self.push_tool_result_block(tc.name.clone(), "discarded".into(), false);
        self.conversation.push(ChatMessage::tool_result(
            &tc.id,
            "[denied] 사용자가 이 도구 호출을 제외했습니다.",
        ));
        self.ui.expanded_confirm_idx = match self.ui.expanded_confirm_idx {
            Some(e) if e == idx => None,
            Some(e) if e > idx => Some(e - 1),
            other => other,
        };
        if self.pending_write_calls.is_empty() {
            return self.continue_after_writes(true);
        }
        Task::none()
    }

    // ── Inference engine config helpers ───────────────────────────

    fn select_inference_engine(&mut self, engine: InferenceEngine) -> Task<Message> {
        let prev = self.inference_engine;
        self.inference_engine = engine;
        self.inference_port_input = engine.default_port().to_string();
        if !prev.shares_model_namespace(engine) {
            self.inference_selected_model.clear();
        }
        match engine {
            InferenceEngine::TabbyApi => {
                self.tabby_url_input = format!("http://localhost:{}", engine.default_port());
                let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                self.openai_compat_label = "TabbyAPI".into();
                let _ = keystore::write_openai_compat_label("TabbyAPI");
            }
            InferenceEngine::TabbyMl => {
                self.tabby_url_input = format!("http://localhost:{}", engine.default_port());
                let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                self.openai_compat_label = "TabbyML".into();
                let _ = keystore::write_openai_compat_label("TabbyML");
            }
            _ => {}
        }
        Task::none()
    }

    fn set_inference_port(&mut self, value: String) -> Task<Message> {
        let prev_port = self.inference_port_input.trim().parse::<u16>().ok();
        self.inference_port_input = value.clone();
        if let Ok(new_port) = value.trim().parse::<u16>() {
            if matches!(
                self.inference_engine,
                InferenceEngine::XLlm
                    | InferenceEngine::VLlm
                    | InferenceEngine::LlamaServer
                    | InferenceEngine::TabbyMl
                    | InferenceEngine::TabbyApi
            ) {
                let current_url = self.tabby_url_input.trim();
                let current_url_port = extract_loopback_port(current_url);
                let should_sync = current_url.is_empty()
                    || (is_loopback_url(current_url)
                        && (current_url_port == prev_port || current_url_port.is_none()));
                if should_sync {
                    self.tabby_url_input = format!("http://localhost:{}", new_port);
                    let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                }
            }
        }
        Task::none()
    }

    fn on_inference_binary_picked(
        &mut self,
        maybe_path: Option<std::path::PathBuf>,
    ) -> Task<Message> {
        if let Some(path) = maybe_path {
            let s = path.display().to_string();
            if matches!(self.inference_engine, InferenceEngine::TabbyApi) {
                if let Err(msg) = validate_tabbyapi_launcher_path(&s) {
                    self.status = msg.clone();
                    self.tabby_status = Some(Err(msg));
                    return Task::none();
                }
            }
            let _ = keystore::write_inference_binary(&s);
            self.inference_binary_path = s;
            self.status = if matches!(self.inference_engine, InferenceEngine::TabbyApi) {
                "TabbyAPI script 경로 저장됨".into()
            } else {
                "바이너리 경로 저장됨".into()
            };
        }
        Task::none()
    }

    fn pick_inference_binary(&self) -> Task<Message> {
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("inference 엔진 바이너리/스크립트 선택")
                    .pick_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::InferenceBinaryPicked,
        )
    }

    fn install_tabbyapi_runtime_cmd(&mut self) -> Task<Message> {
        self.busy = true;
        let runtime_dir = default_tabbyapi_runtime_dir();
        self.status = format!("TabbyAPI 런타임 설치 중: {}", runtime_dir.display());
        Task::perform(
            install_tabbyapi_runtime(runtime_dir),
            Message::TabbyApiRuntimeInstalled,
        )
    }

    fn on_tabbyapi_runtime_installed(
        &mut self,
        result: Result<std::path::PathBuf, String>,
    ) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(launcher) => {
                let s = launcher.display().to_string();
                self.inference_engine = InferenceEngine::TabbyApi;
                self.inference_binary_path = s.clone();
                self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                self.tabby_url_input = format!("http://localhost:{}", TABBY_API_DEFAULT_PORT);
                self.openai_compat_label = "TabbyAPI".into();
                let _ = keystore::write_inference_binary(&s);
                let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                let _ = keystore::write_openai_compat_label("TabbyAPI");
                self.status = format!(
                    "TabbyAPI 런타임 설치/감지 완료: {} — 모델 선택 후 시작하세요.",
                    launcher.display()
                );
                self.ui.settings_tab = SettingsTab::Runtime;
            }
            Err(e) => {
                self.status = format!(
                    "TabbyAPI 런타임 설치 실패: {}. Git/Python 설치와 네트워크를 확인해 주세요.",
                    e
                );
                self.tabby_status = Some(Err(self.status.clone()));
            }
        }
        Task::none()
    }

    // ── Tabby model fetch helpers ─────────────────────────────────

    fn fetch_tabby_models(&mut self) -> Task<Message> {
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        let url = self.tabby_url_input.clone();
        if url.trim().is_empty() {
            self.tabby_status = Some(Err("URL 비어있음".into()));
            return Task::none();
        }
        let token = if self.tabby_token_input.trim().is_empty() {
            None
        } else {
            Some(self.tabby_token_input.clone())
        };
        self.status = "Tabby 모델 가져오는 중…".into();
        Task::perform(tabby::list_models(url, token), Message::TabbyModelsLoaded)
    }

    fn retry_fetch_tabby_models(&mut self, generation: u64) -> Task<Message> {
        if generation != self.tabby_retry_generation {
            return Task::none();
        }
        let url = self.tabby_url_input.clone();
        if url.trim().is_empty() {
            self.tabby_status = Some(Err("URL 비어있음".into()));
            return Task::none();
        }
        let token = if self.tabby_token_input.trim().is_empty() {
            None
        } else {
            Some(self.tabby_token_input.clone())
        };
        self.status = "Tabby 모델 재시도 중…".into();
        Task::perform(tabby::list_models(url, token), Message::TabbyModelsLoaded)
    }

    fn on_tabby_models_loaded(&mut self, result: Result<Vec<String>, String>) -> Task<Message> {
        self.model_options
            .retain(|o| o.provider != LlmProvider::OpenAICompat);
        match result {
            Ok(ids) => {
                self.tabby_connect_retry_left = 0;
                let label = if ids.is_empty() {
                    "ok (모델 없음)".to_string()
                } else {
                    format!("{}개", ids.len())
                };
                self.status = format!("Tabby 연결됨 — {}", label);
                self.tabby_status = Some(Ok(label));
                let provider_label = self.openai_compat_label.clone();
                let mut first_tabby_id: Option<String> = None;
                for id in ids {
                    if first_tabby_id.is_none() {
                        first_tabby_id = Some(id.clone());
                    }
                    let ko_friendly = is_korean_friendly(&id);
                    let favorite = self.model_filter.favorites.contains(&id);
                    self.model_options.push(ModelOption {
                        id,
                        provider: LlmProvider::OpenAICompat,
                        provider_label: provider_label.clone(),
                        ko_friendly,
                        favorite,
                        context_length: None,
                        prompt_per_million: Some(0.0),
                        completion_per_million: Some(0.0),
                    });
                }
                if let Some(id) = first_tabby_id {
                    let selected_is_tabby = self
                        .selected_model
                        .as_deref()
                        .map(|selected| {
                            self.model_options.iter().any(|o| {
                                o.provider == LlmProvider::OpenAICompat && o.id == selected
                            })
                        })
                        .unwrap_or(false);
                    if !selected_is_tabby {
                        self.selected_model = Some(id.clone());
                        self.selected_model_provider = Some(LlmProvider::OpenAICompat);
                        let _ = keystore::write_selected_model(&id);
                    }
                }
            }
            Err(e) => {
                let actionable = self.compose_tabby_connection_error(&e);
                let should_retry = self.inference_pid.is_some()
                    && self.tabby_connect_retry_left > 0
                    && tabby_connection_error_looks_unreachable(&e, &tabby::humanize_error(&e));
                if should_retry {
                    self.tabby_connect_retry_left -= 1;
                    let remain = self.tabby_connect_retry_left;
                    self.status = format!(
                        "Tabby 연결 재시도 예정: {} ({}초 뒤 자동 재시도, 남은 {}회)",
                        actionable, TABBY_CONNECT_RETRY_DELAY_SECS, remain
                    );
                    self.tabby_status = Some(Err(actionable));
                    return Task::perform(
                        async {
                            tokio::time::sleep(std::time::Duration::from_secs(
                                TABBY_CONNECT_RETRY_DELAY_SECS,
                            ))
                            .await;
                        },
                        {
                            let generation = self.tabby_retry_generation;
                            move |_| Message::FetchTabbyModelsRetry(generation)
                        },
                    );
                }
                self.tabby_connect_retry_left = 0;
                self.status = format!("Tabby 연결 실패: {}", actionable);
                self.tabby_status = Some(Err(actionable));
            }
        }
        self.refresh_model_combo();
        Task::none()
    }

    /// 현재 활성 필터/정렬을 적용해 model_options을 좁힌 결과.
    fn filtered_model_options(&self) -> Vec<ModelOption> {
        let mut opts: Vec<ModelOption> = self
            .model_options
            .iter()
            .filter(|opt| {
                if self.model_filter.filter_favorites_only
                    && !self.model_filter.favorites.contains(&opt.id)
                {
                    return false;
                }
                let cats = categorize_model(&opt.id);
                (self.model_filter.filter_coding && cats.contains(&ModelCategory::Coding))
                    || (self.model_filter.filter_reasoning
                        && cats.contains(&ModelCategory::Reasoning))
                    || (self.model_filter.filter_general && cats.contains(&ModelCategory::General))
            })
            .cloned()
            .collect();

        // 정렬: prompt+completion 합 기준
        let total_price = |o: &ModelOption| -> f64 {
            o.prompt_per_million.unwrap_or(0.0) + o.completion_per_million.unwrap_or(0.0)
        };
        match self.model_filter.sort_mode {
            SortMode::Default => {}
            SortMode::PriceAsc => opts.sort_by(|a, b| {
                total_price(a)
                    .partial_cmp(&total_price(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortMode::PriceDesc => opts.sort_by(|a, b| {
                total_price(b)
                    .partial_cmp(&total_price(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
        opts
    }

    fn sync_selected_model_provider(&mut self) {
        let Some(selected_id) = self.selected_model.as_deref() else {
            self.selected_model_provider = None;
            return;
        };

        if let Some(provider) = self.selected_model_provider {
            if self
                .model_options
                .iter()
                .any(|o| o.id == selected_id && o.provider == provider)
            {
                return;
            }
        }

        let mut matches = self
            .model_options
            .iter()
            .filter(|o| o.id == selected_id)
            .map(|o| o.provider);

        let Some(first) = matches.next() else {
            self.selected_model_provider = None;
            return;
        };

        let mut has_openrouter = first == LlmProvider::OpenRouter;
        let mut has_openai_compat = first == LlmProvider::OpenAICompat;
        for provider in matches {
            match provider {
                LlmProvider::OpenRouter => has_openrouter = true,
                LlmProvider::OpenAICompat => has_openai_compat = true,
            }
        }

        self.selected_model_provider = if has_openrouter && has_openai_compat {
            if self.tabby_url_input.trim().is_empty() {
                Some(LlmProvider::OpenRouter)
            } else {
                Some(LlmProvider::OpenAICompat)
            }
        } else if has_openrouter {
            Some(LlmProvider::OpenRouter)
        } else if has_openai_compat {
            Some(LlmProvider::OpenAICompat)
        } else {
            None
        };
    }

    /// 필터/즐겨찾기 변경 시 combo_box::State 재구성.
    fn refresh_model_combo(&mut self) {
        self.sync_selected_model_provider();
        // favorite 필드를 현재 favorites HashSet과 동기화 (Display에 ★ 반영)
        for opt in &mut self.model_options {
            opt.favorite = self.model_filter.favorites.contains(&opt.id);
        }
        self.model_combo_state = combo_box::State::new(self.filtered_model_options());
    }

    /// 현재 활성 세션 + 비활성 세션 모두를 디스크에 저장.
    fn save_session(&self) {
        let current_blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .filter_map(|b| match &b.body {
                BlockBody::User(_) | BlockBody::Assistant(_) => Some(session::PersistedBlock {
                    id: b.id,
                    role: if matches!(&b.body, BlockBody::User(_)) {
                        "user".into()
                    } else {
                        "assistant".into()
                    },
                    content: b.body.to_text(),
                    model: b.model.clone().unwrap_or_default(),
                }),
                BlockBody::ToolResult { .. } => None, // 휘발성 — 저장 안 함
            })
            .collect();

        let mut sessions: Vec<session::PersistedSessionData> = self
            .inactive_sessions
            .iter()
            .map(|s| session::PersistedSessionData {
                id: s.id,
                title: s.title.clone(),
                conversation: s.conversation.clone(),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();
        sessions.push(session::PersistedSessionData {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: self.conversation.clone(),
            blocks: current_blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        });

        let active_idx = sessions
            .iter()
            .position(|s| s.id == self.current_session_id)
            .unwrap_or(sessions.len() - 1);

        let p = session::PersistedAllSessions {
            sessions,
            active_idx,
        };
        let _ = session::save_all(&p);
    }

    /// 현재 활성 세션 제목 자동 갱신 (첫 사용자 메시지 일부).
    fn maybe_update_title(&mut self) {
        if self.current_session_title.is_empty()
            || self.current_session_title.starts_with("새 채팅")
        {
            if let Some(first_user) = self
                .conversation
                .iter()
                .find(|m| m.role == "user")
                .and_then(|m| m.content.as_ref())
            {
                let snippet: String = first_user.chars().take(30).collect();
                self.current_session_title = snippet;
            }
        }
    }

    /// 현재 활성 세션을 inactive_sessions로 이동 (push 또는 update).
    fn snapshot_current_to_inactive(&mut self) {
        if self.conversation.is_empty() && self.blocks.is_empty() {
            return; // 빈 세션은 보관 X
        }
        let blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .filter_map(|b| match &b.body {
                BlockBody::User(_) | BlockBody::Assistant(_) => Some(session::PersistedBlock {
                    id: b.id,
                    role: if matches!(&b.body, BlockBody::User(_)) {
                        "user".into()
                    } else {
                        "assistant".into()
                    },
                    content: b.body.to_text(),
                    model: b.model.clone().unwrap_or_default(),
                }),
                BlockBody::ToolResult { .. } => None,
            })
            .collect();
        let snap = InactiveSession {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: self.conversation.clone(),
            blocks: blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        };
        if let Some(idx) = self.inactive_sessions.iter().position(|s| s.id == snap.id) {
            self.inactive_sessions[idx] = snap;
        } else {
            self.inactive_sessions.push(snap);
        }
    }

    fn allocate_session_id(&mut self) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }

    /// conversation 첫 위치에 cwd를 알려주는 system 메시지를 보장 (없으면 추가, 있으면 갱신).
    fn close_mention(&mut self) {
        self.show_mention = false;
        self.mention_query.clear();
        self.mention_selected = 0;
    }

    fn normalized_attachment_path(&self, path: &std::path::Path) -> std::path::PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }

    fn is_already_attached(&self, path: &std::path::Path) -> bool {
        let needle = self.normalized_attachment_path(path);
        self.attached_files
            .iter()
            .any(|(p, _)| self.normalized_attachment_path(p) == needle)
    }

    fn total_attached_bytes(&self) -> u64 {
        self.attached_files
            .iter()
            .map(|(_, content)| content.len() as u64)
            .sum()
    }

    fn ensure_system_message(&mut self) {
        let mode_block = match self.agent_mode {
            AgentMode::Plan => {
                "현재 모드: Plan (분석/계획 전용)\n\
                Plan 모드에서는 read_file/glob/grep으로 코드를 조사하고 변경 계획만 \
                제시하세요. 실제 파일 변경이나 명령 실행은 Build 모드에서만 가능하므로, \
                계획에 '필요한 변경'을 명확히 적고 사용자가 Build로 전환하기를 기다리세요.\n\n"
            }
            AgentMode::Build => {
                "현재 모드: Build (실행 가능)\n\
                Build 모드에서는 write_file/run_command를 사용해 실제 변경을 적용할 수 \
                있습니다. 단, 두 도구 모두 사용자 승인을 거치므로 부담 없이 호출하세요.\n\n"
            }
        };
        let prompt = format!(
            "당신은 CodeWarp의 코딩 어시스턴트입니다.\n\n\
            작업 디렉토리: '{}'\n\n\
            {}\
            사용 가능한 도구 (적극적으로 호출하세요):\n\
            - read_file(path): 파일 내용 읽기 (즉시 실행)\n\
            - write_file(path, content): 파일 작성/덮어쓰기 (Build 모드 + 사용자 승인)\n\
            - run_command(command): 셸 명령 실행 (Build 모드 + 사용자 승인)\n\
            - glob(pattern): 패턴 매칭 파일 리스트 (예: '**/*.rs', 'examples/**/*')\n\
            - grep(pattern): 정규식으로 모든 파일 검색\n\n\
            규칙:\n\
            1. 파일 시스템을 살펴봐야 할 때는 '확인하겠습니다' 같은 말 없이 즉시 도구를 호출하세요.\n\
            2. 새 파일을 만들기 전에 glob으로 기존 구조를 먼저 확인하세요.\n\
            3. 모든 path 인자는 작업 디렉토리 기준 상대 경로 (절대 경로 거부).\n\
            4. 도구 결과를 받은 뒤 그것을 근거로 한국어로 답하세요.\n\
            5. **마크다운 형식 제약** (한국어 폰트 한계): italic(*text* 또는 _text_)은 \
            사용하지 마세요. 강조는 오직 **굵게**만 사용. 별표 한 개로 감싸지 말고, \
            정말 강조가 필요하면 두 개로 감싸세요.\n\
            6. **Apply 가능한 코드 블록**: 사용자가 그대로 파일에 적용할 수 있도록, \
            새 파일/덮어쓸 파일의 코드 블록은 첫 줄에 다음 주석을 포함하세요:\n\
            - Rust/JS/C 계열: `// path: 상대경로`\n\
            - Python/shell/yaml: `# path: 상대경로`\n\
            예) ```rust\\n// path: src/foo.rs\\nfn main() {{}}\\n```\n\
            그러면 코드 블록 옆에 'Apply' 버튼이 노출되어 사용자가 한 번에 적용할 수 있습니다. \
            단순 예시 코드(개념 설명용)에는 path 주석을 넣지 마세요.",
            self.cwd.display(),
            mode_block,
        );
        if let Some(first) = self.conversation.first_mut() {
            if first.role == "system" {
                first.content = Some(prompt);
                return;
            }
        }
        self.conversation.insert(
            0,
            ChatMessage {
                role: "system".into(),
                content: Some(prompt),
                ..Default::default()
            },
        );
    }

    // Abort active assistant stream and optionally keep partial assistant text.
    pub(crate) fn abort_active_chat_stream(&mut self, keep_partial_assistant: bool) {
        if let Some(h) = self.abort_handle.take() {
            h.abort();
        }
        self.compare_pending = false;
        if keep_partial_assistant {
            if let Some(ai_id) = self.streaming_block_id {
                if let Some(b) = self.blocks.iter().find(|b| b.id == ai_id) {
                    let txt = b.body.to_text();
                    if !txt.is_empty() {
                        self.conversation.push(ChatMessage::assistant(txt));
                    }
                }
            }
        }
        self.streaming_block_id = None;
        self.pending_tool_calls.clear();
        self.tool_round = 0;
    }

    /// pending_tool_calls를 conversation에 반영, 안전한 도구는 즉시 실행하고
    /// mutating 도구가 있으면 사용자 승인 모달을 띄움. 모두 처리되면 새 chat_stream 트리거.
    fn run_tool_round(&mut self, assistant_partial: String) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_tool_calls);

        let tool_calls_json = serde_json::Value::Array(
            calls
                .iter()
                .enumerate()
                .map(|(i, tc)| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "index": i,
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect(),
        );
        let mut assistant_msg = ChatMessage::assistant_tool_calls(tool_calls_json);
        if !assistant_partial.is_empty() {
            assistant_msg.content = Some(assistant_partial);
        }
        self.conversation.push(assistant_msg);

        let mcp_tool_names: std::collections::HashSet<String> =
            self.mcp_tools.iter().map(|t| t.name.clone()).collect();

        let (mcp_calls, local_calls): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|tc| mcp_tool_names.contains(&tc.name));

        if !mcp_calls.is_empty() {
            // 로컬 read-only는 MCP와 함께 즉시 처리, mutating은 승인 대기
            let (local_read, local_write): (Vec<_>, Vec<_>) = local_calls
                .into_iter()
                .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);
            for tc in &local_read {
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                self.conversation
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            if !local_write.is_empty() {
                self.pending_write_calls = local_write;
                self.show_write_confirm = true;
            }

            let servers = self.mcp_servers.clone();
            let mcp_tools = self.mcp_tools.clone();
            let mut tasks = Vec::new();
            for tc in mcp_calls {
                let server = mcp_tools
                    .iter()
                    .find(|t| t.name == tc.name)
                    .and_then(|t| servers.iter().find(|s| s.name == t.server_name))
                    .cloned();
                let tool_name = tc.name.clone();
                let call_id = tc.id.clone();
                let args: serde_json::Value =
                    serde_json::from_str(&tc.arguments).unwrap_or_default();
                tasks.push(Task::perform(
                    async move {
                        match server {
                            Some(s) => mcp::call_tool(&s, &tool_name, args)
                                .await
                                .unwrap_or_else(|e| format!("[MCP 오류] {e}")),
                            None => "[MCP 오류] 서버 찾을 수 없음".into(),
                        }
                    },
                    move |result| Message::McpToolResult(call_id, result),
                ));
            }
            self.status = "MCP tool 실행 중…".into();
            return Task::batch(tasks);
        }

        let (read_calls, write_calls): (Vec<_>, Vec<_>) = local_calls
            .into_iter()
            .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);

        let mut names: Vec<String> = Vec::new();
        for tc in &read_calls {
            names.push(tc.name.clone());
            let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
            self.conversation
                .push(ChatMessage::tool_result(&tc.id, result));
        }
        if !names.is_empty() {
            self.status = format!("도구 호출: {}", names.join(", "));
        }

        if !write_calls.is_empty() {
            self.pending_write_calls = write_calls;
            self.show_write_confirm = true;
            self.status = "파일 쓰기 승인 대기".into();
            return Task::none();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }

    /// inference 서버 로그를 ring buffer에 push (cap 20).
    fn push_inference_log(&mut self, line: String) {
        const CAP: usize = 20;
        self.inference_log.push_back(line);
        while self.inference_log.len() > CAP {
            self.inference_log.pop_front();
        }
    }

    fn push_pty_line(&mut self, line: String) {
        self.pty_output.push_back(line);
        if self.pty_output.len() > PTY_MAX_LINES {
            self.pty_output.pop_front();
        }
    }

    /// 도구 실행 결과 chip 블록을 stream에 push (휘발성 — 세션 저장 안 됨).
    fn push_tool_result_block(&mut self, name: String, summary: String, success: bool) {
        let id = self.next_id();
        self.blocks.push(Block {
            id,
            body: BlockBody::ToolResult {
                name,
                summary,
                success,
            },
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
    }

    /// 사용자 승인/거부 후 호출. true면 mutating 실행, false면 거부 결과를 conversation에 기록.
    fn continue_after_writes(&mut self, approved: bool) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_write_calls);
        self.show_write_confirm = false;

        if approved {
            let mut names: Vec<String> = Vec::new();
            for tc in &calls {
                names.push(tc.name.clone());
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                let (summary, success) = summarize_tool_result(&tc.name, &tc.arguments, &result);
                self.push_tool_result_block(tc.name.clone(), summary, success);
                self.conversation
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            self.status = format!("실행 완료: {}", names.join(", "));
        } else {
            for tc in &calls {
                self.push_tool_result_block(tc.name.clone(), "denied".into(), false);
                self.conversation.push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 파일 쓰기를 거부했습니다.",
                ));
            }
            self.status = "사용자가 파일 쓰기를 거부했습니다".into();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }

    fn resolve_provider(&self) -> Result<(String, Option<String>), String> {
        let id = self
            .selected_model
            .as_deref()
            .ok_or_else(|| "모델 미선택".to_string())?;
        let provider = self
            .selected_option()
            .map(|o| o.provider)
            .ok_or_else(|| format!("선택된 모델을 찾을 수 없습니다: {}", id))?;
        match provider {
            LlmProvider::OpenRouter => {
                let key = keystore::read_api_key()?;
                Ok((openrouter::BASE_URL.to_string(), Some(key)))
            }
            LlmProvider::OpenAICompat => {
                let base = keystore::read_tabby_base_url()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| "Tabby URL 미설정".to_string())?;
                let token = keystore::read_tabby_token().filter(|s| !s.trim().is_empty());
                Ok((tabby::chat_base(&base), token))
            }
        }
    }

    fn selected_provider(&self) -> Option<LlmProvider> {
        self.selected_option().map(|o| o.provider)
    }

    fn selected_model_supports_tools(&self) -> bool {
        matches!(self.selected_provider(), Some(LlmProvider::OpenRouter))
    }

    pub(crate) fn tool_definitions_for_selected_model(&self) -> Option<serde_json::Value> {
        if self.selected_model_supports_tools() {
            Some(tools::tool_definitions(self.agent_mode.allow_mutating()))
        } else {
            None
        }
    }

    fn selected_option(&self) -> Option<&ModelOption> {
        let id = self.selected_model.as_deref()?;
        if let Some(provider) = self.selected_model_provider {
            if let Some(opt) = self
                .model_options
                .iter()
                .find(|o| o.id == id && o.provider == provider)
            {
                return Some(opt);
            }
        }
        self.model_options.iter().find(|o| o.id == id)
    }

    fn selected_model_exists_in_options(&self) -> bool {
        self.selected_model
            .as_deref()
            .map(|id| self.model_options.iter().any(|o| o.id == id))
            .unwrap_or(false)
    }

    fn compare_routes(&self) -> Result<(ChatRoute, ChatRoute), String> {
        let selected = self.selected_option();
        let openrouter_model = selected
            .filter(|o| o.provider == LlmProvider::OpenRouter)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenRouter)
            })
            .ok_or_else(|| "Compare 모드: OpenRouter 모델이 없습니다. OpenRouter 키/모델 목록을 먼저 불러와 주세요.".to_string())?;
        let tabby_model = selected
            .filter(|o| o.provider == LlmProvider::OpenAICompat)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenAICompat)
            })
            .ok_or_else(|| "Compare 모드: Tabby 모델이 없습니다. Provider 연결 테스트로 Tabby 모델을 먼저 불러와 주세요.".to_string())?;

        let openrouter_key = keystore::read_api_key()?;
        let tabby_base = keystore::read_tabby_base_url()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| "Compare 모드: Tabby URL 미설정".to_string())?;
        let tabby_token = keystore::read_tabby_token().filter(|s| !s.trim().is_empty());

        Ok((
            ChatRoute {
                label: "OpenRouter".into(),
                base_url: openrouter::BASE_URL.to_string(),
                api_key: Some(openrouter_key),
                model: openrouter_model.id.clone(),
            },
            ChatRoute {
                label: if tabby_model.provider_label.trim().is_empty() {
                    "Local".into()
                } else {
                    tabby_model.provider_label.trim().to_string()
                },
                base_url: tabby::chat_base(&tabby_base),
                api_key: tabby_token,
                model: tabby_model.id.clone(),
            },
        ))
    }

    fn fill_assistant_block(&mut self, block_id: u64, text: String) {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            if let BlockBody::Assistant(content) = &mut b.body {
                *content = text_editor::Content::with_text(&text);
                b.md_items = markdown::parse(&text).collect();
            }
        }
    }

    fn append_assistant_block_text(&mut self, block_id: u64, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            if let BlockBody::Assistant(content) = &mut b.body {
                let mut raw = content.text();
                raw.push_str(text);
                *content = text_editor::Content::with_text(&raw);
                b.md_items = markdown::parse(&raw).collect();
            }
        }
    }

    /// 누적된 conversation을 가지고 다음 chat_stream을 시작.
    fn kick_chat_stream(&mut self) -> Task<Message> {
        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                self.streaming_block_id = None;
                return Task::none();
            }
        };
        let model = self.selected_model.clone().unwrap_or_default();
        let messages = self.conversation.clone();
        // 기본 tool + MCP tool 합산
        let mut tool_defs = self.tool_definitions_for_selected_model();
        if !self.mcp_tools.is_empty() {
            if let Some(arr) = tool_defs.as_mut().and_then(|v| v.as_array_mut()) {
                for t in &self.mcp_tools {
                    arr.push(t.to_openai_tool());
                }
            }
        }
        let (task, handle) = Task::run(
            openrouter::chat_stream(base_url, api_key, model, messages, tool_defs),
            Message::ChatChunk,
        )
        .abortable();
        self.abort_handle = Some(handle);
        task
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(on_event)
    }

    pub(crate) fn filtered_palette_commands(&self) -> Vec<&'static PaletteCommand> {
        let q = self.ui.command_palette_input.to_lowercase();
        if q.is_empty() {
            PALETTE_COMMANDS.iter().collect()
        } else {
            PALETTE_COMMANDS
                .iter()
                .filter(|c| {
                    c.label.to_lowercase().contains(&q) || c.hint.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compose_hf_download_error, contains_ascii_case_insensitive, default_models_dir,
        extract_hf_error_hint, find_hint_boundary, merge_hint, runtime_command_exists,
        starts_with_ascii_case_insensitive,
    };

    #[test]
    fn extract_hf_error_hint_parses_requested_revision_tail() {
        let raw =
            "HF 404: revision not found (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("requested revision: '4bpw'; available branches: main, 4.0bpw")
        );
    }

    #[test]
    fn extract_hf_error_hint_parses_fallback_retry() {
        let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "fallback retry from").as_deref(),
            Some("fallback retry from '4bpw' to '4.0bpw'")
        );
    }

    #[test]
    fn compose_hf_download_error_appends_revision_hint() {
        let raw =
            "HF 404: revision not found (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        let msg = compose_hf_download_error(raw);
        assert!(msg.contains("requested revision: '4bpw'"));
        assert!(msg.contains("available branches: main, 4.0bpw"));
    }

    #[test]
    fn compose_hf_download_error_appends_fallback_and_revision_hints() {
        let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw'; available branches: main, 4.0bpw)";
        let msg = compose_hf_download_error(raw);
        assert!(msg.contains("fallback retry from '4bpw' to '4.0bpw'"));
        assert!(msg.contains("requested revision: '4bpw'"));
    }

    #[test]
    fn compose_hf_download_error_appends_fallback_lookup_failure_hint() {
        let raw = "HF 404: revision not found (fallback lookup failed: branch refs unavailable; requested revision: '4bpw')";
        let msg = compose_hf_download_error(raw);
        assert!(msg.contains("fallback lookup failed: branch refs unavailable"));
        assert!(msg.contains("requested revision: '4bpw'"));
    }

    #[test]
    fn extract_hf_error_hint_keeps_branch_names_with_parentheses() {
        let raw = "HF 404: revision not found (requested revision: '4bpw'; available branches: exl2(legacy), main)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("requested revision: '4bpw'; available branches: exl2(legacy), main")
        );
    }

    #[test]
    fn extract_hf_error_hint_parses_no_space_parenthesis_separator() {
        let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw')(requested revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "fallback retry from").as_deref(),
            Some("fallback retry from '4bpw' to '4.0bpw'")
        );
    }

    #[test]
    fn merge_hint_prefers_more_specific_hint() {
        let mut hints = vec!["requested revision: '4bpw'".to_string()];
        merge_hint(
            &mut hints,
            "fallback lookup failed: branch refs unavailable; requested revision: '4bpw'"
                .to_string(),
        );
        assert_eq!(hints.len(), 1);
        assert!(hints[0].starts_with("fallback lookup failed:"));
    }

    #[test]
    fn compose_hf_download_error_avoids_overlapping_hint_duplicates() {
        let raw = "HF 404: revision not found (fallback lookup failed: branch refs unavailable; requested revision: '4bpw')";
        let msg = compose_hf_download_error(raw);
        assert_eq!(msg.matches("requested revision: '4bpw'").count(), 1);
    }

    #[test]
    fn extract_hf_error_hint_is_case_insensitive_for_marker() {
        let raw = "HF 404: revision not found (Requested Revision: '4bpw'; available branches: main, 4.0bpw)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("Requested Revision: '4bpw'; available branches: main, 4.0bpw")
        );
    }

    #[test]
    fn contains_ascii_case_insensitive_matches_mixed_case() {
        assert!(contains_ascii_case_insensitive(
            "Requested Revision: '4bpw'",
            "requested revision:"
        ));
    }

    #[test]
    fn merge_hint_deduplicates_case_insensitive_overlap() {
        let mut hints = vec!["Requested Revision: '4bpw'".to_string()];
        merge_hint(&mut hints, "requested revision: '4bpw'".to_string());
        assert_eq!(hints.len(), 1);
    }

    #[test]
    fn starts_with_ascii_case_insensitive_matches_mixed_case_prefix() {
        assert!(starts_with_ascii_case_insensitive(
            "Requested Revision: '4bpw'",
            "requested revision:"
        ));
    }

    #[test]
    fn find_hint_boundary_detects_next_marker_separator() {
        let tail = "fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw')";
        assert_eq!(find_hint_boundary(tail), Some(38));
    }

    #[test]
    fn extract_hf_error_hint_keeps_internal_paren_separator_not_followed_by_marker() {
        let raw = "HF 404: revision not found (requested revision: '4bpw'; available branches: weird)(branch), main)";
        assert_eq!(
            extract_hf_error_hint(raw, "requested revision:").as_deref(),
            Some("requested revision: '4bpw'; available branches: weird)(branch), main")
        );
    }

    #[test]
    fn default_models_dir_returns_non_empty_path() {
        assert!(!default_models_dir().trim().is_empty());
    }

    #[test]
    fn runtime_command_exists_accepts_current_exe_absolute_path() {
        let current = std::env::current_exe().unwrap();
        assert!(runtime_command_exists(&current.to_string_lossy()));
    }

    #[test]
    fn runtime_command_exists_rejects_missing_absolute_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing = tmp.path().join("missing-runtime-binary.exe");
        assert!(!runtime_command_exists(&missing.to_string_lossy()));
    }
}
