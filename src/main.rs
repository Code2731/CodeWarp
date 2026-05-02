// CodeWarp — Iced 진입점
// Phase 2-3a: 3-pane 레이아웃 + 모델 셀렉터 (TopBar) + 입력 echo

mod keystore;
mod openrouter;

use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Alignment, Element, Font, Length, Size, Task, Theme};

use openrouter::{ChatEvent, ChatMessage, OpenRouterModel};

const PRETENDARD_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/Pretendard-Regular.otf");

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .font(PRETENDARD_REGULAR)
        .default_font(Font::with_name("Pretendard"))
        .window(iced::window::Settings {
            size: Size::new(1280.0, 800.0),
            min_size: Some(Size::new(960.0, 600.0)),
            ..Default::default()
        })
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
struct Block {
    id: u64,
    role: Role,
    content: String,
}

#[derive(Default)]
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
        let mut app = Self::default();
        app.has_key = has_key;
        app.show_settings = !has_key;
        app.status = if has_key {
            "준비됨".into()
        } else {
            "OpenRouter API 키 미등록".into()
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
                        if self.selected_model.is_none() {
                            self.selected_model = self.model_ids.first().cloned();
                        }
                        self.models = models;
                        self.status = format!("모델 {} 로드됨", n);
                    }
                    Err(e) => self.status = format!("페치 실패: {}", e),
                }
                Task::none()
            }
            Message::SelectModel(id) => {
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

                // history: 기존 blocks (비어있지 않은) + 새 user
                let mut messages: Vec<ChatMessage> = self
                    .blocks
                    .iter()
                    .filter(|b| !b.content.trim().is_empty())
                    .map(|b| ChatMessage {
                        role: match b.role {
                            Role::User => "user".into(),
                            Role::Assistant => "assistant".into(),
                        },
                        content: b.content.clone(),
                    })
                    .collect();
                messages.push(ChatMessage {
                    role: "user".into(),
                    content: text.clone(),
                });

                let user_id = self.next_id();
                self.blocks.push(Block {
                    id: user_id,
                    role: Role::User,
                    content: text,
                });
                let ai_id = self.next_id();
                self.blocks.push(Block {
                    id: ai_id,
                    role: Role::Assistant,
                    content: String::new(),
                });
                self.streaming_block_id = Some(ai_id);
                self.input.clear();
                self.status = "응답 생성 중…".into();

                Task::run(
                    openrouter::chat_stream(api_key, model, messages),
                    Message::ChatChunk,
                )
            }
            Message::CopyBlock(id) => {
                if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
                    return iced::clipboard::write(b.content.clone());
                }
                Task::none()
            }
            Message::ChatChunk(event) => {
                let Some(ai_id) = self.streaming_block_id else {
                    return Task::none();
                };
                let block = self.blocks.iter_mut().find(|b| b.id == ai_id);
                match event {
                    ChatEvent::Token(t) => {
                        if let Some(b) = block {
                            b.content.push_str(&t);
                        }
                    }
                    ChatEvent::Done => {
                        self.streaming_block_id = None;
                        self.status = "준비됨".into();
                    }
                    ChatEvent::Error(e) => {
                        if let Some(b) = block {
                            b.content = if b.content.is_empty() {
                                format!("[에러] {}", e)
                            } else {
                                format!("{}\n\n[에러] {}", b.content, e)
                            };
                        }
                        self.streaming_block_id = None;
                        self.status = format!("에러: {}", e);
                    }
                }
                Task::none()
            }
        }
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
            pick_list(
                self.model_ids.clone(),
                self.selected_model.clone(),
                Message::SelectModel,
            )
            .placeholder("모델 선택")
            .text_size(12)
            .into()
        };

        let bar = row![
            text("CodeWarp").size(18),
            Space::new().width(Length::Fill),
            model_picker,
            button(text("⚙").size(14)).on_press(Message::OpenSettings),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        container(bar)
            .padding([10, 16])
            .width(Length::Fill)
            .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let body = column![
            text("프로젝트").size(11),
            text("CodeWarp").size(13),
            Space::new().height(Length::Fixed(14.0)),
            text("파일").size(11),
            text("src/").size(13),
            text("Cargo.toml").size(13),
            text("README.md").size(13),
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
                let role_label = match b.role {
                    Role::User => "you",
                    Role::Assistant => "ai",
                };
                let copy_btn: Element<Message> = if b.content.trim().is_empty() {
                    Space::new().width(Length::Shrink).height(Length::Shrink).into()
                } else {
                    button(text("복사").size(10))
                        .on_press(Message::CopyBlock(b.id))
                        .padding([2, 8])
                        .into()
                };
                let header = row![
                    text(role_label).size(11),
                    Space::new().width(Length::Fill),
                    copy_btn,
                ]
                .align_y(Alignment::Center);
                let block_view = container(
                    column![
                        header,
                        text(&b.content).size(13),
                    ]
                    .spacing(6),
                )
                .padding(12)
                .width(Length::Fill);
                col = col.push(block_view);
            }
            scrollable(col)
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
