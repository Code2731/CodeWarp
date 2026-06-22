// update_settings_interact_pty.rs — PTY interaction methods
use super::{pty, App, Message, PTY_MAX_LINES};
use iced::Task;

impl App {
    pub(crate) fn toggle_pty(&mut self) -> Task<Message> {
        self.pty_visible = !self.pty_visible;
        if self.pty_visible && self.pty_session.is_none() {
            return Task::done(Message::PtyStart);
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
        match pty::spawn_pty(&self.cwd) {
            Ok((session, stream)) => {
                self.pty_session = Some(session);
                self.pty_output.clear();
                self.status = "터미널 시작됨".into();
                Task::run(stream, |event| match event {
                    pty::PtyEvent::Line(l) => Message::PtyLine(l),
                    pty::PtyEvent::Exited => Message::PtyExited,
                })
            }
            Err(e) => {
                self.status = format!("터미널 시작 실패: {e}");
                Task::none()
            }
        }
    }
    pub(crate) fn on_pty_line(&mut self, line: &str) -> Task<Message> {
        let clean = pty::strip_ansi(line);
        if !clean.trim().is_empty() {
            self.push_pty_line(clean);
        }
        Task::none()
    }
    pub(crate) fn on_pty_exited(&mut self) -> Task<Message> {
        self.pty_session = None;
        self.push_pty_line("-- 셸 종료 --".into());
        self.status = "터미널 종료됨".into();
        Task::none()
    }
}
