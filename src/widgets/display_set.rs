use iced_native::{
    layout, Background, Color, Element, Hasher, Layout, Length, MouseCursor, Point, Rectangle,
    Size, Widget,
};
use iced_wgpu::{Primitive, Renderer};

use crate::ui::Message;
use crate::DisplaySet;

const DEFAULT_RGBA: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

impl<Msg> Widget<Msg, Renderer> for DisplaySet {
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, _renderer: &Renderer, _limits: &layout::Limits) -> layout::Node {
        layout::Node::new(Size::new(
            f32::from(self.ods().width),
            f32::from(self.ods().height),
        ))
    }

    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;
        self.ods.hash(state);
    }

    fn draw(
        &self,
        _renderer: &mut Renderer,
        layout: Layout<'_>,
        _cursor_position: Point,
    ) -> (Primitive, MouseCursor) {
        let ods = self.ods();
        let w = ods.width;
        let data = &ods.data;

        let quads: Vec<Primitive> = data
            .iter()
            .enumerate()
            .map(|(i, color_id)| {
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

                Primitive::Quad {
                    bounds: Rectangle {
                        x: f32::from(x),
                        y: f32::from(y),
                        width: 1.0,
                        height: 1.0,
                    },
                    background: Background::Color(color),
                    border_radius: 0,
                }
            })
            .collect();

        (
            Primitive::Group { primitives: quads },
            MouseCursor::OutOfBounds,
        )
    }
}

impl<'a, Message> Into<Element<'a, Message, Renderer>> for DisplaySet {
    fn into(self) -> Element<'a, Message, Renderer> {
        Element::new(self)
    }
}
