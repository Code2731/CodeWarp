// update_inference_tabby.rs — Model selection & provider resolution (main.rs child module)
use super::*;

impl App {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn or_opt(id: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenRouter,
            provider_label: String::new(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    fn oai_opt(id: &str, label: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: label.into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    #[test]
    fn local_openai_compat_models_do_not_send_tool_definitions() {
        let (mut app, _) = App::new();
        app.model_options = vec![ModelOption {
            id: "local-model".into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: "TabbyAPI".into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: Some(0.0),
            completion_per_million: Some(0.0),
        }];
        app.selected_model = Some("local-model".into());

        assert!(app.tool_definitions_for_selected_model().is_none());
    }

    #[test]
    fn selected_model_with_same_id_uses_explicit_provider_choice() {
        let (mut app, _) = App::new();
        app.model_options = vec![or_opt("shared-model"), oai_opt("shared-model", "TabbyAPI")];
        app.tabby_url_input = "http://localhost:5000".into();

        let _ = app.update(Message::SelectModel(oai_opt("shared-model", "TabbyAPI")));
        assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenAICompat));
        assert!(app.tool_definitions_for_selected_model().is_none());

        let _ = app.update(Message::SelectModel(or_opt("shared-model")));
        assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenRouter));
        assert!(app.tool_definitions_for_selected_model().is_some());
    }

    #[test]
    fn resolve_provider_prefers_current_tabby_inputs_over_keystore() {
        let (mut app, _) = App::new();
        app.model_options = vec![oai_opt("local-model", "TabbyAPI")];
        app.selected_model = Some("local-model".into());
        app.selected_model_provider = Some(LlmProvider::OpenAICompat);
        app.tabby_url_input = "http://localhost:5001".into();
        app.tabby_token_input = "live-token".into();

        let (base_url, api_key) = app.resolve_provider().expect("provider resolves");

        assert_eq!(base_url, "http://localhost:5001/v1");
        assert_eq!(api_key.as_deref(), Some("live-token"));
    }
}
