use super::shorten_tail;

#[test]
fn shorten_tail_keeps_short_text() {
    assert_eq!(shorten_tail("codewarp", 16), "codewarp");
}

#[test]
fn shorten_tail_keeps_text_when_exactly_max_chars() {
    assert_eq!(shorten_tail("codewarp", 8), "codewarp");
}

#[test]
fn shorten_tail_truncates_to_max_chars() {
    let out = shorten_tail("abcdefghijklmnopqrstuvwxyz", 8);
    assert_eq!(out, "…tuvwxyz");
    assert_eq!(out.chars().count(), 8);
}

#[test]
fn shorten_tail_handles_zero_and_one() {
    assert_eq!(shorten_tail("abcdef", 0), "");
    assert_eq!(shorten_tail("abcdef", 1), "…");
}

#[test]
fn shorten_tail_two_chars_keeps_last_char() {
    let out = shorten_tail("abcdef", 2);
    assert_eq!(out, "…f");
    assert_eq!(out.chars().count(), 2);
}

#[test]
fn shorten_tail_three_chars_keeps_last_two_chars() {
    let out = shorten_tail("abcdef", 3);
    assert_eq!(out, "…ef");
    assert_eq!(out.chars().count(), 3);
}

#[test]
fn shorten_tail_counts_unicode_by_char_not_byte() {
    let src = format!("ab{}cd", '\u{1F600}');
    let out = shorten_tail(&src, 4);
    assert_eq!(out, "…😀cd");
    assert_eq!(out.chars().count(), 4);
    assert!(out.ends_with("😀cd"));
}

#[test]
fn shorten_tail_keeps_unicode_when_exactly_max_chars() {
    let src = format!("ab{}cd", '\u{1F600}');
    assert_eq!(src.chars().count(), 5);
    assert_eq!(shorten_tail(&src, 5), src);
}

use super::{disabled_text_color, with_alpha};
use iced::Color;

#[test]
fn with_alpha_preserves_rgb_and_sets_alpha() {
    let base = Color::from_rgb(0.1, 0.2, 0.3);
    let out = with_alpha(base, 0.7);
    assert_eq!(out.r, base.r);
    assert_eq!(out.g, base.g);
    assert_eq!(out.b, base.b);
    assert_eq!(out.a, 0.7);
}

#[test]
fn disabled_text_color_matches_design_value() {
    let c = disabled_text_color();
    assert_eq!(c.r, 0.92);
    assert_eq!(c.g, 0.94);
    assert_eq!(c.b, 0.98);
    assert_eq!(c.a, 0.75);
}

#[test]
fn with_alpha_handles_alpha_bounds() {
    let base = Color::from_rgb(0.25, 0.5, 0.75);
    let transparent = with_alpha(base, 0.0);
    let opaque = with_alpha(base, 1.0);
    assert_eq!(transparent.a, 0.0);
    assert_eq!(opaque.a, 1.0);
    assert_eq!(transparent.r, base.r);
    assert_eq!(opaque.b, base.b);
}

use super::{bold_font, semibold_font, UI_FONT_FAMILY};
use iced::font::Weight;
use iced::Font;

#[test]
fn semibold_font_uses_semibold_weight() {
    assert_eq!(semibold_font().weight, Weight::Semibold);
}

#[test]
fn bold_font_uses_bold_weight() {
    assert_eq!(bold_font().weight, Weight::Bold);
}

#[test]
fn font_helpers_use_pretendard_family() {
    let expected_family = Font::with_name(UI_FONT_FAMILY).family;
    assert_eq!(semibold_font().family, expected_family);
    assert_eq!(bold_font().family, expected_family);
}
