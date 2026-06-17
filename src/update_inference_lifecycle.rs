// update_inference_lifecycle.rs — Inference lifecycle management (main.rs child module)
use super::*;
use iced::Task;

impl App {
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
}
