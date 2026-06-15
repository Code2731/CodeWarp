// update_settings.rs — Settings/UI App update methods (main.rs child module)
use super::*;

use iced::Task;

impl App {
    pub(crate) fn save_hf_token(&mut self) -> Task<Message> {
        let t = self.hf_token_input.clone();
        Task::perform(
            async move { keystore::write_hf_token(&t) },
            Message::HfTokenSaved,
        )
    }
    pub(crate) fn on_hf_token_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => self.status = "HF 토큰 저장됨".into(),
            Err(e) => self.status = format!("HF 토큰 저장 실패: {}", e),
        }
        Task::none()
    }
    pub(crate) fn apply_model_preset(&mut self, idx: usize) -> Task<Message> {
        if let Some(p) = MODEL_PRESETS.get(idx) {
            self.hf_repo_input = p.repo_id.into();
            self.hf_revision = None;
            self.hf_folder_name = None;
        }
        Task::none()
    }
    pub(crate) fn prepare_exl2_preset_download(&mut self, idx: usize) -> Task<Message> {
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
    pub(crate) fn select_model(&mut self, opt: ModelOption) -> Task<Message> {
        let _ = keystore::write_selected_model(&opt.id);
        self.selected_model_provider = Some(opt.provider);
        self.selected_model = Some(opt.id);
        Task::none()
    }
    pub(crate) fn on_account_loaded(
        &mut self,
        result: Result<openrouter::AuthKeyData, String>,
    ) -> Task<Message> {
        if let Ok(data) = result {
            self.account = Some(data);
        }
        Task::none()
    }
    pub(crate) fn fetch_models_cmd(&mut self) -> Task<Message> {
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
    pub(crate) fn on_models_loaded(
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
    pub(crate) fn fetch_account_cmd(&mut self) -> Task<Message> {
        let key = match keystore::read_api_key() {
            Ok(k) => k,
            Err(_) => return Task::none(),
        };
        Task::perform(openrouter::get_account_info(key), Message::AccountLoaded)
    }
    pub(crate) fn pick_cwd(&self) -> Task<Message> {
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
    pub(crate) fn pick_attachment(&self) -> Task<Message> {
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
    pub(crate) fn on_attachment_picked(
        &mut self,
        maybe_path: Option<std::path::PathBuf>,
    ) -> Task<Message> {
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
    pub(crate) fn on_file_dropped(&mut self, path: std::path::PathBuf) -> Task<Message> {
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
    pub(crate) fn add_mcp_server(&mut self) -> Task<Message> {
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
    pub(crate) fn pty_start(&mut self) -> Task<Message> {
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
    pub(crate) fn on_pty_line(&mut self, line: String) -> Task<Message> {
        let clean = pty::strip_ansi(&line);
        if !clean.trim().is_empty() {
            self.push_pty_line(clean);
        }
        Task::none()
    }
    pub(crate) fn on_pty_exited(&mut self) -> Task<Message> {
        self.pty_session = None;
        self.push_pty_line("-- 셸 종료 --".into());
        self.status = "터미널 종료됨".into();
        Task::none()
    }
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
    pub(crate) fn on_hf_download_event(&mut self, ev: hf::DownloadEvent) -> Task<Message> {
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
    pub(crate) fn cancel_hf_download(&mut self) -> Task<Message> {
        if let Some(h) = self.hf_abort_handle.take() {
            h.abort();
        }
        self.hf_dl = None;
        self.status = "다운로드 취소됨".into();
        Task::none()
    }
    pub(crate) fn remove_mcp_server(&mut self, idx: usize) -> Task<Message> {
        if idx < self.mcp_servers.len() {
            let removed = self.mcp_servers.remove(idx);
            self.mcp_tools.retain(|t| t.server_name != removed.name);
            let _ = mcp::save_servers(&self.mcp_servers);
            self.status = format!("MCP 서버 제거됨: {}", removed.name);
        }
        Task::none()
    }
    pub(crate) fn on_mcp_tools_loaded(
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
    pub(crate) fn on_mcp_tools_failed(&mut self, msg: String) -> Task<Message> {
        self.status = format!("MCP tool 로드 실패: {msg}");
        Task::none()
    }
    pub(crate) fn close_mention(&mut self) {
        self.show_mention = false;
        self.mention_query.clear();
        self.mention_selected = 0;
    }
    pub(crate) fn normalized_attachment_path(&self, path: &std::path::Path) -> std::path::PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }
    pub(crate) fn is_already_attached(&self, path: &std::path::Path) -> bool {
        let needle = self.normalized_attachment_path(path);
        self.attached_files
            .iter()
            .any(|(p, _)| self.normalized_attachment_path(p) == needle)
    }
    pub(crate) fn total_attached_bytes(&self) -> u64 {
        self.attached_files
            .iter()
            .map(|(_, content)| content.len() as u64)
            .sum()
    }
    pub(crate) fn push_pty_line(&mut self, line: String) {
        self.pty_output.push_back(line);
        if self.pty_output.len() > PTY_MAX_LINES {
            self.pty_output.pop_front();
        }
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
    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn stale_tabby_retry_message_is_ignored() {
        let (mut app, _) = App::new();
        app.tabby_retry_generation = 10;
        app.status = "keep".into();

        let _ = app.update(Message::FetchTabbyModelsRetry(9));

        assert_eq!(app.status, "keep");
        assert_eq!(app.tabby_retry_generation, 10);
    }

    #[test]
    fn manual_tabby_fetch_bumps_retry_generation() {
        let (mut app, _) = App::new();
        app.tabby_retry_generation = 7;
        app.tabby_url_input = "http://localhost:5000".into();

        let _ = app.update(Message::FetchTabbyModels);

        assert_eq!(app.tabby_retry_generation, 8);
    }

    #[test]
    fn tabby_url_change_invalidates_retry_generation() {
        let (mut app, _) = App::new();
        app.tabby_retry_generation = 11;
        app.tabby_connect_retry_left = 2;

        let _ = app.update(Message::TabbyUrlChanged(
            "http://localhost:5001".to_string(),
        ));

        assert_eq!(app.tabby_retry_generation, 12);
        assert_eq!(app.tabby_connect_retry_left, 0);
    }

    #[test]
    fn tabby_token_change_invalidates_retry_generation() {
        let (mut app, _) = App::new();
        app.tabby_retry_generation = 5;
        app.tabby_connect_retry_left = 1;

        let _ = app.update(Message::TabbyTokenChanged("secret".to_string()));

        assert_eq!(app.tabby_retry_generation, 6);
        assert_eq!(app.tabby_connect_retry_left, 0);
    }

    #[test]
    fn clear_tabby_invalidates_retry_generation() {
        let (mut app, _) = App::new();
        app.tabby_retry_generation = 3;
        app.tabby_connect_retry_left = 2;
        app.tabby_url_input = "http://localhost:5000".into();
        app.tabby_token_input = "secret".into();

        let _ = app.update(Message::ClearTabby);

        assert_eq!(app.tabby_retry_generation, 4);
        assert_eq!(app.tabby_connect_retry_left, 0);
        assert!(app.tabby_url_input.is_empty());
        assert!(app.tabby_token_input.is_empty());
    }
}
