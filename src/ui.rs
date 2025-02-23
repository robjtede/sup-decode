use std::cmp;

use iced::{
    Element, Task,
    widget::{Button, Canvas, Column, Container, Row, Text, button, column, text},
};

use crate::{DisplaySet, widgets::DisplaySetView};

#[derive(Debug, Clone, Default)]
struct State {
    frames: Vec<DisplaySet>,
    current_frame: usize,
}

impl State {
    pub fn new(frames: Vec<DisplaySet>) -> Self {
        Self {
            frames,
            current_frame: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    NextFrame,
    PrevFrame,
}

#[derive(Debug)]
pub struct SupViewer {
    state: State,
    current_frame: DisplaySetView,
}

impl SupViewer {
    pub(crate) fn new(frames: Vec<DisplaySet>) -> (Self, Task<Message>) {
        (
            Self {
                state: State::new(frames),
                current_frame: DisplaySetView::default(),
            },
            Task::none(),
        )
    }

    pub(crate) fn view(&self) -> Element<Message> {
        let current_frame =
            DisplaySetView::new(self.state.frames[self.state.current_frame].clone());

        // let ods = self.current_frame.ds().ods();
        // let w = ods.width;
        // let h = ods.height;

        let canvas = Canvas::new(current_frame);

        let content = column![
            canvas,
            // Row::new()
            //     // .max_width(400)
            //     .spacing(20)
            //     .align_items(Align::Center)
            //     .push(button("prev").on_press(Message::PrevFrame))
            //     .push(text(format!(
            //         "{} / {}",
            //         self.state.current_frame.to_string(),
            //         self.state.frames.len().to_string()
            //     )))
            //     .push(button("next").on_press(Message::NextFrame)),
        ];

        // Container::new(content)
        //     .width(Length::Fill)
        //     .height(Length::Fill)
        //     .into()

        content.into()
    }

    pub(crate) fn update(&mut self, message: Message) {
        let frames = self.state.frames.len();

        match message {
            _ if frames == 0 => {}
            Message::PrevFrame => {
                self.state.current_frame -= 1;
            }
            Message::NextFrame if self.state.current_frame >= frames - 1 => {}
            Message::NextFrame => {
                self.state.current_frame += 1;
            }
        }
    }
}
