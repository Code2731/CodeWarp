// CodeWarp — Iced 진입점
// Phase 2-3a: 3-pane 레이아웃 + 모델 셀렉터 (TopBar) + 입력 echo

mod keystore;
mod openrouter;
mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::markdown::{self, HeadingLevel, Settings as MdSettings, Text as MdText, Viewer};
use iced::widget::operation::snap_to_end;
use iced::widget::scrollable::{Direction, Scrollbar, Viewport};
use iced::widget::text_editor::{Action, Edit};
use iced::widget::{
    button, column, combo_box, container, row, scrollable, text, text_editor, text_input,
    Id as ScrollId, Space,
};
use iced::{font, Alignment, Color, Element, Font, Length, Size, Task, Theme};

use openrouter::{ChatEvent, ChatMessage, OpenRouterModel};

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

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
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

/// markdown::view_with용 커스텀 Viewer. heading은 Bold weight 강제.
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
    status: String,
    busy: bool,

    models: Vec<OpenRouterModel>,
    model_ids: Vec<String>,
    selected_model: Option<String>,

    blocks: Vec<Block>,
    next_block_id: u64,
    input: String,
    streaming_block_id: Option<u64>,

    show_settings: bool,

    stream_id: ScrollId,
    follow_bottom: bool,

    /// OpenRouter에 보낼 누적 대화 (도구 호출 round trip 포함)
    conversation: Vec<ChatMessage>,
    /// 현재 stream 중 누적되는 tool_calls
    pending_tool_calls: Vec<PendingToolCall>,
    /// 도구 호출 라운드 카운터 (무한 루프 방지)
    tool_round: u32,
    /// 도구 실행 시 기준이 되는 작업 디렉토리
    cwd: PathBuf,

    /// 검색 가능한 모델 셀렉터(combo_box) 상태
    model_combo_state: combo_box::State<String>,

    /// 사용자 승인 대기 중인 mutating tool 호출 목록 (write_file 등)
    pending_write_calls: Vec<PendingToolCall>,
    show_write_confirm: bool,
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
    SelectModel(String),
    InputChanged(String),
    Send,
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
}

impl App {
    fn title(&self) -> String {
        "CodeWarp".to_string()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn new() -> (Self, Task<Message>) {
        let has_key = keystore::has_api_key();
        let saved_model = keystore::read_selected_model();
        let status = if has_key {
            "준비됨".into()
        } else {
            "OpenRouter API 키 미등록".into()
        };
        let app = Self {
            has_key,
            key_input: String::new(),
            status,
            busy: false,
            models: Vec::new(),
            model_ids: Vec::new(),
            selected_model: saved_model,
            blocks: Vec::new(),
            next_block_id: 0,
            input: String::new(),
            streaming_block_id: None,
            show_settings: !has_key,
            stream_id: ScrollId::new("stream"),
            follow_bottom: true,
            conversation: Vec::new(),
            pending_tool_calls: Vec::new(),
            tool_round: 0,
            cwd: keystore::read_cwd()
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from(".")),
            model_combo_state: combo_box::State::new(Vec::new()),
            pending_write_calls: Vec::new(),
            show_write_confirm: false,
        };
        let task = if has_key {
            Task::done(Message::FetchModels)
        } else {
            Task::none()
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
                        self.model_combo_state =
                            combo_box::State::new(self.model_ids.clone());
                        // 저장된 모델이 리스트에 있으면 유지, 없으면 첫 번째로 fallback
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
            Message::SelectModel(id) => {
                let _ = keystore::write_selected_model(&id);
                self.selected_model = Some(id);
                Task::none()
            }
            Message::InputChanged(v) => {
                self.input = v;
                Task::none()
            }
            Message::Send => {
                let text = self.input.trim().to_string();
                if text.is_empty() || self.selected_model.is_none() || self.streaming_block_id.is_some() {
                    return Task::none();
                }
                let api_key = match keystore::read_api_key() {
                    Ok(k) => k,
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
                });
                let ai_id = self.next_id();
                self.blocks.push(Block {
                    id: ai_id,
                    body: BlockBody::Assistant(text_editor::Content::new()),
                    view_mode: ViewMode::Rendered,
                    md_items: Vec::new(),
                });
                self.streaming_block_id = Some(ai_id);
                self.input.clear();
                self.status = "응답 생성 중…".into();
                self.follow_bottom = true; // 새 메시지 전송 시 follow ON

                Task::batch(vec![
                    snap_to_end(self.stream_id.clone()),
                    Task::run(
                        openrouter::chat_stream(
                            api_key,
                            model,
                            messages,
                            Some(tools::tool_definitions()),
                        ),
                        Message::ChatChunk,
                    ),
                ])
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
                    ChatEvent::Done { finish_reason } => {
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
                        self.pending_tool_calls.clear();
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

    /// conversation 첫 위치에 cwd를 알려주는 system 메시지를 보장 (없으면 추가, 있으면 갱신).
    fn ensure_system_message(&mut self) {
        let prompt = format!(
            "당신은 CodeWarp의 코딩 어시스턴트입니다.\n\n\
            작업 디렉토리: '{}'\n\n\
            사용 가능한 도구 (적극적으로 호출하세요):\n\
            - read_file(path): 파일 내용 읽기 (즉시 실행)\n\
            - write_file(path, content): 파일 작성/덮어쓰기 (사용자 승인 후 실행)\n\
            - glob(pattern): 패턴 매칭 파일 리스트 (예: '**/*.rs', 'examples/**/*')\n\
            - grep(pattern): 정규식으로 모든 파일 검색\n\n\
            규칙:\n\
            1. 파일 시스템을 살펴봐야 할 때는 '확인하겠습니다' 같은 말 없이 즉시 도구를 호출하세요.\n\
            2. 새 파일을 만들기 전에 glob으로 기존 구조를 먼저 확인하세요.\n\
            3. 모든 path 인자는 작업 디렉토리 기준 상대 경로 (절대 경로 거부).\n\
            4. 도구 결과를 받은 뒤 그것을 근거로 한국어로 답하세요.",
            self.cwd.display()
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
        self.kick_chat_stream()
    }

    /// 누적된 conversation을 가지고 다음 chat_stream을 시작.
    fn kick_chat_stream(&mut self) -> Task<Message> {
        let api_key = match keystore::read_api_key() {
            Ok(k) => k,
            Err(e) => {
                self.status = e;
                self.streaming_block_id = None;
                return Task::none();
            }
        };
        let model = self.selected_model.clone().unwrap_or_default();
        let messages = self.conversation.clone();
        Task::run(
            openrouter::chat_stream(
                api_key,
                model,
                messages,
                Some(tools::tool_definitions()),
            ),
            Message::ChatChunk,
        )
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }

    fn view(&self) -> Element<'_, Message> {
        let topbar = self.view_topbar();

        let middle: Element<Message> = if self.show_settings {
            self.view_settings()
        } else if self.show_write_confirm {
            self.view_write_confirm()
        } else {
            row![
                self.view_sidebar(),
                self.view_stream(),
                self.view_rightpanel(),
            ]
            .height(Length::Fill)
            .into()
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
            iced::widget::container(
                combo_box(
                    &self.model_combo_state,
                    "모델 검색…",
                    self.selected_model.as_ref(),
                    Message::SelectModel,
                )
                .size(12),
            )
            .width(Length::Fixed(420.0))
            .into()
        };

        let bar = row![
            Space::new().width(Length::Fill),
            model_picker,
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

        let body = column![
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
                let header = row![
                    text(role_label).size(11),
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
                        let mut settings: markdown::Settings = (&Theme::Dark).into();
                        settings.style.inline_code_font = Font::with_name("JetBrains Mono");
                        settings.style.code_block_font = Font::with_name("JetBrains Mono");
                        markdown::view_with(b.md_items.iter(), settings, &CodewarpViewer)
                    }
                };

                let block_view = container(
                    column![header, body_view].spacing(6),
                )
                .padding(12)
                .width(Length::Fill);
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
        let input_row = row![
            text_input("질문을 입력하세요…", &self.input)
                .on_input(Message::InputChanged)
                .on_submit(Message::Send)
                .padding(10),
            button(text("Send").size(13)).on_press_maybe(if send_disabled {
                None
            } else {
                Some(Message::Send)
            }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        column![
            container(blocks_view)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([14, 18]),
            container(input_row)
                .padding([10, 14])
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

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
            let card: Element<Message> = match tools::WriteFileArgs::parse(&tc.arguments) {
                Ok(args) => {
                    // 기존 파일 (있으면) 읽어서 diff 표시, 없으면 새 파일 표시
                    let abs_path = self.cwd.join(&args.path);
                    let old_content = std::fs::read_to_string(&abs_path).ok();
                    let header = match &old_content {
                        Some(_) => format!("📝 {} ({} bytes)", args.path, args.content.len()),
                        None => format!("✨ 새 파일: {} ({} bytes)", args.path, args.content.len()),
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

        let body = column![
            header,
            Space::new().height(Length::Fixed(12.0)),
            key_status,
            key_input,
            actions,
            Space::new().height(Length::Fixed(8.0)),
            text("키는 OS Credential Manager에 저장됩니다.").size(11),
            text("https://openrouter.ai/keys 에서 발급").size(11),
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
        let bar = row![
            text(&self.status).size(11),
            Space::new().width(Length::Fill),
            text(format!("모델: {}", model_label)).size(11),
            text(if self.has_key {
                "키: 등록됨"
            } else {
                "키: 미등록"
            })
            .size(11),
        ]
        .spacing(14)
        .align_y(Alignment::Center);

        container(bar)
            .padding([4, 14])
            .width(Length::Fill)
            .into()
    }
}
