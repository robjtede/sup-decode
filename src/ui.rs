use iced::{
    Color, Element, Length, Point, Renderer, Size, Task, Theme,
    mouse::Cursor,
    widget::{Canvas, Container, Row, button, canvas, column, text},
};

use crate::DisplaySet;

const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
#[expect(dead_code)]
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[derive(Debug, Clone, Copy)]
pub(crate) enum Message {
    NextFrame,
    PrevFrame,
}

#[derive(Debug)]
pub(crate) struct SupViewer {
    frames: Vec<DisplaySet>,
    current_frame: usize,
}

impl SupViewer {
    pub(crate) fn new(frames: Vec<DisplaySet>) -> (Self, Task<Message>) {
        (
            Self {
                frames,
                current_frame: 0,
            },
            Task::none(),
        )
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        if self.frames.is_empty() {
            return Container::new(text("No frames decoded"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let ds = &self.frames[self.current_frame];

        let canvas = Canvas::new(self).width(Length::Fill).height(Length::Fill);

        let back_button = button("prev").on_press(Message::PrevFrame);
        let next_button = button("next").on_press(Message::NextFrame);

        let content = column![
            text(format!(
                "frame {} / {}  video={}x{}  object={}x{}",
                self.current_frame + 1,
                self.frames.len(),
                ds.pcs.width,
                ds.pcs.height,
                ds.ods.width,
                ds.ods.height
            )),
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
        let frames = self.frames.len();

        match message {
            _ if frames == 0 => {}

            Message::PrevFrame if self.current_frame == 0 => {}

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
        _state: &Self::State,
        _renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let ds = &self.frames[self.current_frame];

        let mut frame = canvas::Frame::new(_renderer, bounds.size());

        // Fill the whole preview area black.
        let bg = canvas::Path::rectangle(Point::ORIGIN, frame.size());
        frame.fill(
            &bg,
            canvas::Fill {
                style: Color::BLACK.into(),
                rule: canvas::fill::Rule::NonZero,
            },
        );

        let video_w = ds.pcs.width as f32;
        let video_h = ds.pcs.height as f32;

        if video_w <= 0.0 || video_h <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let scale = (bounds.width / video_w).min(bounds.height / video_h);
        let preview_w = video_w * scale;
        let preview_h = video_h * scale;
        let offset_x = (bounds.width - preview_w) / 2.0;
        let offset_y = (bounds.height - preview_h) / 2.0;

        // Draw the video frame boundary so we can spot placement issues quickly.
        let outline = canvas::Path::rectangle(
            Point::new(offset_x, offset_y),
            Size::new(preview_w, preview_h),
        );
        frame.stroke(
            &outline,
            canvas::Stroke::default()
                .with_color(Color::WHITE)
                .with_width(1.0),
        );

        let ods = &ds.ods;
        let obj = ds.pcs.find_object_by_id(ods.id).unwrap();

        // Draw the object bounding box in red to make obvious misalignment visible.
        let object_box = canvas::Path::rectangle(
            Point::new(
                offset_x + obj.x as f32 * scale,
                offset_y + obj.y as f32 * scale,
            ),
            Size::new(ods.width as f32 * scale, ods.height as f32 * scale),
        );
        frame.stroke(
            &object_box,
            canvas::Stroke::default()
                .with_color(Color::from_rgb(1.0, 0.2, 0.2))
                .with_width(1.0),
        );

        let data = &ods.data;
        let w = ods.width as usize;

        for (i, color_id) in data.iter().enumerate() {
            if *color_id == 0 {
                continue;
            }

            let x = (i % w) as f32;
            let y = (i / w) as f32;

            let color = ds
                .pds
                .find_by_id(*color_id)
                .map(|y_cr_cb| y_cr_cb.rgba())
                .unwrap_or(TRANSPARENT);

            let pixel = canvas::Path::rectangle(
                Point::new(
                    offset_x + (obj.x as f32 + x) * scale,
                    offset_y + (obj.y as f32 + y) * scale,
                ),
                Size::new(scale.max(1.0), scale.max(1.0)),
            );

            frame.fill(
                &pixel,
                canvas::Fill {
                    style: canvas::Style::Solid(Color::from(color)),
                    rule: canvas::fill::Rule::NonZero,
                },
            );
        }

        vec![frame.into_geometry()]
    }
}
