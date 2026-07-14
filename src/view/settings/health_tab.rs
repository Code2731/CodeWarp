use crate::{App, list_downloaded_models};

#[derive(Clone, Copy)]
pub(crate) enum TabHealth {
    Good,
    Warn,
    Bad,
}

impl App {
    pub(crate) fn compute_provider_health(&self) -> TabHealth {
        if self.tabby_url_input.trim().is_empty() {
            if self.has_key {
                TabHealth::Good
            } else {
                TabHealth::Warn
            }
        } else {
            match &self.tabby_status {
                None => TabHealth::Warn,
                Some(Ok(_)) => TabHealth::Good,
                Some(Err(_)) => TabHealth::Bad,
            }
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

#[cfg(test)]
mod tests {
    use super::TabHealth;
    use crate::App;

    #[test]
    fn provider_health_uses_openrouter_key_when_endpoint_is_blank() {
        for tabby_status in [
            None,
            Some(Ok("verified".to_owned())),
            Some(Err("unreachable".to_owned())),
        ] {
            let (mut app, _) = App::new();
            app.has_key = true;
            app.tabby_url_input = " \n\t ".to_owned();
            app.tabby_status = tabby_status;

            assert!(matches!(app.compute_provider_health(), TabHealth::Good));
        }
    }

    #[test]
    fn provider_health_warns_without_key_when_endpoint_is_blank() {
        for tabby_status in [
            None,
            Some(Ok("verified".to_owned())),
            Some(Err("unreachable".to_owned())),
        ] {
            let (mut app, _) = App::new();
            app.has_key = false;
            app.tabby_url_input = " \n\t ".to_owned();
            app.tabby_status = tabby_status;

            assert!(matches!(app.compute_provider_health(), TabHealth::Warn));
        }
    }

    #[test]
    fn provider_health_warns_when_endpoint_is_unverified() {
        let (mut app, _) = App::new();
        app.tabby_url_input = "http://localhost:8080".to_owned();

        assert!(matches!(app.compute_provider_health(), TabHealth::Warn));
    }

    #[test]
    fn provider_health_is_good_when_endpoint_is_verified() {
        let (mut app, _) = App::new();
        app.tabby_url_input = "http://localhost:8080".to_owned();
        app.tabby_status = Some(Ok("verified".to_owned()));

        assert!(matches!(app.compute_provider_health(), TabHealth::Good));
    }

    #[test]
    fn provider_health_is_bad_when_endpoint_fails_even_with_openrouter_key() {
        let (mut app, _) = App::new();
        app.has_key = true;
        app.tabby_url_input = "http://localhost:8080".to_owned();
        app.tabby_status = Some(Err("unreachable".to_owned()));

        assert!(matches!(app.compute_provider_health(), TabHealth::Bad));
    }
}
