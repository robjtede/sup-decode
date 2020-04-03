use std::cmp;

use iced::*;

use crate::DisplaySet;

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

    next_frame_button: button::State,
    prev_frame_button: button::State,

    current_frame: DisplaySet,
    current_frame_canvas_cache: canvas::layer::Cache<DisplaySet>,
}

impl Default for SupViewer {
    fn default() -> Self {
        Self {
            state: Default::default(),
            current_frame: Default::default(),
            current_frame_canvas_cache: canvas::layer::Cache::new(),
            next_frame_button: Default::default(),
            prev_frame_button: Default::default(),
        }
    }
}

impl SupViewer {
    pub fn with_frames(frames: Vec<DisplaySet>) -> Self {
        Self {
            state: State::new(frames),
            ..Default::default()
        }
    }
}

impl Application for SupViewer {
    type Executor = executor::Null;
    type Flags = Vec<DisplaySet>;
    type Message = Message;

    fn new(init_frames: Self::Flags) -> (Self, Command<Self::Message>) {
        (SupViewer::with_frames(init_frames), Command::none())
    }

    fn title(&self) -> String {
        "sup viewer".to_owned()
    }

    fn view(&mut self) -> Element<Message> {
        self.current_frame = self.state.frames[self.state.current_frame].clone();

        let ods = self.current_frame.ods();
        let w = ods.width;
        let h = ods.height;

        let canvas = Canvas::new()
            .width(Length::Units(w))
            .height(Length::Units(h))
            .push(self.current_frame_canvas_cache.with(&self.current_frame));

        let content = Column::new().padding(20).spacing(20).push(canvas).push(
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
            .into()
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        let frames = self.state.frames.len();

        match message {
            _ if frames == 0 => Command::none(),
            Message::PrevFrame if self.state.current_frame <= 0 => Command::none(),
            Message::PrevFrame => {
                self.state.current_frame = self.state.current_frame - 1;
                Command::none()
            }
            Message::NextFrame if self.state.current_frame >= frames - 1 => Command::none(),
            Message::NextFrame => {
                self.state.current_frame = self.state.current_frame + 1;
                Command::none()
            }
        }
    }
}
