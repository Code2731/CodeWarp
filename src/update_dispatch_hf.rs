impl App {
    pub(crate) fn dispatch_hf(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::HfTokenChanged(v) => Some(self.set_hf_token_input(v.clone())),
            Message::ToggleHfTokenVisible => Some(self.toggle_hf_token_visible()),
            Message::SaveHfToken => Some(self.save_hf_token()),
            Message::HfTokenSaved(r) => Some(self.on_hf_token_saved(r.clone())),
            Message::ModelDirChanged(v) => Some(self.set_model_dir(v)),
            Message::PickModelDir => Some(Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::ModelDirPicked,
            )),
            Message::ModelDirPicked(maybe) => Some(self.on_model_dir_picked(maybe.clone())),
            Message::HfRepoChanged(v) => Some(self.set_hf_repo_input(v.clone())),
            Message::UsePreset(idx) => Some(self.apply_model_preset(*idx)),
            Message::DownloadExl2Preset(idx) => Some(self.prepare_exl2_preset_download(*idx)),
            Message::SelectDownloadedModel(folder_name) => {
                Some(self.select_downloaded_model(folder_name))
            }
            Message::StartHfDownload => Some(self.start_hf_download()),
            Message::HfDownloadEvent(ev) => Some(self.on_hf_download_event(ev)),
            Message::CancelHfDownload => Some(self.cancel_hf_download()),
            _ => None,
        }
    }
}
