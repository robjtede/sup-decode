use std::cmp;

use iced::{
    Color, Element, Length, Point, Renderer, Size, Task, Theme,
    mouse::Cursor,
    widget::{
        Button, Canvas, Column, Container, Row, Text, button,
        canvas::{self, Cache},
        column, text,
    },
};

use crate::{DisplaySet, DisplaySetBuilder};

const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[derive(Debug, Clone, Copy)]
pub enum Message {
    NextFrame,
    PrevFrame,
}

#[derive(Debug)]
pub struct SupViewer {
    frames: Vec<DisplaySet>,
    current_frame: usize,
    cache: Cache,
}

impl SupViewer {
    pub(crate) fn new(frames: Vec<DisplaySet>) -> (Self, Task<Message>) {
        (
            Self {
                frames,
                current_frame: 0,
                cache: Cache::default(),
            },
            Task::none(),
        )
    }

    pub(crate) fn view(&self) -> Element<Message> {
        let ds = &self.frames[self.current_frame];

        let canvas = Canvas::new(self)
            .width(ds.pcs.width as f32)
            .height(ds.pcs.height as f32);

        let back_button = button("prev").on_press(Message::PrevFrame);
        let next_button = button("next").on_press(Message::NextFrame);

        let content = column![
            canvas,
            Row::new()
                .spacing(20)
                .push(back_button)
                .push(text(format!(
                    "{} / {}",
                    self.current_frame + 1,
                    self.frames.len(),
                )))
                .push(next_button),
        ];

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub(crate) fn update(&mut self, message: Message) {
        self.cache.clear();

        let frames = self.frames.len();

        match message {
            _ if frames == 0 => {}

            Message::PrevFrame if self.current_frame == 0 => {
                self.current_frame -= 1;
            }

            Message::PrevFrame => {
                self.current_frame -= 1;
            }

            Message::NextFrame if self.current_frame >= frames - 1 => {}

            Message::NextFrame => {
                self.current_frame += 1;
            }
        }
    }
}

impl canvas::Program<Message> for SupViewer {
    type State = ();

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let ds = &self.frames[self.current_frame];

        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // fill background black
            let bg = canvas::Path::new(|path| path.rectangle(Point::new(0.0, 0.0), frame.size()));
            frame.fill(
                &bg,
                canvas::Fill {
                    style: Color::BLACK.into(),
                    rule: canvas::fill::Rule::NonZero,
                },
            );

            let ods = &ds.ods;
            let w = ods.width;
            let data = &ods.data;

            for (i, color_id) in data.iter().enumerate() {
                let w = w as usize;

                let x = (i % w) as u16;
                let y = (i / w) as u16;

                let color = if *color_id == 0 {
                    BLACK
                } else {
                    ds.pds
                        .find_by_id(*color_id)
                        .map(|y_cr_cb| y_cr_cb.rgba())
                        .unwrap_or_else(|| TRANSPARENT)
                };

                let point = canvas::Path::new(|path| {
                    let obj = ds.pcs.find_object_by_id(ods.id).unwrap();

                    path.rectangle(
                        Point::new((obj.x + x) as f32, (obj.y + y) as f32),
                        Size::new(1.0, 1.0),
                    )
                });
                frame.fill(
                    &point,
                    canvas::Fill {
                        style: canvas::Style::Solid(Color::from(color)),
                        rule: canvas::fill::Rule::NonZero,
                    },
                );
            }
        });

        vec![geometry]
    }
}
