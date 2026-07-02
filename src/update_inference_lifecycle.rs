// update_inference_lifecycle.rs — Inference lifecycle management (main.rs child module)
use super::{
    App, InferenceEngine, Message, TABBY_API_DEFAULT_PORT, TABBY_CONNECT_RETRIES_AFTER_START,
    expected_binary_name, keystore, mcp, resolve_tabbyapi_model_dir, resolve_user_path,
    runtime_command_exists, spawn_inference_stream, validate_tabbyapi_launcher_path,
    write_tabbyapi_config_for_launcher,
};
use iced::Task;

impl App {
    pub(crate) fn start_inference(&mut self) -> Task<Message> {
        if self.inference_pid.is_some() {
            self.status = "이미 실행 중".into();
            return Task::none();
        }
        let port = self.parse_inference_port();

        if self.inference_engine == InferenceEngine::Ollama {
            return self.start_ollama(port);
        }

        let (program, args) = if self.inference_engine == InferenceEngine::Custom {
            match self.start_custom() {
                Some(v) => v,
                None => return Task::none(),
            }
        } else {
            match self.prepare_spawn_engine(port) {
                Some(v) => v,
                None => return Task::none(),
            }
        };

        self.set_inference_url_and_label(port);
        self.spawn_inference_process(program, args)
    }

    fn parse_inference_port(&self) -> u16 {
        self.inference_port_input
            .trim()
            .parse()
            .unwrap_or_else(|_| self.inference_engine.default_port())
    }

    fn start_custom(&mut self) -> Option<(String, Vec<String>)> {
        let cmd_str = self.inference_command_input.trim();
        if cmd_str.is_empty() {
            self.status = "시작 명령 비어있음".into();
            return None;
        }
        let parts = match mcp::parse_command(cmd_str) {
            Ok(v) => v,
            Err(e) => {
                self.status = format!("시작 명령 파싱 실패: {e}");
                return None;
            }
        };
        let mut iter = parts.into_iter();
        let p = iter.next()?;
        Some((p, iter.collect::<Vec<_>>()))
    }

    fn start_ollama(&mut self, port: u16) -> Task<Message> {
        self.tabby_url_input = format!("http://localhost:{port}");
        self.try_persist(
            keystore::write_tabby_base_url(&self.tabby_url_input),
            "Tabby URL 저장",
        );
        if self.openai_compat_label.trim().is_empty() {
            self.openai_compat_label = "Ollama".into();
            self.try_persist(
                keystore::write_openai_compat_label("Ollama"),
                "호환 레이블 저장",
            );
        }
        self.status = "Ollama daemon endpoint 등록 — 연결 테스트".into();
        Task::done(Message::FetchTabbyModels)
    }

    fn prepare_spawn_engine(&mut self, port: u16) -> Option<(String, Vec<String>)> {
        let eng = self.inference_engine;
        let model = self.inference_selected_model.trim().to_string();

        if model.is_empty() && !matches!(eng, InferenceEngine::TabbyApi) {
            self.status = "모델 선택 안 됨".into();
            return None;
        }

        if matches!(eng, InferenceEngine::TabbyApi) {
            self.setup_tabbyapi_launch(port, &model)?;
        }

        if matches!(eng, InferenceEngine::TabbyMl) && std::path::Path::new(&model).exists() {
            let msg = format!(
                "EXL2 로컬 폴더는 TabbyAPI용입니다. TabbyAPI(Start.bat 또는 python main.py)를 실행한 뒤 Provider URL을 http://localhost:{TABBY_API_DEFAULT_PORT} 로 연결 테스트해 주세요."
            );
            self.status.clone_from(&msg);
            self.tabby_status = Some(Err(msg));
            return None;
        }

        if matches!(
            eng,
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer
        ) && !self.has_selected_local_model_available()
        {
            self.status = "Selected local model was not found in the current model directory. Verify Models > download status and Runtime > model directory/path, then try Start again.".into();
            return None;
        }

        self.compose_engine_command(eng, &model, port)
    }

    fn setup_tabbyapi_launch(&mut self, port: u16, model: &str) -> Option<()> {
        let launcher = self.inference_binary_path.trim();
        if let Err(msg) = validate_tabbyapi_launcher_path(launcher) {
            self.status.clone_from(&msg);
            self.tabby_status = Some(Err(msg));
            return None;
        }
        if !model.is_empty() {
            let model_path = std::path::Path::new(&model);
            let Some(resolved_model_path) = resolve_tabbyapi_model_dir(model_path) else {
                let msg = format!(
                    "TabbyAPI 모델 폴더가 완전하지 않습니다: {} (config.json과 실제 모델 가중치 파일이 필요합니다.)",
                    model_path.display()
                );
                self.status.clone_from(&msg);
                self.tabby_status = Some(Err(msg));
                return None;
            };
            let resolved_model = resolved_model_path.display().to_string();
            if let Err(e) = write_tabbyapi_config_for_launcher(launcher, &resolved_model, port) {
                self.status.clone_from(&e);
                self.tabby_status = Some(Err(e));
                return None;
            }
            self.inference_selected_model = resolved_model;
        }
        Some(())
    }

    fn compose_engine_command(
        &self,
        eng: InferenceEngine,
        model: &str,
        port: u16,
    ) -> Option<(String, Vec<String>)> {
        let abs_model = if matches!(eng, InferenceEngine::TabbyMl | InferenceEngine::TabbyApi) {
            model.to_string()
        } else {
            resolve_user_path(&self.model_dir_input)
                .join(model)
                .display()
                .to_string()
        };
        let cmd = eng.compose_command(&abs_model, port)?;
        let mut iter = cmd.into_iter();
        Some((iter.next().unwrap_or_default(), iter.collect()))
    }

    fn set_inference_url_and_label(&mut self, port: u16) {
        self.tabby_url_input = format!("http://localhost:{port}");
        self.try_persist(
            keystore::write_tabby_base_url(&self.tabby_url_input),
            "Tabby URL 저장",
        );
        if self.openai_compat_label.trim().is_empty() {
            let label = self
                .inference_engine
                .label()
                .split_whitespace()
                .next()
                .unwrap_or("Local")
                .to_string();
            self.openai_compat_label.clone_from(&label);
            self.try_persist(
                keystore::write_openai_compat_label(&label),
                "호환 레이블 저장",
            );
        }
    }

    fn spawn_inference_process(&mut self, program: String, args: Vec<String>) -> Task<Message> {
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
                    "Runtime binary path '{override_path}' is a directory, but '{expected}' was not found inside it. Select the executable file directly or place '{expected}' in that folder."
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
        self.status = format!("실행 시작: {final_program} {}", args.join(" "));
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
                    move |()| Message::FetchTabbyModelsRetry(generation)
                },
            ),
        ])
    }
}
