// update.rs — App update + 헬퍼 메서드 (main.rs child module)
use super::*;
use iced::widget::text_editor::{self, Action};
use iced::{Task, Subscription};

impl App {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenSettings => {
                self.show_settings = true;
                Task::none()
            }
            Message::CloseSettings => {
                self.show_settings = false;
                Task::none()
            }
            Message::KeyInputChanged(v) => {
                self.key_input = v;
                Task::none()
            }
            Message::SaveKey => {
                let key = self.key_input.clone();
                self.busy = true;
                self.status = "키 저장 중…".into();
                Task::perform(
                    async move { keystore::write_api_key(&key) },
                    Message::KeySaved,
                )
            }
            Message::KeySaved(r) => {
                self.busy = false;
                match r {
                    Ok(()) => {
                        self.has_key = true;
                        self.key_input.clear();
                        self.show_settings = false;
                        self.status = "키 저장됨".into();
                        Task::done(Message::FetchModels)
                    }
                    Err(e) => {
                        self.status = format!("저장 실패: {}", e);
                        Task::none()
                    }
                }
            }
            Message::ClearKey => {
                self.busy = true;
                Task::perform(async { keystore::delete_api_key() }, Message::KeyCleared)
            }
            Message::KeyCleared(r) => {
                self.busy = false;
                match r {
                    Ok(()) => {
                        self.has_key = false;
                        self.models.clear();
                        self.model_ids.clear();
                        self.selected_model = None;
                        let _ = keystore::clear_selected_model();
                        self.status = "키 삭제됨".into();
                    }
                    Err(e) => self.status = format!("삭제 실패: {}", e),
                }
                Task::none()
            }
            Message::TabbyUrlChanged(v) => {
                self.tabby_url_input = v;
                Task::none()
            }
            Message::TabbyTokenChanged(v) => {
                self.tabby_token_input = v;
                Task::none()
            }
            Message::ToggleTabbyTokenVisible => {
                self.show_tabby_token = !self.show_tabby_token;
                Task::none()
            }
            Message::InferenceCommandChanged(v) => {
                self.inference_command_input = v.clone();
                let _ = keystore::write_inference_command(&v);
                Task::none()
            }
            Message::SelectInferenceEngine(e) => {
                self.inference_engine = e;
                self.inference_port_input = e.default_port().to_string();
                Task::none()
            }
            Message::SelectInferenceModel(m) => {
                self.inference_selected_model = m;
                Task::none()
            }
            Message::InferencePortChanged(v) => {
                self.inference_port_input = v;
                Task::none()
            }
            Message::InferenceBinaryChanged(v) => {
                self.inference_binary_path = v.clone();
                let _ = keystore::write_inference_binary(&v);
                Task::none()
            }
            Message::PickInferenceBinary => {
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("inference 엔진 바이너리 선택 (xllm.exe / python.exe 등)")
                            .pick_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::InferenceBinaryPicked,
                )
            }
            Message::InferenceBinaryPicked(maybe) => {
                if let Some(path) = maybe {
                    let s = path.display().to_string();
                    let _ = keystore::write_inference_binary(&s);
                    self.inference_binary_path = s;
                    self.status = "바이너리 경로 저장됨".into();
                }
                Task::none()
            }
            Message::StartInference => {
                if self.inference_pid.is_some() {
                    self.status = "이미 실행 중".into();
                    return Task::none();
                }
                // 포트 parse
                let port: u16 = self.inference_port_input.trim().parse().unwrap_or_else(|_| {
                    self.inference_engine.default_port()
                });
                // 엔진별 명령 합성 + URL 자동 등록
                let (program, args) = match self.inference_engine {
                    InferenceEngine::Custom => {
                        let cmd_str = self.inference_command_input.trim();
                        if cmd_str.is_empty() {
                            self.status = "시작 명령 비어있음".into();
                            return Task::none();
                        }
                        let parts: Vec<String> =
                            cmd_str.split_whitespace().map(|s| s.to_string()).collect();
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
                        let model = self.inference_selected_model.trim();
                        if model.is_empty() {
                            self.status = "모델 선택 안 됨".into();
                            return Task::none();
                        }
                        // xLLM/vLLM/llama-server는 받은 폴더를 absolute path로
                        let abs_model = if matches!(eng, InferenceEngine::Tabby) {
                            // Tabby는 카탈로그 ID 그대로
                            model.to_string()
                        } else {
                            std::path::PathBuf::from(&self.model_dir_input)
                                .join(model)
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
                    let label = self.inference_engine.label().split_whitespace().next()
                        .unwrap_or("Local").to_string();
                    self.openai_compat_label = label.clone();
                    let _ = keystore::write_openai_compat_label(&label);
                }

                // 바이너리 경로가 명시되어 있으면 PATH 의존 안 하고 절대 경로 사용
                let final_program = if !self.inference_binary_path.trim().is_empty() {
                    self.inference_binary_path.trim().to_string()
                } else {
                    program
                };
                self.inference_log.clear();
                self.status = format!("실행 시작: {} {}", final_program, args.join(" "));
                Task::batch(vec![
                    Task::run(spawn_inference_stream(final_program, args), |ev| ev),
                    Task::perform(
                        async {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        },
                        |_| Message::FetchTabbyModels,
                    ),
                ])
            }
            Message::StopInference => {
                if let Some(pid) = self.inference_pid.take() {
                    kill_pid(pid);
                    self.status = format!("inference 서버 중지 (pid {})", pid);
                    self.push_inference_log(format!("[stopped] pid {}", pid));
                }
                Task::none()
            }
            Message::InferenceLogLine(line) => {
                if line.starts_with("[pid:") {
                    if let Some(pid) = line
                        .strip_prefix("[pid:")
                        .and_then(|r| r.split(']').next())
                        .and_then(|s| s.trim().parse::<u32>().ok())
                    {
                        self.inference_pid = Some(pid);
                    }
                }
                self.push_inference_log(line);
                Task::none()
            }
            Message::InferenceExited(code) => {
                self.push_inference_log(format!("[exited] code {}", code));
                self.inference_pid = None;
                self.status = format!("inference 서버 종료 (exit {})", code);
                // endpoint 끊김 표시
                self.tabby_status = Some(Err("inference 서버 종료됨".into()));
                self.model_options.retain(|o| o.provider != LlmProvider::OpenAICompat);
                self.refresh_model_combo();
                Task::none()
            }
            Message::OpenAICompatLabelChanged(v) => {
                self.openai_compat_label = v;
                let _ = keystore::write_openai_compat_label(&self.openai_compat_label);
                // 라벨이 바뀌면 기존 OpenAICompat 모델 옵션의 라벨도 갱신
                let new_label = self.openai_compat_label.clone();
                for opt in &mut self.model_options {
                    if opt.provider == LlmProvider::OpenAICompat {
                        opt.provider_label = new_label.clone();
                    }
                }
                self.refresh_model_combo();
                Task::none()
            }
            Message::SaveTabby => {
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
            Message::TabbySaved(r) => {
                self.busy = false;
                match r {
                    Ok(()) => {
                        self.status = "Tabby 설정 저장됨".into();
                        // 저장 직후 자동 모델 fetch (= 연결 테스트 겸용)
                        if !self.tabby_url_input.trim().is_empty() {
                            return Task::done(Message::FetchTabbyModels);
                        }
                    }
                    Err(e) => self.status = format!("Tabby 저장 실패: {}", e),
                }
                Task::none()
            }
            Message::ClearTabby => {
                let _ = keystore::clear_tabby_base_url();
                let _ = keystore::clear_tabby_token();
                self.tabby_url_input.clear();
                self.tabby_token_input.clear();
                self.tabby_status = None;
                self.status = "Tabby 설정 삭제됨".into();
                // 모델 리스트에서 Tabby 항목 제거
                self.model_options.retain(|o| o.provider != LlmProvider::OpenAICompat);
                self.refresh_model_combo();
                // 선택된 모델이 Tabby였다면 해제
                if let Some(sel) = self.selected_model.clone() {
                    if !self.model_options.iter().any(|o| o.id == sel) {
                        self.selected_model = self.model_options.first().map(|o| o.id.clone());
                        if let Some(id) = &self.selected_model {
                            let _ = keystore::write_selected_model(id);
                        }
                    }
                }
                Task::none()
            }
            Message::FetchTabbyModels => {
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
            Message::TabbyModelsLoaded(r) => {
                // 기존 Tabby 항목 제거 후 새로 채움 (성공/실패 모두 동일하게 비움)
                self.model_options.retain(|o| o.provider != LlmProvider::OpenAICompat);
                match r {
                    Ok(ids) => {
                        let label = if ids.is_empty() {
                            "ok (모델 없음)".to_string()
                        } else {
                            format!("{}개", ids.len())
                        };
                        self.status = format!("Tabby 연결됨 — {}", label);
                        self.tabby_status = Some(Ok(label));
                        let provider_label = self.openai_compat_label.clone();
                        for id in ids {
                            let ko_friendly = is_korean_friendly(&id);
                            let favorite = self.favorites.contains(&id);
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
                    }
                    Err(e) => {
                        let actionable = tabby::humanize_error(&e);
                        self.status = format!("Tabby 연결 실패: {}", actionable);
                        self.tabby_status = Some(Err(actionable));
                    }
                }
                self.refresh_model_combo();
                Task::none()
            }
            // ── HF 모델 매니저 ────────────────────────────────────
            Message::HfTokenChanged(v) => { self.hf_token_input = v; Task::none() }
            Message::ToggleHfTokenVisible => {
                self.show_hf_token = !self.show_hf_token;
                Task::none()
            }
            Message::SaveHfToken => {
                let t = self.hf_token_input.clone();
                Task::perform(
                    async move { keystore::write_hf_token(&t) },
                    Message::HfTokenSaved,
                )
            }
            Message::HfTokenSaved(r) => {
                match r {
                    Ok(()) => self.status = "HF 토큰 저장됨".into(),
                    Err(e) => self.status = format!("HF 토큰 저장 실패: {}", e),
                }
                Task::none()
            }
            Message::ModelDirChanged(v) => { self.model_dir_input = v; Task::none() }
            Message::PickModelDir => {
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::ModelDirPicked,
                )
            }
            Message::ModelDirPicked(maybe) => {
                if let Some(path) = maybe {
                    let s = path.display().to_string();
                    let _ = keystore::write_model_dir(&s);
                    self.model_dir_input = s;
                    self.status = "모델 다운로드 경로 저장됨".into();
                }
                Task::none()
            }
            Message::HfRepoChanged(v) => { self.hf_repo_input = v; Task::none() }
            Message::UsePreset(idx) => {
                if let Some(p) = MODEL_PRESETS.get(idx) {
                    self.hf_repo_input = p.repo_id.into();
                    self.hf_revision = None;
                    self.hf_folder_name = None;
                }
                Task::none()
            }
            Message::DownloadExl2Preset(idx) => {
                if let Some(p) = EXL2_PRESETS.get(idx) {
                    self.hf_repo_input = p.repo_id.into();
                    self.hf_revision = Some(p.revision.into());
                    self.hf_folder_name = Some(p.folder_name.into());
                    return Task::done(Message::StartHfDownload);
                }
                Task::none()
            }
            Message::StartHfDownload => {
                let repo = self.hf_repo_input.trim().to_string();
                if repo.is_empty() {
                    self.status = "HF repo ID 비어있음".into();
                    return Task::none();
                }
                let dir = self.model_dir_input.trim().to_string();
                if dir.is_empty() {
                    self.status = "다운로드 경로 미설정".into();
                    return Task::none();
                }
                // 경로 keystore에도 반영
                let _ = keystore::write_model_dir(&dir);
                let token = keystore::read_hf_token();
                self.hf_dl = Some(HfDownload {
                    repo_id: repo.clone(),
                    total_files: 0,
                    file_idx: 0,
                    file_name: String::new(),
                    file_bytes_done: 0,
                    file_bytes_total: None,
                });
                self.status = format!("다운로드 시작: {}", repo);
                let (task, handle) = Task::run(
                    hf::download_repo(repo, std::path::PathBuf::from(dir), token, self.hf_revision.take(), self.hf_folder_name.take()),
                    Message::HfDownloadEvent,
                )
                .abortable();
                // 기존 abort_handle은 chat 전용이라 별도 보관 X — Cancel 시 hf_dl=None만 set
                // 단순함 우선: 다운로드 abort handle을 abort_handle에 저장 (chat과 공용)
                self.abort_handle = Some(handle);
                task
            }
            Message::HfDownloadEvent(ev) => {
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
                        hf::DownloadEvent::FileDone { .. } => {}
                        hf::DownloadEvent::AllDone => {
                            self.status = format!(
                                "다운로드 완료: {} — 이 경로를 inference 엔진(xLLM 등)에 지정해 띄운 뒤 Tabby URL 자리에 그 endpoint 입력",
                                dl.repo_id
                            );
                            self.hf_dl = None;
                            self.abort_handle = None;
                        }
                        hf::DownloadEvent::Error(e) => {
                            self.status = format!("다운로드 실패: {}", e);
                            self.hf_dl = None;
                            self.abort_handle = None;
                        }
                    }
                }
                Task::none()
            }
            Message::CancelHfDownload => {
                if let Some(h) = self.abort_handle.take() {
                    h.abort();
                }
                self.hf_dl = None;
                self.status = "다운로드 취소됨".into();
                Task::none()
            }
            Message::RegenerateLast => {
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
                    view_mode: ViewMode::Rendered,
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
                        Some(tools::tool_definitions(self.agent_mode.allow_mutating())),
                    ),
                    Message::ChatChunk,
                )
                .abortable();
                self.abort_handle = Some(handle);
                Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
            }
            Message::ApplyChange(block_id, idx) => {
                // 1) 먼저 candidate 정보 복사 (immutable borrow 후 종료)
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
                // 2) write_file dispatch (cwd 보안 검증 포함)
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
            Message::EditLastUser => {
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
                // blocks: 마지막 user 위치부터 끝까지 제거
                self.blocks.truncate(idx);
                // conversation: 마지막 user 다음 다 제거 → 그 user도 pop
                truncate_after_last_user(&mut self.conversation);
                self.conversation.pop();
                self.tool_round = 0;
                self.pending_tool_calls.clear();
                self.input = user_text;
                self.status = "편집 모드 — 수정 후 Enter".into();
                Task::none()
            }

            // ── 파일 컨텍스트 첨부 ────────────────────────────────
            Message::FileDropped(path) => {
                if self.is_already_attached(&path) {
                    return Task::none();
                }
                Task::perform(
                    async move {
                        let content = tokio::fs::read_to_string(&path).await
                            .map_err(|e| format!("읽기 실패: {e}"))?;
                        if content.len() > MAX_ATTACH_BYTES as usize {
                            return Err(format!("첨부 거부 (512KB 초과): {}", path.display()));
                        }
                        Ok((path, content))
                    },
                    |r| match r {
                        Ok((p, s)) => Message::FileReadDone(p, s),
                        Err(msg) => Message::FileAttachError(msg),
                    },
                )
            }
            Message::FileDragHover => Task::none(),
            Message::FileReadDone(path, content) => {
                if !self.is_already_attached(&path) {
                    self.status = format!("첨부됨: {}", path.display());
                    self.attached_files.push((path, content));
                }
                Task::none()
            }
            Message::FileAttachError(msg) => {
                self.status = msg;
                Task::none()
            }

            // ── MCP ───────────────────────────────────────────────────
            Message::McpNameChanged(v) => { self.mcp_name_input = v; Task::none() }
            Message::McpCommandChanged(v) => { self.mcp_command_input = v; Task::none() }
            Message::AddMcpServer => {
                let name = self.mcp_name_input.trim().to_string();
                let command = self.mcp_command_input.trim().to_string();
                if name.is_empty() || command.is_empty() {
                    self.status = "MCP 서버 이름과 명령을 모두 입력하세요.".into();
                    return Task::none();
                }
                let server = mcp::McpServer { name: name.clone(), command };
                self.mcp_servers.push(server.clone());
                self.mcp_name_input.clear();
                self.mcp_command_input.clear();
                if let Err(e) = mcp::save_servers(&self.mcp_servers) {
                    self.status = format!("MCP 저장 실패: {e}");
                    return Task::none();
                }
                self.status = format!("MCP 서버 추가됨: {name} — tool 목록 로드 중…");
                Task::perform(
                    async move {
                        mcp::list_tools(&server).await
                            .map(|tools| (name.clone(), tools))
                            .map_err(|e| format!("[{name}] {e}"))
                    },
                    |r| match r {
                        Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                        Err(msg) => Message::McpToolsFailed(msg),
                    },
                )
            }
            Message::RemoveMcpServer(idx) => {
                if idx < self.mcp_servers.len() {
                    let removed = self.mcp_servers.remove(idx);
                    self.mcp_tools.retain(|t| t.server_name != removed.name);
                    let _ = mcp::save_servers(&self.mcp_servers);
                    self.status = format!("MCP 서버 제거됨: {}", removed.name);
                }
                Task::none()
            }
            Message::McpToolsLoaded(server_name, tools) => {
                self.mcp_tools.retain(|t| t.server_name != server_name);
                let count = tools.len();
                self.mcp_tools.extend(tools);
                self.status = format!("MCP [{server_name}] tool {count}개 로드 완료");
                Task::none()
            }
            Message::McpToolsFailed(msg) => {
                self.status = format!("MCP tool 로드 실패: {msg}");
                Task::none()
            }
            Message::McpToolResult(tool_call_id, result) => {
                self.conversation.push(ChatMessage::tool_result(&tool_call_id, result));
                self.tool_round += 1;
                self.kick_chat_stream()
            }

            // ── PTY 터미널 ─────────────────────────────────────────
            Message::PtyToggle => {
                self.pty_visible = !self.pty_visible;
                if self.pty_visible && self.pty_session.is_none() {
                    return Task::done(Message::PtyStart);
                }
                Task::none()
            }
            Message::PtyStart => {
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
            Message::PtyLine(line) => {
                let clean = pty::strip_ansi(&line);
                if !clean.trim().is_empty() {
                    self.pty_output.push_back(clean);
                    while self.pty_output.len() > PTY_MAX_LINES {
                        self.pty_output.pop_front();
                    }
                }
                Task::none()
            }
            Message::PtyExited => {
                self.pty_session = None;
                self.pty_output.push_back("-- 셸 종료 --".into());
                self.status = "터미널 종료됨".into();
                Task::none()
            }
            Message::PtyInputChanged(v) => {
                self.pty_input = v;
                Task::none()
            }
            Message::PtySend => {
                let line = self.pty_input.trim_end().to_string();
                if let Some(s) = &self.pty_session {
                    s.write_line(&line);
                    // 에코 (터미널은 보통 자체 에코를 하지만, raw 모드에서 안 될 수도)
                    self.pty_output.push_back(format!("> {line}"));
                    while self.pty_output.len() > PTY_MAX_LINES {
                        self.pty_output.pop_front();
                    }
                }
                self.pty_input.clear();
                Task::none()
            }
            Message::PtyCtrlC => {
                if let Some(s) = &self.pty_session {
                    s.ctrl_c();
                }
                Task::none()
            }
            Message::PtyClear => {
                self.pty_output.clear();
                Task::none()
            }

            Message::RemoveAttachment(idx) => {
                if idx < self.attached_files.len() {
                    self.attached_files.remove(idx);
                }
                Task::none()
            }

            // ── @-mention ─────────────────────────────────────────
            Message::MentionMove(delta) => {
                if !self.show_mention || self.mention_candidates.is_empty() {
                    return Task::none();
                }
                let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
                let n = filtered.len();
                if n == 0 { return Task::none(); }
                self.mention_selected = (self.mention_selected as i64 + delta as i64)
                    .rem_euclid(n as i64) as usize;
                Task::none()
            }
            Message::MentionConfirm => {
                if !self.show_mention { return Task::none(); }
                let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
                let Some(chosen) = filtered.into_iter().nth(self.mention_selected) else {
                    return Task::none();
                };
                // input에서 '@query' 제거
                if let Some(at_pos) = self.input.rfind('@') {
                    self.input.truncate(at_pos);
                }
                self.close_mention();
                if self.is_already_attached(&chosen) {
                    return Task::none();
                }
                let full_path = self.cwd.join(&chosen);
                Task::perform(
                    async move {
                        let content = tokio::fs::read_to_string(&full_path).await
                            .map_err(|e| format!("읽기 실패: {e}"))?;
                        if content.len() > MAX_ATTACH_BYTES as usize {
                            return Err(format!("첨부 거부 (512KB 초과): {}", chosen.display()));
                        }
                        Ok((chosen, content))
                    },
                    |r| match r {
                        Ok((p, s)) => Message::FileReadDone(p, s),
                        Err(msg) => Message::FileAttachError(msg),
                    },
                )
            }
            Message::MentionCandidatesLoaded(paths) => {
                self.mention_candidates = paths;
                Task::none()
            }

            Message::FetchModels => {
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
            Message::ModelsLoaded(r) => {
                self.busy = false;
                match r {
                    Ok(models) => {
                        let n = models.len();
                        self.model_ids = models.iter().map(|m| m.id.clone()).collect();
                        // OpenRouter 항목만 교체, Tabby 항목 보존
                        self.model_options.retain(|o| o.provider != LlmProvider::OpenRouter);
                        self.model_options.extend(models.iter().map(|m| {
                            let id = m.id.clone();
                            let ko_friendly = is_korean_friendly(&id);
                            let favorite = self.favorites.contains(&id);
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
                        let saved_in_list = self
                            .selected_model
                            .as_ref()
                            .map(|id| self.model_ids.iter().any(|m| m == id))
                            .unwrap_or(false);
                        if !saved_in_list {
                            self.selected_model = self.model_ids.first().cloned();
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
            Message::SelectModel(opt) => {
                let _ = keystore::write_selected_model(&opt.id);
                self.selected_model = Some(opt.id);
                Task::none()
            }
            Message::FetchAccount => {
                let key = match keystore::read_api_key() {
                    Ok(k) => k,
                    Err(_) => return Task::none(),
                };
                Task::perform(
                    openrouter::get_account_info(key),
                    Message::AccountLoaded,
                )
            }
            Message::AccountLoaded(r) => {
                if let Ok(data) = r {
                    self.account = Some(data);
                }
                Task::none()
            }
            Message::InputChanged(v) => {
                self.input = v;
                // @-mention 팝업 감지
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
                        if self.show_mention { self.close_mention(); }
                    }
                }
                Task::none()
            }
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
                if self.selected_model.is_none() || self.streaming_block_id.is_some() {
                    return Task::none();
                }
                let (base_url, api_key) = match self.resolve_provider() {
                    Ok(v) => v,
                    Err(e) => {
                        self.status = e;
                        return Task::none();
                    }
                };
                let model = self.selected_model.clone().unwrap();

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
                    view_mode: ViewMode::Rendered,
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
                        Some(tools::tool_definitions(self.agent_mode.allow_mutating())),
                    ),
                    Message::ChatChunk,
                )
                .abortable();
                self.abort_handle = Some(handle);
                Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
            }
            Message::StopStream => {
                if let Some(h) = self.abort_handle.take() {
                    h.abort();
                }
                if let Some(ai_id) = self.streaming_block_id {
                    if let Some(b) = self.blocks.iter().find(|b| b.id == ai_id) {
                        let txt = b.body.to_text();
                        if !txt.is_empty() {
                            self.conversation.push(ChatMessage::assistant(txt));
                        }
                    }
                }
                self.streaming_block_id = None;
                self.pending_tool_calls.clear();
                self.tool_round = 0;
                self.status = "중지됨".into();
                self.maybe_update_title();
                self.save_session();
                Task::none()
            }
            Message::CopyBlock(id) => {
                if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
                    return iced::clipboard::write(b.body.to_text());
                }
                Task::none()
            }
            Message::ChatChunk(event) => {
                let Some(ai_id) = self.streaming_block_id else {
                    return Task::none();
                };
                match event {
                    ChatEvent::Token(t) => {
                        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == ai_id) {
                            if let BlockBody::Assistant(content) = &mut b.body {
                                content.perform(Action::Edit(Edit::Paste(Arc::new(t))));
                                let raw = content.text();
                                b.md_items = markdown::parse(&raw).collect();
                            }
                        }
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
                        if self.tool_round >= MAX_TOOL_ROUNDS && !self.pending_tool_calls.is_empty() {
                            self.status =
                                format!("최대 도구 라운드 {} 초과", MAX_TOOL_ROUNDS);
                        } else {
                            self.status = "준비됨".into();
                        }
                        if !assistant_text.is_empty() {
                            self.conversation
                                .push(ChatMessage::assistant(assistant_text.clone()));
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
                                let prefix =
                                    if content.text().is_empty() { "" } else { "\n\n" };
                                let msg = format!("{}[에러] {}", prefix, e);
                                content.perform(Action::Edit(Edit::Paste(Arc::new(msg))));
                                let raw = content.text();
                                b.md_items = markdown::parse(&raw).collect();
                            }
                        }
                        self.streaming_block_id = None;
                        self.abort_handle = None;
                        self.pending_tool_calls.clear();
                        self.status = format!("에러: {}", openrouter::humanize_error(&e));
                    }
                }
                if self.follow_bottom {
                    snap_to_end(self.stream_id.clone())
                } else {
                    Task::none()
                }
            }
            Message::StreamScrolled(viewport) => {
                // 사용자가 거의 끝까지 내려가 있으면 follow ON, 아니면 OFF
                let rel = viewport.relative_offset();
                self.follow_bottom = rel.y > 0.95;
                self.current_scroll_y = viewport.absolute_offset().y;
                Task::none()
            }
            Message::EditorAction(id, action) => {
                // read-only: Edit 액션은 무시 (사용자 키보드 입력 차단), 나머지(선택/스크롤)는 처리
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
            Message::ToggleBlockView(id) => {
                if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                    b.view_mode = match b.view_mode {
                        ViewMode::Rendered => ViewMode::Raw,
                        ViewMode::Raw => ViewMode::Rendered,
                    };
                }
                Task::none()
            }
            Message::LinkClicked(_uri) => {
                // TODO: 시스템 브라우저 열기 (webbrowser crate 등)
                Task::none()
            }
            Message::PickCwd => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("작업 폴더 선택")
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::CwdPicked,
            ),
            Message::ApproveWrites => {
                self.expanded_confirm_idx = None;
                self.continue_after_writes(true)
            }
            Message::DenyWrites => {
                self.expanded_confirm_idx = None;
                self.continue_after_writes(false)
            }
            Message::ToggleConfirmExpand(idx) => {
                self.expanded_confirm_idx = if self.expanded_confirm_idx == Some(idx) {
                    None
                } else {
                    Some(idx)
                };
                Task::none()
            }
            Message::DiscardWriteCall(idx) => {
                if idx >= self.pending_write_calls.len() {
                    return Task::none();
                }
                let tc = self.pending_write_calls.remove(idx);
                self.push_tool_result_block(tc.name.clone(), "discarded".into(), false);
                self.conversation.push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 이 도구 호출을 제외했습니다.",
                ));
                // 펼친 인덱스 보정 (제거된 항목 이후는 한 칸 당김)
                self.expanded_confirm_idx = match self.expanded_confirm_idx {
                    Some(e) if e == idx => None,
                    Some(e) if e > idx => Some(e - 1),
                    other => other,
                };
                // 모든 항목이 제거됐다면 자동 진행 (= 모두 거부와 동일)
                if self.pending_write_calls.is_empty() {
                    return self.continue_after_writes(true);
                }
                Task::none()
            }
            Message::ToggleFilterCoding(v) => {
                self.filter_coding = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::ToggleFilterReasoning(v) => {
                self.filter_reasoning = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::ToggleFilterGeneral(v) => {
                self.filter_general = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::ToggleFilterFavorites(v) => {
                self.filter_favorites_only = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::CycleSortMode => {
                self.sort_mode = self.sort_mode.cycle();
                self.refresh_model_combo();
                Task::none()
            }
            Message::SetAgentMode(mode) => {
                self.agent_mode = mode;
                self.status = format!("{} 모드", mode.label());
                Task::none()
            }
            Message::ToggleAgentMode => {
                self.agent_mode = match self.agent_mode {
                    AgentMode::Plan => AgentMode::Build,
                    AgentMode::Build => AgentMode::Plan,
                };
                self.status = format!("{} 모드", self.agent_mode.label());
                Task::none()
            }
            Message::NewChat => {
                // 현재 세션 보존 + 새 빈 세션 시작
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
                self.current_session_id = self.allocate_session_id();
                self.current_session_title = "새 채팅".into();
                self.status = "새 채팅".into();
                self.save_session();
                Task::none()
            }
            Message::SwitchSession(target_id) => {
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
                // 현재 활성을 inactive로 보관
                self.snapshot_current_to_inactive();
                // target 활성화
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
                self.status = "세션 전환됨".into();
                self.save_session();
                // 새 세션의 마지막 scroll 위치로 복원
                iced::widget::operation::scroll_to(
                    self.stream_id.clone(),
                    iced::widget::scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: target.scroll_y,
                    },
                )
            }
            Message::OpenCommandPalette => {
                self.show_command_palette = true;
                self.command_palette_input.clear();
                Task::none()
            }
            Message::CloseCommandPalette => {
                self.show_command_palette = false;
                Task::none()
            }
            Message::CloseAllOverlays => {
                self.show_command_palette = false;
                self.show_settings = false;
                self.show_write_confirm = false;
                self.close_mention();
                Task::none()
            }
            Message::CommandPaletteChanged(v) => {
                self.command_palette_input = v;
                Task::none()
            }
            Message::ExecuteCommand(idx) => {
                let filtered = self.filtered_palette_commands();
                let Some(cmd) = filtered.get(idx) else {
                    return Task::none();
                };
                let action = cmd.action;
                self.show_command_palette = false;
                self.command_palette_input.clear();
                match action {
                    PaletteAction::NewChat => return Task::done(Message::NewChat),
                    PaletteAction::PlanMode => {
                        return Task::done(Message::SetAgentMode(AgentMode::Plan))
                    }
                    PaletteAction::BuildMode => {
                        return Task::done(Message::SetAgentMode(AgentMode::Build))
                    }
                    PaletteAction::OpenSettings => return Task::done(Message::OpenSettings),
                    PaletteAction::PickCwd => return Task::done(Message::PickCwd),
                    PaletteAction::CycleSort => return Task::done(Message::CycleSortMode),
                    PaletteAction::ToggleFavorite => {
                        return Task::done(Message::ToggleFavorite)
                    }
                }
            }
            Message::GenerationLoaded(r) => {
                if let Ok(data) = r {
                    let cost = data.total_cost.unwrap_or(0.0);
                    self.last_response_cost = Some(cost);
                    let model_id = data.model.clone().unwrap_or_default();
                    if !model_id.is_empty() {
                        let entry = self
                            .usage
                            .by_model
                            .entry(model_id)
                            .or_default();
                        entry.total_cost += cost;
                        entry.prompt_tokens += data.native_tokens_prompt.unwrap_or(0);
                        entry.completion_tokens += data.native_tokens_completion.unwrap_or(0);
                        entry.call_count += 1;
                    }
                    let _ = session::save_usage(&self.usage);
                    // 사용 후 잔액 갱신을 위해 account 다시 fetch
                    return Task::done(Message::FetchAccount);
                }
                Task::none()
            }
            Message::AskDeleteSession(id) => {
                self.pending_delete_session = if self.pending_delete_session == Some(id) {
                    None // 같은 ✕ 다시 클릭 → 취소
                } else {
                    Some(id)
                };
                Task::none()
            }
            Message::CancelDeleteSession => {
                self.pending_delete_session = None;
                Task::none()
            }
            Message::DeleteSession(target_id) => {
                self.pending_delete_session = None;
                if target_id == self.current_session_id {
                    // 현재 활성을 삭제 → 빈 세션으로 대체
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
            Message::ToggleFavorite => {
                if let Some(id) = &self.selected_model {
                    if self.favorites.contains(id) {
                        self.favorites.remove(id);
                    } else {
                        self.favorites.insert(id.clone());
                    }
                    let favs: Vec<String> = self.favorites.iter().cloned().collect();
                    let _ = session::write_favorites(&favs);
                    self.refresh_model_combo();
                }
                Task::none()
            }
            Message::CwdPicked(maybe_path) => {
                if let Some(path) = maybe_path {
                    self.cwd = path.clone();
                    let _ = keystore::write_cwd(&path.display().to_string());
                    self.status = format!("작업 폴더: {}", path.display());
                    // system 메시지(cwd 안내) 갱신
                    self.ensure_system_message();
                }
                Task::none()
            }
        }
    }

    /// 현재 활성 필터/정렬을 적용해 model_options을 좁힌 결과.
    fn filtered_model_options(&self) -> Vec<ModelOption> {
        let mut opts: Vec<ModelOption> = self
            .model_options
            .iter()
            .filter(|opt| {
                if self.filter_favorites_only && !self.favorites.contains(&opt.id) {
                    return false;
                }
                let cats = categorize_model(&opt.id);
                (self.filter_coding && cats.contains(&ModelCategory::Coding))
                    || (self.filter_reasoning && cats.contains(&ModelCategory::Reasoning))
                    || (self.filter_general && cats.contains(&ModelCategory::General))
            })
            .cloned()
            .collect();

        // 정렬: prompt+completion 합 기준
        let total_price = |o: &ModelOption| -> f64 {
            o.prompt_per_million.unwrap_or(0.0) + o.completion_per_million.unwrap_or(0.0)
        };
        match self.sort_mode {
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

    /// 필터/즐겨찾기 변경 시 combo_box::State 재구성.
    fn refresh_model_combo(&mut self) {
        // favorite 필드를 현재 favorites HashSet과 동기화 (Display에 ★ 반영)
        for opt in &mut self.model_options {
            opt.favorite = self.favorites.contains(&opt.id);
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
                    role: match &b.body {
                        BlockBody::User(_) => "user".into(),
                        BlockBody::Assistant(_) => "assistant".into(),
                        _ => unreachable!(),
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
                    role: match &b.body {
                        BlockBody::User(_) => "user".into(),
                        BlockBody::Assistant(_) => "assistant".into(),
                        _ => unreachable!(),
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
        if let Some(idx) = self
            .inactive_sessions
            .iter()
            .position(|s| s.id == snap.id)
        {
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

    fn is_already_attached(&self, path: &std::path::Path) -> bool {
        self.attached_files.iter().any(|(p, _)| p == path)
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
                self.conversation.push(ChatMessage::tool_result(&tc.id, result));
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
                            Some(s) => mcp::call_tool(&s, &tool_name, args).await
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
            .model_options
            .iter()
            .find(|o| o.id == id)
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
        let mut tool_defs = tools::tool_definitions(self.agent_mode.allow_mutating());
        if !self.mcp_tools.is_empty() {
            if let Some(arr) = tool_defs.as_array_mut() {
                for t in &self.mcp_tools {
                    arr.push(t.to_openai_tool());
                }
            }
        }
        let (task, handle) = Task::run(
            openrouter::chat_stream(
                base_url,
                api_key,
                model,
                messages,
                Some(tool_defs),
            ),
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
        let q = self.command_palette_input.to_lowercase();
        if q.is_empty() {
            PALETTE_COMMANDS.iter().collect()
        } else {
            PALETTE_COMMANDS
                .iter()
                .filter(|c| {
                    c.label.to_lowercase().contains(&q)
                        || c.hint.to_lowercase().contains(&q)
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
