use iced::widget::*;
use iced_native::{
    layout, Background, Color, Element, Hasher, Layout, Length, MouseCursor, Point, Rectangle,
    Size, Widget,
};
use iced_wgpu::{Defaults, Primitive, Renderer};

use crate::ui::Message;
use crate::DisplaySet;

const DEFAULT_RGBA: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

impl canvas::Drawable for DisplaySet {
    fn draw(&self, frame: &mut canvas::Frame) {
        use canvas::{Fill, Path, Stroke};

        // fill background black
        let bg = Path::new(|path| path.rectangle(Point::new(0.0, 0.0), frame.size()));
        frame.fill(&bg, Fill::Color(Color::BLACK));

        let ods = self.ods();
        let w = ods.width;
        let data = &ods.data;

        for (i, color_id) in data.iter().enumerate() {
            let x = (i % w as usize) as u16;
            let y = (i / w as usize) as u16;

            let color: Color = if *color_id == 0 {
                DEFAULT_RGBA.into()
            } else {
                let colors = self.pds().entries.clone();
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
            frame.fill(&point, Fill::Color(color));
        }
    }
}
