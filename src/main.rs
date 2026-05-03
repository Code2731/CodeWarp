// CodeWarp — Iced 진입점
// Phase 2-3a: 3-pane 레이아웃 + 모델 셀렉터 (TopBar) + 입력 echo

mod keystore;
mod openrouter;
mod session;
mod tabby;
mod tools;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::markdown::{self, HeadingLevel, Settings as MdSettings, Text as MdText, Viewer};
use iced::widget::operation::snap_to_end;
use iced::widget::scrollable::{Direction, Scrollbar, Viewport};
use iced::widget::text_editor::{Action, Edit};
use iced::widget::{
    button, checkbox, column, combo_box, container, row, scrollable, stack, text, text_editor,
    text_input, Id as ScrollId, Space,
};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use iced::task;
use iced::{font, Alignment, Color, Element, Font, Length, Size, Subscription, Task, Theme};

use openrouter::{AuthKeyData, ChatEvent, ChatMessage, GenerationData, OpenRouterModel};

/// 모델을 어느 백엔드로 라우팅할지.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LlmProvider {
    OpenRouter,
    Tabby,
}

impl LlmProvider {
    /// combo_box Display에 prefix로 붙는 짧은 태그.
    fn tag(self) -> &'static str {
        match self {
            LlmProvider::OpenRouter => "[OR]",
            LlmProvider::Tabby => "[Tb]",
        }
    }
}

/// combo_box에 표시할 모델 항목 (가격 정보 포함).
/// Display 형식: "[OR][KO] model-id  $in/$out" 또는 "[Tb] model-id  free"
#[derive(Debug, Clone, PartialEq)]
struct ModelOption {
    id: String,
    provider: LlmProvider,
    /// 한국어 토크나이저 친화 모델 휴리스틱 결과
    ko_friendly: bool,
    /// 입력 100만 토큰당 USD
    prompt_per_million: Option<f64>,
    /// 출력 100만 토큰당 USD
    completion_per_million: Option<f64>,
}

impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag = self.provider.tag();
        let ko = if self.ko_friendly { "[KO]" } else { "" };
        match (self.prompt_per_million, self.completion_per_million) {
            (Some(p), Some(c)) if p == 0.0 && c == 0.0 => {
                write!(f, "{}{} {}  free", tag, ko, self.id)
            }
            (Some(p), Some(c)) => {
                write!(f, "{}{} {}  ${:.2}/${:.2}", tag, ko, self.id, p, c)
            }
            _ => write!(f, "{}{} {}", tag, ko, self.id),
        }
    }
}

/// 모델 ID에 한국어 친화로 알려진 패턴이 들어있는지.
/// 휴리스틱 — 누락/오탐 가능. 화이트리스트 갱신은 여기 한 줄.
fn is_korean_friendly(id: &str) -> bool {
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

fn parse_price_per_million(s: Option<&str>) -> Option<f64> {
    let v = s?.parse::<f64>().ok()?;
    Some(v * 1_000_000.0)
}

// 본문용 — 한국어/영문 동시 지원 (Pretendard, OFL)
const PRETENDARD_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/Pretendard-Regular.otf");
const PRETENDARD_SEMIBOLD: &[u8] =
    include_bytes!("../assets/fonts/Pretendard-SemiBold.otf");
const PRETENDARD_BOLD: &[u8] =
    include_bytes!("../assets/fonts/Pretendard-Bold.otf");
// 코드용 monospace (JetBrains Mono, OFL)
const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");

fn handle_key(key: Key, modifiers: Modifiers) -> Option<Message> {
    // Esc — 열려있는 오버레이(팔레트/설정) 모두 닫기
    if matches!(key.as_ref(), Key::Named(Named::Escape)) {
        return Some(Message::CloseAllOverlays);
    }
    // Ctrl/Cmd + 단축키
    if modifiers.command() {
        return match key.as_ref() {
            Key::Character("k") => Some(Message::OpenCommandPalette),
            Key::Character("n") => Some(Message::NewChat),
            Key::Character(",") => Some(Message::OpenSettings),
            Key::Character("p") if modifiers.shift() => {
                Some(Message::SetAgentMode(AgentMode::Plan))
            }
            Key::Character("b") if modifiers.shift() => {
                Some(Message::SetAgentMode(AgentMode::Build))
            }
            _ => None,
        };
    }
    None
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .font(PRETENDARD_REGULAR)
        .font(PRETENDARD_SEMIBOLD)
        .font(PRETENDARD_BOLD)
        .font(JETBRAINS_MONO_REGULAR)
        .font(JETBRAINS_MONO_BOLD)
        .default_font(Font::with_name("Pretendard"))
        .window(iced::window::Settings {
            size: Size::new(1280.0, 800.0),
            min_size: Some(Size::new(960.0, 600.0)),
            ..Default::default()
        })
        .run()
}

/// 사용자 입력은 짧은 plain text, AI 응답은 read-only text_editor (부분 선택 + 복사 가능).
enum BlockBody {
    User(String),
    Assistant(text_editor::Content),
}

impl BlockBody {
    fn role_label(&self) -> &'static str {
        match self {
            BlockBody::User(_) => "you",
            BlockBody::Assistant(_) => "ai",
        }
    }

    fn to_text(&self) -> String {
        match self {
            BlockBody::User(s) => s.clone(),
            BlockBody::Assistant(c) => c.text(),
        }
    }

    fn is_empty_for_history(&self) -> bool {
        match self {
            BlockBody::User(s) => s.trim().is_empty(),
            BlockBody::Assistant(c) => c.text().trim().is_empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    /// 마크다운으로 예쁘게 렌더 (기본). 코드 블록은 syntax highlight.
    Rendered,
    /// 원문(read-only text_editor). 부분 텍스트 드래그 선택 + Ctrl+C 가능.
    Raw,
}

struct Block {
    id: u64,
    body: BlockBody,
    view_mode: ViewMode,
    /// assistant Rendered용 캐시. 토큰 도착 시마다 갱신.
    md_items: Vec<markdown::Item>,
    /// assistant 블록을 만든 모델 ID (user 블록은 None).
    model: Option<String>,
}

fn persisted_to_block(pb: session::PersistedBlock) -> Block {
    let role = pb.role;
    let content = pb.content;
    let md_items = if role == "assistant" {
        markdown::parse(&content).collect()
    } else {
        Vec::new()
    };
    let body = if role == "user" {
        BlockBody::User(content)
    } else {
        BlockBody::Assistant(text_editor::Content::with_text(&content))
    };
    let model = if pb.model.is_empty() {
        None
    } else {
        Some(pb.model)
    };
    Block {
        id: pb.id,
        body,
        view_mode: ViewMode::Rendered,
        md_items,
        model,
    }
}

/// 두 텍스트의 line-by-line diff를 색상 표시된 Element로 변환.
/// 추가 라인은 녹색, 삭제 라인은 빨강, 동일 라인은 흐리게.
fn render_diff<'a>(old: &str, new: &str) -> Element<'a, Message> {
    use similar::{ChangeTag, TextDiff};

    const MAX_LINES: usize = 400;
    let added = Color::from_rgb(0.55, 0.85, 0.55);
    let removed = Color::from_rgb(0.95, 0.45, 0.45);
    let equal = Color::from_rgb(0.5, 0.5, 0.55);

    let diff = TextDiff::from_lines(old, new);
    let mut col = column![].spacing(0);
    let mut count = 0usize;
    for change in diff.iter_all_changes() {
        if count >= MAX_LINES {
            col = col.push(
                text(format!("…(diff 라인 {}+ 생략)", MAX_LINES))
                    .size(11)
                    .color(equal),
            );
            break;
        }
        let (sign, color) = match change.tag() {
            ChangeTag::Delete => ("-", removed),
            ChangeTag::Insert => ("+", added),
            ChangeTag::Equal => (" ", equal),
        };
        let raw = change.value().trim_end_matches('\n');
        // 너무 긴 줄은 잘라서 화면 폭 보호
        let line_text = if raw.len() > 200 {
            format!("{} {}…", sign, &raw[..200])
        } else {
            format!("{} {}", sign, raw)
        };
        col = col.push(
            text(line_text)
                .size(11)
                .font(Font::with_name("JetBrains Mono"))
                .color(color),
        );
        count += 1;
    }
    container(col).padding(10).width(Length::Fill).into()
}

/// 모달 오버레이: 반투명 백드롭 + 가운데 정렬된 콘텐츠 박스.
/// content는 view_settings/view_write_confirm 같은 기존 화면 함수의 결과.
fn modal_overlay<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    let modal_box = container(content)
        .padding(0)
        .width(Length::Shrink)
        .max_width(720.0)
        .max_height(620.0)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.base.color.into()),
                border: iced::Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        });

    container(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55).into()),
            ..Default::default()
        })
        .into()
}

/// markdown::view_with용 커스텀 Viewer.
/// - heading: Bold weight 강제
/// - paragraph: italic span을 Normal+SemiBold로 변환 (한국어 글리프 깨짐 회피)
struct CodewarpViewer;

impl<'a> Viewer<'a, Message> for CodewarpViewer {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::LinkClicked(url)
    }

    fn heading(
        &self,
        mut settings: MdSettings,
        level: &'a HeadingLevel,
        text: &'a MdText,
        index: usize,
    ) -> Element<'a, Message> {
        let mut bold = Font::with_name("Pretendard");
        bold.weight = font::Weight::Bold;
        settings.style.font = bold;
        markdown::heading(settings, level, text, index, Self::on_link_click)
    }

    fn paragraph(
        &self,
        settings: MdSettings,
        text: &MdText,
    ) -> Element<'a, Message> {
        let spans_arc = text.spans(settings.style);
        let normalized: Vec<iced::advanced::text::Span<'static, markdown::Uri>> = spans_arc
            .iter()
            .map(|s| {
                let mut s = s.clone();
                if let Some(font) = s.font.as_mut() {
                    if !matches!(font.style, iced::font::Style::Normal) {
                        font.style = iced::font::Style::Normal;
                        if matches!(font.weight, iced::font::Weight::Normal) {
                            font.weight = iced::font::Weight::Semibold;
                        }
                    }
                }
                s
            })
            .collect();
        iced::widget::rich_text(normalized)
            .on_link_click(Self::on_link_click)
            .into()
    }
}

/// 도구 호출이 SSE delta로 부분씩 도착하는 동안 누적할 임시 구조.
#[derive(Default, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

const MAX_TOOL_ROUNDS: u32 = 5;

struct App {
    has_key: bool,
    key_input: String,
    /// Tabby base URL 입력값 (Settings 화면). 저장된 값 + 사용자 편집 반영.
    tabby_url_input: String,
    /// Tabby token 입력값 (선택). 비어있으면 인증 없이 호출.
    tabby_token_input: String,
    /// 마지막 ping 결과 — None=미시도, Some(Ok)=정상, Some(Err)=실패 사유.
    tabby_status: Option<Result<String, String>>,
    status: String,
    busy: bool,

    models: Vec<OpenRouterModel>,
    model_ids: Vec<String>,
    selected_model: Option<String>,

    blocks: Vec<Block>,
    next_block_id: u64,
    input: String,
    streaming_block_id: Option<u64>,
    /// 진행 중인 chat_stream task의 abort handle (Stop 버튼이 사용).
    abort_handle: Option<task::Handle>,
    /// 사이드바에서 삭제 확인 대기 중인 세션 ID (✕ → ✓/✗ 토글).
    pending_delete_session: Option<u64>,

    show_settings: bool,

    stream_id: ScrollId,
    follow_bottom: bool,
    /// 활성 세션의 현재 stream scroll y (StreamScrolled로 갱신)
    current_scroll_y: f32,

    /// OpenRouter에 보낼 누적 대화 (도구 호출 round trip 포함)
    conversation: Vec<ChatMessage>,
    /// 현재 stream 중 누적되는 tool_calls
    pending_tool_calls: Vec<PendingToolCall>,
    /// 도구 호출 라운드 카운터 (무한 루프 방지)
    tool_round: u32,
    /// 도구 실행 시 기준이 되는 작업 디렉토리
    cwd: PathBuf,

    /// 검색 가능한 모델 셀렉터(combo_box) 상태 (가격 포함 표시)
    model_combo_state: combo_box::State<ModelOption>,
    /// 가격 포함 전체 모델 옵션 (필터링 전)
    model_options: Vec<ModelOption>,
    /// OpenRouter 계정 사용량/한도
    account: Option<AuthKeyData>,

    /// 사용자 승인 대기 중인 mutating tool 호출 목록 (write_file 등)
    pending_write_calls: Vec<PendingToolCall>,
    show_write_confirm: bool,

    /// 모델 카테고리/즐겨찾기 필터
    filter_coding: bool,
    filter_reasoning: bool,
    filter_general: bool,
    filter_favorites_only: bool,
    favorites: HashSet<String>,
    /// 모델 리스트 정렬 모드
    sort_mode: SortMode,
    /// 에이전트 모드: Plan(읽기 전용 도구만) ↔ Build(파일/명령 실행 포함)
    agent_mode: AgentMode,

    // ── 멀티 세션 ─────────────────────────────────
    /// 비활성 세션 목록 (활성 세션은 위 conversation/blocks에)
    inactive_sessions: Vec<InactiveSession>,
    current_session_id: u64,
    current_session_title: String,
    next_session_id: u64,

    /// 모델별 누적 사용량 (메모리 + usage.json)
    usage: session::UsageStore,
    /// 마지막 응답의 비용 (status bar 표시용)
    last_response_cost: Option<f64>,

    /// 명령 팔레트 (Ctrl+K)
    show_command_palette: bool,
    command_palette_input: String,
}

#[derive(Debug, Clone, Copy)]
enum PaletteAction {
    NewChat,
    PlanMode,
    BuildMode,
    OpenSettings,
    PickCwd,
    CycleSort,
    ToggleFavorite,
}

struct PaletteCommand {
    action: PaletteAction,
    label: &'static str,
    hint: &'static str,
}

const PALETTE_COMMANDS: &[PaletteCommand] = &[
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

/// 비활성 세션 (메모리 절약 위해 blocks를 plain text로 보관)
#[derive(Debug, Clone)]
struct InactiveSession {
    id: u64,
    title: String,
    conversation: Vec<ChatMessage>,
    blocks: Vec<session::PersistedBlock>,
    next_block_id: u64,
    scroll_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelCategory {
    Coding,
    Reasoning,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentMode {
    /// 읽기 전용 도구만 (read_file, glob, grep) — 분석/계획 단계
    Plan,
    /// 모든 도구 (write_file, run_command 포함) — 실제 변경 단계
    Build,
}

impl AgentMode {
    fn allow_mutating(self) -> bool {
        matches!(self, AgentMode::Build)
    }
    fn label(self) -> &'static str {
        match self {
            AgentMode::Plan => "🔍 Plan",
            AgentMode::Build => "🔧 Build",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortMode {
    /// 기본 (OpenRouter 응답 순)
    Default,
    /// 입력+출력 합산 가격 오름차순 (저렴 → 비싼)
    PriceAsc,
    /// 입력+출력 합산 가격 내림차순 (비싼 → 저렴)
    PriceDesc,
}

impl SortMode {
    fn cycle(self) -> Self {
        match self {
            SortMode::Default => SortMode::PriceAsc,
            SortMode::PriceAsc => SortMode::PriceDesc,
            SortMode::PriceDesc => SortMode::Default,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortMode::Default => "정렬: -",
            SortMode::PriceAsc => "💰 ↑",
            SortMode::PriceDesc => "💰 ↓",
        }
    }
}

/// 모델 ID에서 카테고리를 추정. 키워드 매칭 기반.
/// 코딩/추론 전용 모델만 좁게 매칭하고, 나머지(Claude/GPT-4/Gemini 등)는 범용으로.
fn categorize_model(model_id: &str) -> Vec<ModelCategory> {
    let id = model_id.to_lowercase();
    // 코딩 전용 모델만 (이름에 'code', 'coder' 등이 명시된 것)
    let coding_keywords = [
        "coder", "codex", "codestral", "codellama", "starcoder", "codegen", "code-",
    ];
    // 추론 전용 모델 (chain-of-thought / thinking 모드)
    let reasoning_keywords = [
        "o1-", "o3-", "o4-", "/o1", "/o3", "/o4", "thinking", "-reasoning", "-r1", "-qwq", "/qwq",
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

#[derive(Debug, Clone)]
enum Message {
    OpenSettings,
    CloseSettings,
    KeyInputChanged(String),
    SaveKey,
    KeySaved(Result<(), String>),
    ClearKey,
    KeyCleared(Result<(), String>),
    FetchModels,
    ModelsLoaded(Result<Vec<OpenRouterModel>, String>),
    SelectModel(ModelOption),
    AccountLoaded(Result<AuthKeyData, String>),
    FetchAccount,
    InputChanged(String),
    Send,
    /// 진행 중인 chat_stream을 중지 (Stop 버튼).
    StopStream,
    ChatChunk(ChatEvent),
    CopyBlock(u64),
    StreamScrolled(Viewport),
    EditorAction(u64, Action),
    ToggleBlockView(u64),
    LinkClicked(markdown::Uri),
    PickCwd,
    CwdPicked(Option<PathBuf>),
    ApproveWrites,
    DenyWrites,
    ToggleFilterCoding(bool),
    ToggleFilterReasoning(bool),
    ToggleFilterGeneral(bool),
    ToggleFilterFavorites(bool),
    ToggleFavorite,
    CycleSortMode,
    NewChat,
    SetAgentMode(AgentMode),
    ToggleAgentMode,
    SwitchSession(u64),
    /// ✕ 클릭 → 삭제 확인 토글 (같은 id면 취소, 다른 id면 그쪽으로 이동).
    AskDeleteSession(u64),
    DeleteSession(u64),
    CancelDeleteSession,
    GenerationLoaded(Result<GenerationData, String>),
    OpenCommandPalette,
    CloseCommandPalette,
    CloseAllOverlays,
    CommandPaletteChanged(String),
    ExecuteCommand(usize),
    TabbyUrlChanged(String),
    TabbyTokenChanged(String),
    SaveTabby,
    TabbySaved(Result<(), String>),
    ClearTabby,
    FetchTabbyModels,
    TabbyModelsLoaded(Result<Vec<String>, String>),
}

impl App {
    fn title(&self) -> String {
        "CodeWarp".to_string()
    }

    fn theme(&self) -> Theme {
        Theme::custom(
            "CodeWarp Dark".to_string(),
            iced::theme::Palette {
                background: Color::from_rgb(0.055, 0.062, 0.086), // 깊은 다크 navy
                text: Color::from_rgb(0.92, 0.92, 0.94),
                primary: Color::from_rgb(0.66, 0.55, 0.96),       // 보라 액센트
                success: Color::from_rgb(0.31, 0.80, 0.66),       // teal
                warning: Color::from_rgb(0.95, 0.78, 0.42),       // amber
                danger: Color::from_rgb(0.96, 0.53, 0.45),        // coral red
            },
        )
    }

    fn new() -> (Self, Task<Message>) {
        let has_key = keystore::has_api_key();
        let saved_model = keystore::read_selected_model();
        let status = if has_key {
            "준비됨".into()
        } else {
            "OpenRouter API 키 미등록".into()
        };
        let saved_tabby_url = keystore::read_tabby_base_url().unwrap_or_default();
        let saved_tabby_token = keystore::read_tabby_token().unwrap_or_default();
        let mut app = Self {
            has_key,
            key_input: String::new(),
            tabby_url_input: saved_tabby_url,
            tabby_token_input: saved_tabby_token,
            tabby_status: None,
            status,
            busy: false,
            models: Vec::new(),
            model_ids: Vec::new(),
            selected_model: saved_model,
            blocks: Vec::new(),
            next_block_id: 0,
            input: String::new(),
            streaming_block_id: None,
            abort_handle: None,
            pending_delete_session: None,
            show_settings: !has_key,
            stream_id: ScrollId::new("stream"),
            follow_bottom: true,
            current_scroll_y: 0.0,
            conversation: Vec::new(),
            pending_tool_calls: Vec::new(),
            tool_round: 0,
            cwd: keystore::read_cwd()
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from(".")),
            model_combo_state: combo_box::State::new(Vec::new()),
            model_options: Vec::new(),
            account: None,
            pending_write_calls: Vec::new(),
            show_write_confirm: false,
            filter_coding: true,
            filter_reasoning: true,
            filter_general: true,
            filter_favorites_only: false,
            favorites: session::read_favorites().into_iter().collect(),
            sort_mode: SortMode::Default,
            agent_mode: AgentMode::Plan,
            inactive_sessions: Vec::new(),
            current_session_id: 1,
            current_session_title: String::new(),
            next_session_id: 1,
            usage: session::load_usage(),
            last_response_cost: None,
            show_command_palette: false,
            command_palette_input: String::new(),
        };

        // 멀티 세션 복원
        let mut persisted = session::load_all();
        if persisted.sessions.is_empty() {
            persisted = session::load_all(); // 빈 → default 채워짐
        }
        let active_idx = persisted.active_idx.min(persisted.sessions.len().saturating_sub(1));
        let active = persisted.sessions[active_idx].clone();
        let inactive: Vec<InactiveSession> = persisted
            .sessions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != active_idx)
            .map(|(_, s)| InactiveSession {
                id: s.id,
                title: s.title.clone(),
                conversation: s.conversation.clone(),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();

        app.current_session_id = active.id;
        app.current_session_title = active.title;
        app.conversation = active.conversation;
        app.next_block_id = active.next_block_id;
        app.blocks = active
            .blocks
            .into_iter()
            .map(persisted_to_block)
            .collect();
        app.current_scroll_y = active.scroll_y;
        app.inactive_sessions = inactive;
        app.next_session_id = persisted
            .sessions
            .iter()
            .map(|s| s.id)
            .max()
            .unwrap_or(0)
            + 1;
        // 활성 세션의 마지막 scroll 위치 복원 task
        let restore_scroll = if app.current_scroll_y > 0.0 {
            Some(iced::widget::operation::scroll_to(
                app.stream_id.clone(),
                iced::widget::scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: app.current_scroll_y,
                },
            ))
        } else {
            None
        };

        let mut tasks: Vec<Task<Message>> = Vec::new();
        if has_key {
            tasks.push(Task::done(Message::FetchModels));
            tasks.push(Task::done(Message::FetchAccount));
        }
        if !app.tabby_url_input.trim().is_empty() {
            tasks.push(Task::done(Message::FetchTabbyModels));
        }
        if let Some(t) = restore_scroll {
            tasks.push(t);
        }
        let task = if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        };
        (app, task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenSettings => {
                self.show_settings = true;
                Task::none()
            }
            Message::CloseSettings => {
                self.show_settings = false;
                Task::none()
            }
            Message::KeyInputChanged(v) => {
                self.key_input = v;
                Task::none()
            }
            Message::SaveKey => {
                let key = self.key_input.clone();
                self.busy = true;
                self.status = "키 저장 중…".into();
                Task::perform(
                    async move { keystore::write_api_key(&key) },
                    Message::KeySaved,
                )
            }
            Message::KeySaved(r) => {
                self.busy = false;
                match r {
                    Ok(()) => {
                        self.has_key = true;
                        self.key_input.clear();
                        self.show_settings = false;
                        self.status = "키 저장됨".into();
                        Task::done(Message::FetchModels)
                    }
                    Err(e) => {
                        self.status = format!("저장 실패: {}", e);
                        Task::none()
                    }
                }
            }
            Message::ClearKey => {
                self.busy = true;
                Task::perform(async { keystore::delete_api_key() }, Message::KeyCleared)
            }
            Message::KeyCleared(r) => {
                self.busy = false;
                match r {
                    Ok(()) => {
                        self.has_key = false;
                        self.models.clear();
                        self.model_ids.clear();
                        self.selected_model = None;
                        let _ = keystore::clear_selected_model();
                        self.status = "키 삭제됨".into();
                    }
                    Err(e) => self.status = format!("삭제 실패: {}", e),
                }
                Task::none()
            }
            Message::TabbyUrlChanged(v) => {
                self.tabby_url_input = v;
                Task::none()
            }
            Message::TabbyTokenChanged(v) => {
                self.tabby_token_input = v;
                Task::none()
            }
            Message::SaveTabby => {
                let url = self.tabby_url_input.clone();
                let token = self.tabby_token_input.clone();
                self.busy = true;
                self.status = "Tabby 설정 저장 중…".into();
                Task::perform(
                    async move {
                        keystore::write_tabby_base_url(&url)?;
                        keystore::write_tabby_token(&token)?;
                        Ok(())
                    },
                    Message::TabbySaved,
                )
            }
            Message::TabbySaved(r) => {
                self.busy = false;
                match r {
                    Ok(()) => {
                        self.status = "Tabby 설정 저장됨".into();
                        // 저장 직후 자동 모델 fetch (= 연결 테스트 겸용)
                        if !self.tabby_url_input.trim().is_empty() {
                            return Task::done(Message::FetchTabbyModels);
                        }
                    }
                    Err(e) => self.status = format!("Tabby 저장 실패: {}", e),
                }
                Task::none()
            }
            Message::ClearTabby => {
                let _ = keystore::clear_tabby_base_url();
                let _ = keystore::clear_tabby_token();
                self.tabby_url_input.clear();
                self.tabby_token_input.clear();
                self.tabby_status = None;
                self.status = "Tabby 설정 삭제됨".into();
                // 모델 리스트에서 Tabby 항목 제거
                self.model_options.retain(|o| o.provider != LlmProvider::Tabby);
                self.refresh_model_combo();
                // 선택된 모델이 Tabby였다면 해제
                if let Some(sel) = self.selected_model.clone() {
                    if !self.model_options.iter().any(|o| o.id == sel) {
                        self.selected_model = self.model_options.first().map(|o| o.id.clone());
                        if let Some(id) = &self.selected_model {
                            let _ = keystore::write_selected_model(id);
                        }
                    }
                }
                Task::none()
            }
            Message::FetchTabbyModels => {
                let url = self.tabby_url_input.clone();
                if url.trim().is_empty() {
                    self.tabby_status = Some(Err("URL 비어있음".into()));
                    return Task::none();
                }
                let token = if self.tabby_token_input.trim().is_empty() {
                    None
                } else {
                    Some(self.tabby_token_input.clone())
                };
                self.status = "Tabby 모델 가져오는 중…".into();
                Task::perform(tabby::list_models(url, token), Message::TabbyModelsLoaded)
            }
            Message::TabbyModelsLoaded(r) => {
                // 기존 Tabby 항목 제거 후 새로 채움 (성공/실패 모두 동일하게 비움)
                self.model_options.retain(|o| o.provider != LlmProvider::Tabby);
                match r {
                    Ok(ids) => {
                        let label = if ids.is_empty() {
                            "ok (모델 없음)".to_string()
                        } else {
                            format!("{}개", ids.len())
                        };
                        self.status = format!("Tabby 연결됨 — {}", label);
                        self.tabby_status = Some(Ok(label));
                        for id in ids {
                            let ko_friendly = is_korean_friendly(&id);
                            self.model_options.push(ModelOption {
                                id,
                                provider: LlmProvider::Tabby,
                                ko_friendly,
                                prompt_per_million: Some(0.0),
                                completion_per_million: Some(0.0),
                            });
                        }
                    }
                    Err(e) => {
                        self.status = format!("Tabby 연결 실패: {}", e);
                        self.tabby_status = Some(Err(e));
                    }
                }
                self.refresh_model_combo();
                Task::none()
            }
            Message::FetchModels => {
                let key = match keystore::read_api_key() {
                    Ok(k) => k,
                    Err(e) => {
                        self.status = e;
                        return Task::none();
                    }
                };
                self.busy = true;
                self.status = "모델 리스트 가져오는 중…".into();
                Task::perform(openrouter::list_models(key), Message::ModelsLoaded)
            }
            Message::ModelsLoaded(r) => {
                self.busy = false;
                match r {
                    Ok(models) => {
                        let n = models.len();
                        self.model_ids = models.iter().map(|m| m.id.clone()).collect();
                        // OpenRouter 항목만 교체, Tabby 항목 보존
                        self.model_options.retain(|o| o.provider != LlmProvider::OpenRouter);
                        self.model_options.extend(models.iter().map(|m| {
                            let id = m.id.clone();
                            let ko_friendly = is_korean_friendly(&id);
                            ModelOption {
                                id,
                                provider: LlmProvider::OpenRouter,
                                ko_friendly,
                                prompt_per_million: parse_price_per_million(
                                    m.pricing.as_ref().and_then(|p| p.prompt.as_deref()),
                                ),
                                completion_per_million: parse_price_per_million(
                                    m.pricing.as_ref().and_then(|p| p.completion.as_deref()),
                                ),
                            }
                        }));
                        self.refresh_model_combo();
                        let saved_in_list = self
                            .selected_model
                            .as_ref()
                            .map(|id| self.model_ids.iter().any(|m| m == id))
                            .unwrap_or(false);
                        if !saved_in_list {
                            self.selected_model = self.model_ids.first().cloned();
                            if let Some(id) = &self.selected_model {
                                let _ = keystore::write_selected_model(id);
                            }
                        }
                        self.models = models;
                        self.status = format!("모델 {} 로드됨", n);
                    }
                    Err(e) => self.status = format!("페치 실패: {}", e),
                }
                Task::none()
            }
            Message::SelectModel(opt) => {
                let _ = keystore::write_selected_model(&opt.id);
                self.selected_model = Some(opt.id);
                Task::none()
            }
            Message::FetchAccount => {
                let key = match keystore::read_api_key() {
                    Ok(k) => k,
                    Err(_) => return Task::none(),
                };
                Task::perform(
                    openrouter::get_account_info(key),
                    Message::AccountLoaded,
                )
            }
            Message::AccountLoaded(r) => {
                if let Ok(data) = r {
                    self.account = Some(data);
                }
                Task::none()
            }
            Message::InputChanged(v) => {
                self.input = v;
                Task::none()
            }
            Message::Send => {
                let text = self.input.trim().to_string();
                if text.is_empty() {
                    return Task::none();
                }
                // 슬래시 커맨드 처리
                match text.as_str() {
                    "/plan" => {
                        self.agent_mode = AgentMode::Plan;
                        self.input.clear();
                        self.status = format!("{} 모드", AgentMode::Plan.label());
                        return Task::none();
                    }
                    "/build" => {
                        self.agent_mode = AgentMode::Build;
                        self.input.clear();
                        self.status = format!("{} 모드", AgentMode::Build.label());
                        return Task::none();
                    }
                    s if s.starts_with('/') => {
                        self.status = format!("알 수 없는 슬래시 명령: {}", s);
                        return Task::none();
                    }
                    _ => {}
                }
                if self.selected_model.is_none() || self.streaming_block_id.is_some() {
                    return Task::none();
                }
                let (base_url, api_key) = match self.resolve_provider() {
                    Ok(v) => v,
                    Err(e) => {
                        self.status = e;
                        return Task::none();
                    }
                };
                let model = self.selected_model.clone().unwrap();

                // 새 turn 시작: system 메시지(cwd 안내) 보장 → user 메시지 push.
                self.ensure_system_message();
                self.conversation.push(ChatMessage::user(text.clone()));
                self.pending_tool_calls.clear();
                self.tool_round = 0;
                let messages = self.conversation.clone();

                let user_id = self.next_id();
                self.blocks.push(Block {
                    id: user_id,
                    body: BlockBody::User(text),
                    view_mode: ViewMode::Rendered,
                    md_items: Vec::new(),
                    model: None,
                });
                let ai_id = self.next_id();
                self.blocks.push(Block {
                    id: ai_id,
                    body: BlockBody::Assistant(text_editor::Content::new()),
                    view_mode: ViewMode::Rendered,
                    md_items: Vec::new(),
                    model: self.selected_model.clone(),
                });
                self.streaming_block_id = Some(ai_id);
                self.input.clear();
                self.status = "응답 생성 중…".into();
                self.follow_bottom = true; // 새 메시지 전송 시 follow ON

                let (chat_task, handle) = Task::run(
                    openrouter::chat_stream(
                        base_url,
                        api_key,
                        model,
                        messages,
                        Some(tools::tool_definitions(self.agent_mode.allow_mutating())),
                    ),
                    Message::ChatChunk,
                )
                .abortable();
                self.abort_handle = Some(handle);
                Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
            }
            Message::StopStream => {
                if let Some(h) = self.abort_handle.take() {
                    h.abort();
                }
                if let Some(ai_id) = self.streaming_block_id {
                    if let Some(b) = self.blocks.iter().find(|b| b.id == ai_id) {
                        let txt = b.body.to_text();
                        if !txt.is_empty() {
                            self.conversation.push(ChatMessage::assistant(txt));
                        }
                    }
                }
                self.streaming_block_id = None;
                self.pending_tool_calls.clear();
                self.tool_round = 0;
                self.status = "중지됨".into();
                self.maybe_update_title();
                self.save_session();
                Task::none()
            }
            Message::CopyBlock(id) => {
                if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
                    return iced::clipboard::write(b.body.to_text());
                }
                Task::none()
            }
            Message::ChatChunk(event) => {
                let Some(ai_id) = self.streaming_block_id else {
                    return Task::none();
                };
                match event {
                    ChatEvent::Token(t) => {
                        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == ai_id) {
                            if let BlockBody::Assistant(content) = &mut b.body {
                                content.perform(Action::Edit(Edit::Paste(Arc::new(t))));
                                let raw = content.text();
                                b.md_items = markdown::parse(&raw).collect();
                            }
                        }
                    }
                    ChatEvent::ToolCallDelta {
                        index,
                        id,
                        name,
                        arguments,
                    } => {
                        let i = index as usize;
                        while self.pending_tool_calls.len() <= i {
                            self.pending_tool_calls.push(PendingToolCall::default());
                        }
                        let tc = &mut self.pending_tool_calls[i];
                        if let Some(id) = id {
                            tc.id = id;
                        }
                        if let Some(name) = name {
                            tc.name = name;
                        }
                        if let Some(args) = arguments {
                            tc.arguments.push_str(&args);
                        }
                    }
                    ChatEvent::Done {
                        finish_reason,
                        generation_id,
                    } => {
                        // 현재 assistant block에 누적된 텍스트
                        let assistant_text = self
                            .blocks
                            .iter()
                            .find(|b| b.id == ai_id)
                            .and_then(|b| match &b.body {
                                BlockBody::Assistant(c) => Some(c.text()),
                                _ => None,
                            })
                            .unwrap_or_default();

                        let has_tools = !self.pending_tool_calls.is_empty()
                            && (finish_reason.as_deref() == Some("tool_calls")
                                || finish_reason.is_none());

                        if has_tools && self.tool_round < MAX_TOOL_ROUNDS {
                            return self.run_tool_round(assistant_text);
                        }

                        // 정상 종료 (또는 라운드 한도 초과)
                        if self.tool_round >= MAX_TOOL_ROUNDS && !self.pending_tool_calls.is_empty() {
                            self.status =
                                format!("최대 도구 라운드 {} 초과", MAX_TOOL_ROUNDS);
                        } else {
                            self.status = "준비됨".into();
                        }
                        if !assistant_text.is_empty() {
                            self.conversation
                                .push(ChatMessage::assistant(assistant_text));
                        }
                        self.streaming_block_id = None;
                        self.abort_handle = None;
                        self.pending_tool_calls.clear();
                        self.maybe_update_title();
                        self.save_session();
                        if let Some(id) = generation_id {
                            if let Ok(api_key) = keystore::read_api_key() {
                                return Task::perform(
                                    openrouter::get_generation(api_key, id),
                                    Message::GenerationLoaded,
                                );
                            }
                        }
                    }
                    ChatEvent::Error(e) => {
                        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == ai_id) {
                            if let BlockBody::Assistant(content) = &mut b.body {
                                let prefix =
                                    if content.text().is_empty() { "" } else { "\n\n" };
                                let msg = format!("{}[에러] {}", prefix, e);
                                content.perform(Action::Edit(Edit::Paste(Arc::new(msg))));
                                let raw = content.text();
                                b.md_items = markdown::parse(&raw).collect();
                            }
                        }
                        self.streaming_block_id = None;
                        self.abort_handle = None;
                        self.pending_tool_calls.clear();
                        self.status = format!("에러: {}", e);
                    }
                }
                if self.follow_bottom {
                    snap_to_end(self.stream_id.clone())
                } else {
                    Task::none()
                }
            }
            Message::StreamScrolled(viewport) => {
                // 사용자가 거의 끝까지 내려가 있으면 follow ON, 아니면 OFF
                let rel = viewport.relative_offset();
                self.follow_bottom = rel.y > 0.95;
                self.current_scroll_y = viewport.absolute_offset().y;
                Task::none()
            }
            Message::EditorAction(id, action) => {
                // read-only: Edit 액션은 무시 (사용자 키보드 입력 차단), 나머지(선택/스크롤)는 처리
                if action.is_edit() {
                    return Task::none();
                }
                if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                    if let BlockBody::Assistant(content) = &mut b.body {
                        content.perform(action);
                    }
                }
                Task::none()
            }
            Message::ToggleBlockView(id) => {
                if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                    b.view_mode = match b.view_mode {
                        ViewMode::Rendered => ViewMode::Raw,
                        ViewMode::Raw => ViewMode::Rendered,
                    };
                }
                Task::none()
            }
            Message::LinkClicked(_uri) => {
                // TODO: 시스템 브라우저 열기 (webbrowser crate 등)
                Task::none()
            }
            Message::PickCwd => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("작업 폴더 선택")
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::CwdPicked,
            ),
            Message::ApproveWrites => self.continue_after_writes(true),
            Message::DenyWrites => self.continue_after_writes(false),
            Message::ToggleFilterCoding(v) => {
                self.filter_coding = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::ToggleFilterReasoning(v) => {
                self.filter_reasoning = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::ToggleFilterGeneral(v) => {
                self.filter_general = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::ToggleFilterFavorites(v) => {
                self.filter_favorites_only = v;
                self.refresh_model_combo();
                Task::none()
            }
            Message::CycleSortMode => {
                self.sort_mode = self.sort_mode.cycle();
                self.refresh_model_combo();
                Task::none()
            }
            Message::SetAgentMode(mode) => {
                self.agent_mode = mode;
                self.status = format!("{} 모드", mode.label());
                Task::none()
            }
            Message::ToggleAgentMode => {
                self.agent_mode = match self.agent_mode {
                    AgentMode::Plan => AgentMode::Build,
                    AgentMode::Build => AgentMode::Plan,
                };
                self.status = format!("{} 모드", self.agent_mode.label());
                Task::none()
            }
            Message::NewChat => {
                // 현재 세션 보존 + 새 빈 세션 시작
                self.snapshot_current_to_inactive();
                self.blocks.clear();
                self.conversation.clear();
                self.pending_tool_calls.clear();
                self.pending_write_calls.clear();
                self.show_write_confirm = false;
                self.streaming_block_id = None;
                self.tool_round = 0;
                self.next_block_id = 0;
                self.input.clear();
                self.current_session_id = self.allocate_session_id();
                self.current_session_title = "새 채팅".into();
                self.status = "새 채팅".into();
                self.save_session();
                Task::none()
            }
            Message::SwitchSession(target_id) => {
                if target_id == self.current_session_id {
                    return Task::none();
                }
                let Some(idx) = self
                    .inactive_sessions
                    .iter()
                    .position(|s| s.id == target_id)
                else {
                    return Task::none();
                };
                // 현재 활성을 inactive로 보관
                self.snapshot_current_to_inactive();
                // target 활성화
                let target = self.inactive_sessions.remove(idx);
                self.current_session_id = target.id;
                self.current_session_title = target.title;
                self.conversation = target.conversation;
                self.next_block_id = target.next_block_id;
                self.blocks = target.blocks.into_iter().map(persisted_to_block).collect();
                self.current_scroll_y = target.scroll_y;
                self.pending_tool_calls.clear();
                self.pending_write_calls.clear();
                self.show_write_confirm = false;
                self.streaming_block_id = None;
                self.tool_round = 0;
                self.input.clear();
                self.status = "세션 전환됨".into();
                self.save_session();
                // 새 세션의 마지막 scroll 위치로 복원
                iced::widget::operation::scroll_to(
                    self.stream_id.clone(),
                    iced::widget::scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: target.scroll_y,
                    },
                )
            }
            Message::OpenCommandPalette => {
                self.show_command_palette = true;
                self.command_palette_input.clear();
                Task::none()
            }
            Message::CloseCommandPalette => {
                self.show_command_palette = false;
                Task::none()
            }
            Message::CloseAllOverlays => {
                self.show_command_palette = false;
                self.show_settings = false;
                self.show_write_confirm = false;
                Task::none()
            }
            Message::CommandPaletteChanged(v) => {
                self.command_palette_input = v;
                Task::none()
            }
            Message::ExecuteCommand(idx) => {
                let filtered = self.filtered_palette_commands();
                let Some(cmd) = filtered.get(idx) else {
                    return Task::none();
                };
                let action = cmd.action;
                self.show_command_palette = false;
                self.command_palette_input.clear();
                match action {
                    PaletteAction::NewChat => return Task::done(Message::NewChat),
                    PaletteAction::PlanMode => {
                        return Task::done(Message::SetAgentMode(AgentMode::Plan))
                    }
                    PaletteAction::BuildMode => {
                        return Task::done(Message::SetAgentMode(AgentMode::Build))
                    }
                    PaletteAction::OpenSettings => return Task::done(Message::OpenSettings),
                    PaletteAction::PickCwd => return Task::done(Message::PickCwd),
                    PaletteAction::CycleSort => return Task::done(Message::CycleSortMode),
                    PaletteAction::ToggleFavorite => {
                        return Task::done(Message::ToggleFavorite)
                    }
                }
            }
            Message::GenerationLoaded(r) => {
                if let Ok(data) = r {
                    let cost = data.total_cost.unwrap_or(0.0);
                    self.last_response_cost = Some(cost);
                    let model_id = data.model.clone().unwrap_or_default();
                    if !model_id.is_empty() {
                        let entry = self
                            .usage
                            .by_model
                            .entry(model_id)
                            .or_default();
                        entry.total_cost += cost;
                        entry.prompt_tokens += data.native_tokens_prompt.unwrap_or(0);
                        entry.completion_tokens += data.native_tokens_completion.unwrap_or(0);
                        entry.call_count += 1;
                    }
                    let _ = session::save_usage(&self.usage);
                    // 사용 후 잔액 갱신을 위해 account 다시 fetch
                    return Task::done(Message::FetchAccount);
                }
                Task::none()
            }
            Message::AskDeleteSession(id) => {
                self.pending_delete_session = if self.pending_delete_session == Some(id) {
                    None // 같은 ✕ 다시 클릭 → 취소
                } else {
                    Some(id)
                };
                Task::none()
            }
            Message::CancelDeleteSession => {
                self.pending_delete_session = None;
                Task::none()
            }
            Message::DeleteSession(target_id) => {
                self.pending_delete_session = None;
                if target_id == self.current_session_id {
                    // 현재 활성을 삭제 → 빈 세션으로 대체
                    self.blocks.clear();
                    self.conversation.clear();
                    self.next_block_id = 0;
                    self.current_session_id = self.allocate_session_id();
                    self.current_session_title = "새 채팅".into();
                } else {
                    self.inactive_sessions.retain(|s| s.id != target_id);
                }
                self.save_session();
                Task::none()
            }
            Message::ToggleFavorite => {
                if let Some(id) = &self.selected_model {
                    if self.favorites.contains(id) {
                        self.favorites.remove(id);
                    } else {
                        self.favorites.insert(id.clone());
                    }
                    let favs: Vec<String> = self.favorites.iter().cloned().collect();
                    let _ = session::write_favorites(&favs);
                    self.refresh_model_combo();
                }
                Task::none()
            }
            Message::CwdPicked(maybe_path) => {
                if let Some(path) = maybe_path {
                    self.cwd = path.clone();
                    let _ = keystore::write_cwd(&path.display().to_string());
                    self.status = format!("작업 폴더: {}", path.display());
                    // system 메시지(cwd 안내) 갱신
                    self.ensure_system_message();
                }
                Task::none()
            }
        }
    }

    /// 현재 활성 필터/정렬을 적용해 model_options을 좁힌 결과.
    fn filtered_model_options(&self) -> Vec<ModelOption> {
        let mut opts: Vec<ModelOption> = self
            .model_options
            .iter()
            .filter(|opt| {
                if self.filter_favorites_only && !self.favorites.contains(&opt.id) {
                    return false;
                }
                let cats = categorize_model(&opt.id);
                (self.filter_coding && cats.contains(&ModelCategory::Coding))
                    || (self.filter_reasoning && cats.contains(&ModelCategory::Reasoning))
                    || (self.filter_general && cats.contains(&ModelCategory::General))
            })
            .cloned()
            .collect();

        // 정렬: prompt+completion 합 기준
        let total_price = |o: &ModelOption| -> f64 {
            o.prompt_per_million.unwrap_or(0.0) + o.completion_per_million.unwrap_or(0.0)
        };
        match self.sort_mode {
            SortMode::Default => {}
            SortMode::PriceAsc => opts.sort_by(|a, b| {
                total_price(a)
                    .partial_cmp(&total_price(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortMode::PriceDesc => opts.sort_by(|a, b| {
                total_price(b)
                    .partial_cmp(&total_price(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
        opts
    }

    /// 필터/즐겨찾기 변경 시 combo_box::State 재구성.
    fn refresh_model_combo(&mut self) {
        self.model_combo_state = combo_box::State::new(self.filtered_model_options());
    }

    /// 현재 활성 세션 + 비활성 세션 모두를 디스크에 저장.
    fn save_session(&self) {
        let current_blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .map(|b| session::PersistedBlock {
                id: b.id,
                role: match &b.body {
                    BlockBody::User(_) => "user".into(),
                    BlockBody::Assistant(_) => "assistant".into(),
                },
                content: b.body.to_text(),
                model: b.model.clone().unwrap_or_default(),
            })
            .collect();

        let mut sessions: Vec<session::PersistedSessionData> = self
            .inactive_sessions
            .iter()
            .map(|s| session::PersistedSessionData {
                id: s.id,
                title: s.title.clone(),
                conversation: s.conversation.clone(),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();
        sessions.push(session::PersistedSessionData {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: self.conversation.clone(),
            blocks: current_blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        });

        let active_idx = sessions
            .iter()
            .position(|s| s.id == self.current_session_id)
            .unwrap_or(sessions.len() - 1);

        let p = session::PersistedAllSessions {
            sessions,
            active_idx,
        };
        let _ = session::save_all(&p);
    }

    /// 현재 활성 세션 제목 자동 갱신 (첫 사용자 메시지 일부).
    fn maybe_update_title(&mut self) {
        if self.current_session_title.is_empty()
            || self.current_session_title.starts_with("새 채팅")
        {
            if let Some(first_user) = self
                .conversation
                .iter()
                .find(|m| m.role == "user")
                .and_then(|m| m.content.as_ref())
            {
                let snippet: String = first_user.chars().take(30).collect();
                self.current_session_title = snippet;
            }
        }
    }

    /// 현재 활성 세션을 inactive_sessions로 이동 (push 또는 update).
    fn snapshot_current_to_inactive(&mut self) {
        if self.conversation.is_empty() && self.blocks.is_empty() {
            return; // 빈 세션은 보관 X
        }
        let blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .map(|b| session::PersistedBlock {
                id: b.id,
                role: match &b.body {
                    BlockBody::User(_) => "user".into(),
                    BlockBody::Assistant(_) => "assistant".into(),
                },
                content: b.body.to_text(),
                model: b.model.clone().unwrap_or_default(),
            })
            .collect();
        let snap = InactiveSession {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: self.conversation.clone(),
            blocks: blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        };
        if let Some(idx) = self
            .inactive_sessions
            .iter()
            .position(|s| s.id == snap.id)
        {
            self.inactive_sessions[idx] = snap;
        } else {
            self.inactive_sessions.push(snap);
        }
    }

    fn allocate_session_id(&mut self) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }

    /// conversation 첫 위치에 cwd를 알려주는 system 메시지를 보장 (없으면 추가, 있으면 갱신).
    fn ensure_system_message(&mut self) {
        let mode_block = match self.agent_mode {
            AgentMode::Plan => {
                "현재 모드: Plan (분석/계획 전용)\n\
                Plan 모드에서는 read_file/glob/grep으로 코드를 조사하고 변경 계획만 \
                제시하세요. 실제 파일 변경이나 명령 실행은 Build 모드에서만 가능하므로, \
                계획에 '필요한 변경'을 명확히 적고 사용자가 Build로 전환하기를 기다리세요.\n\n"
            }
            AgentMode::Build => {
                "현재 모드: Build (실행 가능)\n\
                Build 모드에서는 write_file/run_command를 사용해 실제 변경을 적용할 수 \
                있습니다. 단, 두 도구 모두 사용자 승인을 거치므로 부담 없이 호출하세요.\n\n"
            }
        };
        let prompt = format!(
            "당신은 CodeWarp의 코딩 어시스턴트입니다.\n\n\
            작업 디렉토리: '{}'\n\n\
            {}\
            사용 가능한 도구 (적극적으로 호출하세요):\n\
            - read_file(path): 파일 내용 읽기 (즉시 실행)\n\
            - write_file(path, content): 파일 작성/덮어쓰기 (Build 모드 + 사용자 승인)\n\
            - run_command(command): 셸 명령 실행 (Build 모드 + 사용자 승인)\n\
            - glob(pattern): 패턴 매칭 파일 리스트 (예: '**/*.rs', 'examples/**/*')\n\
            - grep(pattern): 정규식으로 모든 파일 검색\n\n\
            규칙:\n\
            1. 파일 시스템을 살펴봐야 할 때는 '확인하겠습니다' 같은 말 없이 즉시 도구를 호출하세요.\n\
            2. 새 파일을 만들기 전에 glob으로 기존 구조를 먼저 확인하세요.\n\
            3. 모든 path 인자는 작업 디렉토리 기준 상대 경로 (절대 경로 거부).\n\
            4. 도구 결과를 받은 뒤 그것을 근거로 한국어로 답하세요.\n\
            5. **마크다운 형식 제약** (한국어 폰트 한계): italic(*text* 또는 _text_)은 \
            사용하지 마세요. 강조는 오직 **굵게**만 사용. 별표 한 개로 감싸지 말고, \
            정말 강조가 필요하면 두 개로 감싸세요.",
            self.cwd.display(),
            mode_block,
        );
        if let Some(first) = self.conversation.first_mut() {
            if first.role == "system" {
                first.content = Some(prompt);
                return;
            }
        }
        self.conversation.insert(
            0,
            ChatMessage {
                role: "system".into(),
                content: Some(prompt),
                ..Default::default()
            },
        );
    }

    /// pending_tool_calls를 conversation에 반영, 안전한 도구는 즉시 실행하고
    /// mutating 도구가 있으면 사용자 승인 모달을 띄움. 모두 처리되면 새 chat_stream 트리거.
    fn run_tool_round(&mut self, assistant_partial: String) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_tool_calls);

        // 1) assistant tool_calls 메시지 누적
        let tool_calls_json = serde_json::Value::Array(
            calls
                .iter()
                .enumerate()
                .map(|(i, tc)| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "index": i,
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect(),
        );
        let mut assistant_msg = ChatMessage::assistant_tool_calls(tool_calls_json);
        if !assistant_partial.is_empty() {
            assistant_msg.content = Some(assistant_partial);
        }
        self.conversation.push(assistant_msg);

        // 2) 즉시 실행 가능한 것(read_only)과 승인 필요한 것(mutating) 분리
        let (read_calls, write_calls): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);

        let mut names: Vec<String> = Vec::new();
        for tc in &read_calls {
            names.push(tc.name.clone());
            let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
            self.conversation
                .push(ChatMessage::tool_result(&tc.id, result));
        }
        if !names.is_empty() {
            self.status = format!("도구 호출: {}", names.join(", "));
        }

        if !write_calls.is_empty() {
            // mutating 도구는 사용자 승인 모달로 일시정지
            self.pending_write_calls = write_calls;
            self.show_write_confirm = true;
            self.status = "파일 쓰기 승인 대기".into();
            return Task::none();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }

    /// 사용자 승인/거부 후 호출. true면 mutating 실행, false면 거부 결과를 conversation에 기록.
    fn continue_after_writes(&mut self, approved: bool) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_write_calls);
        self.show_write_confirm = false;

        if approved {
            let mut names: Vec<String> = Vec::new();
            for tc in &calls {
                names.push(tc.name.clone());
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                self.conversation
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            self.status = format!("실행 완료: {}", names.join(", "));
        } else {
            for tc in &calls {
                self.conversation.push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 파일 쓰기를 거부했습니다.",
                ));
            }
            self.status = "사용자가 파일 쓰기를 거부했습니다".into();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }

    fn resolve_provider(&self) -> Result<(String, Option<String>), String> {
        let id = self
            .selected_model
            .as_deref()
            .ok_or_else(|| "모델 미선택".to_string())?;
        let provider = self
            .model_options
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.provider)
            .ok_or_else(|| format!("선택된 모델을 찾을 수 없습니다: {}", id))?;
        match provider {
            LlmProvider::OpenRouter => {
                let key = keystore::read_api_key()?;
                Ok((openrouter::BASE_URL.to_string(), Some(key)))
            }
            LlmProvider::Tabby => {
                let base = keystore::read_tabby_base_url()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| "Tabby URL 미설정".to_string())?;
                let token = keystore::read_tabby_token().filter(|s| !s.trim().is_empty());
                Ok((tabby::chat_base(&base), token))
            }
        }
    }

    /// 누적된 conversation을 가지고 다음 chat_stream을 시작.
    fn kick_chat_stream(&mut self) -> Task<Message> {
        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                self.streaming_block_id = None;
                return Task::none();
            }
        };
        let model = self.selected_model.clone().unwrap_or_default();
        let messages = self.conversation.clone();
        let (task, handle) = Task::run(
            openrouter::chat_stream(
                base_url,
                api_key,
                model,
                messages,
                Some(tools::tool_definitions(self.agent_mode.allow_mutating())),
            ),
            Message::ChatChunk,
        )
        .abortable();
        self.abort_handle = Some(handle);
        task
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::keyboard::listen().filter_map(|event| match event {
            iced::keyboard::Event::KeyPressed {
                key, modifiers, ..
            } => handle_key(key, modifiers),
            _ => None,
        })
    }

    fn filtered_palette_commands(&self) -> Vec<&'static PaletteCommand> {
        let q = self.command_palette_input.to_lowercase();
        if q.is_empty() {
            PALETTE_COMMANDS.iter().collect()
        } else {
            PALETTE_COMMANDS
                .iter()
                .filter(|c| {
                    c.label.to_lowercase().contains(&q)
                        || c.hint.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }

    fn view(&self) -> Element<'_, Message> {
        let topbar = self.view_topbar();

        let main_view: Element<Message> = row![
            self.view_sidebar(),
            self.view_stream(),
            self.view_rightpanel(),
        ]
        .height(Length::Fill)
        .into();

        // overlay가 필요하면 stack으로 메인 위에 띄움 (backdrop + 가운데 모달 박스)
        let middle: Element<Message> = if self.show_command_palette {
            stack![main_view, modal_overlay(self.view_command_palette())].into()
        } else if self.show_settings {
            stack![main_view, modal_overlay(self.view_settings())].into()
        } else {
            // write_confirm은 입력창 위 인라인 패널(view_stream 안에서 처리)
            main_view
        };

        let statusbar = self.view_statusbar();

        column![topbar, middle, statusbar]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_topbar(&self) -> Element<'_, Message> {
        let model_picker: Element<Message> = if self.model_ids.is_empty() {
            text("모델 없음").size(12).into()
        } else {
            {
                let selected_opt = self.selected_model.as_ref().and_then(|id| {
                    self.model_options.iter().find(|o| &o.id == id)
                });
                iced::widget::container(
                    combo_box(
                        &self.model_combo_state,
                        "모델 검색…",
                        selected_opt,
                        Message::SelectModel,
                    )
                    .size(12),
                )
                .width(Length::Fixed(420.0))
                .into()
            }
        };

        let is_fav = self
            .selected_model
            .as_ref()
            .map(|id| self.favorites.contains(id))
            .unwrap_or(false);
        let fav_btn = button(text(if is_fav { "★" } else { "☆" }).size(16))
            .on_press(Message::ToggleFavorite)
            .padding([6, 10]);

        let filters = row![
            checkbox(self.filter_coding)
                .label("코딩")
                .on_toggle(Message::ToggleFilterCoding)
                .size(14)
                .text_size(12),
            checkbox(self.filter_reasoning)
                .label("추론")
                .on_toggle(Message::ToggleFilterReasoning)
                .size(14)
                .text_size(12),
            checkbox(self.filter_general)
                .label("범용")
                .on_toggle(Message::ToggleFilterGeneral)
                .size(14)
                .text_size(12),
            checkbox(self.filter_favorites_only)
                .label("⭐만")
                .on_toggle(Message::ToggleFilterFavorites)
                .size(14)
                .text_size(12),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let sort_btn = button(text(self.sort_mode.label()).size(12))
            .on_press(Message::CycleSortMode)
            .padding([6, 10]);

        let bar = row![
            filters,
            Space::new().width(Length::Fill),
            sort_btn,
            model_picker,
            fav_btn,
            button(
                text("⚙")
                    .size(16)
                    .align_y(Alignment::Center)
            )
            .on_press(Message::OpenSettings)
            .padding([6, 12]),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        container(bar)
            .padding([10, 16])
            .width(Length::Fill)
            .into()
    }

    fn view_usage_summary(&self) -> Element<'_, Message> {
        if self.usage.by_model.is_empty() {
            return text("(사용 기록 없음)").size(11).into();
        }
        // 비용 큰 순 5개
        let mut entries: Vec<(&String, &session::ModelUsage)> =
            self.usage.by_model.iter().collect();
        entries.sort_by(|a, b| {
            b.1.total_cost
                .partial_cmp(&a.1.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut col = column![].spacing(2);
        for (id, u) in entries.iter().take(5) {
            // model id가 너무 길면 끝부분만
            let short_id: String = if id.chars().count() > 24 {
                let tail: String = id
                    .chars()
                    .rev()
                    .take(22)
                    .collect::<Vec<char>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("…{}", tail)
            } else {
                (*id).clone()
            };
            col = col.push(
                row![
                    text(short_id).size(11),
                    Space::new().width(Length::Fill),
                    text(format!("${:.4}", u.total_cost)).size(11),
                ]
                .spacing(6),
            );
        }
        let total: f64 = self.usage.by_model.values().map(|u| u.total_cost).sum();
        col = col.push(Space::new().height(Length::Fixed(4.0)));
        col = col.push(
            row![
                text("총합").size(11),
                Space::new().width(Length::Fill),
                text(format!("${:.4}", total)).size(11),
            ]
            .spacing(6),
        );
        col.into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let cwd_display = self.cwd.display().to_string();
        // 너무 긴 경로는 끝부분만 표시
        let cwd_short = if cwd_display.chars().count() > 36 {
            let tail: String = cwd_display
                .chars()
                .rev()
                .take(34)
                .collect::<Vec<char>>()
                .into_iter()
                .rev()
                .collect();
            format!("…{}", tail)
        } else {
            cwd_display.clone()
        };

        // 세션 목록 (활성 + 비활성)
        let active_label = if self.current_session_title.trim().is_empty() {
            "새 채팅".to_string()
        } else {
            self.current_session_title.clone()
        };
        let mut sessions_col = column![
            container(text(format!("📌 {}", active_label)).size(12))
                .padding([6, 8])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    container::Style {
                        background: Some(p.primary.weak.color.into()),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
        ]
        .spacing(2);
        for s in &self.inactive_sessions {
            let title = if s.title.trim().is_empty() {
                "(빈 세션)".to_string()
            } else {
                s.title.clone()
            };
            let is_pending = self.pending_delete_session == Some(s.id);
            let trailing: Element<Message> = if is_pending {
                row![
                    button(text("✓").size(11))
                        .on_press(Message::DeleteSession(s.id))
                        .padding([2, 6]),
                    button(text("✗").size(11))
                        .on_press(Message::CancelDeleteSession)
                        .padding([2, 6]),
                ]
                .spacing(2)
                .into()
            } else {
                button(text("✕").size(11))
                    .on_press(Message::AskDeleteSession(s.id))
                    .padding([2, 6])
                    .into()
            };
            let row_widget = row![
                button(text(format!("📂 {}", title)).size(12))
                    .on_press(Message::SwitchSession(s.id))
                    .padding([4, 8])
                    .width(Length::Fill),
                trailing,
            ]
            .spacing(2);
            sessions_col = sessions_col.push(row_widget);
        }

        let body = column![
            button(text("＋ 새 채팅").size(13))
                .on_press(Message::NewChat)
                .padding([6, 12])
                .width(Length::Fill),
            Space::new().height(Length::Fixed(8.0)),
            text("채팅").size(11),
            scrollable(sessions_col)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(6).scroller_width(6).margin(2),
                ))
                .height(Length::Fixed(220.0)),
            Space::new().height(Length::Fixed(14.0)),
            text("모델 사용량 (누적)").size(11),
            self.view_usage_summary(),
            Space::new().height(Length::Fixed(14.0)),
            text("작업 폴더").size(11),
            text(cwd_short).size(12),
            button(text("📁 폴더 변경").size(11))
                .on_press(Message::PickCwd)
                .padding([4, 8]),
            Space::new().height(Length::Fixed(14.0)),
            text("프로젝트").size(11),
            text("CodeWarp").size(13),
            Space::new().height(Length::Fixed(14.0)),
            text("컨텍스트").size(11),
            text("선택 안 됨").size(13),
        ]
        .spacing(6);

        container(scrollable(body)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(6).scroller_width(6).margin(2),
                ))
                .height(Length::Fill))
            .width(Length::Fixed(220.0))
            .height(Length::Fill)
            .padding(14)
            .into()
    }

    fn view_rightpanel(&self) -> Element<'_, Message> {
        let body = column![
            text("Plan / Diff / History").size(11),
            Space::new().height(Length::Fixed(8.0)),
            text("// 에이전트 단계가 여기 표시됩니다.").size(12),
        ]
        .spacing(6);

        container(scrollable(body)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(6).scroller_width(6).margin(2),
                ))
                .height(Length::Fill))
            .width(Length::Fixed(280.0))
            .height(Length::Fill)
            .padding(14)
            .into()
    }

    fn view_stream(&self) -> Element<'_, Message> {
        let blocks_view: Element<Message> = if self.blocks.is_empty() {
            container(
                text("$ CodeWarp ready — 입력 후 Enter")
                    .size(13),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            let mut col = column![].spacing(10).width(Length::Fill);
            for b in &self.blocks {
                let role_label = b.body.role_label();
                let has_content = !b.body.is_empty_for_history();
                let copy_btn: Element<Message> = if has_content {
                    button(text("복사").size(10))
                        .on_press(Message::CopyBlock(b.id))
                        .padding([2, 8])
                        .into()
                } else {
                    Space::new().width(Length::Shrink).height(Length::Shrink).into()
                };
                let toggle_btn: Element<Message> =
                    if has_content && matches!(&b.body, BlockBody::Assistant(_)) {
                        let label = match b.view_mode {
                            ViewMode::Rendered => "원문",
                            ViewMode::Raw => "예쁘게",
                        };
                        button(text(label).size(10))
                            .on_press(Message::ToggleBlockView(b.id))
                            .padding([2, 8])
                            .into()
                    } else {
                        Space::new().width(Length::Shrink).height(Length::Shrink).into()
                    };
                let model_label: Element<Message> = match &b.model {
                    Some(m) => text(format!("· {}", m)).size(10).into(),
                    None => Space::new().width(Length::Shrink).height(Length::Shrink).into(),
                };
                let header = row![
                    text(role_label).size(11),
                    model_label,
                    Space::new().width(Length::Fill),
                    toggle_btn,
                    copy_btn,
                ]
                .spacing(6)
                .align_y(Alignment::Center);

                let body_view: Element<Message> = match (&b.body, b.view_mode) {
                    (BlockBody::User(s), _) => text(s).size(13).into(),
                    (BlockBody::Assistant(content), ViewMode::Raw) => {
                        let id = b.id;
                        text_editor(content)
                            .on_action(move |action| Message::EditorAction(id, action))
                            .height(Length::Shrink)
                            .padding(0)
                            .size(13)
                            .into()
                    }
                    (BlockBody::Assistant(_), ViewMode::Rendered) => {
                        let mut settings: markdown::Settings = (&self.theme()).into();
                        settings.style.inline_code_font = Font::with_name("JetBrains Mono");
                        settings.style.code_block_font = Font::with_name("JetBrains Mono");
                        markdown::view_with(b.md_items.iter(), settings, &CodewarpViewer)
                    }
                };

                let is_user = matches!(&b.body, BlockBody::User(_));
                let block_view = container(
                    column![header, body_view].spacing(6),
                )
                .padding(12)
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let bg = if is_user {
                        p.primary.weak.color
                    } else {
                        p.background.weak.color
                    };
                    container::Style {
                        background: Some(bg.into()),
                        border: iced::Border {
                            color: p.background.strong.color,
                            width: 1.0,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    }
                });
                col = col.push(block_view);
            }
            scrollable(col)
                .id(self.stream_id.clone())
                .on_scroll(Message::StreamScrolled)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(6).scroller_width(6).margin(2),
                ))
                .height(Length::Fill)
                .into()
        };

        let send_disabled =
            self.input.trim().is_empty() || self.selected_model.is_none();

        // 입력창 좌측 모드 라벨 (클릭으로 Plan ↔ Build 토글)
        let mode_label = button(text(self.agent_mode.label()).size(11))
            .on_press(Message::ToggleAgentMode)
            .padding([6, 10]);

        // 슬래시 hint: 입력이 '/'로 시작하면 입력창 위에 명령 버튼 줄
        let slash_hint: Element<Message> = if self.input.starts_with('/') {
            container(
                row![
                    text("커맨드:").size(11),
                    button(text("/plan").size(11))
                        .on_press(Message::SetAgentMode(AgentMode::Plan))
                        .padding([2, 8]),
                    button(text("/build").size(11))
                        .on_press(Message::SetAgentMode(AgentMode::Build))
                        .padding([2, 8]),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([4, 8])
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        let action_btn: Element<Message> = if self.streaming_block_id.is_some() {
            button(text("■ 중지").size(13))
                .on_press(Message::StopStream)
                .into()
        } else {
            button(text("Send").size(13))
                .on_press_maybe(if send_disabled {
                    None
                } else {
                    Some(Message::Send)
                })
                .into()
        };

        let input_row = row![
            mode_label,
            text_input("질문을 입력하세요…  (/plan, /build로 모드 전환)", &self.input)
                .on_input(Message::InputChanged)
                .on_submit(Message::Send)
                .padding(10),
            action_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let confirm_panel: Element<Message> = if self.show_write_confirm {
            self.view_inline_confirm()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        column![
            container(blocks_view)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([14, 18]),
            container(confirm_panel).padding([0, 14]),
            container(slash_hint).padding([0, 14]),
            container(input_row)
                .padding([10, 14])
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn view_command_palette(&self) -> Element<'_, Message> {
        let header = text("명령 팔레트").size(18);
        let hint = text(
            "Esc 닫기 · Ctrl+K 팔레트 · Ctrl+N 새 채팅 · Ctrl+, 설정 · \
            Ctrl+Shift+P/B 모드",
        )
        .size(11);
        let input = text_input("명령 검색…", &self.command_palette_input)
            .on_input(Message::CommandPaletteChanged)
            .on_submit(Message::ExecuteCommand(0))
            .padding(10);

        let filtered = self.filtered_palette_commands();
        let mut list = column![].spacing(4);
        if filtered.is_empty() {
            list = list.push(text("(매칭 없음)").size(12));
        } else {
            for (i, cmd) in filtered.iter().enumerate() {
                list = list.push(
                    button(
                        column![
                            text(cmd.label).size(13),
                            text(cmd.hint).size(11),
                        ]
                        .spacing(2),
                    )
                    .on_press(Message::ExecuteCommand(i))
                    .padding([6, 10])
                    .width(Length::Fill),
                );
            }
        }

        let body = column![
            header,
            hint,
            Space::new().height(Length::Fixed(8.0)),
            input,
            Space::new().height(Length::Fixed(8.0)),
            scrollable(list)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(6).scroller_width(6).margin(2),
                ))
                .height(Length::Fixed(320.0)),
            Space::new().height(Length::Fixed(8.0)),
            row![
                Space::new().width(Length::Fill),
                button(text("닫기").size(12))
                    .on_press(Message::CloseCommandPalette)
                    .padding([4, 12]),
            ],
        ]
        .spacing(4);

        container(body)
            .padding(20)
            .width(Length::Fixed(560.0))
            .into()
    }

    fn view_inline_confirm(&self) -> Element<'_, Message> {
        let n = self.pending_write_calls.len();
        let header = text(format!(
            "⚠ AI가 {}개 도구 실행을 요청했습니다",
            n
        ))
        .size(12);

        let mut cards = column![].spacing(4);
        for tc in &self.pending_write_calls {
            let card: Element<Message> = match tc.name.as_str() {
                "write_file" => match tools::WriteFileArgs::parse(&tc.arguments) {
                    Ok(args) => {
                        let abs_path = self.cwd.join(&args.path);
                        let exists = abs_path.exists();
                        let icon = if exists { "📝" } else { "✨" };
                        text(format!(
                            "{}  {}  ({} bytes)",
                            icon,
                            args.path,
                            args.content.len()
                        ))
                        .size(12)
                        .into()
                    }
                    Err(e) => text(format!("[err] {}", e)).size(12).into(),
                },
                "run_command" => match tools::RunCommandArgs::parse(&tc.arguments) {
                    Ok(args) => text(format!("🖥  $ {}", args.command))
                        .size(12)
                        .font(Font::with_name("JetBrains Mono"))
                        .into(),
                    Err(e) => text(format!("[err] {}", e)).size(12).into(),
                },
                _ => text(format!("[?] {}", tc.name)).size(12).into(),
            };
            cards = cards.push(card);
        }

        let actions = row![
            button(text("거부").size(12))
                .on_press(Message::DenyWrites)
                .padding([4, 14]),
            Space::new().width(Length::Fill),
            button(text("✓ 모두 승인").size(12))
                .on_press(Message::ApproveWrites)
                .padding([4, 14]),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        container(
            column![
                header,
                Space::new().height(Length::Fixed(4.0)),
                container(
                    scrollable(cards)
                        .direction(Direction::Vertical(
                            Scrollbar::new().width(6).scroller_width(6).margin(2),
                        ))
                )
                .max_height(140.0),
                Space::new().height(Length::Fixed(6.0)),
                actions,
            ]
            .spacing(2),
        )
        .padding(10)
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            container::Style {
                background: Some(p.background.weak.color.into()),
                border: iced::Border {
                    color: p.danger.weak.color,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }

    #[allow(dead_code)]
    fn view_write_confirm(&self) -> Element<'_, Message> {
        let mut col = column![
            text("파일 쓰기 승인 대기").size(22),
            text(format!(
                "AI가 {}개의 파일을 변경하려고 합니다. 내용을 검토한 뒤 승인 또는 거부하세요.",
                self.pending_write_calls.len()
            ))
            .size(13),
            Space::new().height(Length::Fixed(14.0)),
        ]
        .spacing(6);

        for tc in &self.pending_write_calls {
            let card: Element<Message> = match tc.name.as_str() {
                "write_file" => match tools::WriteFileArgs::parse(&tc.arguments) {
                    Ok(args) => {
                        let abs_path = self.cwd.join(&args.path);
                        let old_content = std::fs::read_to_string(&abs_path).ok();
                        let header = match &old_content {
                            Some(_) => format!(
                                "📝 {} ({} bytes)",
                                args.path,
                                args.content.len()
                            ),
                            None => format!(
                                "✨ 새 파일: {} ({} bytes)",
                                args.path,
                                args.content.len()
                            ),
                        };
                        let diff_view: Element<Message> = match old_content {
                            Some(old) => render_diff(&old, &args.content),
                            None => container(
                                text(args.content.clone())
                                    .size(12)
                                    .font(Font::with_name("JetBrains Mono")),
                            )
                            .padding(10)
                            .width(Length::Fill)
                            .into(),
                        };
                        column![
                            text(header).size(15),
                            Space::new().height(Length::Fixed(6.0)),
                            diff_view,
                        ]
                        .spacing(4)
                        .into()
                    }
                    Err(e) => column![
                        text(format!("[arguments 파싱 실패] {}", e)).size(13),
                        text(tc.arguments.clone()).size(11),
                    ]
                    .spacing(4)
                    .into(),
                },
                "run_command" => match tools::RunCommandArgs::parse(&tc.arguments) {
                    Ok(args) => column![
                        text("🖥 셸 명령 실행").size(15),
                        Space::new().height(Length::Fixed(6.0)),
                        container(
                            text(format!("$ {}", args.command))
                                .size(13)
                                .font(Font::with_name("JetBrains Mono")),
                        )
                        .padding(10)
                        .width(Length::Fill),
                    ]
                    .spacing(4)
                    .into(),
                    Err(e) => column![
                        text(format!("[arguments 파싱 실패] {}", e)).size(13),
                        text(tc.arguments.clone()).size(11),
                    ]
                    .spacing(4)
                    .into(),
                },
                other => column![
                    text(format!("[알 수 없는 도구] {}", other)).size(13),
                    text(tc.arguments.clone()).size(11),
                ]
                .spacing(4)
                .into(),
            };
            col = col.push(container(card).padding(12).width(Length::Fill));
        }

        let actions = row![
            button(text("거부").size(13))
                .on_press(Message::DenyWrites)
                .padding([6, 16]),
            button(text("✓ 모두 승인").size(13))
                .on_press(Message::ApproveWrites)
                .padding([6, 16]),
        ]
        .spacing(8);

        col = col.push(Space::new().height(Length::Fixed(14.0)));
        col = col.push(actions);

        container(
            scrollable(col)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(6).scroller_width(6).margin(2),
                ))
                .height(Length::Fill),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let header = row![
            text("Settings").size(18),
            Space::new().width(Length::Fill),
            button(text("닫기").size(12)).on_press_maybe(if self.has_key {
                Some(Message::CloseSettings)
            } else {
                None
            }),
        ]
        .align_y(Alignment::Center);

        let key_status = if self.has_key {
            text("OpenRouter 키: 저장됨 ✓").size(13)
        } else {
            text("OpenRouter 키 미등록").size(13)
        };

        let key_input = text_input("sk-or-v1-...", &self.key_input)
            .on_input(Message::KeyInputChanged)
            .on_submit(Message::SaveKey)
            .padding(10)
            .width(Length::Fixed(420.0));

        let actions = row![
            button(text("저장").size(13)).on_press_maybe(
                if self.busy || self.key_input.trim().is_empty() {
                    None
                } else {
                    Some(Message::SaveKey)
                }
            ),
            button(text("삭제").size(13)).on_press_maybe(
                if self.busy || !self.has_key {
                    None
                } else {
                    Some(Message::ClearKey)
                }
            ),
        ]
        .spacing(8);

        // ── Tabby 섹션 ────────────────────────────────────────────
        let tabby_header = text("Tabby (로컬 / 자체 호스팅)").size(14);
        let tabby_url = text_input("http://localhost:8080", &self.tabby_url_input)
            .on_input(Message::TabbyUrlChanged)
            .padding(10)
            .width(Length::Fixed(420.0));
        let tabby_token = text_input("token (선택)", &self.tabby_token_input)
            .on_input(Message::TabbyTokenChanged)
            .padding(10)
            .width(Length::Fixed(420.0));
        let tabby_actions = row![
            button(text("저장").size(13)).on_press_maybe(if self.busy {
                None
            } else {
                Some(Message::SaveTabby)
            }),
            button(text("연결 테스트").size(13)).on_press_maybe(
                if self.busy || self.tabby_url_input.trim().is_empty() {
                    None
                } else {
                    Some(Message::FetchTabbyModels)
                }
            ),
            button(text("삭제").size(13)).on_press_maybe(
                if self.busy
                    || (self.tabby_url_input.is_empty() && self.tabby_token_input.is_empty())
                {
                    None
                } else {
                    Some(Message::ClearTabby)
                }
            ),
        ]
        .spacing(8);
        let tabby_status_label = match &self.tabby_status {
            Some(Ok(label)) => text(format!("연결됨: {}", label)).size(11),
            Some(Err(e)) => text(format!("연결 실패: {}", e)).size(11),
            None => text("미시도").size(11),
        };

        let body = column![
            header,
            Space::new().height(Length::Fixed(12.0)),
            key_status,
            key_input,
            actions,
            Space::new().height(Length::Fixed(8.0)),
            text("키는 OS Credential Manager에 저장됩니다.").size(11),
            text("https://openrouter.ai/keys 에서 발급").size(11),
            Space::new().height(Length::Fixed(20.0)),
            tabby_header,
            tabby_url,
            tabby_token,
            tabby_actions,
            tabby_status_label,
            text("Tabby가 켜져있어야 동작 (기본 8080).").size(11),
        ]
        .spacing(8)
        .max_width(520);

        container(body)
            .padding(28)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_statusbar(&self) -> Element<'_, Message> {
        let model_label = self
            .selected_model
            .clone()
            .unwrap_or_else(|| "(없음)".into());
        let credit_label = match &self.account {
            Some(a) => match (a.usage, a.limit) {
                (Some(u), Some(l)) => format!("잔액: ${:.2} / ${:.2}", (l - u).max(0.0), l),
                (Some(u), None) => format!("사용: ${:.4}", u),
                _ => "잔액: -".into(),
            },
            None => "잔액: -".into(),
        };
        let last_cost_label = match self.last_response_cost {
            Some(c) if c > 0.0 => format!("최근: ${:.4}", c),
            _ => String::new(),
        };
        let mut bar = row![
            text(&self.status).size(11),
            Space::new().width(Length::Fill),
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        if !last_cost_label.is_empty() {
            bar = bar.push(text(last_cost_label).size(11));
        }
        bar = bar
            .push(text(credit_label).size(11))
            .push(text(format!("모델: {}", model_label)).size(11))
            .push(
                text(if self.has_key {
                    "키: 등록됨"
                } else {
                    "키: 미등록"
                })
                .size(11),
            );

        container(bar)
            .padding([4, 14])
            .width(Length::Fill)
            .into()
    }
}
