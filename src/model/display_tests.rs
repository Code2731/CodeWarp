use super::*;

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
    let s = format!("{m}");
    assert!(s.starts_with("[OR]"), "got: {}", s);
    assert!(s.contains("gpt-4o"));
}

#[test]
fn display_openai_compat_with_label() {
    let m = oai_opt("qwen2.5-coder", "xLLM");
    let s = format!("{m}");
    assert!(s.starts_with("[xLLM]"), "got: {}", s);
    assert!(s.contains("qwen2.5-coder"));
}

#[test]
fn display_openai_compat_empty_label_defaults_to_local() {
    let m = oai_opt("starcoder", "");
    let s = format!("{m}");
    assert!(s.starts_with("[Local]"), "got: {}", s);
}

#[test]
fn display_openai_compat_whitespace_label_defaults() {
    let m = oai_opt("foo", "   ");
    let s = format!("{m}");
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
    let s = format!("{m}");
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
    let s = format!("{m}");
    assert!(s.contains("[xLLM]"));
    assert!(s.contains("free"));
}

// ── Tests migrated from types.rs inline mod tests ────────────────

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
    assert!(m.to_string().contains('\u{2605}'), "got: {}", m);
}

#[test]
fn display_not_favorite() {
    let m = opt(LlmProvider::OpenRouter, "", false, false, None, None, None);
    assert!(!m.to_string().contains('\u{2605}'), "got: {}", m);
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
    assert!(s.starts_with("[xLLM][KO]\u{2605}"), "got: {s}");
    assert!(s.contains("gpt-4"), "got: {s}");
    assert!(s.contains("128k"), "got: {s}");
    assert!(s.contains("$10.00/$30.00"), "got: {s}");
}
