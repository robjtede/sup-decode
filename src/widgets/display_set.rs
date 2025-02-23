use iced::mouse::Cursor;
use iced::widget::*;
use iced::{Background, Color, Element, Length, Point, Rectangle, Size};

use crate::DisplaySet;
use crate::ui::Message;

const DEFAULT_RGBA: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

#[derive(Debug, Default)]
pub struct DisplaySetView {
    ds: DisplaySet,
    cache: canvas::Cache,
}

impl DisplaySetView {
    pub fn new(ds: DisplaySet) -> Self {
        Self {
            ds,
            cache: canvas::Cache::default(),
        }
    }

    pub fn ds(&self) -> &DisplaySet {
        &self.ds
    }
}

impl canvas::Program<Message> for DisplaySetView {
    type State = ();

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let display_set = self.cache.draw(renderer, bounds.size(), |frame| {
            // fill background black
            // let bg = canvas::Path::new(|path| path.rectangle(Point::new(0.0, 0.0), frame.size()));
            // frame.fill(
            //     &bg,
            //     canvas::Fill {
            //         style: Color::BLACK.into(),
            //         rule: canvas::path::lyon_path::FillRule::NonZero,
            //     },
            // );

            let ods = self.ds.ods();
            let w = ods.width;
            let data = &ods.data;

            for (i, color_id) in data.iter().enumerate() {
                let x = (i % w as usize) as u16;
                let y = (i / w as usize) as u16;

                let color = if *color_id == 0 {
                    DEFAULT_RGBA
                } else {
                    let colors = self.ds.pds().entries.clone();
                    let color = colors
                        .iter()
                        .find(|entry| entry.id == *color_id)
                        .map(|ycrcb| ycrcb.rgba())
                        .unwrap_or(DEFAULT_RGBA);

                    color
                };

                let point = canvas::Path::new(|path| {
                    path.rectangle(Point::new(f32::from(x), f32::from(y)), Size::new(1.0, 1.0))
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

        vec![display_set]
    }
}
