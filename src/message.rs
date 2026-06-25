use super::{
    AuthKeyData, ChatEvent, GenerationData, InferenceEngine, ModelOption, OpenRouterModel, hf, mcp,
};
use iced::widget::markdown;
use iced::widget::scrollable::Viewport;
use iced::widget::text_editor::Action;
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
    ModelsLoaded(Result<Vec<OpenRouterModel>, String>),
    SelectModel(ModelOption),
    AccountLoaded(Result<AuthKeyData, String>),
    FetchAccount,
    InputChanged(String),
    Send,
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
    MentionMove(i32),
    MentionConfirm,
    MentionCandidatesLoaded(Vec<PathBuf>),
    FileAttachError(String),
    McpNameChanged(String),
    McpCommandChanged(String),
    AddMcpServer,
    RemoveMcpServer(usize),
    McpToolsLoaded(String, Vec<mcp::McpTool>),
    McpToolsFailed(String),
    McpToolResult(String, String),
    PtyToggle,
    PtyStart,
    PtyLine(String),
    PtyExited,
    PtyInputChanged(String),
    PtySend,
    PtyCtrlC,
    PtyClear,
}
