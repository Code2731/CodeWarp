impl App {
    pub(crate) fn dispatch_settings(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::OpenSettings => Some(self.open_settings_overlay()),
            Message::CloseSettings => Some(self.close_settings_overlay()),
            Message::SetSettingsTab(tab) => Some(self.set_settings_tab(*tab)),
            Message::KeyInputChanged(v) => Some(self.set_key_input(v.clone())),
            Message::SaveKey => Some(self.save_api_key()),
            Message::KeySaved(r) => Some(self.on_key_saved(r.clone())),
            Message::ClearKey => Some(self.clear_api_key()),
            Message::KeyCleared(r) => Some(self.on_key_cleared(r.clone())),
            Message::TabbyUrlChanged(v) => Some(self.set_tabby_url(v.clone())),
            Message::TabbyTokenChanged(v) => Some(self.set_tabby_token(v.clone())),
            Message::ToggleTabbyTokenVisible => Some(self.toggle_tabby_token_visible()),
            Message::InferenceCommandChanged(v) => Some(self.set_inference_command(v)),
            Message::SelectInferenceEngine(e) => Some(self.select_inference_engine(*e)),
            Message::SelectInferenceModel(m) => Some(self.set_inference_model(m.clone())),
            Message::InferencePortChanged(v) => Some(self.set_inference_port(v)),
            Message::InferenceBinaryChanged(v) => Some(self.set_inference_binary(v)),
            Message::PickInferenceBinary => Some(App::pick_inference_binary()),
            Message::InferenceBinaryPicked(maybe) => {
                Some(self.on_inference_binary_picked(maybe.clone()))
            }
            Message::InstallTabbyApiRuntime => Some(self.install_tabbyapi_runtime_cmd()),
            Message::TabbyApiRuntimeInstalled(result) => {
                Some(self.on_tabbyapi_runtime_installed(result.clone()))
            }
            Message::StartInference => Some(self.start_inference()),
            Message::StopInference => Some(self.stop_inference()),
            Message::InferenceLogLine(line) => Some(self.on_inference_log_line(line.clone())),
            Message::InferenceExited(code) => Some(self.on_inference_exited(*code)),
            Message::OpenAICompatLabelChanged(v) => Some(self.set_openai_compat_label(v.clone())),
            Message::SaveTabby => Some(self.save_tabby_settings()),
            Message::TabbySaved(r) => Some(self.on_tabby_saved(r.clone())),
            Message::ClearTabby => Some(self.clear_tabby_settings()),
            Message::FetchTabbyModels => Some(self.fetch_tabby_models()),
            Message::FetchTabbyModelsRetry(generation) => Some(self.retry_fetch_tabby_models(*generation)),
            Message::TabbyModelsLoaded(r) => Some(self.on_tabby_models_loaded(r.clone())),
            _ => None,
        }
    }
}
