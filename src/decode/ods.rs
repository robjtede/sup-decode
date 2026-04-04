use std::{
    fmt,
    io::{Cursor, Read, Seek, SeekFrom},
};

use byteorder::{BigEndian, ReadBytesExt};
use winnow::{
    Bytes,
    binary::{be_u8, be_u16, be_u24},
    prelude::*,
    token::{rest, take},
};

use crate::decode;

#[derive(Clone, Hash)]
pub struct ObjectDefinition {
    pub id: u16,
    pub version: u8,
    pub sequence_flag: SequenceFlag,
    pub width: u16,
    pub height: u16,
    data_len: u32,
    pub data: Vec<u8>,
}

impl fmt::Debug for ObjectDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id={}", self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum SequenceFlag {
    First,
    Last,
    Both,
}

pub fn decode_ods(input: &mut &Bytes) -> winnow::Result<ObjectDefinition> {
    // let data_len = c.read_u24::<BigEndian>().unwrap();

    // let width = c.read_u16::<BigEndian>().unwrap();
    // let height = c.read_u16::<BigEndian>().unwrap();

    let mut obj = (
        be_u16,
        be_u8,
        be_u8.verify_map(|flag| {
            Some(match flag {
                0x40 => SequenceFlag::Last,
                0x80 => SequenceFlag::First,
                0xC0 => SequenceFlag::Both,
                x => return None,
            })
        }),
        be_u24,
        // TODO: something about ONLY IF first segment then
        // take width and height
        be_u16,
        be_u16,
    )
        .map(
            |(object_id, version, sequence_flag, data_len, width, height)| ObjectDefinition {
                id: object_id,
                version,
                sequence_flag,
                width,
                height,
                data_len,
                data: vec![],
            },
        )
        .parse_next(input)?;

    let image = take(obj.data_len as usize)
        .map(Bytes::new)
        .and_then(decode::rle)
        .parse_next(input)?;

    obj.data = image;

    Ok(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn ods() {
        // let data = hex!("");
    }
}
