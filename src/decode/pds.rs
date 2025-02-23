use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};
use log::trace;

use simple_matrix::Matrix;

#[derive(Debug, Clone)]
pub struct PaletteDefinition {
    pub id: u8,
    pub version: u8,
    pub entries: Vec<PaletteEntry>,
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub id: u8,
    pub y: u8,     // (Y) Luminance
    pub cr: u8,    // (Cr) Color Difference Red
    pub cb: u8,    // (Cb) Color Difference Blue
    pub alpha: u8, // Transparency
}

impl PaletteEntry {
    pub fn new(id: u8, y: u8, cr: u8, cb: u8, alpha: u8) -> Self {
        Self {
            id,
            y,
            cr,
            cb,
            alpha,
        }
    }

    pub fn rgba(&self) -> [f32; 4] {
        let conv_matrix = Matrix::from_iter(
            3,
            3,
            vec![1.0f64, 1., 1., 0., -0.1873, 1.8556, 1.5748, -0.4682, 0.],
        );

        let ycrcb = Matrix::from_iter(
            3,
            1,
            vec![
                self.y as f64 / 255.0,
                self.cb as f64 / 255.0,
                self.cr as f64 / 255.0,
            ],
        );

        let rgb = conv_matrix * ycrcb;

        // HACK: clamps should not be necessary
        [
            (*rgb.get(0, 0).unwrap() as f32).clamp(0., 1.),
            (*rgb.get(1, 0).unwrap() as f32).clamp(0., 1.),
            (*rgb.get(2, 0).unwrap() as f32).clamp(0., 1.),
            (self.alpha as f32 / 255.0).clamp(0., 1.),
        ]
    }
}

pub fn decode_pds(data: &[u8]) -> PaletteDefinition {
    // trace!("{:x?}", &data);
    let mut c = Cursor::new(data);

    let palette_id = c.read_u8().unwrap();
    let version = c.read_u8().unwrap();

    let mut entries: Vec<PaletteEntry> = vec![];

    let mut entry_data: Vec<u8> = vec![];
    c.read_to_end(&mut entry_data).unwrap();
    // trace!("{:x?}", &entry_data);

    assert_eq!(entry_data.len() % 5, 0);
    let num_entries = entry_data.len() / 5;

    let mut c = Cursor::new(entry_data);

    for i in 0..num_entries {
        let id = c.read_u8().unwrap();
        let y = c.read_u8().unwrap();
        let cr = c.read_u8().unwrap();
        let cb = c.read_u8().unwrap();
        let alpha = c.read_u8().unwrap();

        entries.push(PaletteEntry::new(id, y, cr, cb, alpha));
    }

    PaletteDefinition {
        id: palette_id,
        version,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn pds() {
        let data = std::fs::read("data/test/pds.dat").unwrap();
        // strip segment header
        let data = &data[13..];

        let pds = decode_pds(data);

        assert_eq!(pds.id, 0);
        assert_eq!(pds.version, 0);
        assert_eq!(pds.entries.len(), 31);

        let data = std::fs::read("data/test/pds2.dat").unwrap();
        let data = &data[13..];

        let pds = decode_pds(data);

        assert_eq!(pds.id, 0);
        assert_eq!(pds.version, 0);
        assert_eq!(pds.entries.len(), 29);
    }
}
