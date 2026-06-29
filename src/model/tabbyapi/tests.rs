use super::*;

#[test]
fn is_model_extension_accepts_bin() {
    assert!(is_model_extension("model.bin"));
}

#[test]
fn is_model_extension_accepts_gguf() {
    assert!(is_model_extension("model.gguf"));
}

#[test]
fn is_model_extension_accepts_pt() {
    assert!(is_model_extension("model.pt"));
}

#[test]
fn is_model_extension_accepts_pth() {
    assert!(is_model_extension("model.pth"));
}

#[test]
fn is_model_extension_rejects_uppercase() {
    assert!(!is_model_extension("MODEL.GGUF"));
}

#[test]
fn is_model_extension_rejects_mixed_case() {
    assert!(!is_model_extension("Model.Bin"));
}

#[test]
fn is_model_extension_accepts_multi_dot() {
    assert!(is_model_extension("model.backup.gguf"));
}

#[test]
fn is_model_extension_rejects_other_ext() {
    assert!(!is_model_extension("model.txt"));
}

#[test]
fn is_model_extension_rejects_safetensors() {
    assert!(!is_model_extension("model.safetensors"));
}

#[test]
fn is_model_extension_rejects_substring_suffix() {
    assert!(!is_model_extension("model.bin2"));
}

#[test]
fn is_model_extension_rejects_empty_string() {
    assert!(!is_model_extension(""));
}

#[test]
fn is_model_extension_rejects_no_extension() {
    assert!(!is_model_extension("model"));
}

#[test]
fn is_model_extension_rejects_just_dot() {
    assert!(!is_model_extension("model."));
}

#[test]
fn is_model_extension_rejects_dot_only() {
    assert!(!is_model_extension("."));
}

#[test]
fn extract_bpw_hint_whole_string_4bpw() {
    assert_eq!(extract_bpw_hint("4bpw"), Some("4bpw".into()));
}

#[test]
fn extract_bpw_hint_float_8dot0bpw() {
    assert_eq!(extract_bpw_hint("8.0bpw"), Some("8.0bpw".into()));
}

#[test]
fn extract_bpw_hint_middle_of_text() {
    assert_eq!(
        extract_bpw_hint("llama-3.2-6.5bpw-h6"),
        Some("6.5bpw".into())
    );
}

#[test]
fn extract_bpw_hint_uppercase() {
    assert_eq!(extract_bpw_hint("6.5BPW"), Some("6.5bpw".into()));
}

#[test]
fn extract_bpw_hint_returns_first_match() {
    assert_eq!(extract_bpw_hint("4.0bpw-or-8.0bpw"), Some("4.0bpw".into()));
}

#[test]
fn extract_bpw_hint_no_number_before() {
    assert_eq!(extract_bpw_hint("modelbpw"), None);
}

#[test]
fn extract_bpw_hint_just_bpw_no_number() {
    assert_eq!(extract_bpw_hint("bpw"), None);
}

#[test]
fn extract_bpw_hint_empty_string() {
    assert_eq!(extract_bpw_hint(""), None);
}

#[test]
fn extract_bpw_hint_no_match() {
    assert_eq!(extract_bpw_hint("hello world"), None);
}

#[test]
fn extract_bpw_hint_digits_only_before() {
    assert_eq!(extract_bpw_hint("42bpw"), Some("42bpw".into()));
}

#[test]
fn extract_bpw_hint_leading_zero() {
    assert_eq!(extract_bpw_hint("0.5bpw"), Some("0.5bpw".into()));
}

#[test]
fn extract_bpw_hint_trailing_dot() {
    assert_eq!(extract_bpw_hint("8.bpw"), Some("8.bpw".into()));
}

#[test]
fn exl2_repo_model_stem_simple() {
    assert_eq!(
        exl2_repo_model_stem("author/ModelName-exl2"),
        Some("ModelName".into())
    );
}

#[test]
fn exl2_repo_model_stem_uppercase_suffix() {
    assert_eq!(
        exl2_repo_model_stem("author/ModelName-EXL2"),
        Some("ModelName".into())
    );
}

#[test]
fn exl2_repo_model_stem_no_author() {
    assert_eq!(exl2_repo_model_stem("model-exl2"), Some("model".into()));
}

#[test]
fn exl2_repo_model_stem_no_suffix() {
    assert_eq!(exl2_repo_model_stem("author/ModelName"), None);
}

#[test]
fn exl2_repo_model_stem_different_suffix() {
    assert_eq!(exl2_repo_model_stem("author/ModelName-other"), None);
}

#[test]
fn exl2_repo_model_stem_empty_string() {
    assert_eq!(exl2_repo_model_stem(""), None);
}

#[test]
fn exl2_repo_model_stem_trailing_slash() {
    assert_eq!(exl2_repo_model_stem("author/"), None);
}

#[test]
fn exl2_repo_model_stem_only_suffix() {
    assert_eq!(exl2_repo_model_stem("something/exl2"), None);
}

#[test]
fn exl2_repo_model_stem_trimmed_whitespace_is_stripped() {
    assert_eq!(
        exl2_repo_model_stem("author/  model-exl2  "),
        Some("model".into())
    );
}

#[test]
fn exl2_repo_model_stem_empty_after_trim() {
    assert_eq!(exl2_repo_model_stem("author/   "), None);
}
