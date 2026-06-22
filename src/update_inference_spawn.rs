// update_inference_spawn.rs — Runtime spawn resolution (main.rs child module)
use super::{
    is_loopback_url, is_tabbyapi_launcher_path, list_downloaded_models, resolve_binary_from_dir,
    tabby, tabby_connection_error_looks_unreachable, App, InferenceEngine, PathBuf,
};

impl App {
    pub(crate) fn resolve_runtime_spawn_command(
        &self,
        program: String,
        args: Vec<String>,
    ) -> (String, Vec<String>, Option<PathBuf>) {
        let override_path = self.inference_binary_path.trim();
        if !matches!(self.inference_engine, InferenceEngine::TabbyApi) {
            let final_program =
                if override_path.is_empty() || is_tabbyapi_launcher_path(override_path) {
                    program
                } else if std::path::Path::new(override_path).is_dir() {
                    resolve_binary_from_dir(std::path::Path::new(override_path), &program)
                        .map_or_else(|| override_path.to_string(), |p| p.display().to_string())
                } else {
                    override_path.to_string()
                };
            return (final_program, args, None);
        }

        let script = if override_path.is_empty() {
            program
        } else {
            override_path.to_string()
        };
        let script_path = std::path::Path::new(&script);
        let work_dir = script_path.parent().map(std::path::Path::to_path_buf);
        let file_name = script_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&script)
            .to_string();
        let ext = script_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        #[cfg(windows)]
        {
            if ext == "bat" || ext == "cmd" {
                let mut final_args = vec!["/C".into(), file_name];
                final_args.extend(args);
                return ("cmd.exe".into(), final_args, work_dir);
            }
            if ext == "py" {
                let mut final_args = vec![file_name];
                final_args.extend(args);
                return ("python".into(), final_args, work_dir);
            }
        }

        #[cfg(not(windows))]
        {
            if ext == "sh" {
                return (format!("./{file_name}"), args, work_dir);
            }
            if ext == "py" {
                let mut final_args = vec![file_name];
                final_args.extend(args);
                return ("python3".into(), final_args, work_dir);
            }
        }

        (script, args, work_dir)
    }
    pub(crate) fn compose_tabby_connection_error(&self, raw: &str) -> String {
        let actionable = tabby::humanize_error(raw);
        if self.inference_pid.is_some()
            || !is_loopback_url(&self.tabby_url_input)
            || !tabby_connection_error_looks_unreachable(raw, &actionable)
        {
            return actionable;
        }

        if matches!(self.inference_engine, InferenceEngine::TabbyApi) {
            if self.inference_binary_path.trim().is_empty() {
                return "TabbyAPI 서버가 아직 실행되지 않았습니다. Runtime 탭에서 TabbyAPI script에 Start.bat/start.sh/main.py 경로를 지정하고 시작한 뒤 연결 테스트해 주세요."
                    .into();
            }
            if let Ok(port) = self.inference_port_input.trim().parse::<u16>() {
                let expected_base = format!("http://localhost:{port}");
                let normalized_current = self
                    .tabby_url_input
                    .trim()
                    .trim_end_matches('/')
                    .trim_end_matches("/v1")
                    .trim_end_matches('/')
                    .to_string();
                if !normalized_current.is_empty()
                    && !normalized_current.eq_ignore_ascii_case(&expected_base)
                {
                    return format!(
                        "Provider URL과 Runtime 포트가 다릅니다. Runtime 포트가 {port} 이므로 Provider URL을 {expected_base} 로 맞춘 뒤 연결 테스트해 주세요."
                    );
                }
            }
            return format!(
                "TabbyAPI 서버가 아직 응답하지 않습니다. Runtime 탭의 시작 상태와 로그를 확인한 뒤 {} 로 연결 테스트해 주세요.",
                self.tabby_url_input.trim()
            );
        }

        if !list_downloaded_models(std::path::Path::new(&self.model_dir_input)).is_empty() {
            return "서버가 아직 실행 중이 아닙니다. Runtime 탭에서 현재 엔진을 시작한 뒤 연결 테스트를 다시 실행해 주세요."
                .into();
        }

        actionable
    }
}
