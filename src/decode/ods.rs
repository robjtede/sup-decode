use std::fmt;

use winnow::{
    Bytes,
    binary::{be_u8, be_u16, be_u24},
    combinator::repeat,
    error::{ContextError, StrContext, StrContextValue},
    prelude::*,
    token::take,
};

use crate::decode::rle::decode_rle;

#[derive(Clone, Hash)]
pub(crate) struct ObjectDefinition {
    pub(crate) id: u16,
    pub(crate) version: u8,
    pub(crate) sequence_flag: SequenceFlag,
    pub(crate) width: u16,
    pub(crate) height: u16,
    data_len: u32,
    pub(crate) data: Vec<u8>,
}

impl fmt::Debug for ObjectDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id={}", self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub(crate) enum SequenceFlag {
    Middle,
    First,
    Last,
    Both,
}

impl SequenceFlag {
    fn has_dimensions(self) -> bool {
        matches!(self, Self::First | Self::Both)
    }
}

fn parse_sequence_flag(input: &mut &Bytes) -> winnow::Result<SequenceFlag> {
    be_u8
        .verify_map(|flag| {
            Some(match flag {
                0x00 => SequenceFlag::Middle,
                0x40 => SequenceFlag::Last,
                0x80 => SequenceFlag::First,
                0xC0 => SequenceFlag::Both,
                _ => return None,
            })
        })
        .context(StrContext::Label("ODS sequence flag"))
        .context(StrContext::Expected(StrContextValue::Description(
            "0x00, 0x40, 0x80, or 0xC0",
        )))
        .parse_next(input)
}

fn parse_dimensions(input: &mut &Bytes) -> winnow::Result<(u16, u16)> {
    (
        be_u16.context(StrContext::Label("ODS width")),
        be_u16.context(StrContext::Label("ODS height")),
    )
        .context(StrContext::Label("ODS dimensions"))
        .parse_next(input)
}

fn decode_rle_stream(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    repeat(0.., decode_rle)
        .map(|chunks: Vec<Vec<u8>>| chunks.into_iter().flatten().collect())
        .context(StrContext::Label("ODS RLE data"))
        .parse_next(input)
}

fn parse_object_data(
    input: &mut &Bytes,
    sequence_flag: SequenceFlag,
    data_len: u32,
) -> winnow::Result<(u16, u16, Vec<u8>)> {
    let (width, height, rle_len) = if sequence_flag.has_dimensions() {
        let (width, height) = parse_dimensions.parse_next(input)?;
        let rle_len = data_len.checked_sub(4).ok_or_else(|| {
            let mut err = ContextError::new();
            err.push(StrContext::Label("ODS object data length"));
            err
        })?;
        (width, height, rle_len)
    } else {
        (0, 0, data_len)
    };

    let data = take(rle_len as usize)
        .map(Bytes::new)
        .and_then(decode_rle_stream)
        .context(StrContext::Label("ODS object data"))
        .parse_next(input)?;

    Ok((width, height, data))
}

pub(crate) fn decode_ods(input: &mut &Bytes) -> winnow::Result<ObjectDefinition> {
    let (id, version, sequence_flag, data_len) = (
        be_u16.context(StrContext::Label("ODS object id")),
        be_u8.context(StrContext::Label("ODS version")),
        parse_sequence_flag,
        be_u24.context(StrContext::Label("ODS object data length")),
    )
        .context(StrContext::Label("ODS header"))
        .parse_next(input)?;

    let (width, height, data) = parse_object_data(input, sequence_flag, data_len)?;

    Ok(ObjectDefinition {
        id,
        version,
        sequence_flag,
        width,
        height,
        data_len,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ods() {
        let data = std::fs::read("data/test/ods.dat").unwrap();
        let ods = decode_ods.parse(&mut Bytes::new(&data)).unwrap();

        assert_eq!(0, ods.id);
        assert_eq!(0, ods.version);
        assert_eq!(SequenceFlag::Both, ods.sequence_flag);
        assert_eq!(741, ods.width);
        assert_eq!(60, ods.height);
        assert_eq!(0x00_4506, ods.data_len);
        assert_eq!(ods.width as usize * ods.height as usize, ods.data.len());
    }

    #[test]
    fn invalid_sequence_flag_errors() {
        parse_sequence_flag(&mut Bytes::new(&[0x20])).unwrap_err();
    }

    #[test]
    fn middle_fragment_omits_dimensions() {
        let data = [
            0x12, 0x34, // object id
            0x02, // version
            0x00, // middle fragment
            0x00, 0x00, 0x01, // object data length
            0x2a, // one pixel
        ];

        let ods = decode_ods(&mut Bytes::new(&data)).unwrap();

        assert_eq!(0x1234, ods.id);
        assert_eq!(2, ods.version);
        assert_eq!(SequenceFlag::Middle, ods.sequence_flag);
        assert_eq!(0, ods.width);
        assert_eq!(0, ods.height);
        assert_eq!(vec![0x2a], ods.data);
    }
}
