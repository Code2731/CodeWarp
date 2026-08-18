// update_inference_lifecycle_control.rs — Inference lifecycle control (main.rs child module)
use super::{App, LlmProvider, Message, keystore};
use iced::Task;

impl App {
    pub(crate) fn set_inference_binary(&mut self, value: &str) -> Task<Message> {
        self.inference_binary_path = value.to_string();
        self.try_persist(
            keystore::write_inference_binary(value),
            "Inference 바이너리 저장",
        );
        Task::none()
    }
    pub(crate) fn set_model_dir(&mut self, value: &str) -> Task<Message> {
        self.model_dir_input = value.to_string();
        self.try_persist(keystore::write_model_dir(value), "모델 디렉토리 저장");
        self.sync_selected_local_model_for_model_dir();
        Task::none()
    }
    pub(crate) fn stop_inference(&mut self) -> Task<Message> {
        if let Some(handle) = &self.inference_stop {
            self.inference_stopping = true;
            let requested = handle.request_stop();
            let process = self.inference_pid.map_or_else(
                || "starting process".to_string(),
                |pid| format!("pid {pid}"),
            );
            self.status = if requested {
                format!("inference 서버 중지 중 ({process})")
            } else {
                format!("inference 서버 종료 요청 실패 ({process})")
            };
        }
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }
    pub(crate) fn on_inference_log_line(&mut self, generation: u64, line: String) -> Task<Message> {
        if generation != self.inference_generation {
            return Task::none();
        }
        if line.starts_with("[pid:")
            && let Some(pid) = line
                .strip_prefix("[pid:")
                .and_then(|r| r.split(']').next())
                .and_then(|s| s.trim().parse::<u32>().ok())
        {
            self.inference_pid = Some(pid);
        }
        if let Some(detail) = line.strip_prefix("[spawn 실패] ") {
            self.status = detail.to_string();
            self.tabby_status = Some(Err(detail.to_string()));
        }
        if let Some(endpoint) = line.strip_prefix("[ready] ") {
            self.status = format!("inference 서버 시작됨: {endpoint}");
        }
        self.push_inference_log(line);
        Task::none()
    }
    pub(crate) fn on_inference_exited(&mut self, generation: u64, code: i32) -> Task<Message> {
        if generation != self.inference_generation {
            return Task::none();
        }
        let last_error = self
            .inference_log
            .iter()
            .rev()
            .find(|line| line.starts_with("[spawn 실패]") || line.starts_with("[err]"))
            .cloned();
        self.push_inference_log(format!("[exited] code {code}"));
        self.inference_pid = None;
        self.inference_stop = None;
        self.inference_stopping = false;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        self.status = format!("inference 서버 종료 (exit {code})");
        self.tabby_status = Some(Err("inference 서버 종료됨".into()));
        let status = if code == -1 {
            last_error
                .and_then(|line| line.strip_prefix("[spawn 실패] ").map(str::to_string))
                .unwrap_or_else(|| "inference 서버 시작 실패".into())
        } else if code == 0 {
            format!("inference 서버 종료 (exit {code})")
        } else if let Some(line) = last_error {
            format!("inference 서버 종료 (exit {code}) — {line}")
        } else {
            format!("inference 서버 종료 (exit {code})")
        };
        self.status.clone_from(&status);
        self.tabby_status = Some(Err(status));
        self.model_options
            .retain(|o| o.provider != LlmProvider::OpenAICompat);
        self.refresh_model_combo();
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_health_line_reports_runtime_started() {
        // Given: an application whose managed child is still starting.
        let (mut app, _startup) = App::new();
        app.status = "starting".to_string();

        // When: the lifecycle reports its successful endpoint health response.
        let _task = app.on_inference_log_line(
            app.inference_generation,
            "[ready] http://127.0.0.1:9000/v1/models".to_string(),
        );

        // Then: user-visible state reports startup only after that response.
        assert_eq!(
            app.status,
            "inference 서버 시작됨: http://127.0.0.1:9000/v1/models"
        );
    }

    #[test]
    fn unexpected_exit_clears_stale_runtime_state() {
        // Given: application state that identifies an owned running child.
        let (mut app, _startup) = App::new();
        app.inference_pid = Some(42);

        // When: the process owner reports an unexpected nonzero exit.
        let _task = app.on_inference_exited(app.inference_generation, 17);

        // Then: stale child identity is cleared and the failure remains visible.
        assert!(app.inference_pid.is_none());
        assert!(app.status.contains("exit 17"));
        assert!(app.tabby_status.as_ref().is_some_and(Result::is_err));
    }

    #[test]
    fn stale_runtime_events_do_not_mutate_current_runtime_state() {
        // Given: a newer managed runtime is the current owner of lifecycle state.
        let (mut app, _startup) = App::new();
        app.inference_generation = 2;
        app.inference_pid = Some(42);
        app.status = "current runtime".into();

        // When: the previous runtime delivers late log and exit events.
        let _ = app.on_inference_log_line(1, "[ready] stale".into());
        let _ = app.on_inference_exited(1, 17);

        // Then: neither event can clear or overwrite the current runtime state.
        assert_eq!(app.inference_pid, Some(42));
        assert_eq!(app.status, "current runtime");
    }
}
