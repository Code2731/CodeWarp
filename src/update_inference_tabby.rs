// update_inference_tabby.rs — Tabby model & provider update methods (main.rs child module)
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
