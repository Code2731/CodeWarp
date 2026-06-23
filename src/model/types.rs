// model/types.rs — Core model types (model child module)
use crate::util::fmt_context_length;

/// 모델을 어느 백엔드로 라우팅할지. `OpenAICompat은` 사용자 임의 endpoint
/// (xLLM / vLLM / Tabby / llama-server / Ollama 등 — 모두 `OpenAI` 호환).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LlmProvider {
    OpenRouter,
    OpenAICompat,
}

/// `combo_box에` 표시할 모델 항목 (가격 정보 포함).
/// Display 형식: "[OR][KO]★ model-id  128k  $in/$out" 또는 "[xLLM] model-id  free"
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelOption {
    pub(crate) id: String,
    pub(crate) provider: LlmProvider,
    /// `OpenAICompat의` 사용자 지정 라벨 (xLLM/TabbyML/TabbyAPI/Local 등). 빈 값이면 "Local".
    /// `OpenRouter일` 땐 무의미 (Display에서 사용 안 함).
    pub(crate) provider_label: String,
    /// 한국어 토크나이저 친화 모델 휴리스틱 결과
    pub(crate) ko_friendly: bool,
    /// 즐겨찾기 여부 (`refresh_model_combo에서` self.favorites 기준으로 set)
    pub(crate) favorite: bool,
    /// context window 토큰 수 (있을 때만 표시)
    pub(crate) context_length: Option<u64>,
    /// 입력 100만 토큰당 USD
    pub(crate) prompt_per_million: Option<f64>,
    /// 출력 100만 토큰당 USD
    pub(crate) completion_per_million: Option<f64>,
}

impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag: String = match self.provider {
            LlmProvider::OpenRouter => "[OR]".into(),
            LlmProvider::OpenAICompat => {
                let label = self.provider_label.trim();
                if label.is_empty() {
                    "[Local]".into()
                } else {
                    format!("[{label}]")
                }
            }
        };
        let ko = if self.ko_friendly { "[KO]" } else { "" };
        let star = if self.favorite { "★" } else { "" };
        let ctx = self
            .context_length
            .map(|n| format!("  {}", fmt_context_length(n)))
            .unwrap_or_default();
        match (self.prompt_per_million, self.completion_per_million) {
            (Some(p), Some(c)) if p == 0.0 && c == 0.0 => {
                write!(f, "{}{}{} {}{}  free", tag, ko, star, self.id, ctx)
            }
            (Some(p), Some(c)) => {
                write!(
                    f,
                    "{}{}{} {}{}  ${:.2}/${:.2}",
                    tag, ko, star, self.id, ctx, p, c
                )
            }
            _ => write!(f, "{}{}{} {}{}", tag, ko, star, self.id, ctx),
        }
    }
}

// ── Model category ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelCategory {
    Coding,
    Reasoning,
    General,
}

// ── Inference engine ────────────────────────────────────────────────

pub(crate) const TABBY_API_DEFAULT_PORT: u16 = 5000;
pub(crate) const TABBY_API_REPO_URL: &str = "https://github.com/theroyallab/tabbyAPI.git";

/// inference 엔진 종류 — 사용자가 dropdown으로 선택.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InferenceEngine {
    XLlm,
    VLlm,
    LlamaServer,
    TabbyMl,
    TabbyApi,
    /// daemon 형태 — 이미 떠있다고 가정, `CodeWarp는` spawn 안 함.
    Ollama,
    /// 사용자가 직접 명령 입력
    Custom,
}

impl InferenceEngine {
    pub(crate) const ALL: &'static [InferenceEngine] = &[
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::TabbyMl,
        InferenceEngine::TabbyApi,
        InferenceEngine::Ollama,
        InferenceEngine::Custom,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::XLlm => "xLLM",
            Self::VLlm => "vLLM",
            Self::LlamaServer => "llama-server",
            Self::TabbyMl => "TabbyML",
            Self::TabbyApi => "TabbyAPI (EXL2)",
            Self::Ollama => "Ollama (이미 떠있는 daemon)",
            Self::Custom => "Custom (직접 명령)",
        }
    }

    pub(crate) fn default_port(self) -> u16 {
        match self {
            Self::TabbyMl => 8080,
            Self::TabbyApi => TABBY_API_DEFAULT_PORT,
            Self::Ollama => 11434,
            _ => 9000,
        }
    }

    pub(crate) fn shares_model_namespace(self, other: InferenceEngine) -> bool {
        matches!(
            (self, other),
            (
                Self::XLlm | Self::VLlm | Self::LlamaServer,
                Self::XLlm | Self::VLlm | Self::LlamaServer
            ) | (Self::TabbyMl, Self::TabbyMl)
                | (Self::TabbyApi, Self::TabbyApi)
                | (Self::Ollama, Self::Ollama)
                | (Self::Custom, Self::Custom)
        )
    }

    /// 모델 path/ID + port를 받아 spawn할 Command 인자 리스트 반환.
    /// `None`이면 spawn 안 함 (Ollama는 외부 daemon, Custom은 사용자 정의).
    pub(crate) fn compose_command(self, model: &str, port: u16) -> Option<Vec<String>> {
        let port_s = port.to_string();
        match self {
            Self::XLlm => Some(vec![
                "xllm".into(),
                "serve".into(),
                "--model".into(),
                model.into(),
                "--port".into(),
                port_s,
            ]),
            Self::VLlm => Some(vec![
                "vllm".into(),
                "serve".into(),
                model.into(),
                "--port".into(),
                port_s,
            ]),
            Self::LlamaServer => Some(vec![
                "llama-server".into(),
                "-m".into(),
                model.into(),
                "--port".into(),
                port_s,
            ]),
            Self::TabbyMl => Some(vec![
                "tabby".into(),
                "serve".into(),
                "--model".into(),
                model.into(),
                "--chat-model".into(),
                model.into(),
            ]),
            Self::TabbyApi => {
                #[cfg(windows)]
                {
                    Some(vec![
                        "Start.bat".into(),
                        "--config".into(),
                        "config.yml".into(),
                    ])
                }
                #[cfg(not(windows))]
                {
                    Some(vec![
                        "./start.sh".into(),
                        "--config".into(),
                        "config.yml".into(),
                    ])
                }
            }
            Self::Ollama | Self::Custom => None,
        }
    }
}

impl std::fmt::Display for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────

    fn opt(
        provider: LlmProvider,
        label: &str,
        ko: bool,
        fav: bool,
        ctx: Option<u64>,
        prompt: Option<f64>,
        comp: Option<f64>,
    ) -> ModelOption {
        ModelOption {
            id: "test-model".into(),
            provider,
            provider_label: label.into(),
            ko_friendly: ko,
            favorite: fav,
            context_length: ctx,
            prompt_per_million: prompt,
            completion_per_million: comp,
        }
    }

    // ── ModelOption::fmt() ──────────────────────────────────────────

    #[test]
    fn display_openrouter_tag() {
        let m = opt(LlmProvider::OpenRouter, "", false, false, None, None, None);
        assert!(m.to_string().starts_with("[OR]"), "got: {}", m);
    }

    #[test]
    fn display_openai_compat_empty_label() {
        let m = opt(
            LlmProvider::OpenAICompat,
            "",
            false,
            false,
            None,
            None,
            None,
        );
        assert!(m.to_string().starts_with("[Local]"), "got: {}", m);
    }

    #[test]
    fn display_openai_compat_custom_label() {
        for label in &["xLLM", "vLLM", "TabbyML", "TabbyAPI"] {
            let m = opt(
                LlmProvider::OpenAICompat,
                label,
                false,
                false,
                None,
                None,
                None,
            );
            assert!(
                m.to_string().starts_with(&format!("[{label}]")),
                "got: {}",
                m
            );
        }
    }

    #[test]
    fn display_ko_friendly() {
        let m = opt(LlmProvider::OpenRouter, "", true, false, None, None, None);
        assert!(m.to_string().contains("[KO]"), "got: {}", m);
    }

    #[test]
    fn display_ko_not_friendly() {
        let m = opt(LlmProvider::OpenRouter, "", false, false, None, None, None);
        assert!(!m.to_string().contains("[KO]"), "got: {}", m);
    }

    #[test]
    fn display_favorite() {
        let m = opt(LlmProvider::OpenRouter, "", false, true, None, None, None);
        assert!(m.to_string().contains('★'), "got: {}", m);
    }

    #[test]
    fn display_not_favorite() {
        let m = opt(LlmProvider::OpenRouter, "", false, false, None, None, None);
        assert!(!m.to_string().contains('★'), "got: {}", m);
    }

    #[test]
    fn display_context_length() {
        let cases: &[(u64, &str)] = &[(8000, "8k"), (128_000, "128k"), (1_000_000, "1.0M")];
        for (n, expected) in cases {
            let m = opt(
                LlmProvider::OpenRouter,
                "",
                false,
                false,
                Some(*n),
                None,
                None,
            );
            assert!(
                m.to_string().contains(expected),
                "n={n} expected={expected} got: {}",
                m
            );
        }
    }

    #[test]
    fn display_no_context_length() {
        let m = opt(LlmProvider::OpenRouter, "", false, false, None, None, None);
        let s = m.to_string();
        assert!(!s.contains('k') && !s.contains('M'), "got: {s}");
    }

    #[test]
    fn display_free_model() {
        let m = opt(
            LlmProvider::OpenRouter,
            "",
            false,
            false,
            None,
            Some(0.0),
            Some(0.0),
        );
        assert!(m.to_string().contains("free"), "got: {}", m);
    }

    #[test]
    fn display_paid_model() {
        let m = opt(
            LlmProvider::OpenRouter,
            "",
            false,
            false,
            None,
            Some(0.15),
            Some(0.60),
        );
        assert!(m.to_string().contains("$0.15/$0.60"), "got: {}", m);
    }

    #[test]
    fn display_paid_rounding() {
        let m = opt(
            LlmProvider::OpenRouter,
            "",
            false,
            false,
            None,
            Some(1.5),
            Some(2.0),
        );
        assert!(m.to_string().contains("$1.50/$2.00"), "got: {}", m);
    }

    #[test]
    fn display_no_pricing() {
        let m = opt(LlmProvider::OpenRouter, "", false, false, None, None, None);
        let s = m.to_string();
        assert!(!s.contains("free") && !s.contains('$'), "got: {s}");
    }

    #[test]
    fn display_combined_flags() {
        let m = ModelOption {
            id: "gpt-4".into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: "xLLM".into(),
            ko_friendly: true,
            favorite: true,
            context_length: Some(128_000),
            prompt_per_million: Some(10.0),
            completion_per_million: Some(30.0),
        };
        let s = m.to_string();
        assert!(s.starts_with("[xLLM][KO]★"), "got: {s}");
        assert!(s.contains("gpt-4"), "got: {s}");
        assert!(s.contains("128k"), "got: {s}");
        assert!(s.contains("$10.00/$30.00"), "got: {s}");
    }

    // ── InferenceEngine::label() ────────────────────────────────────

    #[test]
    fn label_xllm() {
        assert_eq!(InferenceEngine::XLlm.label(), "xLLM");
    }

    #[test]
    fn label_vllm() {
        assert_eq!(InferenceEngine::VLlm.label(), "vLLM");
    }

    #[test]
    fn label_llama_server() {
        assert_eq!(InferenceEngine::LlamaServer.label(), "llama-server");
    }

    #[test]
    fn label_tabby_ml() {
        assert_eq!(InferenceEngine::TabbyMl.label(), "TabbyML");
    }

    #[test]
    fn label_tabby_api() {
        assert_eq!(InferenceEngine::TabbyApi.label(), "TabbyAPI (EXL2)");
    }

    #[test]
    fn label_ollama() {
        assert_eq!(
            InferenceEngine::Ollama.label(),
            "Ollama (이미 떠있는 daemon)"
        );
    }

    #[test]
    fn label_custom() {
        assert_eq!(InferenceEngine::Custom.label(), "Custom (직접 명령)");
    }

    // ── InferenceEngine::default_port() ──────────────────────────────

    #[test]
    fn default_port_tabby_ml() {
        assert_eq!(InferenceEngine::TabbyMl.default_port(), 8080);
    }

    #[test]
    fn default_port_tabby_api() {
        assert_eq!(InferenceEngine::TabbyApi.default_port(), 5000);
    }

    #[test]
    fn default_port_ollama() {
        assert_eq!(InferenceEngine::Ollama.default_port(), 11434);
    }

    #[test]
    fn default_port_fallback() {
        for eng in &[
            InferenceEngine::XLlm,
            InferenceEngine::VLlm,
            InferenceEngine::LlamaServer,
            InferenceEngine::Custom,
        ] {
            assert_eq!(eng.default_port(), 9000, "engine: {eng:?}");
        }
    }

    // ── InferenceEngine::shares_model_namespace() ────────────────────

    #[test]
    fn namespace_same_variant() {
        for eng in InferenceEngine::ALL {
            assert!(eng.shares_model_namespace(*eng), "engine: {eng:?}");
        }
    }

    #[test]
    fn namespace_xllm_vllm_llama_server_cross() {
        let group = [
            InferenceEngine::XLlm,
            InferenceEngine::VLlm,
            InferenceEngine::LlamaServer,
        ];
        for a in &group {
            for b in &group {
                assert!(
                    a.shares_model_namespace(*b),
                    "{a:?} should share with {b:?}"
                );
            }
        }
    }

    #[test]
    fn namespace_tabby_ml_only_self() {
        let others = [
            InferenceEngine::XLlm,
            InferenceEngine::VLlm,
            InferenceEngine::LlamaServer,
            InferenceEngine::TabbyApi,
            InferenceEngine::Ollama,
            InferenceEngine::Custom,
        ];
        for o in &others {
            assert!(
                !InferenceEngine::TabbyMl.shares_model_namespace(*o),
                "TabbyMl should not share with {o:?}"
            );
            assert!(
                !o.shares_model_namespace(InferenceEngine::TabbyMl),
                "{o:?} should not share with TabbyMl"
            );
        }
    }

    #[test]
    fn namespace_tabby_api_only_self() {
        let others = [
            InferenceEngine::XLlm,
            InferenceEngine::VLlm,
            InferenceEngine::LlamaServer,
            InferenceEngine::TabbyMl,
            InferenceEngine::Ollama,
            InferenceEngine::Custom,
        ];
        for o in &others {
            assert!(
                !InferenceEngine::TabbyApi.shares_model_namespace(*o),
                "TabbyApi should not share with {o:?}"
            );
            assert!(
                !o.shares_model_namespace(InferenceEngine::TabbyApi),
                "{o:?} should not share with TabbyApi"
            );
        }
    }

    #[test]
    fn namespace_ollama_only_self() {
        let others = [
            InferenceEngine::XLlm,
            InferenceEngine::VLlm,
            InferenceEngine::LlamaServer,
            InferenceEngine::TabbyMl,
            InferenceEngine::TabbyApi,
            InferenceEngine::Custom,
        ];
        for o in &others {
            assert!(
                !InferenceEngine::Ollama.shares_model_namespace(*o),
                "Ollama should not share with {o:?}"
            );
            assert!(
                !o.shares_model_namespace(InferenceEngine::Ollama),
                "{o:?} should not share with Ollama"
            );
        }
    }

    #[test]
    fn namespace_custom_only_self() {
        let others = [
            InferenceEngine::XLlm,
            InferenceEngine::VLlm,
            InferenceEngine::LlamaServer,
            InferenceEngine::TabbyMl,
            InferenceEngine::TabbyApi,
            InferenceEngine::Ollama,
        ];
        for o in &others {
            assert!(
                !InferenceEngine::Custom.shares_model_namespace(*o),
                "Custom should not share with {o:?}"
            );
            assert!(
                !o.shares_model_namespace(InferenceEngine::Custom),
                "{o:?} should not share with Custom"
            );
        }
    }

    // ── InferenceEngine::compose_command() ──────────────────────────

    const TEST_MODEL: &str = "mistral-7b";
    const TEST_PORT: u16 = 4321;

    #[test]
    fn compose_xllm() {
        let cmd = InferenceEngine::XLlm.compose_command(TEST_MODEL, TEST_PORT);
        assert_eq!(
            cmd,
            Some(vec![
                "xllm".into(),
                "serve".into(),
                "--model".into(),
                TEST_MODEL.into(),
                "--port".into(),
                "4321".into()
            ])
        );
    }

    #[test]
    fn compose_vllm() {
        let cmd = InferenceEngine::VLlm.compose_command(TEST_MODEL, TEST_PORT);
        assert_eq!(
            cmd,
            Some(vec![
                "vllm".into(),
                "serve".into(),
                TEST_MODEL.into(),
                "--port".into(),
                "4321".into()
            ])
        );
    }

    #[test]
    fn compose_llama_server() {
        let cmd = InferenceEngine::LlamaServer.compose_command(TEST_MODEL, TEST_PORT);
        assert_eq!(
            cmd,
            Some(vec![
                "llama-server".into(),
                "-m".into(),
                TEST_MODEL.into(),
                "--port".into(),
                "4321".into()
            ])
        );
    }

    #[test]
    fn compose_tabby_ml() {
        let cmd = InferenceEngine::TabbyMl.compose_command(TEST_MODEL, TEST_PORT);
        assert_eq!(
            cmd,
            Some(vec![
                "tabby".into(),
                "serve".into(),
                "--model".into(),
                TEST_MODEL.into(),
                "--chat-model".into(),
                TEST_MODEL.into(),
            ])
        );
    }

    #[test]
    fn compose_tabby_api() {
        let cmd = InferenceEngine::TabbyApi.compose_command(TEST_MODEL, TEST_PORT);
        assert_eq!(
            cmd,
            Some(vec![
                "Start.bat".into(),
                "--config".into(),
                "config.yml".into()
            ])
        );
    }

    #[test]
    fn compose_ollama() {
        assert_eq!(
            InferenceEngine::Ollama.compose_command(TEST_MODEL, TEST_PORT),
            None
        );
    }

    #[test]
    fn compose_custom() {
        assert_eq!(
            InferenceEngine::Custom.compose_command(TEST_MODEL, TEST_PORT),
            None
        );
    }
}
