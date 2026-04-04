use chrono::NaiveTime;
use winnow::{
    Bytes, ModalResult,
    binary::{be_u8, be_u16, be_u32, length_take},
    combinator::eof,
    error::{StrContext, StrContextValue},
    prelude::*,
    token::{literal, take},
};

use crate::decode;

#[cfg(test)]
pub(crate) fn segment_on<'a>(bytes: &'a [u8], segmark: &'_ [u8]) -> Vec<&'a [u8]> {
    let split_len = segmark.len();
    let bytes_len = bytes.len();

    if split_len == 0 {
        panic!("segment marker shouldnt be empty");
    }

    let mut segments = vec![];
    let mut seg_start = 0;

    for i in 0..(bytes_len - split_len) {
        if &bytes[i..(i + split_len)] == segmark {
            if i != 0 {
                segments.push(&bytes[seg_start..i]);
            }

            seg_start = i;
        }
    }

    // push final segment
    segments.push(&bytes[seg_start..bytes_len]);

    segments
}

fn convert_ts(ts: u32) -> NaiveTime {
    let millis = ts / 90;
    let seconds = millis / 1000;
    let nanos = (millis % 1000) * 1_000_000;

    NaiveTime::from_num_seconds_from_midnight_opt(seconds, nanos).unwrap()
}

#[derive(Debug, Clone, Copy)]
#[expect(clippy::upper_case_acronyms)]
pub(crate) enum SegmentType {
    PCS,
    WDS,
    PDS,
    ODS,
    END,
}

#[derive(Debug, Clone)]
pub(crate) enum Segment {
    Pcs(NaiveTime, decode::pcs::PresentationComposition),
    Wds(Vec<decode::wds::WindowDefinition>),
    Pds(decode::pds::PaletteDefinition),
    Ods(decode::ods::ObjectDefinition),
    End,
}

pub(crate) fn parse_segment_type(input: &mut &Bytes) -> ModalResult<SegmentType> {
    be_u8
        .verify_map(|byte| {
            Some(match byte {
                0x14 => SegmentType::PDS,
                0x15 => SegmentType::ODS,
                0x16 => SegmentType::PCS,
                0x17 => SegmentType::WDS,
                0x80 => SegmentType::END,
                _ => return None,
            })
        })
        .context(StrContext::Label("PGS segment type"))
        .context(StrContext::Expected(StrContextValue::Description(
            "0x14, 0x15, 0x16, 0x17, or 0x80",
        )))
        .parse_next(input)
}

fn parse_payload<T>(
    seg_data: &[u8],
    mut parser: fn(&mut &Bytes) -> ModalResult<T>,
) -> ModalResult<T> {
    let mut input = Bytes::new(seg_data);
    let parsed = parser.parse_next(&mut input)?;
    eof.context(StrContext::Expected(StrContextValue::Description(
        "end of segment payload",
    )))
    .parse_next(&mut input)?;

    Ok(parsed)
}

pub(crate) fn parse_segment(input: &mut &Bytes) -> ModalResult<Segment> {
    let (pts, seg_type, seg_data) = (
        literal(&[0x50, 0x47])
            .void()
            .context(StrContext::Label("PGS magic number"))
            .context(StrContext::Expected(StrContextValue::Description(
                "\"PG\" segment marker",
            ))),
        be_u32
            .map(convert_ts)
            .context(StrContext::Label("PGS presentation timestamp")),
        take(4_usize)
            .void()
            .context(StrContext::Label("PGS decoding timestamp")),
        parse_segment_type,
        length_take(be_u16.context(StrContext::Label("PGS segment size")))
            .context(StrContext::Label("PGS segment payload")),
    )
        .context(StrContext::Label("PGS segment"))
        .map(|(_, pts, _, seg_type, seg_data)| (pts, seg_type, seg_data))
        .parse_next(input)?;

    match seg_type {
        SegmentType::PCS => parse_payload(seg_data, decode::pcs::decode_pcs)
            .map(|seg| Segment::Pcs(pts, seg))
            .map_err(|err| {
                err.map(|mut inner| {
                    inner.push(StrContext::Label("PCS segment"));
                    inner
                })
            }),
        SegmentType::WDS => parse_payload(seg_data, decode::wds::decode_wds)
            .map(Segment::Wds)
            .map_err(|err| {
                err.map(|mut inner| {
                    inner.push(StrContext::Label("WDS segment"));
                    inner
                })
            }),
        SegmentType::PDS => Ok(Segment::Pds(decode::pds::decode_pds(seg_data))),
        SegmentType::ODS => parse_payload(seg_data, decode::ods::decode_ods)
            .map(Segment::Ods)
            .map_err(|err| {
                err.map(|mut inner| {
                    inner.push(StrContext::Label("ODS segment"));
                    inner
                })
            }),
        SegmentType::END => Ok(Segment::End),
    }
}

// TODO: edge cases
// empty split
// empty bytes
// data before first segmark
// segmark at end
// no segmark

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_marker_segment() {
        let bytes = vec![0xff, 0x0, 0xff, 0x1, 0x2, 0xff, 0x3, 0x4, 0xff, 0x5];
        let segments: Vec<&[u8]> = vec![
            &[0xff, 0x0],
            &[0xff, 0x1, 0x2],
            &[0xff, 0x3, 0x4],
            &[0xff, 0x5],
        ];

        assert_eq!(segment_on(&bytes, &[0xff]), segments)
    }

    #[test]
    fn multi_marker_segment() {
        let bytes = vec![0x42, 0xff, 0x0, 0x42, 0xff, 0x1, 0x2, 0x42, 0xff, 0x3];
        let segments: Vec<&[u8]> = vec![
            &[0x42, 0xff, 0x0],
            &[0x42, 0xff, 0x1, 0x2],
            &[0x42, 0xff, 0x3],
        ];

        assert_eq!(segment_on(&bytes, &[0x42, 0xff]), segments)
    }
}
