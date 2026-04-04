use winnow::{
    Bytes, ModalResult,
    binary::{be_u8, be_u16, length_repeat},
    prelude::*,
};

#[derive(Debug, Clone)]
pub(crate) struct WindowDefinition {
    #[cfg_attr(not(test), expect(dead_code))]
    id: u8,
    #[cfg_attr(not(test), expect(dead_code))]
    x: u16,
    #[cfg_attr(not(test), expect(dead_code))]
    y: u16,
    #[cfg_attr(not(test), expect(dead_code))]
    width: u16,
    #[cfg_attr(not(test), expect(dead_code))]
    height: u16,
}

impl WindowDefinition {
    fn from_tuple((id, x, y, width, height): (u8, u16, u16, u16, u16)) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
        }
    }
}

fn decode_single_window_definiton(input: &mut &Bytes) -> ModalResult<WindowDefinition> {
    (be_u8, be_u16, be_u16, be_u16, be_u16)
        .map(WindowDefinition::from_tuple)
        .parse_next(input)
}

pub(crate) fn decode_wds(input: &mut &Bytes) -> ModalResult<Vec<WindowDefinition>> {
    length_repeat(be_u8, decode_single_window_definiton).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_window_definitions() {
        let data = std::fs::read("data/test/wds.dat").unwrap();
        // strip segment header
        let data = &data[13..];

        let wds = decode_wds(&mut Bytes::new(data)).unwrap();

        assert_eq!(wds.len(), 2);

        assert_eq!(wds[0].id, 0);
        assert_eq!(wds[0].x, 773);
        assert_eq!(wds[0].y, 108);
        assert_eq!(wds[0].width, 377);
        assert_eq!(wds[0].height, 43);

        assert_eq!(wds[1].id, 1);
        assert_eq!(wds[1].x, 739);
        assert_eq!(wds[1].y, 928);
        assert_eq!(wds[1].width, 472);
        assert_eq!(wds[1].height, 43);
    }
}
