// hf/revision.rs — Branch/revision selection helpers (hf child module)
use std::fmt::Write;

pub(super) fn normalize_revision_name(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

pub(super) fn extract_bpw_value(s: &str) -> Option<f32> {
    let lower = s.to_lowercase();
    let bpw_idx = lower.find("bpw")?;
    let prefix = &lower[..bpw_idx];
    let bytes = prefix.as_bytes();
    let mut end = bytes.len();
    while end > 0 && !bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_digit() || b == b'.' {
            start -= 1;
        } else {
            break;
        }
    }
    if start >= end {
        return None;
    }
    prefix[start..end].parse::<f32>().ok()
}

pub(super) fn choose_revision_fallback(requested: &str, branches: &[String]) -> Option<String> {
    if branches.is_empty() {
        return None;
    }

    if let Some(hit) = branches
        .iter()
        .find(|b| b.eq_ignore_ascii_case(requested))
        .cloned()
    {
        return Some(hit);
    }

    let requested_norm = normalize_revision_name(requested);
    if !requested_norm.is_empty()
        && let Some(hit) = branches
            .iter()
            .find(|b| normalize_revision_name(b) == requested_norm)
            .cloned()
    {
        return Some(hit);
    }

    if let Some(target) = extract_bpw_value(requested) {
        let mut best: Option<(f32, String)> = None;
        for b in branches {
            if let Some(v) = extract_bpw_value(b) {
                let dist = (v - target).abs();
                match &best {
                    Some((best_dist, best_name))
                        if dist > *best_dist
                            || ((dist - *best_dist).abs() < f32::EPSILON && b >= best_name) => {}
                    _ => best = Some((dist, b.clone())),
                }
            }
        }
        if let Some((_, name)) = best {
            return Some(name);
        }
    }

    branches
        .iter()
        .find(|b| b.eq_ignore_ascii_case("main"))
        .cloned()
        .or_else(|| branches.first().cloned())
}

pub(super) fn format_branch_suggestions(branches: &[String], limit: usize) -> String {
    let shown: Vec<&str> = branches
        .iter()
        .map(|b| b.trim())
        .filter(|b| !b.is_empty())
        .take(limit)
        .collect();
    if shown.is_empty() {
        return String::new();
    }
    let mut text = shown.join(", ");
    if branches.len() > shown.len() {
        let _ = write!(text, " ... +{} more", branches.len() - shown.len());
    }
    text
}

pub(super) fn annotate_revision_not_found_error(
    base: &str,
    requested: &str,
    branches: &[String],
) -> String {
    let suggested = format_branch_suggestions(branches, 8);
    if suggested.is_empty() {
        return base.to_string();
    }
    format!("{base} (requested revision: '{requested}'; available branches: {suggested})")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) {
        assert!((a - b).abs() < f32::EPSILON, "expected {b}, got {a}");
    }

    // ── normalize_revision_name ─────────────────────────────────────────
    #[test]
    fn test_normalize_revision_name_empty() {
        assert_eq!(normalize_revision_name(""), "");
    }

    #[test]
    fn test_normalize_revision_name_alphanumeric() {
        assert_eq!(normalize_revision_name("HelloWorld"), "helloworld");
    }

    #[test]
    fn test_normalize_revision_name_strips_special() {
        assert_eq!(normalize_revision_name("main-v2.0!"), "mainv20");
    }

    #[test]
    fn test_normalize_revision_name_already_lower() {
        assert_eq!(normalize_revision_name("abc123"), "abc123");
    }

    #[test]
    fn test_normalize_revision_name_non_ascii() {
        assert_eq!(normalize_revision_name("café-ñ"), "caf");
    }

    // ── extract_bpw_value ───────────────────────────────────────────────
    #[test]
    fn test_extract_bpw_value_none() {
        assert_eq!(extract_bpw_value("no bpw here"), None);
    }

    #[test]
    fn test_extract_bpw_value_empty() {
        assert_eq!(extract_bpw_value(""), None);
    }

    #[test]
    fn test_extract_bpw_value_just_bpw() {
        assert_eq!(extract_bpw_value("bpw"), None);
    }

    #[test]
    fn test_extract_bpw_value_simple() {
        let v = extract_bpw_value("4.25bpw").unwrap();
        approx_eq(v, 4.25);
    }

    #[test]
    fn test_extract_bpw_value_integer() {
        let v = extract_bpw_value("8bpw").unwrap();
        approx_eq(v, 8.0);
    }

    #[test]
    fn test_extract_bpw_value_trailing_text() {
        let v = extract_bpw_value("3.5bpw_ggml").unwrap();
        approx_eq(v, 3.5);
    }

    #[test]
    fn test_extract_bpw_value_prefixed_text() {
        let v = extract_bpw_value("q4_2.75bpw").unwrap();
        approx_eq(v, 2.75);
    }

    #[test]
    fn test_extract_bpw_value_case_insensitive() {
        let v = extract_bpw_value("6.0BPW").unwrap();
        approx_eq(v, 6.0);
    }

    #[test]
    fn test_extract_bpw_value_multiple_bpw_first_wins() {
        let v = extract_bpw_value("3.0bpw_some_4.5bpw").unwrap();
        approx_eq(v, 3.0);
    }

    #[test]
    fn test_extract_bpw_value_no_digit_before() {
        assert_eq!(extract_bpw_value(".bpw"), None);
    }

    // ── choose_revision_fallback ────────────────────────────────────────
    #[test]
    fn test_choose_revision_fallback_empty_branches() {
        assert_eq!(choose_revision_fallback("main", &[]), None);
    }

    #[test]
    fn test_choose_revision_fallback_exact_match() {
        let branches = vec!["main".into(), "dev".into()];
        assert_eq!(
            choose_revision_fallback("main", &branches),
            Some("main".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_case_insensitive_match() {
        let branches = vec!["Main".into()];
        assert_eq!(
            choose_revision_fallback("main", &branches),
            Some("Main".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_normalized_match() {
        let branches = vec!["my-branch_v2".into()];
        assert_eq!(
            choose_revision_fallback("my branch v2!", &branches),
            Some("my-branch_v2".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_bpw_distance() {
        let branches = vec!["3.0bpw".into(), "4.0bpw".into(), "8.0bpw".into()];
        assert_eq!(
            choose_revision_fallback("4.25bpw", &branches),
            Some("4.0bpw".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_bpw_distance_ties_keeps_first_less() {
        let branches = vec!["a2.0bpw".into(), "b2.0bpw".into()];
        assert_eq!(
            choose_revision_fallback("2.0bpw", &branches),
            Some("a2.0bpw".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_no_bpw_match_falls_to_main() {
        let branches = vec!["main".into(), "other".into()];
        assert_eq!(
            choose_revision_fallback("nonexistent", &branches),
            Some("main".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_no_bpw_no_main_falls_to_first() {
        let branches = vec!["first".into(), "second".into()];
        assert_eq!(
            choose_revision_fallback("nonexistent", &branches),
            Some("first".into())
        );
    }

    #[test]
    fn test_choose_revision_fallback_bpw_exact_then_first() {
        let branches = vec!["first".into(), "4.0bpw".into()];
        assert_eq!(
            choose_revision_fallback("4.0bpw", &branches),
            Some("4.0bpw".into())
        );
    }

    // ── format_branch_suggestions ───────────────────────────────────────
    #[test]
    fn test_format_branch_suggestions_empty() {
        assert_eq!(format_branch_suggestions(&[], 5), "");
    }

    #[test]
    fn test_format_branch_suggestions_under_limit() {
        let b = vec!["a".into(), "b".into()];
        assert_eq!(format_branch_suggestions(&b, 5), "a, b");
    }

    #[test]
    fn test_format_branch_suggestions_exact_limit() {
        let b = vec!["x".into(), "y".into()];
        assert_eq!(format_branch_suggestions(&b, 2), "x, y");
    }

    #[test]
    fn test_format_branch_suggestions_truncated() {
        let b = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(format_branch_suggestions(&b, 2), "a, b ... +2 more");
    }

    #[test]
    fn test_format_branch_suggestions_trims_whitespace() {
        let b = vec!["  a  ".into(), " b ".into()];
        assert_eq!(format_branch_suggestions(&b, 5), "a, b");
    }

    #[test]
    fn test_format_branch_suggestions_skips_empty() {
        let b = vec!["".into(), "a".into(), "".into(), "b".into()];
        assert_eq!(format_branch_suggestions(&b, 5), "a, b ... +2 more");
    }

    #[test]
    fn test_format_branch_suggestions_all_empty() {
        let b = vec!["".into(), "  ".into()];
        assert_eq!(format_branch_suggestions(&b, 5), "");
    }

    // ── annotate_revision_not_found_error ───────────────────────────────
    #[test]
    fn test_annotate_revision_not_found_empty_branches() {
        assert_eq!(
            annotate_revision_not_found_error("base err", "v1", &[]),
            "base err"
        );
    }

    #[test]
    fn test_annotate_revision_not_found_with_suggestions() {
        let b = vec!["main".into(), "dev".into()];
        let msg = annotate_revision_not_found_error("error", "v1", &b);
        assert!(msg.contains("error"));
        assert!(msg.contains("requested revision: 'v1'"));
        assert!(msg.contains("main, dev"));
    }
}
