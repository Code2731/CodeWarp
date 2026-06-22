// update_settings.rs — Settings/UI App update methods (main.rs child module)
use super::{
    is_korean_friendly, keystore, openrouter, parse_price_per_million, App, LlmProvider, Message,
    ModelOption, PaletteCommand, EXL2_PRESETS, MODEL_PRESETS, PALETTE_COMMANDS,
};

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
            Err(e) => self.status = format!("HF 토큰 저장 실패: {e}"),
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
        self.status = format!("잘못된 프리셋 인덱스: {idx}");
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
                self.status = format!("모델 {n} 로드됨");
            }
            Err(e) => self.status = format!("페치 실패: {}", openrouter::humanize_error(&e)),
        }
        Task::none()
    }
    pub(crate) fn fetch_account_cmd() -> Task<Message> {
        let Ok(key) = keystore::read_api_key() else {
            return Task::none();
        };
        Task::perform(openrouter::get_account_info(key), Message::AccountLoaded)
    }
    pub(crate) fn pick_cwd() -> Task<Message> {
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
mod tests;
