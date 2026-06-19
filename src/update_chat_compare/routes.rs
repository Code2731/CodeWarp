use super::*;

#[derive(Clone)]
pub(super) struct ChatRoute {
    pub(crate) label: String,
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
    pub(crate) model: String,
}

impl App {
    pub(super) fn compare_routes(&self) -> Result<(ChatRoute, ChatRoute), String> {
        let selected = self.selected_option();
        let openrouter_model = selected
            .filter(|o| o.provider == LlmProvider::OpenRouter)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenRouter)
            })
            .ok_or_else(|| "Compare 모드: OpenRouter 모델이 없습니다. OpenRouter 키/모델 목록을 먼저 불러와 주세요.".to_string())?;
        let tabby_model = selected
            .filter(|o| o.provider == LlmProvider::OpenAICompat)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenAICompat)
            })
            .ok_or_else(|| "Compare 모드: Tabby 모델이 없습니다. Provider 연결 테스트로 Tabby 모델을 먼저 불러와 주세요.".to_string())?;

        let openrouter_key = keystore::read_api_key()?;
        let tabby_base = if self.tabby_url_input.trim().is_empty() {
            keystore::read_tabby_base_url()
        } else {
            Some(self.tabby_url_input.clone())
        }
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Compare 모드: Tabby URL 미설정".to_string())?;
        let tabby_token = if self.tabby_token_input.trim().is_empty() {
            keystore::read_tabby_token()
        } else {
            Some(self.tabby_token_input.clone())
        }
        .filter(|s| !s.trim().is_empty());

        Ok((
            ChatRoute {
                label: "OpenRouter".into(),
                base_url: openrouter::BASE_URL.to_string(),
                api_key: Some(openrouter_key),
                model: openrouter_model.id.clone(),
            },
            ChatRoute {
                label: if tabby_model.provider_label.trim().is_empty() {
                    "Local".into()
                } else {
                    tabby_model.provider_label.trim().to_string()
                },
                base_url: tabby::chat_base(&tabby_base),
                api_key: tabby_token,
                model: tabby_model.id.clone(),
            },
        ))
    }
}
