// update_settings_interact_pty.rs — PTY interaction methods
use super::{App, Message, PTY_MAX_LINES, pty};
use iced::Task;

impl App {
    pub(crate) fn toggle_pty(&mut self) -> Task<Message> {
        self.ui.pty_visible = !self.ui.pty_visible;
        if self.ui.pty_visible && self.pty_session.is_none() {
            return Task::done(Message::PtyStart);
        }
        if !self.ui.pty_visible {
            return self.stop_pty(false, true);
        }
        Task::none()
    }
    pub(crate) fn send_pty_input(&mut self) -> Task<Message> {
        let line = self.pty_input.trim_end().to_string();
        if let Some(s) = &self.pty_session {
            s.write_line(&line);
        }
        self.pty_input.clear();
        Task::none()
    }
    pub(crate) fn push_pty_line(&mut self, line: String) {
        self.pty_output.push_back(line);
        if self.pty_output.len() > PTY_MAX_LINES {
            self.pty_output.pop_front();
        }
    }
    pub(crate) fn pty_start(&mut self) -> Task<Message> {
        if self.pty_session.is_some() {
            return self.stop_pty(true, true);
        }
        match pty::spawn_pty(&self.cwd) {
            Ok((session, stream)) => {
                let pid = session.pid();
                self.pty_generation = self.pty_generation.saturating_add(1);
                let generation = self.pty_generation;
                self.pty_session = Some(session);
                self.pty_output.clear();
                self.status = pid.map_or_else(
                    || "터미널 시작됨".into(),
                    |pid| format!("터미널 시작됨 (pid {pid})"),
                );
                Task::run(stream, move |event| match event {
                    pty::PtyEvent::Line(line) => Message::PtyLine { generation, line },
                    pty::PtyEvent::Exited => Message::PtyExited { generation },
                })
            }
            Err(e) => {
                self.status = format!("터미널 시작 실패: {e}");
                Task::none()
            }
        }
    }
    pub(crate) fn on_pty_line(&mut self, generation: u64, line: &str) -> Task<Message> {
        if generation != self.pty_generation {
            return Task::none();
        }
        let clean = pty::strip_ansi(line);
        if !clean.trim().is_empty() {
            self.push_pty_line(clean);
        }
        Task::none()
    }
    pub(crate) fn on_pty_exited(&mut self, generation: u64) -> Task<Message> {
        if generation != self.pty_generation {
            return Task::none();
        }
        self.stop_pty(false, false)
    }

    pub(crate) fn stop_pty(&mut self, restart: bool, graceful: bool) -> Task<Message> {
        let Some(session) = self.pty_session.take() else {
            return if restart {
                Task::done(Message::PtyStart)
            } else {
                Task::none()
            };
        };
        let generation = self.pty_generation;
        Task::perform(
            async move {
                if graceful {
                    session.shutdown().await
                } else {
                    session.wait_for_exit().await
                }
            },
            move |result| Message::PtyStopped(result, restart, generation),
        )
    }

    pub(crate) fn on_pty_stopped(
        &mut self,
        result: Result<pty::PtyReceipt, pty::PtyShutdownFailure>,
        restart: bool,
        generation: u64,
    ) -> Task<Message> {
        if generation != self.pty_generation {
            return Task::none();
        }
        match result {
            Ok(receipt) => {
                self.push_pty_line(format!(
                    "-- 셸 종료 pid={} forced={} exit={} elapsed={:.3}s --",
                    receipt.pid,
                    receipt.forced,
                    receipt.status.exit_code(),
                    receipt.elapsed.as_secs_f64()
                ));
                self.status = format!("터미널 종료됨 (pid {})", receipt.pid);
            }
            Err(failure) => {
                self.status = format!("터미널 종료 실패: {}", failure.message);
                self.pty_session = Some(failure.session);
                return Task::none();
            }
        }
        if restart {
            Task::done(Message::PtyStart)
        } else {
            Task::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_pty_events_do_not_mutate_a_restarted_session() {
        let (mut app, _) = App::new();
        app.pty_generation = 2;

        let _ = app.on_pty_line(1, "stale output");
        let _ = app.on_pty_exited(1);

        assert!(app.pty_output.is_empty());
        assert!(app.pty_session.is_none());
    }
}
