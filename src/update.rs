// update.rs — App update 메서드 (main.rs child module)
use super::{App, Message, on_event, session};
use iced::{Subscription, Task};
use std::time::Duration;

impl App {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        if let Some(task) = self.dispatch_settings(&message) {
            return task;
        }
        if let Some(task) = self.dispatch_hf(&message) {
            return task;
        }
        if let Some(task) = self.dispatch_io(&message) {
            return task;
        }
        if let Some(task) = self.dispatch_session(&message) {
            return task;
        }
        if let Some(task) = self.dispatch_chat(&message) {
            return task;
        }
        if let Some(task) = self.dispatch_ui(&message) {
            return task;
        }
        Task::none()
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let event_sub = iced::event::listen_with(on_event);
        let interval = if self.streaming_block_id.is_some() {
            Duration::from_secs(15)
        } else {
            Duration::from_secs(60)
        };
        let timer_sub = iced::time::every(interval).map(|_| Message::AutoSave);
        let skeleton_sub = if self.streaming_block_id.is_some() {
            iced::time::every(Duration::from_millis(600)).map(|_| Message::SkeletonTick)
        } else {
            Subscription::none()
        };
        Subscription::batch(vec![event_sub, timer_sub, skeleton_sub])
    }
}

include!("update_dispatch_settings.rs");
include!("update_dispatch_hf.rs");
include!("update_dispatch_io.rs");
include!("update_dispatch_session.rs");
include!("update_dispatch_chat.rs");
include!("update_dispatch_ui.rs");
