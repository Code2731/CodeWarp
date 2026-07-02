// update_inference_lifecycle_control.rs — Inference lifecycle control (main.rs child module)
use super::{App, LlmProvider, Message, keystore, kill_pid};
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
        if let Some(pid) = self.inference_pid.take() {
            kill_pid(pid);
            self.status = format!("inference 서버 중지 (pid {pid})");
            self.push_inference_log(format!("[stopped] pid {pid}"));
        }
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }
    pub(crate) fn on_inference_log_line(&mut self, line: String) -> Task<Message> {
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
        self.push_inference_log(line);
        Task::none()
    }
    pub(crate) fn on_inference_exited(&mut self, code: i32) -> Task<Message> {
        let last_error = self
            .inference_log
            .iter()
            .rev()
            .find(|line| line.starts_with("[spawn 실패]") || line.starts_with("[err]"))
            .cloned();
        self.push_inference_log(format!("[exited] code {code}"));
        self.inference_pid = None;
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
