use canvas::FillRule;
use iced::canvas::{Fill, Frame, Path, Program, Stroke};
use iced::widget::*;
use iced_native::{
    layout, Background, Color, Element, Hasher, Layout, Length, Point, Rectangle, Size, Widget,
};
use iced_wgpu::{Defaults, Primitive, Renderer};

use crate::ui::Message;
use crate::DisplaySet;

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
    fn draw(&self, bounds: Rectangle, cursor: canvas::Cursor) -> Vec<canvas::Geometry> {
        let display_set = self.cache.draw(bounds.size(), |frame| {
            // fill background black
            let bg = Path::new(|path| path.rectangle(Point::new(0.0, 0.0), frame.size()));
            frame.fill(
                &bg,
                Fill {
                    color: Color::BLACK,
                    rule: FillRule::NonZero,
                },
            );

            let ods = self.ds.ods();
            let w = ods.width;
            let data = &ods.data;

            for (i, color_id) in data.iter().enumerate() {
                let x = (i % w as usize) as u16;
                let y = (i / w as usize) as u16;

                let color: Color = if *color_id == 0 {
                    DEFAULT_RGBA.into()
                } else {
                    let colors = self.ds.pds().entries.clone();
                    let color = colors
                        .iter()
                        .find(|entry| entry.id == *color_id)
                        .map(|ycrcb| ycrcb.rgba())
                        .unwrap_or(DEFAULT_RGBA);

                    color.into()
                };

                let point = Path::new(|path| {
                    path.rectangle(Point::new(f32::from(x), f32::from(y)), [1, 1].into())
                });
                frame.fill(
                    &point,
                    Fill {
                        color,
                        rule: FillRule::NonZero,
                    },
                );
            }
        });

        vec![display_set]
    }
}
