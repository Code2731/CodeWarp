mod presets;
pub(crate) use presets::*;

mod types;
pub(crate) use types::*;

mod tabbyapi;
pub(crate) use tabbyapi::*;

#[cfg(test)]
mod tests;

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
