// model/types.rs — Core model types (model child module)
use crate::util::fmt_context_length;

/// 모델을 어느 백엔드로 라우팅할지. OpenAICompat은 사용자 임의 endpoint
/// (xLLM / vLLM / Tabby / llama-server / Ollama 등 — 모두 OpenAI 호환).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LlmProvider {
    OpenRouter,
    OpenAICompat,
}

/// combo_box에 표시할 모델 항목 (가격 정보 포함).
/// Display 형식: "[OR][KO]★ model-id  128k  $in/$out" 또는 "[xLLM] model-id  free"
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelOption {
    pub(crate) id: String,
    pub(crate) provider: LlmProvider,
    /// OpenAICompat의 사용자 지정 라벨 (xLLM/TabbyML/TabbyAPI/Local 등). 빈 값이면 "Local".
    /// OpenRouter일 땐 무의미 (Display에서 사용 안 함).
    pub(crate) provider_label: String,
    /// 한국어 토크나이저 친화 모델 휴리스틱 결과
    pub(crate) ko_friendly: bool,
    /// 즐겨찾기 여부 (refresh_model_combo에서 self.favorites 기준으로 set)
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
                    format!("[{}]", label)
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
    /// daemon 형태 — 이미 떠있다고 가정, CodeWarp는 spawn 안 함.
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

    pub(crate) fn label(&self) -> &'static str {
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

    pub(crate) fn default_port(&self) -> u16 {
        match self {
            Self::TabbyMl => 8080,
            Self::TabbyApi => TABBY_API_DEFAULT_PORT,
            Self::Ollama => 11434,
            _ => 9000,
        }
    }

    pub(crate) fn shares_model_namespace(&self, other: InferenceEngine) -> bool {
        matches!(
            (self, other),
            (Self::XLlm, Self::XLlm | Self::VLlm | Self::LlamaServer)
                | (Self::VLlm, Self::XLlm | Self::VLlm | Self::LlamaServer)
                | (
                    Self::LlamaServer,
                    Self::XLlm | Self::VLlm | Self::LlamaServer
                )
                | (Self::TabbyMl, Self::TabbyMl)
                | (Self::TabbyApi, Self::TabbyApi)
                | (Self::Ollama, Self::Ollama)
                | (Self::Custom, Self::Custom)
        )
    }

    /// 모델 path/ID + port를 받아 spawn할 Command 인자 리스트 반환.
    /// `None`이면 spawn 안 함 (Ollama는 외부 daemon, Custom은 사용자 정의).
    pub(crate) fn compose_command(&self, model: &str, port: u16) -> Option<Vec<String>> {
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
