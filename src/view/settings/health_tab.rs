use crate::{App, list_downloaded_models};

#[derive(Clone, Copy)]
pub(crate) enum TabHealth {
    Good,
    Warn,
    Bad,
}

impl App {
    pub(crate) fn compute_provider_health(&self) -> TabHealth {
        match &self.tabby_status {
            Some(Err(_)) => TabHealth::Bad,
            _ if self.has_key || !self.tabby_url_input.trim().is_empty() => TabHealth::Good,
            _ => TabHealth::Warn,
        }
    }

    pub(crate) fn compute_runtime_health(&self) -> TabHealth {
        if self.inference_pid.is_some() {
            TabHealth::Good
        } else {
            TabHealth::Warn
        }
    }

    pub(crate) fn compute_model_health(&self) -> TabHealth {
        let count = list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len();
        if count > 0 {
            TabHealth::Good
        } else {
            TabHealth::Warn
        }
    }

    pub(crate) fn compute_mcp_health(&self) -> TabHealth {
        if self.mcp_servers.is_empty() || self.mcp_tools.is_empty() {
            TabHealth::Warn
        } else {
            TabHealth::Good
        }
    }
}
