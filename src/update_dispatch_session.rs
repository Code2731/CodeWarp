impl App {
    fn record_close_event(&mut self, event: CloseLifecycleEvent) {
        #[cfg(test)]
        self.close_lifecycle_events.push(event);
        #[cfg(not(test))]
        let _ = event;
    }

    fn finish_window_close<Save, Mark>(&mut self, save: Save, mark_clean: Mark) -> bool
    where
        Save: FnOnce(&mut Self) -> bool,
        Mark: FnOnce() -> Result<(), String>,
    {
        self.record_close_event(CloseLifecycleEvent::Persist);
        if !save(self) {
            self.close_in_progress = false;
            return false;
        }
        self.record_close_event(CloseLifecycleEvent::MarkClean);
        if let Err(error) = mark_clean() {
            self.status = format!("정상 종료 표시 실패: {error}");
        }
        self.record_close_event(CloseLifecycleEvent::CloseWindow);
        true
    }

    #[cfg(test)]
    fn window_close_at(&mut self, storage_dir: &std::path::Path, marker_dir: &std::path::Path) {
        let _ = self.finish_window_close(
            |app| app.save_session_at(storage_dir),
            || session::mark_clean_shutdown_at(marker_dir),
        );
    }

    pub(crate) fn begin_window_close(&mut self) -> Task<Message> {
        if self.close_in_progress {
            return Task::none();
        }
        self.close_in_progress = true;
        self.record_close_event(CloseLifecycleEvent::CancelStreamAndMcp);
        self.abort_active_chat_stream(true);
        if let Some(handle) = self.mcp_abort_handle.take() {
            handle.abort();
        }
        let inference = self.inference_stop.take();
        let pty = self.pty_session.take();
        self.record_close_event(CloseLifecycleEvent::ReapInferenceAndPty);
        Task::perform(
            reap_owned_processes(inference, pty),
            Message::WindowProcessesReaped,
        )
    }

    fn complete_window_close(&mut self, outcome: CloseReapOutcome) -> Task<Message> {
        if let Some(handle) = outcome.inference {
            self.inference_stop = Some(handle);
        }
        if let Some(session) = outcome.pty {
            self.pty_session = Some(session);
        }
        if let Err(error) = outcome.result {
            self.close_in_progress = false;
            self.status = format!("종료 전 프로세스 정리 실패: {error}");
            return Task::none();
        }
        if self.finish_window_close(|app| app.save_session(), session::mark_clean_shutdown) {
            iced::window::latest().and_then(iced::window::close)
        } else {
            Task::none()
        }
    }

    #[cfg(test)]
    fn complete_window_close_at(
        &mut self,
        reap_result: Result<(), String>,
        storage_dir: &std::path::Path,
        marker_dir: &std::path::Path,
    ) -> bool {
        if let Err(error) = reap_result {
            self.close_in_progress = false;
            self.status = format!("종료 전 프로세스 정리 실패: {error}");
            return false;
        }
        self.finish_window_close(
            |app| app.save_session_at(storage_dir),
            || session::mark_clean_shutdown_at(marker_dir),
        )
    }

    pub(crate) fn dispatch_session(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::AutoSave => {
                self.save_session();
                Some(Task::none())
            }
            Message::WindowCloseRequested => Some(self.begin_window_close()),
            Message::WindowProcessesReaped(outcome) => {
                Some(self.complete_window_close(outcome.clone()))
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

async fn reap_owned_processes(
    inference: Option<crate::runtime_process::RuntimeStopHandle>,
    pty: Option<crate::pty::PtySession>,
) -> CloseReapOutcome {
    let inference_reap = async {
        match inference {
            Some(handle) => match handle.stop_and_wait().await {
                Ok(()) => (Ok(()), None),
                Err(error) => (Err(error), Some(handle)),
            },
            None => (Ok(()), None),
        }
    };
    let pty_reap = async {
        match pty {
            Some(session) => match session.shutdown().await {
                Ok(_) => (Ok(()), None),
                Err(failure) => (Err(failure.message), Some(failure.session)),
            },
            None => (Ok(()), None),
        }
    };
    let ((inference_result, inference), (pty_result, pty)) =
        tokio::join!(inference_reap, pty_reap);
    let result = match (inference_result, pty_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(inference_error), Ok(())) => Err(inference_error),
        (Ok(()), Err(pty_error)) => Err(pty_error),
        (Err(inference_error), Err(pty_error)) => Err(format!(
            "inference cleanup failed: {inference_error}; PTY cleanup failed: {pty_error}"
        )),
    };
    CloseReapOutcome {
        result,
        inference,
        pty,
    }
}

#[cfg(test)]
#[path = "update/session_dispatch_tests.rs"]
mod session_dispatch_tests;
