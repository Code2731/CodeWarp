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
mod tests;
