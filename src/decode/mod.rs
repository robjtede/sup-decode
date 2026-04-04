pub(crate) mod ods;
pub(crate) mod pcs;
pub(crate) mod pds;
pub(crate) mod rle;
pub(crate) mod wds;

use chrono::NaiveTime;
use winnow::{
    Bytes,
    error::ContextError,
    prelude::*,
};

use crate::segment::{Segment, parse_segment};

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
) -> Result<Vec<DisplaySet>, ContextError> {
    let mut input = Bytes::new(bytes);
    let mut display_sets = Vec::new();
    let mut running_ds = DisplaySetBuilder::default();

    while !input.is_empty() {
        let segment = parse_segment.parse_next(&mut input).map_err(|err| {
            err.into_inner().unwrap_or_else(|_err| {
                panic!("complete parsers should not report `ErrMode::Incomplete(_)`")
            })
        })?;

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
                let completed = std::mem::take(&mut running_ds);

                if completed.state() == DisplaySetState::Complete {
                    display_sets.push(completed.build());
                }
            }
        }
    }

    Ok(display_sets)
}
