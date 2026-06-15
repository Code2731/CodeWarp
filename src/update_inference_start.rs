// update_inference_start.rs — Inference startup/lifecycle update methods (main.rs child module)
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
}
