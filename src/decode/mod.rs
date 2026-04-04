pub(crate) mod ods;
pub(crate) mod pcs;
pub(crate) mod pds;
pub(crate) mod rle;
pub(crate) mod wds;

use chrono::NaiveTime;
use winnow::{
    Bytes,
    combinator::eof,
    error::{ContextError, ParseError},
    prelude::*,
};

use crate::segment::{Segment, parse_segments};

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

pub(crate) fn parse_frames(
    bytes: &[u8],
) -> Result<Vec<DisplaySet>, ParseError<&'_ Bytes, ContextError>> {
    let input = Bytes::new(bytes);
    let segments = (parse_segments, eof)
        .map(|(segments, _)| segments)
        .parse(input)?;

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
