impl App {
    pub(crate) fn dispatch_session(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::AutoSave => {
                self.save_session();
                Some(Task::none())
            }
            Message::WindowCloseRequested => {
                self.save_session();
                session::mark_clean_shutdown();
                Some(Task::none())
            }
            Message::NewChat => {
                self.toast = None;
                Some(self.new_chat())
            }
            Message::SwitchSession(target_id) => Some(self.switch_session(*target_id)),
            Message::AskDeleteSession(id) => Some(self.ask_delete_session(*id)),
            Message::CancelDeleteSession => Some(self.cancel_delete_session()),
            Message::DeleteSession(target_id) => Some(self.delete_session(*target_id)),
            Message::StartRenameSession(id) => Some(self.start_rename_session(*id)),
            Message::RenameSession(id, title) => Some(self.rename_session(*id, title.clone())),
            Message::CancelRenameSession => Some(self.cancel_rename_session()),
            Message::SessionSearchChanged(v) => Some(self.update_session_search(v.clone())),
            _ => None,
        }
    }
}
