// CodeWarp — Iced 진입점
// Phase 2-3a: 3-pane 레이아웃 + 모델 셀렉터 (TopBar) + 입력 echo

mod hf;
mod keystore;
mod mcp;
mod openrouter;
mod session;
mod tabby;
mod tools;
mod update;
mod view;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::markdown::{self, HeadingLevel, Settings as MdSettings, Text as MdText, Viewer};
use iced::widget::operation::snap_to_end;
use iced::widget::scrollable::{Scrollbar, Viewport};
use iced::widget::text_editor::{Action, Edit};
use iced::widget::{
    column, combo_box, container, text,
    text_editor, Id as ScrollId,
};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use iced::task;
use iced::{font, Color, Element, Font, Length, Size, Task, Theme};

use openrouter::{AuthKeyData, ChatEvent, ChatMessage, GenerationData, OpenRouterModel};

/// 모델을 어느 백엔드로 라우팅할지. OpenAICompat은 사용자 임의 endpoint
/// (xLLM / vLLM / Tabby / llama-server / Ollama 등 — 모두 OpenAI 호환).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LlmProvider {
    OpenRouter,
    OpenAICompat,
}

/// combo_box에 표시할 모델 항목 (가격 정보 포함).
/// Display 형식: "[OR][KO]★ model-id  128k  $in/$out" 또는 "[xLLM] model-id  free"
#[derive(Debug, Clone, PartialEq)]
struct ModelOption {
    id: String,
    provider: LlmProvider,
    /// OpenAICompat의 사용자 지정 라벨 (xLLM/Tabby/Local 등). 빈 값이면 "Local".
    /// OpenRouter일 땐 무의미 (Display에서 사용 안 함).
    provider_label: String,
    /// 한국어 토크나이저 친화 모델 휴리스틱 결과
    ko_friendly: bool,
    /// 즐겨찾기 여부 (refresh_model_combo에서 self.favorites 기준으로 set)
    favorite: bool,
    /// context window 토큰 수 (있을 때만 표시)
    context_length: Option<u64>,
    /// 입력 100만 토큰당 USD
    prompt_per_million: Option<f64>,
    /// 출력 100만 토큰당 USD
    completion_per_million: Option<f64>,
}

fn vscrollbar() -> Scrollbar {
    Scrollbar::new().width(8).scroller_width(8).margin(2)
}

/// 바이트 수를 KB/MB/GB 단위로 표시 (1024 진법).
fn fmt_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.0} KB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}

fn fmt_context_length(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag: String = match self.provider {
            LlmProvider::OpenRouter => "[OR]".into(),
            LlmProvider::OpenAICompat => {
                let label = self.provider_label.trim();
                if label.is_empty() {
                    "[Local]".into()
                } else {
                    format!("[{}]", label)
                }
            }
        };
        let ko = if self.ko_friendly { "[KO]" } else { "" };
        let star = if self.favorite { "★" } else { "" };
        let ctx = self
            .context_length
            .map(|n| format!("  {}", fmt_context_length(n)))
            .unwrap_or_default();
        match (self.prompt_per_million, self.completion_per_million) {
            (Some(p), Some(c)) if p == 0.0 && c == 0.0 => {
                write!(f, "{}{}{} {}{}  free", tag, ko, star, self.id, ctx)
            }
            (Some(p), Some(c)) => {
                write!(f, "{}{}{} {}{}  ${:.2}/${:.2}", tag, ko, star, self.id, ctx, p, c)
            }
            _ => write!(f, "{}{}{} {}{}", tag, ko, star, self.id, ctx),
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

/// input 문자열에서 마지막 '@' 이후 mention query 추출.
/// 공백·개행이 포함되면 None (이미 완성된 멘션이므로 팝업 불필요).
fn extract_mention_query(input: &str) -> Option<&str> {
    let at_pos = input.rfind('@')?;
    let rest = &input[at_pos + 1..];
    if rest.bytes().any(|b| matches!(b, b' ' | b'\n' | b'\t')) {
        return None;
    }
    Some(rest)
}

/// PathBuf 목록을 query로 fuzzy filter (대소문자 무시, 부분 포함).
fn fuzzy_match_paths(candidates: &[PathBuf], query: &str, max_results: usize) -> Vec<PathBuf> {
    if query.is_empty() {
        return candidates.iter().take(max_results).cloned().collect();
    }
    let q = query.to_lowercase();
    candidates
        .iter()
        .filter(|p| p.to_string_lossy().to_lowercase().contains(&q))
        .take(max_results)
        .cloned()
        .collect()
}

/// 첨부 파일 목록을 코드 펜스 블록 컨텍스트 문자열로 변환.
fn build_file_context(files: &[(PathBuf, String)]) -> String {
    files
        .iter()
        .map(|(path, content)| {
            let name = path.display();
            format!("```{name}\n{content}\n```")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// cwd 기준으로 파일 목록을 비동기 수집 (최대 200개, max_depth=5).
async fn collect_mention_candidates(cwd: PathBuf) -> Vec<PathBuf> {
    tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for entry in ignore::WalkBuilder::new(&cwd).max_depth(Some(5)).build() {
            let entry = match entry { Ok(e) => e, Err(_) => continue };
            if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
            if let Ok(rel) = entry.path().strip_prefix(&cwd) {
                results.push(rel.to_path_buf());
            }
            if results.len() >= 200 { break; }
        }
        results
    })
    .await
    .unwrap_or_default()
}

// 첨부 파일 크기 상한 (512 KB 초과 시 거부)
const MAX_ATTACH_BYTES: u64 = 512 * 1024;

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

/// keyboard + window 이벤트를 하나의 listen_with로 통합.
/// (listen_with는 fn pointer만 받으므로 closure 캡처 불가)
fn on_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            match key.as_ref() {
                Key::Named(Named::ArrowUp) => Some(Message::MentionMove(-1)),
                Key::Named(Named::ArrowDown) => Some(Message::MentionMove(1)),
                _ => handle_key(key, modifiers),
            }
        }
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        iced::Event::Window(iced::window::Event::FileHovered(_)) => {
            Some(Message::FileDragHover)
        }
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
            Some(Message::FileDragHover)
        }
        _ => None,
    }
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
    /// 도구 호출 실행 결과 (휘발성 — 세션 저장 안 됨, 시각 알림용).
    ToolResult {
        name: String,
        summary: String,
        success: bool,
    },
}

impl BlockBody {
    fn role_label(&self) -> &'static str {
        match self {
            BlockBody::User(_) => "you",
            BlockBody::Assistant(_) => "ai",
            BlockBody::ToolResult { .. } => "tool",
        }
    }

    fn to_text(&self) -> String {
        match self {
            BlockBody::User(s) => s.clone(),
            BlockBody::Assistant(c) => c.text(),
            BlockBody::ToolResult { summary, .. } => summary.clone(),
        }
    }

    fn is_empty_for_history(&self) -> bool {
        match self {
            BlockBody::User(s) => s.trim().is_empty(),
            BlockBody::Assistant(c) => c.text().trim().is_empty(),
            BlockBody::ToolResult { .. } => false,
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
    /// 응답 끝난 후 추출된 Apply 가능한 변경사항 + 적용 여부.
    apply_candidates: Vec<(ApplyCandidate, bool)>,
}

impl Drop for App {
    fn drop(&mut self) {
        // 앱 종료 시 inference child process 정리 (좀비 방지)
        if let Some(pid) = self.inference_pid {
            kill_pid(pid);
        }
    }
}

/// inference 서버를 child process로 spawn + stdout/stderr line stream으로 emit.
/// 첫 message는 `[pid:NNN]` 형식 (App가 inference_pid 저장).
fn spawn_inference_stream(
    program: String,
    args: Vec<String>,
) -> impl futures_util::Stream<Item = Message> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    async_stream::stream! {
        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                yield Message::InferenceLogLine(format!("[spawn 실패] {}: {}", program, e));
                yield Message::InferenceExited(-1);
                return;
            }
        };
        if let Some(pid) = child.id() {
            yield Message::InferenceLogLine(format!("[pid:{}] {} {}", pid, program, args.join(" ")));
        }
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        if let Some(out) = stdout {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(out).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        if let Some(err) = stderr {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(err).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(format!("[err] {}", line)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        // child 종료 + 로그 라인을 동시 처리
        let mut child_done = false;
        let mut exit_code: i32 = 0;
        loop {
            tokio::select! {
                line = rx.recv() => {
                    match line {
                        Some(l) => yield Message::InferenceLogLine(l),
                        None => {
                            if child_done { break; }
                        }
                    }
                }
                status = child.wait(), if !child_done => {
                    child_done = true;
                    exit_code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
                }
            }
        }
        yield Message::InferenceExited(exit_code);
    }
}

/// inference 엔진 종류 — 사용자가 dropdown으로 선택.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferenceEngine {
    XLlm,
    VLlm,
    LlamaServer,
    Tabby,
    /// daemon 형태 — 이미 떠있다고 가정, CodeWarp는 spawn 안 함.
    Ollama,
    /// 사용자가 직접 명령 입력
    Custom,
}

impl InferenceEngine {
    const ALL: &'static [InferenceEngine] = &[
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::Tabby,
        InferenceEngine::Ollama,
        InferenceEngine::Custom,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::XLlm => "xLLM",
            Self::VLlm => "vLLM",
            Self::LlamaServer => "llama-server",
            Self::Tabby => "Tabby",
            Self::Ollama => "Ollama (이미 떠있는 daemon)",
            Self::Custom => "Custom (직접 명령)",
        }
    }

    fn default_port(&self) -> u16 {
        match self {
            Self::Tabby => 8080,
            Self::Ollama => 11434,
            _ => 9000,
        }
    }

    /// 모델 path/ID + port를 받아 spawn할 Command 인자 리스트 반환.
    /// `None`이면 spawn 안 함 (Ollama는 외부 daemon, Custom은 사용자 정의).
    fn compose_command(&self, model: &str, port: u16) -> Option<Vec<String>> {
        let port_s = port.to_string();
        match self {
            Self::XLlm => Some(vec![
                "xllm".into(),
                "serve".into(),
                "--model".into(),
                model.into(),
                "--port".into(),
                port_s,
            ]),
            Self::VLlm => Some(vec![
                "vllm".into(),
                "serve".into(),
                model.into(),
                "--port".into(),
                port_s,
            ]),
            Self::LlamaServer => Some(vec![
                "llama-server".into(),
                "-m".into(),
                model.into(),
                "--port".into(),
                port_s,
            ]),
            Self::Tabby => Some(vec![
                "tabby".into(),
                "serve".into(),
                "--model".into(),
                model.into(),
            ]),
            Self::Ollama | Self::Custom => None,
        }
    }
}

impl std::fmt::Display for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 모델 매니저 다운로드 폴더 안의 받은 모델(서브폴더) 리스트.
/// 빈 폴더는 모델 아님 — skip.
fn list_downloaded_models(dir: &std::path::Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // 빈 폴더 skip
        let has_files = std::fs::read_dir(&path)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false);
        if !has_files {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            out.push(name.to_string());
        }
    }
    out
}

/// 윈도우는 taskkill /T /F (자식 트리 포함), 그 외는 kill SIGTERM.
fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status();
    }
}

/// AI 응답의 fenced code block 첫 줄에서 `// path: ...` 또는 `# path: ...`를
/// 검사해 적용 후보를 추출. 닫는 fence가 없거나 path가 첫 줄이 아니면 skip.
#[derive(Debug, Clone, PartialEq)]
struct ApplyCandidate {
    path: String,
    language: String,
    content: String,
}

fn extract_path_from_comment(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    for prefix in ["//", "#", "--"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.trim_start();
            if let Some(p) = rest.strip_prefix("path:") {
                let path = p.trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn parse_apply_candidates(markdown: &str) -> Vec<ApplyCandidate> {
    let mut out = Vec::new();
    let mut lines = markdown.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("```") else {
            continue;
        };
        // fence 시작 — language는 fence info의 첫 단어
        let language = rest.trim().split_whitespace().next().unwrap_or("").to_string();
        // 첫 본문 라인에서 path 추출
        let Some(first) = lines.next() else { break };
        let Some(path) = extract_path_from_comment(first) else {
            // path 없는 코드 블록 — 닫는 fence까지 skip
            for inner in lines.by_ref() {
                if inner.trim_start().starts_with("```") {
                    break;
                }
            }
            continue;
        };
        // 본문 수집 — 닫는 fence까지
        let mut content = String::new();
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                closed = true;
                break;
            }
            content.push_str(inner);
            content.push('\n');
        }
        if closed && !content.is_empty() {
            out.push(ApplyCandidate {
                path,
                language,
                content,
            });
        }
    }
    out
}

/// 마지막 user 메시지 다음의 모든 메시지를 conversation에서 제거.
/// regenerate 또는 edit 직전 호출. user가 전혀 없으면 conversation을 비움.
fn truncate_after_last_user(conv: &mut Vec<crate::openrouter::ChatMessage>) {
    while let Some(last) = conv.last() {
        if last.role == "user" {
            return;
        }
        conv.pop();
    }
}

/// 가장 마지막 BlockBody::User 인덱스 (없으면 None).
fn last_user_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .rposition(|b| matches!(b.body, BlockBody::User(_)))
}

/// 가장 마지막 BlockBody::Assistant 인덱스 (없으면 None).
fn last_assistant_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .rposition(|b| matches!(b.body, BlockBody::Assistant(_)))
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
        apply_candidates: Vec::new(),
    }
}

/// 도구 호출 결과를 ToolResult 칩에 표시할 한 줄 요약 + 성공 여부로 변환.
fn summarize_tool_result(name: &str, args_json: &str, result: &str) -> (String, bool) {
    let lower = result.to_ascii_lowercase();
    let success = !(result.starts_with("Error")
        || lower.contains("[err]")
        || lower.starts_with("error"));
    let summary = match name {
        "write_file" => tools::WriteFileArgs::parse(args_json)
            .map(|a| format!("{} ({} bytes)", a.path, a.content.len()))
            .unwrap_or_else(|_| "?".into()),
        "run_command" => tools::RunCommandArgs::parse(args_json)
            .map(|a| format!("$ {}", a.command.chars().take(60).collect::<String>()))
            .unwrap_or_else(|_| "?".into()),
        _ => result.lines().next().unwrap_or("").chars().take(80).collect(),
    };
    (summary, success)
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

/// 진행 중 HF 다운로드의 UI state.
struct HfDownload {
    repo_id: String,
    total_files: usize,
    file_idx: usize,
    file_name: String,
    file_bytes_done: u64,
    file_bytes_total: Option<u64>,
}

/// 추천 프리셋 — 클릭 시 hf_repo_input에 채움.
struct ModelPreset {
    repo_id: &'static str,
    label: &'static str,
    note: &'static str,
}
// xLLM / vLLM / llama-server 등 자체 띄울 OpenAI 호환 백엔드용 HF 본판 모델.
// Tabby는 자체 카탈로그·cache를 사용하므로 Tabby를 위해서는 이 매니저 대신
// `tabby serve --model X` 명령을 직접 실행 (그러면 Tabby가 자동 다운로드).
const MODEL_PRESETS: &[ModelPreset] = &[
    ModelPreset {
        repo_id: "Qwen/Qwen2.5-Coder-7B-Instruct",
        label: "Qwen2.5-Coder 7B Instruct",
        note: "코딩 + 한국어 친화 (xLLM/vLLM)",
    },
    ModelPreset {
        repo_id: "Qwen/Qwen2.5-7B-Instruct",
        label: "Qwen2.5 7B Instruct",
        note: "범용 + 한국어 친화 (xLLM/vLLM)",
    },
    ModelPreset {
        repo_id: "LGAI-EXAONE/EXAONE-3.5-7.8B-Instruct",
        label: "EXAONE 3.5 7.8B",
        note: "한국어 특화 (LG AI)",
    },
    ModelPreset {
        repo_id: "upstage/SOLAR-10.7B-Instruct-v1.0",
        label: "SOLAR 10.7B",
        note: "한국어 친화 (Upstage)",
    },
    ModelPreset {
        repo_id: "deepseek-ai/DeepSeek-Coder-V2-Lite-Instruct",
        label: "DeepSeek-Coder V2 Lite",
        note: "코딩 (16B-MoE 활성 2.4B)",
    },
];

/// EXL2 프리셋 — TabbyAPI용. 클릭하면 해당 branch를 바로 다운로드.
struct Exl2Preset {
    repo_id: &'static str,
    revision: &'static str, // HF branch (bpw 수치)
    folder_name: &'static str, // models 폴더 아래 저장될 이름
    label: &'static str,
    note: &'static str,
    vram: &'static str,
}

const EXL2_PRESETS: &[Exl2Preset] = &[
    Exl2Preset {
        repo_id: "turboderp/Llama-3.2-1B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.2-1B-Instruct-4.0bpw",
        label: "Llama 3.2 1B Instruct",
        note: "검증·테스트용 초소형",
        vram: "~600MB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.2-3B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.2-3B-Instruct-4.0bpw",
        label: "Llama 3.2 3B Instruct",
        note: "소형 범용",
        vram: "~1.8GB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.1-8B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.1-8B-Instruct-4.0bpw",
        label: "Llama 3.1 8B Instruct 4bpw",
        note: "RTX 3080 최적 균형",
        vram: "~5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.1-8B-Instruct-exl2",
        revision: "6.0bpw",
        folder_name: "Llama-3.1-8B-Instruct-6.0bpw",
        label: "Llama 3.1 8B Instruct 6bpw",
        note: "품질 우선 (RTX 3080 10GB 내)",
        vram: "~7.5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/gemma-2-9b-it-exl2",
        revision: "4.0bpw",
        folder_name: "Gemma-2-9B-it-4.0bpw",
        label: "Gemma 2 9B Instruct",
        note: "Google 범용 (강력한 instruction following)",
        vram: "~5.5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/gemma-3-12b-it-exl2",
        revision: "4.0bpw",
        folder_name: "Gemma-3-12B-it-4.0bpw",
        label: "Gemma 3 12B Instruct",
        note: "최신 Gemma 3 (멀티모달 지원)",
        vram: "~7GB",
    },
];

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
    /// Tabby 토큰 입력란 노출 여부 (대부분 사용자는 토큰 불필요).
    show_tabby_token: bool,
    /// OpenAICompat endpoint 사용자 라벨 (xLLM/Tabby/Local 등). 빈 값이면 [Local].
    openai_compat_label: String,
    /// inference 서버 시작 명령 (Custom 엔진일 때만 사용).
    inference_command_input: String,
    /// 선택된 엔진 (xLLM/vLLM/llama-server/Tabby/Ollama/Custom).
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
    /// 인라인 confirm에서 펼친 카드 인덱스 (한 번에 하나만 펼침).
    expanded_confirm_idx: Option<usize>,

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

    // ── MCP 서버 ──────────────────────────────────────────────
    /// 등록된 MCP 서버 목록
    mcp_servers: Vec<mcp::McpServer>,
    /// 로드된 MCP tool 목록 (모든 서버 합산)
    mcp_tools: Vec<mcp::McpTool>,
    /// Settings MCP 섹션 입력 상태
    mcp_name_input: String,
    mcp_command_input: String,
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
            SortMode::Default => "정렬: 기본",
            SortMode::PriceAsc => "정렬: 가격↑",
            SortMode::PriceDesc => "정렬: 가격↓",
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
    /// 인라인 confirm 카드 펼침/접음 토글.
    ToggleConfirmExpand(usize),
    /// 단일 도구 호출만 거부 — pending에서 제거 + denied tool_result 기록.
    DiscardWriteCall(usize),
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
    ToggleTabbyTokenVisible,
    OpenAICompatLabelChanged(String),
    InferenceCommandChanged(String),
    SelectInferenceEngine(InferenceEngine),
    SelectInferenceModel(String),
    InferencePortChanged(String),
    InferenceBinaryChanged(String),
    PickInferenceBinary,
    InferenceBinaryPicked(Option<std::path::PathBuf>),
    StartInference,
    StopInference,
    InferenceLogLine(String),
    InferenceExited(i32),
    SaveTabby,
    TabbySaved(Result<(), String>),
    ClearTabby,
    FetchTabbyModels,
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
    /// 첨부 파일 제거 (인덱스)
    RemoveAttachment(usize),
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
    /// MCP tool 호출 결과 (tool_call_id, 결과 문자열)
    McpToolResult(String, String),
}

impl App {
    pub(crate) fn title(&self) -> String {
        "CodeWarp".to_string()
    }

    pub(crate) fn theme(&self) -> Theme {
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
            expanded_confirm_idx: None,
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
            attached_files: Vec::new(),
            show_mention: false,
            mcp_servers: mcp::load_servers(),
            mcp_tools: Vec::new(),
            mcp_name_input: String::new(),
            mcp_command_input: String::new(),
            mention_query: String::new(),
            mention_candidates: Vec::new(),
            mention_selected: 0,
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
        // 저장된 inference 명령 있으면 boot 시 자동 시작
        if !app.inference_command_input.trim().is_empty() {
            tasks.push(Task::done(Message::StartInference));
        }
        // 등록된 MCP 서버 tool 목록 로드
        for server in app.mcp_servers.clone() {
            let name = server.name.clone();
            tasks.push(Task::perform(
                async move { mcp::list_tools(&server).await },
                move |r| match r {
                    Ok(tools) => Message::McpToolsLoaded(name, tools),
                    Err(_) => Message::McpToolsLoaded(String::new(), Vec::new()),
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
mod tests {
    use super::*;

    // ── fmt_bytes ───────────────────────────────────────────────────

    #[test]
    fn fmt_bytes_units() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1024), "1 KB");
        assert_eq!(fmt_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(fmt_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn fmt_bytes_large_gb() {
        let n = 5_500_000_000u64; // 5.5GB 모델 정도
        let s = fmt_bytes(n);
        assert!(s.ends_with(" GB"), "got: {}", s);
        assert!(s.starts_with("5.1"), "got: {}", s); // 5.5e9 / 2^30 ≈ 5.12
    }

    // ── fmt_context_length ──────────────────────────────────────────

    #[test]
    fn fmt_context_length_units() {
        assert_eq!(fmt_context_length(500), "500");
        assert_eq!(fmt_context_length(8000), "8k");
        assert_eq!(fmt_context_length(128_000), "128k");
        assert_eq!(fmt_context_length(1_000_000), "1.0M");
        assert_eq!(fmt_context_length(2_500_000), "2.5M");
    }

    // ── parse_price_per_million ─────────────────────────────────────

    #[test]
    fn parse_price_per_million_typical() {
        // OpenRouter는 토큰당 USD를 문자열로 줌 (e.g. "0.000005" = $5/M)
        let p = parse_price_per_million(Some("0.000005"));
        assert!(matches!(p, Some(v) if (v - 5.0).abs() < 1e-9));
    }

    #[test]
    fn parse_price_per_million_free() {
        let p = parse_price_per_million(Some("0"));
        assert_eq!(p, Some(0.0));
    }

    #[test]
    fn parse_price_per_million_invalid() {
        assert_eq!(parse_price_per_million(None), None);
        assert_eq!(parse_price_per_million(Some("")), None);
        assert_eq!(parse_price_per_million(Some("abc")), None);
    }

    // ── is_korean_friendly ──────────────────────────────────────────

    #[test]
    fn ko_friendly_known_models() {
        assert!(is_korean_friendly("openai/gpt-4o"));
        assert!(is_korean_friendly("anthropic/claude-3.5-sonnet"));
        assert!(is_korean_friendly("google/gemini-1.5-pro"));
        assert!(is_korean_friendly("qwen/qwen2.5-coder-7b"));
        assert!(is_korean_friendly("meta-llama/llama-3.1-70b-instruct"));
        assert!(is_korean_friendly("upstage/solar-10.7b"));
        assert!(is_korean_friendly("LGAI-EXAONE/EXAONE-3.5-7.8B")); // 대문자도 매칭
        assert!(is_korean_friendly("deepseek/deepseek-v3"));
    }

    #[test]
    fn ko_friendly_negative() {
        assert!(!is_korean_friendly("mistralai/mistral-7b"));
        assert!(!is_korean_friendly("openai/gpt-3.5-turbo"));
        assert!(!is_korean_friendly("starcoder2:7b"));
    }

    // ── categorize_model ────────────────────────────────────────────

    #[test]
    fn categorize_coding_models() {
        let cats = categorize_model("qwen/qwen2.5-coder-7b");
        assert!(cats.contains(&ModelCategory::Coding));
    }

    #[test]
    fn categorize_reasoning_models() {
        let cats = categorize_model("deepseek/deepseek-r1");
        assert!(cats.contains(&ModelCategory::Reasoning));
    }

    #[test]
    fn categorize_general_fallback() {
        let cats = categorize_model("mistralai/mistral-7b-instruct");
        assert!(cats.contains(&ModelCategory::General));
    }

    // ── summarize_tool_result ───────────────────────────────────────

    #[test]
    fn summarize_write_file_success() {
        let args = r#"{"path":"src/foo.rs","content":"hello"}"#;
        let (summary, success) = summarize_tool_result("write_file", args, "OK: wrote 5 bytes");
        assert!(summary.contains("src/foo.rs"));
        assert!(summary.contains("5 bytes"));
        assert!(success);
    }

    #[test]
    fn summarize_write_file_error_result() {
        let args = r#"{"path":"src/foo.rs","content":"x"}"#;
        let (_, success) = summarize_tool_result("write_file", args, "Error: permission denied");
        assert!(!success);
    }

    #[test]
    fn summarize_run_command_truncates_long_command() {
        let long = "x".repeat(200);
        let args = format!(r#"{{"command":"{}"}}"#, long);
        let (summary, _) = summarize_tool_result("run_command", &args, "OK");
        assert!(summary.starts_with("$ "));
        // command 부분만 60자 제한 (prefix "$ " 포함 62자 이내)
        assert!(summary.len() <= 62);
    }

    #[test]
    fn summarize_unknown_tool() {
        let (summary, success) = summarize_tool_result("foo", "{}", "first line\nsecond line");
        assert_eq!(summary, "first line");
        assert!(success);
    }

    #[test]
    fn summarize_err_marker() {
        let (_, success) = summarize_tool_result("foo", "{}", "[err] something broke");
        assert!(!success);
    }

    // ── truncate_after_last_user (P4-3 regenerate/edit) ────────────

    fn cm(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    #[test]
    fn truncate_empty_conv() {
        let mut conv: Vec<ChatMessage> = Vec::new();
        truncate_after_last_user(&mut conv);
        assert!(conv.is_empty());
    }

    #[test]
    fn truncate_user_only() {
        let mut conv = vec![cm("user", "hi")];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 1);
        assert_eq!(conv[0].role, "user");
    }

    #[test]
    fn truncate_user_assistant() {
        let mut conv = vec![cm("user", "hi"), cm("assistant", "hello")];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 1);
        assert_eq!(conv[0].content.as_deref(), Some("hi"));
    }

    #[test]
    fn truncate_keeps_last_user_intact() {
        let mut conv = vec![
            cm("user", "first"),
            cm("assistant", "answer1"),
            cm("user", "second"),
        ];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 3); // 마지막이 user면 그대로
        assert_eq!(conv[2].content.as_deref(), Some("second"));
    }

    #[test]
    fn truncate_user_tool_assistant_chain() {
        // user / assistant(tool_calls) / tool / assistant 시퀀스
        let mut conv = vec![
            cm("system", "sys"),
            cm("user", "hi"),
            cm("assistant", "let me check"),
            cm("tool", "result"),
            cm("assistant", "done"),
        ];
        truncate_after_last_user(&mut conv);
        assert_eq!(conv.len(), 2);
        assert_eq!(conv[0].role, "system");
        assert_eq!(conv[1].role, "user");
    }

    #[test]
    fn truncate_no_user_drops_all() {
        let mut conv = vec![cm("system", "sys"), cm("assistant", "lone")];
        truncate_after_last_user(&mut conv);
        assert!(conv.is_empty());
    }

    // ── last_user_block_idx / last_assistant_block_idx ─────────────

    fn ub(id: u64) -> Block {
        Block {
            id,
            body: BlockBody::User(format!("u{}", id)),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }
    fn ab(id: u64) -> Block {
        Block {
            id,
            body: BlockBody::Assistant(text_editor::Content::with_text(&format!("a{}", id))),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }
    fn tb(id: u64) -> Block {
        Block {
            id,
            body: BlockBody::ToolResult {
                name: "x".into(),
                summary: "y".into(),
                success: true,
            },
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }

    #[test]
    fn last_user_idx_empty() {
        assert_eq!(last_user_block_idx(&[]), None);
    }

    #[test]
    fn last_user_idx_only_user() {
        let blocks = vec![ub(1)];
        assert_eq!(last_user_block_idx(&blocks), Some(0));
    }

    #[test]
    fn last_user_idx_picks_last() {
        let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
        assert_eq!(last_user_block_idx(&blocks), Some(2));
    }

    #[test]
    fn last_user_idx_no_user() {
        let blocks = vec![ab(1), tb(2)];
        assert_eq!(last_user_block_idx(&blocks), None);
    }

    #[test]
    fn last_assistant_idx_picks_last_assistant() {
        let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
        assert_eq!(last_assistant_block_idx(&blocks), Some(3));
    }

    #[test]
    fn last_assistant_idx_no_assistant() {
        let blocks = vec![ub(1), ub(2)];
        assert_eq!(last_assistant_block_idx(&blocks), None);
    }

    // ── parse_apply_candidates (P4-1) ──────────────────────────────

    #[test]
    fn apply_empty_markdown() {
        assert!(parse_apply_candidates("").is_empty());
        assert!(parse_apply_candidates("just plain text\nno code blocks").is_empty());
    }

    #[test]
    fn apply_no_path_comment_skipped() {
        let md = "```rust\nfn main() {}\n```\n";
        assert!(parse_apply_candidates(md).is_empty());
    }

    #[test]
    fn apply_rust_path_comment() {
        let md = "```rust\n// path: src/foo.rs\nfn main() {}\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "src/foo.rs");
        assert_eq!(candidates[0].language, "rust");
        assert_eq!(candidates[0].content, "fn main() {}\n");
    }

    #[test]
    fn apply_python_hash_comment() {
        let md = "```python\n# path: scripts/build.py\nprint('hi')\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "scripts/build.py");
        assert_eq!(candidates[0].language, "python");
    }

    #[test]
    fn apply_multiple_blocks_filters_no_path() {
        let md = "intro\n\
                  ```rust\n// path: a.rs\nA\n```\n\
                  some text\n\
                  ```rust\nB without path\n```\n\
                  ```python\n# path: b.py\nB\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].path, "a.rs");
        assert_eq!(candidates[1].path, "b.py");
    }

    #[test]
    fn apply_path_comment_with_extra_spaces() {
        let md = "```rust\n//    path:    src/x.rs   \nbody\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "src/x.rs");
    }

    #[test]
    fn apply_unclosed_fence_ignored() {
        // 닫는 ``` 없으면 후속 처리는 그래도 path만 매칭됐으면 채택할지 결정.
        // 정책: 닫는 fence 없으면 미완성 → skip.
        let md = "```rust\n// path: a.rs\nbody (no closing)\n";
        assert!(parse_apply_candidates(md).is_empty());
    }

    #[test]
    fn apply_no_language_still_works() {
        let md = "```\n// path: x.txt\nhello\n```\n";
        let candidates = parse_apply_candidates(md);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, "x.txt");
        assert_eq!(candidates[0].language, "");
    }

    #[test]
    fn apply_first_line_must_be_path() {
        // path 주석이 둘째 줄이면 채택 안 함 (정책: 첫 줄만)
        let md = "```rust\nfn main() {}\n// path: a.rs\n```\n";
        assert!(parse_apply_candidates(md).is_empty());
    }

    // ── ModelOption Display + provider_label (OpenAICompat rename) ──

    fn or_opt(id: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenRouter,
            provider_label: String::new(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    fn oai_opt(id: &str, label: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: label.into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    #[test]
    fn display_openrouter_basic() {
        let m = or_opt("gpt-4o");
        let s = format!("{}", m);
        assert!(s.starts_with("[OR]"), "got: {}", s);
        assert!(s.contains("gpt-4o"));
    }

    #[test]
    fn display_openai_compat_with_label() {
        let m = oai_opt("qwen2.5-coder", "xLLM");
        let s = format!("{}", m);
        assert!(s.starts_with("[xLLM]"), "got: {}", s);
        assert!(s.contains("qwen2.5-coder"));
    }

    #[test]
    fn display_openai_compat_empty_label_defaults_to_local() {
        let m = oai_opt("starcoder", "");
        let s = format!("{}", m);
        assert!(s.starts_with("[Local]"), "got: {}", s);
    }

    #[test]
    fn display_openai_compat_whitespace_label_defaults() {
        let m = oai_opt("foo", "   ");
        let s = format!("{}", m);
        assert!(s.starts_with("[Local]"), "got: {}", s);
    }

    #[test]
    fn display_combined_tags() {
        let mut m = or_opt("claude-3.5-sonnet");
        m.ko_friendly = true;
        m.favorite = true;
        m.context_length = Some(200_000);
        m.prompt_per_million = Some(3.0);
        m.completion_per_million = Some(15.0);
        let s = format!("{}", m);
        assert!(s.contains("[OR]"));
        assert!(s.contains("[KO]"));
        assert!(s.contains("★"));
        assert!(s.contains("200k"));
        assert!(s.contains("$3.00/$15.00"));
    }

    #[test]
    fn display_openai_compat_free_marker() {
        let mut m = oai_opt("local-model", "xLLM");
        m.prompt_per_million = Some(0.0);
        m.completion_per_million = Some(0.0);
        let s = format!("{}", m);
        assert!(s.contains("[xLLM]"));
        assert!(s.contains("free"));
    }

    // ── InferenceEngine ─────────────────────────────────────────────

    #[test]
    fn engine_default_ports() {
        assert_eq!(InferenceEngine::Tabby.default_port(), 8080);
        assert_eq!(InferenceEngine::Ollama.default_port(), 11434);
        assert_eq!(InferenceEngine::XLlm.default_port(), 9000);
        assert_eq!(InferenceEngine::VLlm.default_port(), 9000);
        assert_eq!(InferenceEngine::LlamaServer.default_port(), 9000);
    }

    #[test]
    fn engine_compose_xllm() {
        let cmd = InferenceEngine::XLlm
            .compose_command("C:\\models\\Qwen", 9000)
            .unwrap();
        assert_eq!(cmd[0], "xllm");
        assert_eq!(cmd[1], "serve");
        assert!(cmd.contains(&"--model".to_string()));
        assert!(cmd.contains(&"C:\\models\\Qwen".to_string()));
        assert!(cmd.contains(&"--port".to_string()));
        assert!(cmd.contains(&"9000".to_string()));
    }

    #[test]
    fn engine_compose_vllm() {
        let cmd = InferenceEngine::VLlm
            .compose_command("/path/to/model", 9000)
            .unwrap();
        assert_eq!(cmd[0], "vllm");
        assert_eq!(cmd[1], "serve");
        assert!(cmd.contains(&"/path/to/model".to_string()));
    }

    #[test]
    fn engine_compose_llama_server() {
        let cmd = InferenceEngine::LlamaServer
            .compose_command("/path/model.gguf", 9000)
            .unwrap();
        assert_eq!(cmd[0], "llama-server");
        assert!(cmd.contains(&"-m".to_string()));
        assert!(cmd.contains(&"/path/model.gguf".to_string()));
    }

    #[test]
    fn engine_compose_tabby_uses_repo_id() {
        let cmd = InferenceEngine::Tabby
            .compose_command("TabbyML/Qwen2.5-Coder-7B", 8080)
            .unwrap();
        assert_eq!(cmd[0], "tabby");
        assert_eq!(cmd[1], "serve");
        assert!(cmd.contains(&"TabbyML/Qwen2.5-Coder-7B".to_string()));
    }

    #[test]
    fn engine_ollama_no_spawn() {
        // Ollama는 daemon — 이미 떠있다고 가정, spawn 안 함
        assert!(InferenceEngine::Ollama.compose_command("any", 11434).is_none());
    }

    #[test]
    fn engine_custom_no_compose() {
        // Custom은 사용자가 직접 명령 입력
        assert!(InferenceEngine::Custom.compose_command("any", 9000).is_none());
    }

    // ── list_downloaded_models ──────────────────────────────────────

    #[test]
    fn list_models_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(list_downloaded_models(tmp.path()).is_empty());
    }

    #[test]
    fn list_models_returns_subdirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let qwen = tmp.path().join("Qwen--Qwen2.5-Coder-7B");
        std::fs::create_dir_all(&qwen).unwrap();
        std::fs::write(qwen.join("config.json"), "{}").unwrap();
        let solar = tmp.path().join("upstage--SOLAR-10.7B");
        std::fs::create_dir_all(&solar).unwrap();
        std::fs::write(solar.join("model.safetensors"), "x").unwrap();
        // 파일은 무시 (디렉토리 아님)
        std::fs::write(tmp.path().join("ignore.txt"), "x").unwrap();
        let mut models = list_downloaded_models(tmp.path());
        models.sort();
        assert_eq!(models.len(), 2);
        assert!(models[0].contains("Qwen") || models[1].contains("Qwen"));
    }

    #[test]
    fn list_models_skips_empty_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("empty")).unwrap();
        // 빈 폴더는 모델 아님 — skip
        assert!(list_downloaded_models(tmp.path()).is_empty());
    }

    // ── extract_mention_query ───────────────────────────────────────

    #[test]
    fn mention_query_basic() {
        // '@' 뒤에 공백 없으면 Some, 있으면 None
        assert_eq!(extract_mention_query("fix @main"), Some("main"));
        assert_eq!(extract_mention_query("fix @main "), None); // '@main' 이후 공백
        assert_eq!(extract_mention_query("@src/lib"), Some("src/lib"));
        assert_eq!(extract_mention_query("no at sign"), None);
        assert_eq!(extract_mention_query("@"), Some(""));
    }

    #[test]
    fn mention_query_last_at_wins() {
        // 마지막 '@' 기준으로 query 추출
        assert_eq!(extract_mention_query("@foo @bar"), Some("bar")); // 마지막 '@bar' 뒤 공백 없음
        assert_eq!(extract_mention_query("@foo @bar "), None);       // 마지막 '@bar' 뒤 공백 있음
        assert_eq!(extract_mention_query("email@ex.com @file"), Some("file")); // 마지막 '@file'
    }

    // ── fuzzy_match_paths ───────────────────────────────────────────

    #[test]
    fn fuzzy_match_empty_query_returns_all() {
        let paths: Vec<PathBuf> = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/tools.rs"),
        ];
        let result = fuzzy_match_paths(&paths, "", 10);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn fuzzy_match_filters_by_query() {
        let paths: Vec<PathBuf> = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/tools.rs"),
            PathBuf::from("Cargo.toml"),
        ];
        let result = fuzzy_match_paths(&paths, "tool", 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], PathBuf::from("src/tools.rs"));
    }

    #[test]
    fn fuzzy_match_respects_max_results() {
        let paths: Vec<PathBuf> = (0..20).map(|i| PathBuf::from(format!("file{i}.rs"))).collect();
        let result = fuzzy_match_paths(&paths, "file", 5);
        assert_eq!(result.len(), 5);
    }

    // ── build_file_context ──────────────────────────────────────────

    #[test]
    fn build_file_context_single() {
        let files = vec![(PathBuf::from("src/main.rs"), "fn main() {}".to_string())];
        let ctx = build_file_context(&files);
        assert!(ctx.contains("src/main.rs"));
        assert!(ctx.contains("fn main() {}"));
        assert!(ctx.starts_with("```"));
    }

    #[test]
    fn build_file_context_multi_separator() {
        let files = vec![
            (PathBuf::from("a.rs"), "aaa".to_string()),
            (PathBuf::from("b.rs"), "bbb".to_string()),
        ];
        let ctx = build_file_context(&files);
        // 두 파일 사이 빈 줄 구분
        assert!(ctx.contains("\n\n"));
        assert!(ctx.contains("aaa"));
        assert!(ctx.contains("bbb"));
    }
}
