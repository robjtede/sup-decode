use std::cmp;

use iced::*;

use crate::DisplaySet;
use crate::FRAMES;

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

#[derive(Debug, Clone, Default)]
pub struct SupViewer {
    state: State,

    next_frame_button: button::State,
    prev_frame_button: button::State,
}

impl SupViewer {
    pub fn with_frames(frames: Vec<DisplaySet>) -> Self {
        Self {
            state: State::new(frames),
            ..Default::default()
        }
    }

    pub fn with_global_frames() -> Self {
        Self::with_frames(FRAMES.get().expect("frames is not set").clone())
    }
}

impl Sandbox for SupViewer {
    type Message = Message;

    fn new() -> Self {
        Self::with_global_frames()
    }

    fn title(&self) -> String {
        "sup viewer".to_owned()
    }

    fn view(&mut self) -> Element<Message> {
        let current_frame = self.state.frames[self.state.current_frame].clone();

        let content = Column::new()
            .padding(20)
            .spacing(20)
            .push(current_frame)
            .push(
                Row::new()
                    .max_width(400)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(
                        Button::new(&mut self.prev_frame_button, Text::new("prev"))
                            .on_press(Message::PrevFrame),
                    )
                    .push(Text::new(format!(
                        "{} / {}",
                        self.state.current_frame.to_string(),
                        self.state.frames.len().to_string()
                    )))
                    .push(
                        Button::new(&mut self.next_frame_button, Text::new("next"))
                            .on_press(Message::NextFrame),
                    ),
            );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn update(&mut self, message: Message) {
        let frames = self.state.frames.len();

        match message {
            _ if frames == 0 => {}
            Message::PrevFrame if self.state.current_frame <= 0 => {}
            Message::PrevFrame => self.state.current_frame = self.state.current_frame - 1,
            Message::NextFrame if self.state.current_frame >= frames - 1 => {}
            Message::NextFrame => self.state.current_frame = self.state.current_frame + 1,
        }
    }
}
