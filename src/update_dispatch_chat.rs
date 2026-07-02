impl App {
    pub(crate) fn dispatch_chat(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::Send => {
                self.toast = None;
                Some(self.send_message())
            }
            Message::StopStream => Some(self.stop_stream()),
            Message::RegenerateLast => Some(self.regenerate_last()),
            Message::ApplyChange(block_id, idx) => Some(self.apply_change(*block_id, *idx)),
            Message::EditLastUser => Some(self.edit_last_user()),
            Message::ChatChunk(event) => Some(self.on_chat_chunk(event.clone())),
            Message::StreamScrolled(viewport) => Some(self.on_stream_scrolled(viewport)),
            Message::EditorAction(id, action) => Some(self.on_editor_action(*id, action.clone())),
            Message::ToggleBlockView(id) => Some(self.toggle_block_view(*id)),
            Message::ToggleBlockCollapse(id) => {
                if !self.ui.collapsed_blocks.remove(id) {
                    self.ui.collapsed_blocks.insert(*id);
                }
                Some(Task::none())
            }
            Message::LinkClicked(uri) => Some(self.on_link_clicked(uri)),
            Message::CopyBlock(id) => Some(self.copy_block(*id)),
            Message::CopyText(text) => Some(iced::clipboard::write(text.clone())),
            Message::CompareResponsesLoaded {
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            } => Some(self.on_compare_responses_loaded(
                *openrouter_block_id,
                *tabby_block_id,
                openrouter_result.clone(),
                tabby_result.clone(),
            )),
            Message::GenerationLoaded(r) => Some(self.on_generation_loaded(r.clone())),
            Message::FetchModels => Some(self.fetch_models_cmd()),
            Message::ModelsLoaded(r) => Some(self.on_models_loaded(r.clone())),
            Message::SelectModel(opt) => Some(self.select_model(opt.clone())),
            Message::FetchAccount => Some(App::fetch_account_cmd()),
            Message::AccountLoaded(r) => Some(self.on_account_loaded(r.clone())),
            _ => None,
        }
    }
}
