// update_inference.rs — Inference-related App update methods (main.rs child module)
use super::*;
use iced::Task;

impl App {
    pub(crate) fn has_selected_local_model_available(&self) -> bool {
        let selected = self.inference_selected_model.trim();
        if selected.is_empty() {
            return false;
        }
        list_downloaded_models(std::path::Path::new(&self.model_dir_input))
            .iter()
            .any(|m| m == selected)
    }
    pub(crate) fn sync_selected_local_model_for_model_dir(&mut self) {
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
            let final_program =
                if override_path.is_empty() || is_tabbyapi_launcher_path(override_path) {
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
    pub(crate) fn set_inference_binary(&mut self, value: String) -> Task<Message> {
        self.inference_binary_path = value.clone();
        let _ = keystore::write_inference_binary(&value);
        Task::none()
    }
    pub(crate) fn set_model_dir(&mut self, value: String) -> Task<Message> {
        self.model_dir_input = value.clone();
        let _ = keystore::write_model_dir(&value);
        self.sync_selected_local_model_for_model_dir();
        Task::none()
    }
    pub(crate) fn stop_inference(&mut self) -> Task<Message> {
        if let Some(pid) = self.inference_pid.take() {
            kill_pid(pid);
            self.status = format!("inference 서버 중지 (pid {})", pid);
            self.push_inference_log(format!("[stopped] pid {}", pid));
        }
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }
    pub(crate) fn start_inference(&mut self) -> Task<Message> {
        if self.inference_pid.is_some() {
            self.status = "이미 실행 중".into();
            return Task::none();
        }
        let port: u16 = self
            .inference_port_input
            .trim()
            .parse()
            .unwrap_or_else(|_| self.inference_engine.default_port());
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
                        let Some(resolved_model_path) = resolve_tabbyapi_model_dir(model_path)
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
                        if let Err(e) =
                            write_tabbyapi_config_for_launcher(launcher, &resolved_model, port)
                        {
                            self.status = e.clone();
                            self.tabby_status = Some(Err(e));
                            return Task::none();
                        }
                        self.inference_selected_model = resolved_model;
                    }
                }
                if matches!(eng, InferenceEngine::TabbyMl) && std::path::Path::new(&model).exists()
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
                    InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer
                ) && !self.has_selected_local_model_available()
                {
                    self.status = "Selected local model was not found in the current model directory. Verify Models > download status and Runtime > model directory/path, then try Start again.".into();
                    return Task::none();
                }
                let abs_model =
                    if matches!(eng, InferenceEngine::TabbyMl | InferenceEngine::TabbyApi) {
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

        let program_hint = program.clone();
        let (final_program, args, work_dir) = self.resolve_runtime_spawn_command(program, args);
        if matches!(
            self.inference_engine,
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer
        ) && !runtime_command_exists(&final_program)
        {
            let override_path = self.inference_binary_path.trim();
            if !override_path.is_empty()
                && std::path::Path::new(override_path).is_dir()
                && std::path::Path::new(&final_program) == std::path::Path::new(override_path)
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
    pub(crate) fn on_inference_log_line(&mut self, line: String) -> Task<Message> {
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
    pub(crate) fn on_inference_exited(&mut self, code: i32) -> Task<Message> {
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
    pub(crate) fn on_model_dir_picked(
        &mut self,
        maybe_path: Option<std::path::PathBuf>,
    ) -> Task<Message> {
        if let Some(path) = maybe_path {
            let s = path.display().to_string();
            let _ = keystore::write_model_dir(&s);
            self.model_dir_input = s;
            self.sync_selected_local_model_for_model_dir();
            self.status = "모델 다운로드 경로 저장됨".into();
        }
        Task::none()
    }
    pub(crate) fn select_downloaded_model(&mut self, folder_name: String) -> Task<Message> {
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
    pub(crate) fn select_inference_engine(&mut self, engine: InferenceEngine) -> Task<Message> {
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
    pub(crate) fn set_inference_port(&mut self, value: String) -> Task<Message> {
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
    pub(crate) fn on_inference_binary_picked(
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
    pub(crate) fn pick_inference_binary(&self) -> Task<Message> {
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
    pub(crate) fn install_tabbyapi_runtime_cmd(&mut self) -> Task<Message> {
        self.busy = true;
        let runtime_dir = default_tabbyapi_runtime_dir();
        self.status = format!("TabbyAPI 런타임 설치 중: {}", runtime_dir.display());
        Task::perform(
            install_tabbyapi_runtime(runtime_dir),
            Message::TabbyApiRuntimeInstalled,
        )
    }
    pub(crate) fn on_tabbyapi_runtime_installed(
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
    pub(crate) fn fetch_tabby_models(&mut self) -> Task<Message> {
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
    pub(crate) fn retry_fetch_tabby_models(&mut self, generation: u64) -> Task<Message> {
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
    pub(crate) fn on_tabby_models_loaded(
        &mut self,
        result: Result<Vec<String>, String>,
    ) -> Task<Message> {
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
    pub(crate) fn push_inference_log(&mut self, line: String) {
        const CAP: usize = 20;
        self.inference_log.push_back(line);
        while self.inference_log.len() > CAP {
            self.inference_log.pop_front();
        }
    }
    pub(crate) fn filtered_model_options(&self) -> Vec<ModelOption> {
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
    pub(crate) fn sync_selected_model_provider(&mut self) {
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
    pub(crate) fn refresh_model_combo(&mut self) {
        self.sync_selected_model_provider();
        // favorite 필드를 현재 favorites HashSet과 동기화 (Display에 ★ 반영)
        for opt in &mut self.model_options {
            opt.favorite = self.model_filter.favorites.contains(&opt.id);
        }
        self.model_combo_state = combo_box::State::new(self.filtered_model_options());
    }
    pub(crate) fn resolve_provider(&self) -> Result<(String, Option<String>), String> {
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
                let base = if self.tabby_url_input.trim().is_empty() {
                    keystore::read_tabby_base_url()
                } else {
                    Some(self.tabby_url_input.clone())
                }
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "Tabby URL 미설정".to_string())?;
                let token = if self.tabby_token_input.trim().is_empty() {
                    keystore::read_tabby_token()
                } else {
                    Some(self.tabby_token_input.clone())
                }
                .filter(|s| !s.trim().is_empty());
                Ok((tabby::chat_base(&base), token))
            }
        }
    }
    pub(crate) fn selected_provider(&self) -> Option<LlmProvider> {
        self.selected_option().map(|o| o.provider)
    }
    pub(crate) fn selected_model_supports_tools(&self) -> bool {
        matches!(self.selected_provider(), Some(LlmProvider::OpenRouter))
    }
    pub(crate) fn tool_definitions_for_selected_model(&self) -> Option<serde_json::Value> {
        if self.selected_model_supports_tools() {
            Some(tools::tool_definitions(self.agent_mode.allow_mutating()))
        } else {
            None
        }
    }
    pub(crate) fn selected_option(&self) -> Option<&ModelOption> {
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
    pub(crate) fn selected_model_exists_in_options(&self) -> bool {
        self.selected_model
            .as_deref()
            .map(|id| self.model_options.iter().any(|o| o.id == id))
            .unwrap_or(false)
    }
}
