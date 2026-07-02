// update_settings_hf.rs — HF download management methods (main.rs child module)
use super::{
    App, HfDownload, InferenceEngine, Message, TABBY_API_DEFAULT_PORT, default_models_dir,
    default_tabbyapi_runtime_dir, downloaded_model_path, find_tabbyapi_launcher, hf, keystore,
    resolve_tabbyapi_model_dir_for_folder, resolve_user_path,
};
use iced::Task;

impl App {
    pub(crate) fn start_hf_download(&mut self) -> Task<Message> {
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
            self.status = format!("다운로드 경로 자동 설정: {dir}");
        }
        let resolved_dir = resolve_user_path(&dir);
        dir = resolved_dir.display().to_string();
        self.model_dir_input.clone_from(&dir);
        if let Err(e) = std::fs::create_dir_all(&resolved_dir) {
            self.status = format!("다운로드 경로 생성 실패 ({dir}): {e}");
            return Task::none();
        }
        self.try_persist(keystore::write_model_dir(&dir), "모델 디렉토리 저장");
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
        self.status = format!("다운로드 시작: {repo}");
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
    pub(crate) fn on_hf_download_event(&mut self, ev: &hf::DownloadEvent) -> Task<Message> {
        if let Some(dl) = self.hf_dl.as_mut() {
            match ev {
                hf::DownloadEvent::Started { total_files } => {
                    dl.total_files = *total_files;
                }
                hf::DownloadEvent::FileStart { idx, name, size } => {
                    dl.file_idx = *idx;
                    dl.file_name.clone_from(name);
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
                        self.try_persist(
                            keystore::write_inference_binary(&self.inference_binary_path),
                            "Inference 바이너리 저장",
                        );
                    } else {
                        self.inference_binary_path.clear();
                        self.try_persist(
                            keystore::clear_inference_binary(),
                            "Inference 바이너리 초기화",
                        );
                    }
                    self.tabby_url_input =
                        format!("http://localhost:{}", self.inference_port_input);
                    self.try_persist(
                        keystore::write_tabby_base_url(&self.tabby_url_input),
                        "Tabby URL 저장",
                    );
                    if self.openai_compat_label.trim().is_empty() {
                        self.openai_compat_label = "TabbyAPI".into();
                        self.try_persist(
                            keystore::write_openai_compat_label("TabbyAPI"),
                            "호환 레이블 저장",
                        );
                    }
                    self.status = format!(
                        "다운로드 완료: {folder_name} — Runtime에서 시작을 누른 뒤 연결 테스트"
                    );
                    self.hf_dl = None;
                    self.hf_abort_handle = None;
                }
                hf::DownloadEvent::Error(e) => {
                    self.status =
                        format!("다운로드 실패: {}", crate::hf::compose_hf_download_error(e));
                    self.hf_dl = None;
                    self.hf_abort_handle = None;
                }
            }
        }
        Task::none()
    }
    pub(crate) fn cancel_hf_download(&mut self) -> Task<Message> {
        if let Some(h) = self.hf_abort_handle.take() {
            h.abort();
        }
        self.hf_dl = None;
        self.status = "다운로드 취소됨".into();
        Task::none()
    }
}
