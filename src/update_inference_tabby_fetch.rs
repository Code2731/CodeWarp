// update_inference_tabby_fetch.rs — Tabby model fetch/retry methods (main.rs child module)
use super::*;
use iced::Task;

impl App {
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
}
