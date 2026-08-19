use super::{
    AuthKeyData, ChatEvent, GenerationData, InferenceEngine, ModelOption, OpenRouterModel, hf, mcp,
    pty,
};
use crate::state::CloseReapOutcome;
use iced::widget::markdown;
use iced::widget::scrollable::Viewport;
use iced::widget::text_editor::{self, Action};

#[derive(Debug, Clone)]
pub(crate) struct ToolExecutionResult {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) arguments: String,
    pub(crate) result: String,
}
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentMode {
    Plan,
    Build,
}

impl AgentMode {
    pub(crate) fn allow_mutating(self) -> bool {
        matches!(self, AgentMode::Build)
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            AgentMode::Plan => "🔍 Plan",
            AgentMode::Build => "🔧 Build",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortMode {
    Default,
    PriceAsc,
    PriceDesc,
}

impl SortMode {
    pub(crate) fn cycle(self) -> Self {
        match self {
            SortMode::Default => SortMode::PriceAsc,
            SortMode::PriceAsc => SortMode::PriceDesc,
            SortMode::PriceDesc => SortMode::Default,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            SortMode::Default => "정렬: 기본",
            SortMode::PriceAsc => "정렬: 가격↑",
            SortMode::PriceDesc => "정렬: 가격↓",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    Provider,
    Runtime,
    Models,
    Mcp,
    Theme,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    OpenSettings,
    CloseSettings,
    SetSettingsTab(SettingsTab),
    KeyInputChanged(String),
    SaveKey,
    KeySaved(Result<(), String>),
    ClearKey,
    KeyCleared(Result<(), String>),
    FetchModels,
    ModelsLoaded {
        generation: u64,
        result: Result<Vec<OpenRouterModel>, String>,
    },
    SelectModel(ModelOption),
    AccountLoaded {
        generation: u64,
        result: Result<AuthKeyData, String>,
    },
    FetchAccount,
    InputChanged(String),
    InputAction(text_editor::Action),
    Send,
    StopStream,
    ChatChunk {
        block_id: u64,
        stream_generation: u64,
        event: ChatEvent,
    },
    CompareResponsesLoaded {
        generation: u64,
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
    ToggleBlockCollapse(u64),
    LinkClicked(markdown::Uri),
    PickCwd,
    CwdPicked(Option<PathBuf>),
    ApproveWrites,
    DenyWrites,
    ToggleConfirmExpand(usize),
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
    AskDeleteSession(u64),
    DeleteSession(u64),
    CancelDeleteSession,
    GenerationLoaded {
        generation: u64,
        result: Result<GenerationData, String>,
    },
    OpenCommandPalette,
    CloseCommandPalette,
    CloseAllOverlays,
    CommandPaletteChanged(String),
    PaletteMove(i32),
    ActivatePaletteSelection,
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
    InferenceLogLine {
        generation: u64,
        line: String,
    },
    InferenceExited {
        generation: u64,
        code: i32,
    },
    SaveTabby,
    TabbySaved(Result<(), String>),
    ClearTabby,
    FetchTabbyModels,
    FetchTabbyModelsForInference(u64),
    FetchTabbyModelsRetry(u64),
    TabbyModelsLoaded {
        generation: u64,
        result: Result<Vec<String>, String>,
    },
    HfTokenChanged(String),
    ToggleHfTokenVisible,
    SaveHfToken,
    HfTokenSaved(Result<(), String>),
    ModelDirChanged(String),
    PickModelDir,
    ModelDirPicked(Option<std::path::PathBuf>),
    HfRepoChanged(String),
    UsePreset(usize),
    DownloadExl2Preset(usize),
    SelectDownloadedModel(String),
    StartHfDownload,
    HfDownloadEvent(hf::DownloadEvent),
    CancelHfDownload,
    RegenerateLast,
    EditLastUser,
    ApplyChange(u64, usize),
    FileDropped(PathBuf),
    FileDragHover,
    FileReadDone(PathBuf, String),
    PickAttachment,
    AttachmentPicked(Option<PathBuf>),
    RemoveAttachment(usize),
    ClearAttachments,
    AutoSave,
    WindowCloseRequested,
    WindowProcessesReaped(CloseReapOutcome),
    WindowResized(f32, f32),
    MentionMove(i32),
    MentionConfirm,
    MentionCandidatesLoaded(Vec<PathBuf>),
    FileAttachError(String),
    McpNameChanged(String),
    McpCommandChanged(String),
    AddMcpServer,
    RemoveMcpServer(usize),
    McpToolsLoaded {
        generation: u64,
        server_name: String,
        tools: Vec<mcp::McpTool>,
    },
    McpToolsFailed {
        generation: u64,
        server_name: String,
        message: String,
    },
    McpToolResult {
        generation: u64,
        tool_call_id: String,
        result: String,
    },
    ApprovedToolsFinished {
        generation: u64,
        result: Result<Vec<ToolExecutionResult>, String>,
    },
    PtyToggle,
    PtyStart,
    PtyLine {
        generation: u64,
        line: String,
    },
    PtyExited {
        generation: u64,
    },
    PtyStopped(Result<pty::PtyReceipt, pty::PtyShutdownFailure>, bool, u64),
    PtyInputChanged(String),
    PtySend,
    PtyCtrlC,
    PtyClear,
    ThemeHexChanged(String, String),
    ApplyThemePreset(usize),
    ApplyTheme,
    ResetTheme,
    ThemeSaved(Result<(), String>),
    SetReducedMotion(bool),
    FileTreeToggle(std::path::PathBuf),
    RefreshFileTree,
    SkeletonTick,
    ToggleTldrView(u64),
    CodeBlockHovered(u64, bool),
    BlockHovered(Option<u64>),
    SessionHovered(Option<u64>),
    DismissToast,
    ContextHovered(Option<usize>),
    PaletteHovered(Option<usize>),
    SettingsTabHovered(Option<SettingsTab>),
    ConfirmCardHovered(Option<usize>),
    AttachChipHovered(Option<usize>),
    ShortcutHintHovered(Option<usize>),
    PtyPanelHovered(bool),
    McpServerHovered(Option<usize>),
    StartRenameSession(u64),
    RenameSession(u64, String),
    CancelRenameSession,
    SessionSearchChanged(String),
    ToggleShortcutGuide,
}
