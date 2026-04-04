use iced::{
    Alignment, Color, Element, Length, Point, Renderer, Size, Task, Theme,
    mouse::Cursor,
    widget::{Canvas, Container, Row, button, canvas, checkbox, column, text},
};

use crate::{
    DisplaySet,
    ocr::{OcrFrame, OcrState},
};

const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
#[expect(dead_code)]
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[derive(Debug, Clone, Copy)]
pub(crate) enum Message {
    NextFrame,
    PrevFrame,
    ToggleOutlines(bool),
}

#[derive(Debug)]
pub(crate) struct SupViewer {
    frames: Vec<DisplaySet>,
    ocr_frames: Vec<OcrFrame>,
    current_frame: usize,
    show_outlines: bool,
}

impl SupViewer {
    pub(crate) fn new(frames: Vec<DisplaySet>, ocr_frames: Vec<OcrFrame>) -> (Self, Task<Message>) {
        (
            Self {
                frames,
                ocr_frames,
                current_frame: 0,
                show_outlines: true,
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
        let ocr = &self.ocr_frames[self.current_frame];

        let canvas = Canvas::new(self)
            .width(Length::FillPortion(3))
            .height(Length::Fill);

        let back_button = button("prev").on_press(Message::PrevFrame);
        let next_button = button("next").on_press(Message::NextFrame);
        let outline_toggle = checkbox(self.show_outlines)
            .label("Show outlines")
            .on_toggle(Message::ToggleOutlines);
        let start_pts = format_timestamp(self.frames.first().unwrap().pts);
        let current_pts = format_timestamp(ds.pts);
        let end_pts = format_timestamp(self.frames.last().unwrap().pts);
        let timeline = Canvas::new(Timeline {
            frames: &self.frames,
            current_frame: self.current_frame,
        })
        .width(Length::Fill)
        .height(Length::Fixed(36.0));
        let timeline_info = text(format!(
            "{} / {}  {}",
            self.current_frame + 1,
            self.frames.len(),
            current_pts,
        ))
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center);
        let timeline_column = column![
            Row::new()
                .push(text(start_pts))
                .push(Container::new(text("")).width(Length::Fill))
                .push(text(end_pts)),
            timeline,
            timeline_info,
            Row::new()
                .push(Container::new(text("")).width(Length::Fill))
                .push(outline_toggle)
                .push(Container::new(text("")).width(Length::Fill)),
        ]
        .spacing(4)
        .width(Length::Fill)
        .padding([6, 10]);

        let ocr_panel = Container::new(render_ocr_panel(ocr))
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .padding(12);

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
            Row::new()
                .spacing(12)
                .height(Length::Fill)
                .push(canvas)
                .push(ocr_panel),
            Row::new()
                .spacing(12)
                .push(back_button)
                .push(timeline_column)
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

            Message::ToggleOutlines(show_outlines) => {
                self.show_outlines = show_outlines;
            }

            Message::NextFrame if self.current_frame >= frames - 1 => {}

            Message::NextFrame => {
                self.current_frame += 1;
            }
        }
    }
}

struct Timeline<'a> {
    frames: &'a [DisplaySet],
    current_frame: usize,
}

impl canvas::Program<Message> for Timeline<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        if self.frames.is_empty() {
            return vec![frame.into_geometry()];
        }

        let size = frame.size();
        let baseline_y = size.height / 2.0;
        let start_x = 8.0;
        let end_x = (size.width - 8.0).max(start_x);

        let baseline = canvas::Path::line(
            Point::new(start_x, baseline_y),
            Point::new(end_x, baseline_y),
        );
        frame.stroke(
            &baseline,
            canvas::Stroke::default()
                .with_color(Color::from_rgb(0.6, 0.6, 0.6))
                .with_width(1.0),
        );

        let start = self.frames.first().unwrap().pts;
        let end = self.frames.last().unwrap().pts;
        let total_ms = end.signed_duration_since(start).num_milliseconds();

        for (index, display_set) in self.frames.iter().enumerate() {
            let x = if total_ms <= 0 {
                start_x
            } else {
                let elapsed = display_set
                    .pts
                    .signed_duration_since(start)
                    .num_milliseconds() as f32;
                let progress = (elapsed / total_ms as f32).clamp(0.0, 1.0);
                start_x + (end_x - start_x) * progress
            };

            let (height, width, color) = if index == self.current_frame {
                (18.0, 2.5, Color::from_rgb(1.0, 0.25, 0.25))
            } else {
                (10.0, 1.0, Color::WHITE)
            };

            let tick = canvas::Path::line(
                Point::new(x, baseline_y - height / 2.0),
                Point::new(x, baseline_y + height / 2.0),
            );
            frame.stroke(
                &tick,
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(width),
            );
        }

        vec![frame.into_geometry()]
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
        if self.show_outlines {
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
        }

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

fn format_timestamp(pts: chrono::NaiveTime) -> String {
    pts.format("%H:%M:%S%.3f").to_string()
}

fn render_ocr_panel(ocr: &OcrFrame) -> Element<'_, Message> {
    let header = column![
        text("OCR").size(22),
        text(format!("backend: {}", ocr.backend)),
        text(format!(
            "subtitle bitmap: {}x{}",
            ocr.subtitle_size.0, ocr.subtitle_size.1
        )),
        text(format!("pts: {}", format_timestamp(ocr.pts))),
    ]
    .spacing(4);

    let body = match &ocr.state {
        OcrState::NotConfigured(reason) => column![
            text("No OCR backend configured yet."),
            text(reason.as_str()),
        ]
        .spacing(6),
        OcrState::Failed(err) => {
            column![text("OCR failed for this frame."), text(err.as_str()),].spacing(6)
        }
        OcrState::Recognized(data) => {
            let confidence = data
                .mean_confidence
                .map(|confidence| format!("{confidence:.1}%"))
                .unwrap_or_else(|| "n/a".to_owned());
            let words = if data.words.is_empty() {
                "No word-level details".to_owned()
            } else {
                data.words
                    .iter()
                    .map(|word| {
                        let confidence = word
                            .confidence
                            .map(|confidence| format!("{confidence:.1}%"))
                            .unwrap_or_else(|| "n/a".to_owned());
                        format!("{} ({confidence})", word.text)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            column![
                text(format!("mean confidence: {confidence}")),
                text("Recognized text:"),
                text(if data.text.is_empty() {
                    "<empty>"
                } else {
                    data.text.as_str()
                }),
                text("Words:"),
                text(words),
            ]
            .spacing(6)
        }
    };

    Container::new(column![header, body].spacing(12).align_x(Alignment::Start))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
