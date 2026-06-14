// CodeWarp — Iced 진입점
// Phase 2-3a: 3-pane 레이아웃 + 모델 셀렉터 (TopBar) + 입력 echo

mod block;
mod bootstrap;
mod hf;
mod input;
mod keystore;
mod mcp;
mod model;
mod openrouter;
mod palette;
mod pty;
mod runtime_process;
mod session;
mod tabby;
mod tools;
mod update;
mod util;
mod view;

pub(crate) use block::*;
use bootstrap::{
    build_window_icon, JETBRAINS_MONO_BOLD, JETBRAINS_MONO_REGULAR, PRETENDARD_BOLD,
    PRETENDARD_REGULAR, PRETENDARD_SEMIBOLD,
};
pub(crate) use input::on_event;
pub(crate) use model::*;
pub(crate) use palette::*;
#[cfg(test)]
pub(crate) use runtime_process::humanize_inference_spawn_error;
pub(crate) use runtime_process::spawn_inference_stream;
pub(crate) use util::*;

use iced::task;
use iced::widget::markdown::{self, HeadingLevel, Settings as MdSettings, Text as MdText, Viewer};
use iced::widget::operation::snap_to_end;
use iced::widget::scrollable::{Direction, Viewport};
use iced::widget::text_editor::Action;
use iced::widget::{
    button, column, combo_box, container, row, scrollable, text, Id as ScrollId, Space,
};
use iced::{font, Color, Element, Font, Length, Size, Task, Theme};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use openrouter::{AuthKeyData, ChatEvent, ChatMessage, GenerationData, OpenRouterModel};
use view::SIDEBAR_WIDTH;

fn main() -> iced::Result {
    let window_icon = build_window_icon();

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
            icon: window_icon,
            ..Default::default()
        })
        .run()
}

impl Drop for App {
    fn drop(&mut self) {
        // 앱 종료 시 inference child process 정리 (좀비 방지)
        if let Some(pid) = self.inference_pid {
            kill_pid(pid);
        }
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
    for (count, change) in diff.iter_all_changes().enumerate() {
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
        .max_height(720.0)
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

    fn paragraph(&self, settings: MdSettings, text: &MdText) -> Element<'a, Message> {
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

    fn code_block(
        &self,
        _settings: MdSettings,
        language: Option<&'a str>,
        code: &'a str,
        _lines: &'a [MdText],
    ) -> Element<'a, Message> {
        let language_label = language.unwrap_or("text").to_ascii_lowercase();

        let header = row![
            container(
                text(language_label)
                    .size(11)
                    .font(Font::with_name("JetBrains Mono"))
            )
            .padding([2, 8])
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba8(0x30, 0x36, 0x3d, 0.95).into()),
                border: iced::Border {
                    color: Color::from_rgba8(0x58, 0x6e, 0x75, 0.65),
                    width: 1.0,
                    radius: 999.0.into(),
                },
                ..Default::default()
            }),
            Space::new().width(Length::Fill),
            button(
                text("Copy")
                    .size(11)
                    .font(Font::with_name("JetBrains Mono"))
            )
            .on_press(Message::CopyText(code.to_string()))
            .padding([3, 10]),
        ]
        .spacing(8);

        let code_text = container(
            text(code)
                .size(12)
                .line_height(1.35)
                .font(Font::with_name("JetBrains Mono")),
        )
        .padding([12, 14]);

        let code_body = scrollable(code_text)
            .direction(Direction::Horizontal(hscrollbar()))
            .width(Length::Fill);

        container(column![header, code_body].spacing(0))
            .padding(10)
            .width(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgb8(0x0d, 0x11, 0x17).into()),
                border: iced::Border {
                    color: Color::from_rgba8(0x30, 0x36, 0x3d, 0.95),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

/// 진행 중 HF 다운로드의 UI state.
struct HfDownload {
    folder_name: String,
    total_files: usize,
    file_idx: usize,
    file_name: String,
    file_bytes_done: u64,
    file_bytes_total: Option<u64>,
}

struct UiState {
    show_settings: bool,
    settings_tab: SettingsTab,
    show_command_palette: bool,
    command_palette_input: String,
    pending_delete_session: Option<u64>,
    expanded_confirm_idx: Option<usize>,
}

impl UiState {
    fn new(show_settings: bool) -> Self {
        Self {
            show_settings,
            settings_tab: SettingsTab::Provider,
            show_command_palette: false,
            command_palette_input: String::new(),
            pending_delete_session: None,
            expanded_confirm_idx: None,
        }
    }
}

struct ModelFilterState {
    filter_coding: bool,
    filter_reasoning: bool,
    filter_general: bool,
    filter_favorites_only: bool,
    favorites: HashSet<String>,
    sort_mode: SortMode,
}

impl ModelFilterState {
    fn new() -> Self {
        Self {
            filter_coding: true,
            filter_reasoning: true,
            filter_general: true,
            filter_favorites_only: false,
            favorites: session::read_favorites().into_iter().collect(),
            sort_mode: SortMode::Default,
        }
    }
}

#[derive(Default)]
struct McpInputState {
    name_input: String,
    command_input: String,
}

struct App {
    has_key: bool,
    key_input: String,
    /// Tabby base URL 입력값 (Settings 화면). 저장된 값 + 사용자 편집 반영.
    tabby_url_input: String,
    /// Tabby token 입력값 (선택). 비어있으면 인증 없이 호출.
    tabby_token_input: String,
    /// Tabby 토큰 입력란 노출 여부 (대부분 사용자는 토큰 불필요).
    show_tabby_token: bool,
    /// OpenAICompat endpoint 사용자 라벨 (xLLM/TabbyML/TabbyAPI/Local 등). 빈 값이면 [Local].
    openai_compat_label: String,
    /// inference 서버 시작 명령 (Custom 엔진일 때만 사용).
    inference_command_input: String,
    /// 선택된 엔진 (xLLM/vLLM/llama-server/TabbyML/TabbyAPI/Ollama/Custom).
    inference_engine: InferenceEngine,
    /// 엔진 바이너리 절대 경로 (PATH에 없을 때 override). 비어있으면 PATH default.
    inference_binary_path: String,
    /// 받은 모델 폴더 이름 (또는 Tabby/Ollama용 모델 ID).
    inference_selected_model: String,
    inference_port_input: String,
    /// 진행 중인 child process — Some이면 서버 spawn된 상태.
    inference_pid: Option<u32>,
    /// stdout/stderr 마지막 줄 (최대 20줄, FIFO).
    inference_log: std::collections::VecDeque<String>,
    /// 마지막 ping 결과 — None=미시도, Some(Ok)=정상, Some(Err)=실패 사유.
    tabby_status: Option<Result<String, String>>,
    /// Runtime 시작 직후 연결 테스트 자동 재시도 남은 횟수.
    tabby_connect_retry_left: u8,
    /// 자동 재시도 요청 무효화를 위한 세대 카운터.
    tabby_retry_generation: u64,
    status: String,
    busy: bool,
    sidebar_width: f32,

    models: Vec<OpenRouterModel>,
    model_ids: Vec<String>,
    selected_model: Option<String>,
    selected_model_provider: Option<LlmProvider>,
    compare_both: bool,
    compare_pending: bool,

    blocks: Vec<Block>,
    next_block_id: u64,
    input: String,
    streaming_block_id: Option<u64>,
    /// streaming_block_id에 해당하는 block의 self.blocks 내 인덱스 캐시 (O(1) lookup)
    streaming_block_idx: Option<usize>,
    /// 스트리밍 중 누적 raw text (Content rebuild 회피)
    streaming_raw: String,
    /// 진행 중인 chat_stream task의 abort handle (Stop 버튼이 사용).
    abort_handle: Option<task::Handle>,
    hf_abort_handle: Option<task::Handle>,
    ui: UiState,

    // ── HF 모델 매니저 ────────────────────────────────────────────
    hf_token_input: String,
    /// 토큰 입력 토글 (false면 입력란 숨김)
    show_hf_token: bool,
    model_dir_input: String,
    hf_repo_input: String,
    hf_revision: Option<String>,
    hf_folder_name: Option<String>,
    /// 진행 중 다운로드 — 없으면 None
    hf_dl: Option<HfDownload>,

    stream_id: ScrollId,
    follow_bottom: bool,
    /// 활성 세션의 현재 stream scroll y (StreamScrolled로 갱신)
    current_scroll_y: f32,

    /// OpenRouter에 보낼 누적 대화 (도구 호출 round trip 포함)
    conversation: Arc<Vec<ChatMessage>>,
    /// 현재 stream 중 누적되는 tool_calls
    pending_tool_calls: Vec<PendingToolCall>,
    /// 도구 호출 라운드 카운터 (무한 루프 방지)
    tool_round: u32,
    /// mid-stream 오류 재시도 카운터
    mid_stream_retries: u32,
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

    model_filter: ModelFilterState,
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

    // ── 파일 컨텍스트 첨부 ────────────────────────────────────
    /// 전송 대기 중인 첨부 파일 (경로, 내용). Send 후 초기화.
    attached_files: Vec<(PathBuf, String)>,
    /// @-mention 팝업 표시 여부
    show_mention: bool,
    /// '@' 이후 입력 문자열
    mention_query: String,
    /// mention 팝업 후보 파일 목록 (cwd 기준)
    mention_candidates: Vec<PathBuf>,
    /// mention 팝업 현재 선택 인덱스
    mention_selected: usize,

    // ── PTY 터미널 ────────────────────────────────────────────
    /// 터미널 패널 표시 여부
    pty_visible: bool,
    /// PTY 출력 줄 (ANSI strip 후). 최대 PTY_MAX_LINES줄 FIFO.
    pty_output: std::collections::VecDeque<String>,
    /// 터미널 입력창
    pty_input: String,
    /// 활성 PTY 세션 (None이면 셸 미실행)
    pty_session: Option<pty::PtySession>,

    // ── MCP 서버 ──────────────────────────────────────────────
    /// 등록된 MCP 서버 목록
    mcp_servers: Vec<mcp::McpServer>,
    /// 로드된 MCP tool 목록 (모든 서버 합산)
    mcp_tools: Vec<mcp::McpTool>,
    mcp_input: McpInputState,
}

/// 비활성 세션 (메모리 절약 위해 blocks를 plain text로 보관)
#[derive(Debug, Clone)]
struct InactiveSession {
    id: u64,
    title: String,
    conversation: Arc<Vec<ChatMessage>>,
    blocks: Vec<session::PersistedBlock>,
    next_block_id: u64,
    scroll_y: f32,
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
            SortMode::Default => "정렬: 기본",
            SortMode::PriceAsc => "정렬: 가격↑",
            SortMode::PriceDesc => "정렬: 가격↓",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Provider,
    Runtime,
    Models,
    Mcp,
}

#[derive(Debug, Clone)]
enum Message {
    OpenSettings,
    CloseSettings,
    SetSettingsTab(SettingsTab),
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
    CompareResponsesLoaded {
        openrouter_block_id: u64,
        tabby_block_id: u64,
        openrouter_result: Result<String, String>,
        tabby_result: Result<String, String>,
    },
    CopyBlock(u64),
    CopyText(String),
    StreamScrolled(Viewport),
    EditorAction(u64, Action),
    ToggleBlockView(u64),
    LinkClicked(markdown::Uri),
    PickCwd,
    CwdPicked(Option<PathBuf>),
    ApproveWrites,
    DenyWrites,
    /// 인라인 confirm 카드 펼침/접음 토글.
    ToggleConfirmExpand(usize),
    /// 단일 도구 호출만 거부 — pending에서 제거 + denied tool_result 기록.
    DiscardWriteCall(usize),
    ToggleFilterCoding(bool),
    ToggleFilterReasoning(bool),
    ToggleFilterGeneral(bool),
    ToggleFilterFavorites(bool),
    ToggleCompareBoth(bool),
    ToggleFavorite,
    CycleSortMode,
    CycleSidebarWidth,
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
    ToggleTabbyTokenVisible,
    OpenAICompatLabelChanged(String),
    InferenceCommandChanged(String),
    SelectInferenceEngine(InferenceEngine),
    SelectInferenceModel(String),
    InferencePortChanged(String),
    InferenceBinaryChanged(String),
    PickInferenceBinary,
    InferenceBinaryPicked(Option<std::path::PathBuf>),
    InstallTabbyApiRuntime,
    TabbyApiRuntimeInstalled(Result<std::path::PathBuf, String>),
    StartInference,
    StopInference,
    InferenceLogLine(String),
    InferenceExited(i32),
    SaveTabby,
    TabbySaved(Result<(), String>),
    ClearTabby,
    FetchTabbyModels,
    FetchTabbyModelsRetry(u64),
    TabbyModelsLoaded(Result<Vec<String>, String>),
    // ── HF 모델 매니저 ───
    HfTokenChanged(String),
    ToggleHfTokenVisible,
    SaveHfToken,
    HfTokenSaved(Result<(), String>),
    ModelDirChanged(String),
    PickModelDir,
    ModelDirPicked(Option<std::path::PathBuf>),
    HfRepoChanged(String),
    /// 프리셋 클릭 → repo input에 채움
    UsePreset(usize),
    /// EXL2 프리셋 클릭 → 바로 다운로드
    DownloadExl2Preset(usize),
    SelectDownloadedModel(String),
    StartHfDownload,
    HfDownloadEvent(hf::DownloadEvent),
    CancelHfDownload,
    /// 마지막 assistant 응답을 다시 받기.
    RegenerateLast,
    /// 마지막 user 메시지를 입력창으로 옮기고 그 이후 블록/대화 제거.
    EditLastUser,
    /// 코드 블록을 파일에 적용 (block_id, candidate idx).
    ApplyChange(u64, usize),
    // ── 파일 컨텍스트 첨부 ───────────────────────────────────
    /// 창에 파일 드롭됨 (D&D)
    FileDropped(PathBuf),
    /// 드래그 오버 이벤트 (noop — 시각 피드백 미구현)
    FileDragHover,
    /// 파일 읽기 완료 → attached_files에 추가
    FileReadDone(PathBuf, String),
    /// 파일 선택기로 컨텍스트 파일 하나를 첨부
    PickAttachment,
    /// 파일 선택기 결과
    AttachmentPicked(Option<PathBuf>),
    /// 첨부 파일 제거 (인덱스)
    RemoveAttachment(usize),
    /// 첨부 파일 전체 제거
    ClearAttachments,
    /// 주기적 자동 저장 타이머.
    AutoSave,
    /// 창 닫힘 → 세션 저장 + clean shutdown 마커.
    WindowCloseRequested,
    /// mention 팝업 ↑(-1) ↓(+1)
    MentionMove(i32),
    /// mention 팝업에서 선택 확정 (Enter)
    MentionConfirm,
    /// @-mention 파일 후보 목록 로드 완료
    MentionCandidatesLoaded(Vec<PathBuf>),
    /// 파일 첨부 실패 (크기 초과 / 읽기 오류)
    FileAttachError(String),
    // ── MCP ─────────────────────────────────────────────────
    McpNameChanged(String),
    McpCommandChanged(String),
    /// MCP 서버 추가 (현재 입력값으로)
    AddMcpServer,
    /// MCP 서버 제거 (인덱스)
    RemoveMcpServer(usize),
    /// MCP tool 목록 로드 완료 (서버 이름, tool 목록)
    McpToolsLoaded(String, Vec<mcp::McpTool>),
    /// MCP tool 목록 로드 실패
    McpToolsFailed(String),
    /// MCP tool 호출 결과 (tool_call_id, 결과 문자열)
    McpToolResult(String, String),
    // ── PTY 터미널 ──────────────────────────────────────────
    /// 터미널 패널 토글 (Ctrl+`)
    PtyToggle,
    /// PTY 세션 시작
    PtyStart,
    /// PTY 출력 한 줄 도착
    PtyLine(String),
    /// PTY 프로세스 종료
    PtyExited,
    /// 터미널 입력창 변경
    PtyInputChanged(String),
    /// 입력 전송 (Enter)
    PtySend,
    /// Ctrl+C 전송
    PtyCtrlC,
    /// 출력 버퍼 지우기
    PtyClear,
}

impl App {
    pub(crate) fn title(&self) -> String {
        "CodeWarp".to_string()
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::custom(
            "CodeWarp Dark".to_string(),
            iced::theme::Palette {
                background: Color::from_rgb8(0x03, 0x07, 0x12), // obsidian deep navy
                text: Color::from_rgb8(0xe6, 0xec, 0xf8),
                primary: Color::from_rgb8(0x0e, 0xa5, 0xe9), // electric blue
                success: Color::from_rgb8(0x10, 0xb9, 0x81), // mint green
                warning: Color::from_rgb8(0xf5, 0x9e, 0x0b), // amber
                danger: Color::from_rgb8(0xf4, 0x71, 0x74),  // warm red
            },
        )
    }

    pub(crate) fn new() -> (Self, Task<Message>) {
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
            tabby_token_input: saved_tabby_token.clone(),
            show_tabby_token: !saved_tabby_token.trim().is_empty(),
            openai_compat_label: keystore::read_openai_compat_label().unwrap_or_default(),
            inference_command_input: keystore::read_inference_command().unwrap_or_default(),
            inference_engine: InferenceEngine::XLlm,
            inference_binary_path: keystore::read_inference_binary().unwrap_or_default(),
            inference_selected_model: String::new(),
            inference_port_input: "9000".into(),
            inference_pid: None,
            inference_log: std::collections::VecDeque::new(),
            tabby_status: None,
            tabby_connect_retry_left: 0,
            tabby_retry_generation: 0,
            status,
            busy: false,
            sidebar_width: SIDEBAR_WIDTH,
            models: Vec::new(),
            model_ids: Vec::new(),
            selected_model: saved_model,
            selected_model_provider: None,
            compare_both: false,
            compare_pending: false,
            blocks: Vec::new(),
            next_block_id: 0,
            input: String::new(),
            streaming_block_id: None,
            streaming_block_idx: None,
            streaming_raw: String::new(),
            abort_handle: None,
            hf_abort_handle: None,
            ui: UiState::new(!has_key),
            hf_token_input: keystore::read_hf_token().unwrap_or_default(),
            show_hf_token: false,
            model_dir_input: keystore::read_model_dir().unwrap_or_else(|| {
                dirs::data_local_dir()
                    .map(|d| d.join("codewarp").join("models").display().to_string())
                    .unwrap_or_default()
            }),
            hf_repo_input: String::new(),
            hf_revision: None,
            hf_folder_name: None,
            hf_dl: None,
            stream_id: ScrollId::new("stream"),
            follow_bottom: true,
            current_scroll_y: 0.0,
            conversation: Arc::new(Vec::new()),
            pending_tool_calls: Vec::new(),
            tool_round: 0,
            mid_stream_retries: 0,
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
            model_filter: ModelFilterState::new(),
            agent_mode: AgentMode::Plan,
            inactive_sessions: Vec::new(),
            current_session_id: 1,
            current_session_title: String::new(),
            next_session_id: 1,
            usage: session::load_usage(),
            last_response_cost: None,
            attached_files: Vec::new(),
            show_mention: false,
            pty_visible: false,
            pty_output: std::collections::VecDeque::new(),
            pty_input: String::new(),
            pty_session: None,
            mcp_servers: mcp::load_servers(),
            mcp_tools: Vec::new(),
            mcp_input: McpInputState::default(),
            mention_query: String::new(),
            mention_candidates: Vec::new(),
            mention_selected: 0,
        };

        let should_auto_attach_tabbyapi = app.openai_compat_label.eq_ignore_ascii_case("TabbyAPI")
            || app.tabby_url_input.contains(":5000");
        if should_auto_attach_tabbyapi && app.inference_binary_path.trim().is_empty() {
            if let Some(launcher) =
                update::find_tabbyapi_launcher(&update::default_tabbyapi_runtime_dir())
            {
                app.inference_engine = InferenceEngine::TabbyApi;
                app.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                if app.tabby_url_input.trim().is_empty() {
                    app.tabby_url_input = format!("http://localhost:{}", TABBY_API_DEFAULT_PORT);
                }
                app.inference_binary_path = launcher.display().to_string();
                let _ = keystore::write_inference_binary(&app.inference_binary_path);
            }
        }

        // 멀티 세션 복원
        let mut persisted = session::load_all();
        if persisted.sessions.is_empty() {
            persisted = session::load_all(); // 빈 → default 채워짐
        }
        let active_idx = persisted
            .active_idx
            .min(persisted.sessions.len().saturating_sub(1));
        let active = persisted.sessions[active_idx].clone();
        let inactive: Vec<InactiveSession> = persisted
            .sessions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != active_idx)
            .map(|(_, s)| InactiveSession {
                id: s.id,
                title: s.title.clone(),
                conversation: Arc::new(s.conversation.clone()),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();

        app.current_session_id = active.id;
        app.current_session_title = active.title;
        app.conversation = Arc::new(active.conversation);
        app.next_block_id = active.next_block_id;
        app.blocks = active.blocks.into_iter().map(persisted_to_block).collect();
        app.current_scroll_y = active.scroll_y;
        app.inactive_sessions = inactive;
        app.next_session_id = persisted.sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        // Crash recovery: 이전 종료가 비정상이었다면 상태에 표시
        if !session::was_clean_shutdown() && !app.blocks.is_empty() {
            app.status = format!("[복구됨] {}", app.status);
        }
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
        // 저장된 inference 명령 있으면 boot 시 자동 시작
        if !app.inference_command_input.trim().is_empty() {
            tasks.push(Task::done(Message::StartInference));
        }
        // 등록된 MCP 서버 tool 목록 로드
        for server in app.mcp_servers.clone() {
            let name = server.name.clone();
            tasks.push(Task::perform(
                async move {
                    mcp::list_tools(&server)
                        .await
                        .map(|tools| (name.clone(), tools))
                        .map_err(|e| format!("[{name}] {e}"))
                },
                |r| match r {
                    Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                    Err(msg) => Message::McpToolsFailed(msg),
                },
            ));
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
}

#[cfg(test)]
mod main_tests;
