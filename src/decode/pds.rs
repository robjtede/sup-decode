use std::io::{Cursor, Read, Seek, SeekFrom};

use log::trace;
use byteorder::{BigEndian, ReadBytesExt};

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

        let pds = decode_pds(&data);

        assert_eq!(pds.id, 0);
        assert_eq!(pds.version, 0);
        assert_eq!(pds.entries.len(), 31);

        let data = std::fs::read("data/test/pds2.dat").unwrap();
        let data = &data[13..];

        let pds = decode_pds(&data);

        assert_eq!(pds.id, 0);
        assert_eq!(pds.version, 0);
        assert_eq!(pds.entries.len(), 29);
    }
}
