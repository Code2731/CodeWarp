use std::path::PathBuf;

use crate::util::{fmt_context_length, resolve_user_path};

mod presets;
pub(crate) use presets::*;

#[cfg(test)]
mod tests;

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
