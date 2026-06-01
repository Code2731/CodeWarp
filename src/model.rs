// CodeWarp — 모델 관련 타입, 엔진, 디렉토리 유틸리티
//
// main.rs에서 추출: LlmProvider, ModelOption, InferenceEngine, ModelCategory,
// ModelPreset, Exl2Preset, 모델 디렉토리 탐색 헬퍼.

use std::path::PathBuf;

use crate::util::{fmt_context_length, resolve_user_path};

// ── Provider / Model option ─────────────────────────────────────────

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

/// 모델 ID에 한국어 친화로 알려진 패턴이 들어있는지.
/// 휴리스틱 — 누락/오탐 가능. 화이트리스트 갱신은 여기 한 줄.
pub(crate) fn is_korean_friendly(id: &str) -> bool {
    let s = id.to_lowercase();
    const PATTERNS: &[&str] = &[
        "claude",
        "gpt-4o",
        "gpt-4-turbo",
        "gpt-4.1",
        "gemini-1.5",
        "gemini-2",
        "qwen2.5",
        "qwen-2.5",
        "qwen3",
        "llama-3.1",
        "llama-3.2",
        "llama-3.3",
        "exaone",
        "solar",
        "deepseek-v3",
        "deepseek-r1",
        "deepseek-chat",
        "hyperclova",
        "ax-3",
        "a.x",
        "kullm",
        "ko-llama",
        "42dot",
    ];
    PATTERNS.iter().any(|p| s.contains(p))
}

pub(crate) fn parse_price_per_million(s: Option<&str>) -> Option<f64> {
    let v = s?.parse::<f64>().ok()?;
    Some(v * 1_000_000.0)
}

// ── Model category ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelCategory {
    Coding,
    Reasoning,
    General,
}

/// 모델 ID에서 카테고리를 추정. 키워드 매칭 기반.
/// 코딩/추론 전용 모델만 좁게 매칭하고, 나머지(Claude/GPT-4/Gemini 등)는 범용으로.
pub(crate) fn categorize_model(model_id: &str) -> Vec<ModelCategory> {
    let id = model_id.to_lowercase();
    let coding_keywords = [
        "coder",
        "codex",
        "codestral",
        "codellama",
        "starcoder",
        "codegen",
        "code-",
    ];
    let reasoning_keywords = [
        "o1-",
        "o3-",
        "o4-",
        "/o1",
        "/o3",
        "/o4",
        "thinking",
        "-reasoning",
        "-r1",
        "-qwq",
        "/qwq",
    ];
    let is_coding = coding_keywords.iter().any(|k| id.contains(k));
    let is_reasoning = reasoning_keywords.iter().any(|k| id.contains(k));
    let mut cats = Vec::new();
    if is_coding {
        cats.push(ModelCategory::Coding);
    }
    if is_reasoning {
        cats.push(ModelCategory::Reasoning);
    }
    if !is_coding && !is_reasoning {
        cats.push(ModelCategory::General);
    }
    cats
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
        match (self, other) {
            (Self::XLlm, Self::XLlm | Self::VLlm | Self::LlamaServer)
            | (Self::VLlm, Self::XLlm | Self::VLlm | Self::LlamaServer)
            | (Self::LlamaServer, Self::XLlm | Self::VLlm | Self::LlamaServer)
            | (Self::TabbyMl, Self::TabbyMl)
            | (Self::TabbyApi, Self::TabbyApi)
            | (Self::Ollama, Self::Ollama)
            | (Self::Custom, Self::Custom) => true,
            _ => false,
        }
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

// ── Model directory helpers ─────────────────────────────────────────

/// 모델 매니저 다운로드 폴더 안의 받은 모델(서브폴더) 리스트.
/// 빈 폴더는 모델 아님 — skip.
pub(crate) fn has_model_weight_file(dir: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if has_model_weight_file(&path) {
                return true;
            }
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let file_name = file_name.to_ascii_lowercase();
        if file_name.ends_with(".safetensors")
            || file_name.ends_with(".bin")
            || file_name.ends_with(".gguf")
            || file_name.ends_with(".pt")
            || file_name.ends_with(".pth")
        {
            return true;
        }
    }

    false
}

pub(crate) fn is_valid_tabbyapi_model_dir_direct(path: &std::path::Path) -> bool {
    path.is_dir() && path.join("config.json").is_file() && has_model_weight_file(path)
}

pub(crate) fn tabbyapi_direct_model_children(path: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_valid_tabbyapi_model_dir_direct(p))
        .collect()
}

pub(crate) fn extract_bpw_hint(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for (idx, _) in lower.match_indices("bpw") {
        let mut start = idx;
        while start > 0 {
            let ch = bytes[start - 1];
            if ch.is_ascii_digit() || ch == b'.' {
                start -= 1;
            } else {
                break;
            }
        }
        if start < idx {
            return Some(lower[start..idx + 3].to_string());
        }
    }
    None
}

pub(crate) fn resolve_tabbyapi_model_dir_with_hint(
    path: &std::path::Path,
    hint: Option<&str>,
) -> Option<PathBuf> {
    if is_valid_tabbyapi_model_dir_direct(path) {
        return Some(path.to_path_buf());
    }

    let candidates = tabbyapi_direct_model_children(path);
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return candidates.into_iter().next();
    }

    if let Some(bpw_hint) = hint.and_then(extract_bpw_hint) {
        let mut matched: Vec<PathBuf> = candidates
            .iter()
            .filter_map(|candidate| {
                let name = candidate.file_name().and_then(|n| n.to_str())?;
                if name.to_ascii_lowercase().contains(&bpw_hint) {
                    Some(candidate.clone())
                } else {
                    None
                }
            })
            .collect();
        if matched.len() == 1 {
            return matched.pop();
        }
    }

    None
}

pub(crate) fn resolve_tabbyapi_model_dir(path: &std::path::Path) -> Option<PathBuf> {
    resolve_tabbyapi_model_dir_with_hint(path, None)
}

pub(crate) fn has_tabbyapi_model_dir(path: &std::path::Path) -> bool {
    is_valid_tabbyapi_model_dir_direct(path) || !tabbyapi_direct_model_children(path).is_empty()
}

pub(crate) fn resolve_tabbyapi_model_dir_for_folder(
    path: &std::path::Path,
    folder_name: &str,
) -> Option<PathBuf> {
    resolve_tabbyapi_model_dir_with_hint(path, Some(folder_name))
}

pub(crate) fn is_downloaded_exl2_root(path: &std::path::Path) -> bool {
    has_tabbyapi_model_dir(path)
}

pub(crate) fn is_downloaded_model_dir(path: &std::path::Path) -> bool {
    path.is_dir() && has_model_weight_file(path)
}

pub(crate) fn list_downloaded_models(dir: &std::path::Path) -> Vec<String> {
    let resolved_dir = resolve_user_path(&dir.to_string_lossy());
    if resolved_dir.as_os_str().is_empty() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(&resolved_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !is_downloaded_model_dir(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            out.push(name.to_string());
        }
    }
    out.sort_unstable();
    out
}

pub(crate) fn downloaded_model_path(dir: &str, folder_name: &str) -> PathBuf {
    resolve_user_path(dir).join(folder_name)
}

pub(crate) fn exl2_repo_model_stem(repo_id: &str) -> Option<String> {
    let name = repo_id.rsplit('/').next()?.trim();
    if name.is_empty() {
        return None;
    }
    name.strip_suffix("-exl2")
        .or_else(|| name.strip_suffix("-EXL2"))
        .map(str::to_string)
}

pub(crate) fn downloaded_exl2_preset_folder(dir: &str, preset: &Exl2Preset) -> Option<String> {
    let root = resolve_user_path(dir);
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    let mut models: Vec<String> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && is_downloaded_exl2_root(p))
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect();
    models.sort_unstable();
    if let Some(exact) = models
        .iter()
        .find(|m| m.eq_ignore_ascii_case(preset.folder_name))
    {
        return Some(exact.clone());
    }

    let stem = exl2_repo_model_stem(preset.repo_id)?;
    let stem_prefix = format!("{}-", stem.to_ascii_lowercase());
    let mut matches: Vec<String> = models
        .into_iter()
        .filter(|m| {
            let lower = m.to_ascii_lowercase();
            lower.starts_with(&stem_prefix) && lower.contains("bpw")
        })
        .collect();
    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

// ── Presets ─────────────────────────────────────────────────────────

/// 추천 프리셋 — 클릭 시 hf_repo_input에 채움.
pub(crate) struct ModelPreset {
    pub(crate) repo_id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) note: &'static str,
}

pub(crate) const MODEL_PRESETS: &[ModelPreset] = &[
    ModelPreset {
        repo_id: "Qwen/Qwen2.5-Coder-7B-Instruct",
        label: "Qwen2.5-Coder 7B Instruct",
        note: "코딩 + 한국어 친화 (xLLM/vLLM)",
    },
    ModelPreset {
        repo_id: "Qwen/Qwen2.5-7B-Instruct",
        label: "Qwen2.5 7B Instruct",
        note: "범용 + 한국어 친화 (xLLM/vLLM)",
    },
    ModelPreset {
        repo_id: "LGAI-EXAONE/EXAONE-3.5-7.8B-Instruct",
        label: "EXAONE 3.5 7.8B",
        note: "한국어 특화 (LG AI)",
    },
    ModelPreset {
        repo_id: "upstage/SOLAR-10.7B-Instruct-v1.0",
        label: "SOLAR 10.7B",
        note: "한국어 친화 (Upstage)",
    },
    ModelPreset {
        repo_id: "deepseek-ai/DeepSeek-Coder-V2-Lite-Instruct",
        label: "DeepSeek-Coder V2 Lite",
        note: "코딩 (16B-MoE 활성 2.4B)",
    },
];

/// EXL2 프리셋 — TabbyAPI용. 클릭하면 해당 branch를 바로 다운로드.
pub(crate) struct Exl2Preset {
    pub(crate) repo_id: &'static str,
    pub(crate) revision: &'static str,
    pub(crate) folder_name: &'static str,
    pub(crate) label: &'static str,
    pub(crate) note: &'static str,
    pub(crate) vram: &'static str,
}

pub(crate) const EXL2_PRESETS: &[Exl2Preset] = &[
    Exl2Preset {
        repo_id: "turboderp/Llama-3.2-1B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.2-1B-Instruct-4.0bpw",
        label: "Llama 3.2 1B Instruct",
        note: "검증·테스트용 초소형",
        vram: "~600MB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.2-3B-Instruct-exl2",
        revision: "3.5bpw",
        folder_name: "Llama-3.2-3B-Instruct-3.5bpw",
        label: "Llama 3.2 3B Instruct",
        note: "소형 범용",
        vram: "~1.8GB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.1-8B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.1-8B-Instruct-4.0bpw",
        label: "Llama 3.1 8B Instruct 4bpw",
        note: "RTX 3080 최적 균형",
        vram: "~5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.1-8B-Instruct-exl2",
        revision: "6.0bpw",
        folder_name: "Llama-3.1-8B-Instruct-6.0bpw",
        label: "Llama 3.1 8B Instruct 6bpw",
        note: "품질 우선 (RTX 3080 10GB 내)",
        vram: "~7.5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/gemma-2-9b-it-exl2",
        revision: "4.0bpw",
        folder_name: "Gemma-2-9B-it-4.0bpw",
        label: "Gemma 2 9B Instruct",
        note: "Google 범용 (강력한 instruction following)",
        vram: "~5.5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/gemma-3-12b-it-exl2",
        revision: "4.0bpw",
        folder_name: "Gemma-3-12B-it-4.0bpw",
        label: "Gemma 3 12B Instruct",
        note: "최신 Gemma 3 (멀티모달 지원)",
        vram: "~7GB",
    },
];

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_korean_friendly ──────────────────────────────────────────

    #[test]
    fn ko_friendly_known_models() {
        assert!(is_korean_friendly("openai/gpt-4o"));
        assert!(is_korean_friendly("anthropic/claude-3.5-sonnet"));
        assert!(is_korean_friendly("google/gemini-1.5-pro"));
        assert!(is_korean_friendly("qwen/qwen2.5-coder-7b"));
        assert!(is_korean_friendly("meta-llama/llama-3.1-70b-instruct"));
        assert!(is_korean_friendly("upstage/solar-10.7b"));
        assert!(is_korean_friendly("LGAI-EXAONE/EXAONE-3.5-7.8B"));
        assert!(is_korean_friendly("deepseek/deepseek-v3"));
    }

    #[test]
    fn ko_friendly_negative() {
        assert!(!is_korean_friendly("mistralai/mistral-7b"));
        assert!(!is_korean_friendly("openai/gpt-3.5-turbo"));
        assert!(!is_korean_friendly("starcoder2:7b"));
    }

    // ── categorize_model ────────────────────────────────────────────

    #[test]
    fn categorize_coding_models() {
        let cats = categorize_model("qwen/qwen2.5-coder-7b");
        assert!(cats.contains(&ModelCategory::Coding));
    }

    #[test]
    fn categorize_reasoning_models() {
        let cats = categorize_model("deepseek/deepseek-r1");
        assert!(cats.contains(&ModelCategory::Reasoning));
    }

    #[test]
    fn categorize_general_fallback() {
        let cats = categorize_model("mistralai/mistral-7b-instruct");
        assert!(cats.contains(&ModelCategory::General));
    }

    // ── parse_price_per_million ─────────────────────────────────────

    #[test]
    fn parse_price_per_million_typical() {
        let p = parse_price_per_million(Some("0.000005"));
        assert!(matches!(p, Some(v) if (v - 5.0).abs() < 1e-9));
    }

    #[test]
    fn parse_price_per_million_free() {
        let p = parse_price_per_million(Some("0"));
        assert_eq!(p, Some(0.0));
    }

    #[test]
    fn parse_price_per_million_invalid() {
        assert_eq!(parse_price_per_million(None), None);
        assert_eq!(parse_price_per_million(Some("")), None);
        assert_eq!(parse_price_per_million(Some("abc")), None);
    }

    // ── ModelOption Display + provider_label ────────────────────────

    fn or_opt(id: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenRouter,
            provider_label: String::new(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    fn oai_opt(id: &str, label: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: label.into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    #[test]
    fn display_openrouter_basic() {
        let m = or_opt("gpt-4o");
        let s = format!("{}", m);
        assert!(s.starts_with("[OR]"), "got: {}", s);
        assert!(s.contains("gpt-4o"));
    }

    #[test]
    fn display_openai_compat_with_label() {
        let m = oai_opt("qwen2.5-coder", "xLLM");
        let s = format!("{}", m);
        assert!(s.starts_with("[xLLM]"), "got: {}", s);
        assert!(s.contains("qwen2.5-coder"));
    }

    #[test]
    fn display_openai_compat_empty_label_defaults_to_local() {
        let m = oai_opt("starcoder", "");
        let s = format!("{}", m);
        assert!(s.starts_with("[Local]"), "got: {}", s);
    }

    #[test]
    fn display_openai_compat_whitespace_label_defaults() {
        let m = oai_opt("foo", "   ");
        let s = format!("{}", m);
        assert!(s.starts_with("[Local]"), "got: {}", s);
    }

    #[test]
    fn display_combined_tags() {
        let mut m = or_opt("claude-3.5-sonnet");
        m.ko_friendly = true;
        m.favorite = true;
        m.context_length = Some(200_000);
        m.prompt_per_million = Some(3.0);
        m.completion_per_million = Some(15.0);
        let s = format!("{}", m);
        assert!(s.contains("[OR]"));
        assert!(s.contains("[KO]"));
        assert!(s.contains("★"));
        assert!(s.contains("200k"));
        assert!(s.contains("$3.00/$15.00"));
    }

    #[test]
    fn display_openai_compat_free_marker() {
        let mut m = oai_opt("local-model", "xLLM");
        m.prompt_per_million = Some(0.0);
        m.completion_per_million = Some(0.0);
        let s = format!("{}", m);
        assert!(s.contains("[xLLM]"));
        assert!(s.contains("free"));
    }

    // ── InferenceEngine ─────────────────────────────────────────────

    #[test]
    fn engine_default_ports() {
        assert_eq!(InferenceEngine::TabbyMl.default_port(), 8080);
        assert_eq!(
            InferenceEngine::TabbyApi.default_port(),
            TABBY_API_DEFAULT_PORT
        );
        assert_eq!(InferenceEngine::Ollama.default_port(), 11434);
        assert_eq!(InferenceEngine::XLlm.default_port(), 9000);
        assert_eq!(InferenceEngine::VLlm.default_port(), 9000);
        assert_eq!(InferenceEngine::LlamaServer.default_port(), 9000);
    }

    #[test]
    fn engine_model_namespace_rules() {
        assert!(InferenceEngine::XLlm.shares_model_namespace(InferenceEngine::VLlm));
        assert!(InferenceEngine::VLlm.shares_model_namespace(InferenceEngine::LlamaServer));
        assert!(!InferenceEngine::TabbyMl.shares_model_namespace(InferenceEngine::XLlm));
        assert!(!InferenceEngine::Custom.shares_model_namespace(InferenceEngine::TabbyMl));
        assert!(!InferenceEngine::TabbyMl.shares_model_namespace(InferenceEngine::TabbyApi));
    }

    #[test]
    fn engine_compose_xllm() {
        let cmd = InferenceEngine::XLlm
            .compose_command("C:\\models\\Qwen", 9000)
            .unwrap();
        assert_eq!(cmd[0], "xllm");
        assert_eq!(cmd[1], "serve");
        assert!(cmd.contains(&"--model".to_string()));
        assert!(cmd.contains(&"C:\\models\\Qwen".to_string()));
        assert!(cmd.contains(&"--port".to_string()));
        assert!(cmd.contains(&"9000".to_string()));
    }

    #[test]
    fn engine_compose_vllm() {
        let cmd = InferenceEngine::VLlm
            .compose_command("/path/to/model", 9000)
            .unwrap();
        assert_eq!(cmd[0], "vllm");
        assert_eq!(cmd[1], "serve");
        assert!(cmd.contains(&"/path/to/model".to_string()));
    }

    #[test]
    fn engine_compose_llama_server() {
        let cmd = InferenceEngine::LlamaServer
            .compose_command("/path/model.gguf", 9000)
            .unwrap();
        assert_eq!(cmd[0], "llama-server");
        assert!(cmd.contains(&"-m".to_string()));
        assert!(cmd.contains(&"/path/model.gguf".to_string()));
    }

    #[test]
    fn engine_compose_tabby_uses_repo_id() {
        let cmd = InferenceEngine::TabbyMl
            .compose_command("TabbyML/Qwen2.5-Coder-7B", 8080)
            .unwrap();
        assert_eq!(cmd[0], "tabby");
        assert_eq!(cmd[1], "serve");
        assert!(cmd.contains(&"--chat-model".to_string()));
        assert!(cmd.contains(&"TabbyML/Qwen2.5-Coder-7B".to_string()));
    }

    #[test]
    fn engine_compose_tabbyapi_uses_platform_launcher() {
        let cmd = InferenceEngine::TabbyApi
            .compose_command("C:\\models\\Local-EXL2", TABBY_API_DEFAULT_PORT)
            .unwrap();
        #[cfg(windows)]
        assert_eq!(cmd[0], "Start.bat");
        #[cfg(not(windows))]
        assert_eq!(cmd[0], "./start.sh");
        assert_eq!(cmd[1], "--config");
        assert_eq!(cmd[2], "config.yml");
    }

    #[test]
    fn engine_ollama_no_spawn() {
        assert!(InferenceEngine::Ollama
            .compose_command("any", 11434)
            .is_none());
    }

    #[test]
    fn engine_custom_no_compose() {
        assert!(InferenceEngine::Custom
            .compose_command("any", 9000)
            .is_none());
    }

    // ── list_downloaded_models ──────────────────────────────────────

    #[test]
    fn list_models_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(list_downloaded_models(tmp.path()).is_empty());
    }

    #[test]
    fn list_models_returns_subdirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let qwen = tmp.path().join("Qwen--Qwen2.5-Coder-7B");
        std::fs::create_dir_all(&qwen).unwrap();
        std::fs::write(qwen.join("config.json"), "{}").unwrap();
        std::fs::write(qwen.join("model.safetensors"), "x").unwrap();
        let solar = tmp.path().join("upstage--SOLAR-10.7B");
        std::fs::create_dir_all(&solar).unwrap();
        std::fs::write(solar.join("model.safetensors"), "x").unwrap();
        std::fs::write(tmp.path().join("ignore.txt"), "x").unwrap();
        let mut models = list_downloaded_models(tmp.path());
        models.sort();
        assert_eq!(models.len(), 2);
        assert!(models[0].contains("Qwen") || models[1].contains("Qwen"));
    }

    #[test]
    fn list_models_are_sorted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let zulu = tmp.path().join("zulu-model");
        let alpha = tmp.path().join("alpha-model");
        std::fs::create_dir_all(&zulu).unwrap();
        std::fs::create_dir_all(&alpha).unwrap();
        std::fs::write(zulu.join("model.safetensors"), "x").unwrap();
        std::fs::write(alpha.join("model.safetensors"), "x").unwrap();

        let models = list_downloaded_models(tmp.path());
        assert_eq!(
            models,
            vec!["alpha-model".to_string(), "zulu-model".to_string()]
        );
    }

    #[test]
    fn list_models_skips_empty_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("empty")).unwrap();
        assert!(list_downloaded_models(tmp.path()).is_empty());
    }

    #[test]
    fn list_models_skips_metadata_only_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let model = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
        std::fs::create_dir_all(&model).unwrap();
        std::fs::write(model.join("README.md"), "x").unwrap();
        std::fs::write(model.join(".gitattributes"), "x").unwrap();

        assert!(list_downloaded_models(tmp.path()).is_empty());
        assert!(
            downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
                .is_none()
        );
    }

    #[test]
    fn list_models_empty_path_returns_empty() {
        assert!(list_downloaded_models(std::path::Path::new("")).is_empty());
    }

    // ── downloaded_exl2_preset_folder ───────────────────────────────

    #[test]
    fn downloaded_exl2_preset_folder_accepts_same_model_bpw_variant() {
        let tmp = tempfile::TempDir::new().unwrap();
        let model = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
        std::fs::create_dir_all(&model).unwrap();
        std::fs::write(model.join("config.json"), "{}").unwrap();
        std::fs::write(model.join("model.safetensors"), "x").unwrap();

        assert_eq!(
            downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
                .as_deref(),
            Some("Llama-3.2-3B-Instruct-4.0bpw")
        );
    }

    #[test]
    fn downloaded_exl2_preset_folder_accepts_nested_model_layout() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
        let nested = root.join("weights");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("config.json"), "{}").unwrap();
        std::fs::write(nested.join("model.safetensors"), "x").unwrap();

        assert_eq!(
            downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
                .as_deref(),
            Some("Llama-3.2-3B-Instruct-4.0bpw")
        );
    }

    #[test]
    fn downloaded_exl2_preset_folder_accepts_root_with_multiple_nested_variants() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
        let child_35 = root.join("3.5bpw");
        let child_40 = root.join("4.0bpw");
        std::fs::create_dir_all(&child_35).unwrap();
        std::fs::create_dir_all(&child_40).unwrap();
        std::fs::write(child_35.join("config.json"), "{}").unwrap();
        std::fs::write(child_35.join("model.safetensors"), "x").unwrap();
        std::fs::write(child_40.join("config.json"), "{}").unwrap();
        std::fs::write(child_40.join("model.safetensors"), "x").unwrap();

        assert_eq!(
            downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
                .as_deref(),
            Some("Llama-3.2-3B-Instruct-3.5bpw")
        );
    }

    #[test]
    fn downloaded_exl2_preset_folder_exact_match_wins() {
        let tmp = tempfile::TempDir::new().unwrap();
        let exact = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
        let other = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
        std::fs::create_dir_all(&exact).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(exact.join("config.json"), "{}").unwrap();
        std::fs::write(exact.join("model.safetensors"), "x").unwrap();
        std::fs::write(other.join("config.json"), "{}").unwrap();
        std::fs::write(other.join("model.safetensors"), "x").unwrap();

        assert_eq!(
            downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
                .as_deref(),
            Some("Llama-3.2-3B-Instruct-3.5bpw")
        );
    }

    #[test]
    fn downloaded_exl2_preset_folder_avoids_ambiguous_variants() {
        let tmp = tempfile::TempDir::new().unwrap();
        let a = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
        let b = tmp.path().join("Llama-3.2-3B-Instruct-5.0bpw");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("config.json"), "{}").unwrap();
        std::fs::write(a.join("model.safetensors"), "x").unwrap();
        std::fs::write(b.join("config.json"), "{}").unwrap();
        std::fs::write(b.join("model.safetensors"), "x").unwrap();

        assert!(
            downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
                .is_none()
        );
    }

    // ── resolve_tabbyapi_model_dir ──────────────────────────────────

    #[test]
    fn resolve_tabbyapi_model_dir_accepts_single_nested_child() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("Qwen2.5-Coder-7B-Instruct-exl2-4.0bpw");
        let nested = root.join("model");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("config.json"), "{}").unwrap();
        std::fs::write(nested.join("model.safetensors"), "x").unwrap();

        let resolved = resolve_tabbyapi_model_dir(&root).expect("expected nested model dir");
        assert_eq!(resolved, nested);
    }

    #[test]
    fn resolve_tabbyapi_model_dir_for_folder_prefers_matching_bpw_child() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
        let child_35 = root.join("3.5bpw");
        let child_40 = root.join("4.0bpw");
        std::fs::create_dir_all(&child_35).unwrap();
        std::fs::create_dir_all(&child_40).unwrap();
        std::fs::write(child_35.join("config.json"), "{}").unwrap();
        std::fs::write(child_35.join("model.safetensors"), "x").unwrap();
        std::fs::write(child_40.join("config.json"), "{}").unwrap();
        std::fs::write(child_40.join("model.safetensors"), "x").unwrap();

        let resolved = resolve_tabbyapi_model_dir_for_folder(&root, "Llama-3.2-3B-Instruct-3.5bpw")
            .expect("expected bpw-matched nested model dir");
        assert_eq!(resolved, child_35);
    }
}
