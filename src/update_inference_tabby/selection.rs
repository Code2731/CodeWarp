use super::{App, LlmProvider, ModelOption, keystore, openrouter, tabby, tools};

impl App {
    pub(crate) fn selected_option(&self) -> Option<&ModelOption> {
        let id = self.selected_model.as_deref()?;
        if let Some(provider) = self.selected_model_provider
            && let Some(opt) = self
                .model_options
                .iter()
                .find(|o| o.id == id && o.provider == provider)
        {
            return Some(opt);
        }
        self.model_options.iter().find(|o| o.id == id)
    }
    pub(crate) fn selected_model_exists_in_options(&self) -> bool {
        self.selected_model
            .as_deref()
            .is_some_and(|id| self.model_options.iter().any(|o| o.id == id))
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
    pub(crate) fn resolve_provider(&self) -> Result<(String, Option<String>), String> {
        let id = self
            .selected_model
            .as_deref()
            .ok_or("모델 미선택".to_string())?;
        let provider = self
            .selected_option()
            .map(|o| o.provider)
            .ok_or_else(|| format!("선택된 모델을 찾을 수 없습니다: {id}"))?;
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
                .ok_or("Tabby URL 미설정".to_string())?;
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
}
