use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};
use winnow::{
    binary::{be_u8, be_u16, length_repeat},
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct WindowDefinition {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
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

fn decode_single_window_definiton(input: &mut &[u8]) -> winnow::Result<WindowDefinition> {
    (be_u8, be_u16, be_u16, be_u16, be_u16)
        .map(WindowDefinition::from_tuple)
        .parse_next(input)
}

pub fn decode_wds(input: &[u8]) -> Vec<WindowDefinition> {
    length_repeat(be_u8, decode_single_window_definiton)
        .parse(input)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn decode_window_definitions() {
        let data = std::fs::read("data/test/wds.dat").unwrap();
        // strip segment header
        let data = &data[13..];

        let wds = decode_wds(data);

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
