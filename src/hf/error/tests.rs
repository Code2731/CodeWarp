use super::*;

#[test]
fn extract_hf_error_hint_parses_requested_revision_tail() {
    let raw =
        "HF 404: revision not found (requested revision: '4bpw'; available branches: main, 4.0bpw)";
    assert_eq!(
        extract_hf_error_hint(raw, "requested revision:").as_deref(),
        Some("requested revision: '4bpw'; available branches: main, 4.0bpw")
    );
}

#[test]
fn extract_hf_error_hint_parses_fallback_retry() {
    let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw'; available branches: main, 4.0bpw)";
    assert_eq!(
        extract_hf_error_hint(raw, "fallback retry from").as_deref(),
        Some("fallback retry from '4bpw' to '4.0bpw'")
    );
}

#[test]
fn compose_hf_download_error_appends_revision_hint() {
    let raw =
        "HF 404: revision not found (requested revision: '4bpw'; available branches: main, 4.0bpw)";
    let msg = compose_hf_download_error(raw);
    assert!(msg.contains("requested revision: '4bpw'"));
    assert!(msg.contains("available branches: main, 4.0bpw"));
}

#[test]
fn compose_hf_download_error_appends_fallback_and_revision_hints() {
    let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw'; available branches: main, 4.0bpw)";
    let msg = compose_hf_download_error(raw);
    assert!(msg.contains("fallback retry from '4bpw' to '4.0bpw'"));
    assert!(msg.contains("requested revision: '4bpw'"));
}

#[test]
fn compose_hf_download_error_appends_fallback_lookup_failure_hint() {
    let raw = "HF 404: revision not found (fallback lookup failed: branch refs unavailable; requested revision: '4bpw')";
    let msg = compose_hf_download_error(raw);
    assert!(msg.contains("fallback lookup failed: branch refs unavailable"));
    assert!(msg.contains("requested revision: '4bpw'"));
}

#[test]
fn extract_hf_error_hint_keeps_branch_names_with_parentheses() {
    let raw = "HF 404: revision not found (requested revision: '4bpw'; available branches: exl2(legacy), main)";
    assert_eq!(
        extract_hf_error_hint(raw, "requested revision:").as_deref(),
        Some("requested revision: '4bpw'; available branches: exl2(legacy), main")
    );
}

#[test]
fn extract_hf_error_hint_parses_no_space_parenthesis_separator() {
    let raw = "HF 404: revision not found (fallback retry from '4bpw' to '4.0bpw')(requested revision: '4bpw'; available branches: main, 4.0bpw)";
    assert_eq!(
        extract_hf_error_hint(raw, "fallback retry from").as_deref(),
        Some("fallback retry from '4bpw' to '4.0bpw'")
    );
}

#[test]
fn merge_hint_prefers_more_specific_hint() {
    let mut hints = vec!["requested revision: '4bpw'".to_string()];
    merge_hint(
        &mut hints,
        "fallback lookup failed: branch refs unavailable; requested revision: '4bpw'".to_string(),
    );
    assert_eq!(hints.len(), 1);
    assert!(hints[0].starts_with("fallback lookup failed:"));
}

#[test]
fn compose_hf_download_error_avoids_overlapping_hint_duplicates() {
    let raw = "HF 404: revision not found (fallback lookup failed: branch refs unavailable; requested revision: '4bpw')";
    let msg = compose_hf_download_error(raw);
    assert_eq!(msg.matches("requested revision: '4bpw'").count(), 1);
}

#[test]
fn extract_hf_error_hint_is_case_insensitive_for_marker() {
    let raw =
        "HF 404: revision not found (Requested Revision: '4bpw'; available branches: main, 4.0bpw)";
    assert_eq!(
        extract_hf_error_hint(raw, "requested revision:").as_deref(),
        Some("Requested Revision: '4bpw'; available branches: main, 4.0bpw")
    );
}

#[test]
fn contains_ascii_case_insensitive_matches_mixed_case() {
    assert!(contains_ascii_case_insensitive(
        "Requested Revision: '4bpw'",
        "requested revision:"
    ));
}

#[test]
fn merge_hint_deduplicates_case_insensitive_overlap() {
    let mut hints = vec!["Requested Revision: '4bpw'".to_string()];
    merge_hint(&mut hints, "requested revision: '4bpw'".to_string());
    assert_eq!(hints.len(), 1);
}

#[test]
fn starts_with_ascii_case_insensitive_matches_mixed_case_prefix() {
    assert!(starts_with_ascii_case_insensitive(
        "Requested Revision: '4bpw'",
        "requested revision:"
    ));
}

#[test]
fn find_hint_boundary_detects_next_marker_separator() {
    let tail = "fallback retry from '4bpw' to '4.0bpw') (requested revision: '4bpw')";
    assert_eq!(find_hint_boundary(tail), Some(38));
}

#[test]
fn extract_hf_error_hint_keeps_internal_paren_separator_not_followed_by_marker() {
    let raw = "HF 404: revision not found (requested revision: '4bpw'; available branches: weird)(branch), main)";
    assert_eq!(
        extract_hf_error_hint(raw, "requested revision:").as_deref(),
        Some("requested revision: '4bpw'; available branches: weird)(branch), main")
    );
}
