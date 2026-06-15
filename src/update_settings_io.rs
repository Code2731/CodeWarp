// update_settings_io.rs — Authentication/credentials input and save/clear methods
use super::*;
use iced::Task;

impl App {
    pub(crate) fn set_key_input(&mut self, value: String) -> Task<Message> {
        self.key_input = value;
        Task::none()
    }
    pub(crate) fn set_tabby_url(&mut self, value: String) -> Task<Message> {
        self.tabby_url_input = value;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }
    pub(crate) fn set_tabby_token(&mut self, value: String) -> Task<Message> {
        self.tabby_token_input = value;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }
    pub(crate) fn toggle_tabby_token_visible(&mut self) -> Task<Message> {
        self.show_tabby_token = !self.show_tabby_token;
        Task::none()
    }
    pub(crate) fn set_inference_command(&mut self, value: String) -> Task<Message> {
        self.inference_command_input = value.clone();
        let _ = keystore::write_inference_command(&value);
        Task::none()
    }
    pub(crate) fn set_inference_model(&mut self, value: String) -> Task<Message> {
        self.inference_selected_model = value;
        Task::none()
    }
    pub(crate) fn set_hf_token_input(&mut self, value: String) -> Task<Message> {
        self.hf_token_input = value;
        Task::none()
    }
    pub(crate) fn toggle_hf_token_visible(&mut self) -> Task<Message> {
        self.show_hf_token = !self.show_hf_token;
        Task::none()
    }
    pub(crate) fn set_hf_repo_input(&mut self, value: String) -> Task<Message> {
        self.hf_repo_input = value;
        Task::none()
    }
    pub(crate) fn set_pty_input(&mut self, value: String) -> Task<Message> {
        self.pty_input = value;
        Task::none()
    }
    pub(crate) fn pty_ctrl_c(&mut self) -> Task<Message> {
        if let Some(s) = &self.pty_session {
            s.ctrl_c();
        }
        Task::none()
    }
    pub(crate) fn pty_clear(&mut self) -> Task<Message> {
        self.pty_output.clear();
        Task::none()
    }
    pub(crate) fn file_drag_hover(&mut self) -> Task<Message> {
        Task::none()
    }
    pub(crate) fn file_attach_error(&mut self, msg: String) -> Task<Message> {
        self.status = msg;
        Task::none()
    }
    pub(crate) fn save_api_key(&mut self) -> Task<Message> {
        let key = self.key_input.clone();
        self.busy = true;
        self.status = "키 저장 중…".into();
        Task::perform(
            async move { keystore::write_api_key(&key) },
            Message::KeySaved,
        )
    }
    pub(crate) fn on_key_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.has_key = true;
                self.key_input.clear();
                self.ui.show_settings = false;
                self.status = "키 저장됨".into();
                Task::done(Message::FetchModels)
            }
            Err(e) => {
                self.status = format!("저장 실패: {}", e);
                Task::none()
            }
        }
    }
    pub(crate) fn clear_api_key(&mut self) -> Task<Message> {
        self.busy = true;
        Task::perform(async { keystore::delete_api_key() }, Message::KeyCleared)
    }
    pub(crate) fn on_key_cleared(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.has_key = false;
                self.models.clear();
                self.model_ids.clear();
                self.selected_model = None;
                self.selected_model_provider = None;
                let _ = keystore::clear_selected_model();
                self.status = "키 삭제됨".into();
            }
            Err(e) => self.status = format!("삭제 실패: {}", e),
        }
        Task::none()
    }
    pub(crate) fn set_openai_compat_label(&mut self, value: String) -> Task<Message> {
        self.openai_compat_label = value;
        let _ = keystore::write_openai_compat_label(&self.openai_compat_label);
        let new_label = self.openai_compat_label.clone();
        for opt in &mut self.model_options {
            if opt.provider == LlmProvider::OpenAICompat {
                opt.provider_label = new_label.clone();
            }
        }
        self.refresh_model_combo();
        Task::none()
    }
    pub(crate) fn save_tabby_settings(&mut self) -> Task<Message> {
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
    pub(crate) fn on_tabby_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.status = "Tabby 설정 저장됨".into();
                if !self.tabby_url_input.trim().is_empty() {
                    return Task::done(Message::FetchTabbyModels);
                }
            }
            Err(e) => self.status = format!("Tabby 저장 실패: {}", e),
        }
        Task::none()
    }
    pub(crate) fn clear_tabby_settings(&mut self) -> Task<Message> {
        let _ = keystore::clear_tabby_base_url();
        let _ = keystore::clear_tabby_token();
        self.tabby_url_input.clear();
        self.tabby_token_input.clear();
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        self.tabby_status = None;
        self.status = "Tabby 설정 삭제됨".into();
        self.model_options
            .retain(|o| o.provider != LlmProvider::OpenAICompat);
        self.refresh_model_combo();
        if let Some(sel) = self.selected_model.clone() {
            if !self.model_options.iter().any(|o| o.id == sel) {
                if let Some(first) = self.model_options.first() {
                    self.selected_model = Some(first.id.clone());
                    self.selected_model_provider = Some(first.provider);
                } else {
                    self.selected_model = None;
                    self.selected_model_provider = None;
                }
                if let Some(id) = &self.selected_model {
                    let _ = keystore::write_selected_model(id);
                }
            }
        }
        Task::none()
    }
}
