use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

use crate::decode;

#[derive(Debug, Clone, Hash)]
pub struct ObjectDefinition {
    pub id: u16,
    pub version: u8,
    pub sequence_flag: SequenceFlag,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum SequenceFlag {
    First,
    Last,
    Both,
}

pub fn decode_ods(data: Vec<u8>) -> ObjectDefinition {
    let mut c = Cursor::new(data);

    let object_id = c.read_u16::<BigEndian>().unwrap();
    let version = c.read_u8().unwrap();

    let sequence_flag = match c.read_u8().unwrap() {
        0x40 => SequenceFlag::Last,
        0x80 => SequenceFlag::First,
        0xC0 => SequenceFlag::Both,
        x => panic!("unknown sequence flag: {}", x),
    };

    let data_len = c.read_u24::<BigEndian>().unwrap();

    // TODO: something about ONLY IF first segment then
    // take width and height

    let width = c.read_u16::<BigEndian>().unwrap();
    let height = c.read_u16::<BigEndian>().unwrap();

    let mut object_data = vec![];
    c.read_to_end(&mut object_data).unwrap();
    assert_eq!(object_data.len(), (data_len - 4) as usize);

    let image = decode::rle(&object_data);

    ObjectDefinition {
        id: object_id,
        version,
        sequence_flag,
        width,
        height,
        data: image,
    }
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
