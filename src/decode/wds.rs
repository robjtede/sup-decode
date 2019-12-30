use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub struct WindowDefinition {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl WindowDefinition {
    pub fn new(id: u8, x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
        }
    }
}

pub fn decode_wds(data: &[u8]) -> Vec<WindowDefinition> {
    let mut c = Cursor::new(data);

    let num_windows = c.read_u8().unwrap();

    let mut windows: Vec<WindowDefinition> = vec![];

    for i in 0..num_windows {
        let id = c.read_u8().unwrap();
        let x = c.read_u16::<BigEndian>().unwrap();
        let y = c.read_u16::<BigEndian>().unwrap();
        let width = c.read_u16::<BigEndian>().unwrap();
        let height = c.read_u16::<BigEndian>().unwrap();

        windows.push(WindowDefinition::new(id, x, y, width, height));
    }

    windows
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

        let wds = decode_wds(&data);

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
