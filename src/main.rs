// CodeWarp — Iced 진입점
// Phase 2-3a: 3-pane 레이아웃 + 모델 셀렉터 (TopBar) + 입력 echo

mod block;
mod bootstrap;
mod hf;
mod input;
mod keystore;
mod mcp;
mod message;
mod model;
mod openrouter;
mod palette;
mod pty;
mod runtime_process;
mod session;
mod state;
mod tabby;
mod tools;
mod update;
mod update_chat;
mod update_chat_send;
mod update_chat_session;
mod update_chat_stream;
mod update_chat_tools;
mod update_helpers;
mod update_inference;
mod update_inference_config;
mod update_inference_start;
mod update_inference_tabby;
mod update_settings;
mod update_settings_interact;
mod update_settings_io;
mod update_settings_ui;
mod util;
mod view;

pub(crate) use block::*;
use bootstrap::{
    build_window_icon, JETBRAINS_MONO_BOLD, JETBRAINS_MONO_REGULAR, PRETENDARD_BOLD,
    PRETENDARD_REGULAR, PRETENDARD_SEMIBOLD,
};
pub(crate) use input::on_event;
pub(crate) use message::*;
pub(crate) use model::*;
pub(crate) use palette::*;
#[cfg(test)]
pub(crate) use runtime_process::humanize_inference_spawn_error;
pub(crate) use runtime_process::spawn_inference_stream;
pub(crate) use state::*;
pub(crate) use update_helpers::*;
pub(crate) use util::*;

use iced::task;
use iced::widget::markdown;
use iced::widget::operation::snap_to_end;
use iced::widget::{combo_box, Id as ScrollId};
use iced::{Color, Font, Size, Task, Theme};
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
