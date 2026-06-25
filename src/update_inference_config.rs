// update_inference_config.rs — Inference configuration update methods (main.rs child module)
use super::{
    App, InferenceEngine, Message, SettingsTab, TABBY_API_DEFAULT_PORT,
    default_tabbyapi_runtime_dir, downloaded_model_path, extract_loopback_port,
    find_tabbyapi_launcher, install_tabbyapi_runtime, is_loopback_url, keystore,
    resolve_tabbyapi_model_dir_for_folder, validate_tabbyapi_launcher_path,
};
use iced::Task;

impl App {
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
    pub(crate) fn select_downloaded_model(&mut self, folder_name: &str) -> Task<Message> {
        let model_path = downloaded_model_path(&self.model_dir_input, folder_name);
        let Some(resolved_model_path) =
            resolve_tabbyapi_model_dir_for_folder(&model_path, folder_name)
        else {
            let msg = format!(
                "TabbyAPI 모델 폴더를 확정할 수 없습니다: {} (config.json+가중치 파일이 필요하며, 여러 하위 모델이면 폴더 이름에 bpw 힌트가 포함되어야 합니다.)",
                model_path.display()
            );
            self.status.clone_from(&msg);
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
        self.status =
            format!("다운로드된 모델 선택됨: {folder_name} — Runtime에서 시작 후 연결 테스트");
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
    pub(crate) fn set_inference_port(&mut self, value: &str) -> Task<Message> {
        let prev_port = self.inference_port_input.trim().parse::<u16>().ok();
        self.inference_port_input = value.to_string();
        if let Ok(new_port) = value.trim().parse::<u16>()
            && matches!(
                self.inference_engine,
                InferenceEngine::XLlm
                    | InferenceEngine::VLlm
                    | InferenceEngine::LlamaServer
                    | InferenceEngine::TabbyMl
                    | InferenceEngine::TabbyApi
            )
        {
            let current_url = self.tabby_url_input.trim();
            let current_url_port = extract_loopback_port(current_url);
            let should_sync = current_url.is_empty()
                || (is_loopback_url(current_url)
                    && (current_url_port == prev_port || current_url_port.is_none()));
            if should_sync {
                self.tabby_url_input = format!("http://localhost:{new_port}");
                let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
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
            if matches!(self.inference_engine, InferenceEngine::TabbyApi)
                && let Err(msg) = validate_tabbyapi_launcher_path(&s)
            {
                self.status.clone_from(&msg);
                self.tabby_status = Some(Err(msg));
                return Task::none();
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
    pub(crate) fn pick_inference_binary() -> Task<Message> {
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
                self.inference_binary_path.clone_from(&s);
                self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                self.tabby_url_input = format!("http://localhost:{TABBY_API_DEFAULT_PORT}");
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
                    "TabbyAPI 런타임 설치 실패: {e}. Git/Python 설치와 네트워크를 확인해 주세요."
                );
                self.tabby_status = Some(Err(self.status.clone()));
            }
        }
        Task::none()
    }
}
