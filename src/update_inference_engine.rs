// update_inference_engine.rs — Inference engine capability checks (main.rs child module)
use super::{list_downloaded_models, App, InferenceEngine};

impl App {
    pub(crate) fn has_selected_local_model_available(&self) -> bool {
        let selected = self.inference_selected_model.trim();
        if selected.is_empty() {
            return false;
        }
        list_downloaded_models(std::path::Path::new(&self.model_dir_input))
            .iter()
            .any(|m| m == selected)
    }
    pub(crate) fn sync_selected_local_model_for_model_dir(&mut self) {
        if !matches!(
            self.inference_engine,
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer
        ) {
            return;
        }
        if !self.has_selected_local_model_available() {
            self.inference_selected_model.clear();
        }
    }
    pub(crate) fn can_start_inference(&self) -> bool {
        match self.inference_engine {
            InferenceEngine::Custom => !self.inference_command_input.trim().is_empty(),
            InferenceEngine::Ollama => true,
            InferenceEngine::TabbyMl => !self.inference_selected_model.trim().is_empty(),
            InferenceEngine::TabbyApi => !self.inference_binary_path.trim().is_empty(),
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer => {
                self.has_selected_local_model_available()
            }
        }
    }
    pub(crate) fn can_attempt_start_inference(&self) -> bool {
        match self.inference_engine {
            InferenceEngine::TabbyApi => true,
            _ => self.can_start_inference(),
        }
    }
}
