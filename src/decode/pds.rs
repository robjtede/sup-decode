use std::{
    fmt,
    io::{Cursor, Read, Seek, SeekFrom},
};

use byteorder::{BigEndian, ReadBytesExt};
use log::trace;

#[derive(Clone)]
pub struct PaletteDefinition {
    pub id: u8,
    pub version: u8,
    pub entries: Vec<PaletteEntry>,
}

impl PaletteDefinition {
    pub(crate) fn find_by_id(&self, id: u8) -> Option<&PaletteEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

impl fmt::Debug for PaletteDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "id={} version={} ({} entries)",
            self.id,
            self.version,
            self.entries.len(),
        )?;

        if f.alternate() {
            writeln!(f)?;

            for entry in &self.entries {
                writeln!(f, "  {entry:?}")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct PaletteEntry {
    pub id: u8,
    pub y: u8,     // (Y) Luminance
    pub cr: u8,    // (Cr) Color Difference Red
    pub cb: u8,    // (Cb) Color Difference Blue
    pub alpha: u8, // Transparency
}

impl PaletteEntry {
    pub(crate) fn new(id: u8, y: u8, cr: u8, cb: u8, alpha: u8) -> Self {
        Self {
            id,
            y,
            cr,
            cb,
            alpha,
        }
    }

    pub(crate) fn rgba(&self) -> [f32; 4] {
        let Self {
            id: _,
            y,
            cb,
            cr,
            alpha,
        } = self;

        let y = *y as f32;
        let cb = *cb as f32;
        let cr = *cr as f32;
        let alpha = *alpha as f32;

        let r = 1.164 * (y - 16.0) + 1.596 * (cr - 128.0);
        let g = 1.164 * (y - 16.0) - 0.392 * (cb - 128.0) - 0.813 * (cr - 128.0);
        let b = 1.164 * (y - 16.0) + 2.017 * (cb - 128.0);

        // println!("y={y}, cb={cb}, cr={cr}");
        // println!("r={r}, g={g}, b={b}");
        // println!();

        [
            r.clamp(0.0, 255.0) / 255.0,
            g.clamp(0.0, 255.0) / 255.0,
            b.clamp(0.0, 255.0) / 255.0,
            alpha.clamp(0.0, 255.0) / 255.0,
        ]
    }
}

impl fmt::Debug for PaletteEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "id={} YCbCrA={},{},{},{}",
            self.id, self.y, self.cb, self.cr, self.alpha
        )?;

        Ok(())
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
