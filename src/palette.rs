use std::cmp::Ordering;

pub(crate) const PALETTE_INPUT_ID: &str = "command-palette-input";

#[derive(Debug, Clone, Copy)]
pub(crate) enum PaletteAction {
    NewChat,
    PlanMode,
    BuildMode,
    OpenSettings,
    PickCwd,
    CycleSort,
    ToggleFavorite,
}

#[derive(Debug)]
pub(crate) struct PaletteCommand {
    pub(crate) action: PaletteAction,
    pub(crate) label: &'static str,
    pub(crate) hint: &'static str,
}

pub(crate) const PALETTE_COMMANDS: &[PaletteCommand] = &[
    PaletteCommand {
        action: PaletteAction::NewChat,
        label: "새 채팅",
        hint: "현재 세션 보존 후 빈 세션 시작",
    },
    PaletteCommand {
        action: PaletteAction::PlanMode,
        label: "🔍 Plan 모드",
        hint: "읽기 전용 도구만 사용",
    },
    PaletteCommand {
        action: PaletteAction::BuildMode,
        label: "🔧 Build 모드",
        hint: "전체 도구 사용 (사용자 승인 필요)",
    },
    PaletteCommand {
        action: PaletteAction::OpenSettings,
        label: "⚙ 설정",
        hint: "OpenRouter 키 등록/삭제",
    },
    PaletteCommand {
        action: PaletteAction::PickCwd,
        label: "📁 작업 폴더 변경",
        hint: "native folder picker",
    },
    PaletteCommand {
        action: PaletteAction::CycleSort,
        label: "💰 가격 정렬 토글",
        hint: "기본 → 오름차순 → 내림차순",
    },
    PaletteCommand {
        action: PaletteAction::ToggleFavorite,
        label: "★ 현재 모델 즐겨찾기 토글",
        hint: "favorites.json 영구 저장",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteRowState {
    Default,
    Hovered,
    Active,
}

pub(crate) fn palette_selection_for_result_count(result_count: usize) -> Option<usize> {
    (result_count > 0).then_some(0)
}

pub(crate) fn move_palette_selection(
    active_selection: Option<usize>,
    delta: i32,
    result_count: usize,
) -> Option<usize> {
    let last_index = result_count.checked_sub(1)?;
    let current_index = match active_selection {
        Some(index) if index <= last_index => index,
        _ => 0,
    };

    let next_index = match delta.cmp(&0) {
        Ordering::Less => current_index.saturating_sub(1),
        Ordering::Equal => current_index,
        Ordering::Greater => current_index.saturating_add(1).min(last_index),
    };
    Some(next_index)
}

pub(crate) fn palette_selection_for_mouse_enter(
    index: usize,
    result_count: usize,
) -> Option<usize> {
    (index < result_count).then_some(index)
}

pub(crate) fn resolve_palette_selection(
    active_selection: Option<usize>,
    result_count: usize,
) -> Option<usize> {
    active_selection.filter(|index| *index < result_count)
}

pub(crate) fn palette_row_state(
    index: usize,
    active_selection: Option<usize>,
    hovered_selection: Option<usize>,
) -> PaletteRowState {
    if active_selection == Some(index) {
        PaletteRowState::Active
    } else if hovered_selection == Some(index) {
        PaletteRowState::Hovered
    } else {
        PaletteRowState::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_selection_opening_contract() {
        assert_eq!(palette_selection_for_result_count(3), Some(0));
    }

    #[test]
    fn palette_selection_filtering_contract() {
        assert_eq!(palette_selection_for_result_count(1), Some(0));
        assert_eq!(palette_selection_for_result_count(0), None);
    }

    #[test]
    fn palette_selection_bounds_contract() {
        assert_eq!(move_palette_selection(Some(0), -1, 3), Some(0));
        assert_eq!(move_palette_selection(Some(2), 1, 3), Some(2));
    }

    #[test]
    fn palette_selection_mouse_contract() {
        assert_eq!(palette_selection_for_mouse_enter(2, 3), Some(2));
    }

    #[test]
    fn palette_selection_empty_contract() {
        assert_eq!(move_palette_selection(None, 1, 0), None);
        assert_eq!(resolve_palette_selection(Some(0), 0), None);
    }

    #[test]
    fn palette_row_default_state() {
        assert_eq!(palette_row_state(0, None, None), PaletteRowState::Default);
    }

    #[test]
    fn palette_row_hovered_state() {
        assert_eq!(
            palette_row_state(0, None, Some(0)),
            PaletteRowState::Hovered
        );
    }

    #[test]
    fn palette_row_active_state() {
        assert_eq!(
            palette_row_state(0, Some(0), Some(0)),
            PaletteRowState::Active
        );
    }

    #[test]
    fn palette_row_mouse_exit_preserves_active_state() {
        assert_eq!(palette_row_state(0, Some(0), None), PaletteRowState::Active);
    }
}
