#![allow(unused_imports, unused_variables, dead_code)]

use std::{
    env, fs,
    io::{Cursor, Read, Seek, SeekFrom},
};

use iced::Application as _;

use byteorder::{BigEndian, ReadBytesExt};
use chrono::NaiveTime;

mod decode;
mod segment;
mod ui;
mod widgets;

fn convert_ts(ts: u32) -> NaiveTime {
    let millis = ts / 90;
    let seconds = millis / 1000;
    let nanos = (millis % 1000) * 1_000_000;

    NaiveTime::from_num_seconds_from_midnight_opt(seconds, nanos).unwrap()
}

// Codec information taken from:
// http://blog.thescorpius.com/index.php/2017/07/15/presentation-graphic-stream-sup-files-bluray-subtitle-format/

// .sup files are called PGS (Presentation Graphic Streams)

// A Presentation Graphic Stream (PGS) is made of several functional segments one after another. These segments have the following header:
// Name            Bytes    Description
// Magic Number    2        "PG" (0x5047)
// PTS             4        Presentation Timestamp
// DTS             4        Decoding Timestamp
// Segment Type    1        0x14: PDS
//                          0x15: ODS
//                          0x16: PCS
//                          0x17: WDS
//                          0x80: END
// Segment Size    2        Size of the segment

#[derive(Debug, Clone, Copy)]
#[expect(clippy::upper_case_acronyms)]
enum SegmentType {
    /// Presentation Composition Segment
    PCS,

    /// Window Definition Segment
    WDS,

    /// Palette Definition Segment
    PDS,

    /// Object Definition Segment
    ODS,

    /// End of Display Set Segment
    END,
}

#[derive(Debug, Clone)]
enum Segment {
    Pcs(NaiveTime, decode::pcs::PresentationComposition),
    Wds(NaiveTime, Vec<decode::wds::WindowDefinition>),
    Pds(NaiveTime, decode::pds::PaletteDefinition),
    Ods(NaiveTime, decode::ods::ObjectDefinition),
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplaySetState {
    Incomplete,
    EmptyFrame,
    Complete,
}

#[derive(Debug, Clone, Default)]
pub struct DisplaySet {
    pts: Option<NaiveTime>,
    pcs: Option<decode::pcs::PresentationComposition>,
    wds: Vec<decode::wds::WindowDefinition>,
    pds: Option<decode::pds::PaletteDefinition>,
    ods: Option<decode::ods::ObjectDefinition>,
}

impl DisplaySet {
    pub fn empty() -> Self {
        Self {
            pts: None,
            pcs: None,
            wds: vec![],
            pds: None,
            ods: None,
        }
    }

    pub fn state(&self) -> DisplaySetState {
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

    pub fn pts(&self) -> NaiveTime {
        self.pts.unwrap()
    }

    pub fn ods(&self) -> &decode::ods::ObjectDefinition {
        self.ods.as_ref().unwrap()
    }

    pub fn pds(&self) -> &decode::pds::PaletteDefinition {
        self.pds.as_ref().unwrap()
    }
}

fn main() -> iced::Result {
    let mut args = env::args();
    let file = args.nth(1).unwrap();
    let bytes = fs::read(file).unwrap();
    let bytes_len = bytes.len();

    let mut segments: Vec<Segment> = vec![];
    let mut display_sets: Vec<DisplaySet> = vec![];

    let mut c = Cursor::new(bytes);

    loop {
        if c.position() >= bytes_len as u64 {
            break;
        }

        println!("processing segment at: {:#X}", c.position());

        // skip PG magic number
        let mut magic = [0u8; 2];
        c.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, &[0x50, 0x47]);

        let pts = c.read_u32::<BigEndian>().unwrap();
        let pts = convert_ts(pts);

        // skip "DTS" useless value
        // DTS is always 0
        c.seek(SeekFrom::Current(4)).unwrap();

        let segtype = c.read_u8().unwrap();
        let segtype = match segtype {
            0x14 => SegmentType::PDS,
            0x15 => SegmentType::ODS,
            0x16 => SegmentType::PCS,
            0x17 => SegmentType::WDS,
            0x80 => SegmentType::END,
            byte => panic!("invalid segment type {byte:?}"),
        };

        let segment_size = c.read_u16::<BigEndian>().unwrap();

        let mut segdata = vec![0u8; segment_size as usize];
        c.read_exact(&mut segdata).unwrap();
        assert_eq!(segment_size as usize, segdata.len());

        let segment = match segtype {
            SegmentType::PCS => Segment::Pcs(pts, decode::pcs(segdata)),
            SegmentType::WDS => Segment::Wds(pts, decode::wds(&segdata)),
            SegmentType::PDS => Segment::Pds(pts, decode::pds(&segdata)),
            SegmentType::ODS => Segment::Ods(pts, decode::ods(segdata)),
            SegmentType::END => Segment::End,
        };

        segments.push(segment);
    }

    let segs_len = segments.len();

    let mut running_ds = DisplaySet::empty();
    for segment in segments {
        match segment {
            Segment::Pcs(pts, seg) => {
                running_ds.pts = Some(pts);
                running_ds.pcs = Some(seg);
            }
            Segment::Wds(pts, seg) => {
                let mut seg = seg.clone();
                running_ds.wds.append(&mut seg);
            }
            Segment::Pds(pts, seg) => {
                running_ds.pds = Some(seg);
            }
            Segment::Ods(pts, seg) => {
                running_ds.ods = Some(seg);
            }
            Segment::End => {
                display_sets.push(running_ds);
                running_ds = DisplaySet::empty();
            }
        }
    }

    println!("processed {} segments", segs_len);
    println!("processed {} display sets", display_sets.len());

    let frames: Vec<_> = display_sets
        .iter()
        .filter(|&x| x.state() == DisplaySetState::Complete)
        .collect();
    let num_frames = frames.len();

    println!("processed {} frames", frames.len());

    // for (i, frame) in frames.iter().enumerate() {
    //     println!(
    //         "frame {:>3} / {}  @  {}",
    //         i,
    //         num_frames,
    //         frame.pts().format("%H:%M:%S%.3f"),
    //     );

    //     let ods = frame.ods();

    //     image::save_buffer(
    //         format!("output/frame-{}.png", i),
    //         &ods.data,
    //         ods.width as u32,
    //         ods.height as u32,
    //         image::ColorType::L8,
    //     )
    //     .unwrap();
    // }

    let frames = frames.into_iter().cloned().collect();

    iced::application("sup-decode", ui::SupViewer::update, ui::SupViewer::view)
        // .subscription(ui::SupViewer::subscription)
        .antialiasing(true)
        .centered()
        .run_with(|| ui::SupViewer::new(frames))
}
