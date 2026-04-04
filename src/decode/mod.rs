pub(crate) mod ods;
pub(crate) mod pcs;
pub(crate) mod pds;
pub(crate) mod rle;
pub(crate) mod wds;

use chrono::NaiveTime;
use winnow::{
    Bytes,
    binary::{be_u8, be_u16, be_u32, length_take},
    combinator::{eof, repeat},
    error::{ContextError, StrContext, StrContextValue},
    prelude::*,
    token::{literal, take},
};

fn convert_ts(ts: u32) -> NaiveTime {
    let millis = ts / 90;
    let seconds = millis / 1000;
    let nanos = (millis % 1000) * 1_000_000;

    NaiveTime::from_num_seconds_from_midnight_opt(seconds, nanos).unwrap()
}

#[derive(Debug, Clone, Copy)]
#[expect(clippy::upper_case_acronyms)]
enum SegmentType {
    PCS,
    WDS,
    PDS,
    ODS,
    END,
}

#[derive(Debug, Clone)]
enum Segment {
    Pcs(NaiveTime, pcs::PresentationComposition),
    Wds(Vec<wds::WindowDefinition>),
    Pds(pds::PaletteDefinition),
    Ods(ods::ObjectDefinition),
    End,
}

fn parse_segment_type(input: &mut &Bytes) -> winnow::Result<SegmentType> {
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
    mut parser: fn(&mut &Bytes) -> winnow::Result<T>,
) -> winnow::Result<T> {
    let mut input = Bytes::new(seg_data);
    let parsed = parser.parse_next(&mut input)?;
    eof.context(StrContext::Expected(StrContextValue::Description(
        "end of segment payload",
    )))
    .parse_next(&mut input)?;

    Ok(parsed)
}

fn parse_segment(input: &mut &Bytes) -> winnow::Result<Segment> {
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
        SegmentType::PCS => parse_payload(seg_data, pcs::decode_pcs)
            .map(|seg| Segment::Pcs(pts, seg))
            .map_err(|mut err| {
                err.push(StrContext::Label("PCS segment"));
                err
            }),
        SegmentType::WDS => parse_payload(seg_data, wds::decode_wds)
            .map(Segment::Wds)
            .map_err(|mut err| {
                err.push(StrContext::Label("WDS segment"));
                err
            }),
        SegmentType::PDS => Ok(Segment::Pds(pds::decode_pds(seg_data))),
        SegmentType::ODS => parse_payload(seg_data, ods::decode_ods)
            .map(Segment::Ods)
            .map_err(|mut err| {
                err.push(StrContext::Label("ODS segment"));
                err
            }),
        SegmentType::END => Ok(Segment::End),
    }
}

fn parse_segments(input: &mut &Bytes) -> winnow::Result<Vec<Segment>> {
    repeat(0.., parse_segment)
        .context(StrContext::Label("PGS segments"))
        .parse_next(input)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DisplaySetState {
    Incomplete,
    EmptyFrame,
    Complete,
}

#[derive(Debug, Clone)]
pub(crate) struct DisplaySet {
    #[expect(dead_code)]
    pub(crate) pts: NaiveTime,
    pub(crate) pcs: pcs::PresentationComposition,
    #[expect(dead_code)]
    pub(crate) wds: Vec<wds::WindowDefinition>,
    pub(crate) pds: pds::PaletteDefinition,
    pub(crate) ods: ods::ObjectDefinition,
}

#[derive(Debug, Clone, Default)]
struct DisplaySetBuilder {
    pts: Option<NaiveTime>,
    pcs: Option<pcs::PresentationComposition>,
    wds: Vec<wds::WindowDefinition>,
    pds: Option<pds::PaletteDefinition>,
    ods: Option<ods::ObjectDefinition>,
}

impl DisplaySetBuilder {
    fn state(&self) -> DisplaySetState {
        if self.pts.is_none() {
            return DisplaySetState::Incomplete;
        }

        if self.pcs.is_some() && !self.wds.is_empty() {
            if self.pds.is_some() && self.ods.is_some() {
                return DisplaySetState::Complete;
            }

            return DisplaySetState::EmptyFrame;
        }

        DisplaySetState::Incomplete
    }

    fn build(self) -> DisplaySet {
        DisplaySet {
            pts: self.pts.unwrap(),
            pcs: self.pcs.unwrap(),
            wds: self.wds,
            pds: self.pds.unwrap(),
            ods: self.ods.unwrap(),
        }
    }
}

pub(crate) fn parse_frames(bytes: &[u8]) -> Result<Vec<DisplaySet>, ContextError> {
    let mut input = Bytes::new(bytes);
    let segments = (parse_segments, eof)
        .map(|(segments, _)| segments)
        .parse_next(&mut input)?;

    let mut display_sets = Vec::new();
    let mut running_ds = DisplaySetBuilder::default();

    for segment in segments {
        match segment {
            Segment::Pcs(pts, seg) => {
                running_ds.pts = Some(pts);
                running_ds.pcs = Some(seg);
            }
            Segment::Wds(mut seg) => {
                running_ds.wds.append(&mut seg);
            }
            Segment::Pds(seg) => {
                running_ds.pds = Some(seg);
            }
            Segment::Ods(seg) => {
                running_ds.ods = Some(seg);
            }
            Segment::End => {
                display_sets.push(running_ds);
                running_ds = DisplaySetBuilder::default();
            }
        }
    }

    Ok(display_sets
        .into_iter()
        .filter(|x| x.state() == DisplaySetState::Complete)
        .map(|x| x.build())
        .collect())
}
