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
