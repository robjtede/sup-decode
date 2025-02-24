#![allow(unused_imports, unused_variables, dead_code)]

use std::{
    env, fs,
    io::{Cursor, Read, Seek, SeekFrom},
    mem,
};

use iced::Application as _;

use byteorder::{BigEndian, ReadBytesExt};
use chrono::NaiveTime;
use strum::IntoDiscriminant as _;

mod decode;
mod segment;
mod ui;

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

#[derive(Debug, Clone, strum::EnumDiscriminants)]
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

#[derive(Debug, Clone)]
pub struct DisplaySet {
    pts: NaiveTime,
    pcs: decode::pcs::PresentationComposition,
    wds: Vec<decode::wds::WindowDefinition>,
    pds: decode::pds::PaletteDefinition,
    ods: decode::ods::ObjectDefinition,
}

#[derive(Debug, Clone, Default)]
pub struct DisplaySetBuilder {
    pts: Option<NaiveTime>,
    pcs: Option<decode::pcs::PresentationComposition>,
    wds: Vec<decode::wds::WindowDefinition>,
    pds: Option<decode::pds::PaletteDefinition>,
    ods: Option<decode::ods::ObjectDefinition>,
}

impl DisplaySetBuilder {
    pub fn new() -> Self {
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

    pub fn build(self) -> DisplaySet {
        DisplaySet {
            pts: self.pts.unwrap(),
            pcs: self.pcs.unwrap(),
            wds: self.wds,
            pds: self.pds.unwrap(),
            ods: self.ods.unwrap(),
        }
    }
}

fn main() -> iced::Result {
    let mut args = env::args();
    let file = args.nth(1).unwrap();
    let bytes = fs::read(file).unwrap();
    let bytes_len = bytes.len();

    let mut segments = Vec::new();
    let mut display_sets = Vec::new();

    let mut c = Cursor::new(bytes);

    loop {
        if c.position() >= bytes_len as u64 {
            break;
        }

        print!("processing segment at: {:#X} - ", c.position());

        // skip PG magic number
        let mut magic = [0u8; 2];
        c.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, &[0x50, 0x47]);

        let pts = c.read_u32::<BigEndian>().unwrap();
        let pts = convert_ts(pts);

        // skip "DTS" useless value
        // DTS is always 0
        c.seek(SeekFrom::Current(4)).unwrap();

        let seg_type = c.read_u8().unwrap();
        let seg_type = match seg_type {
            0x14 => SegmentType::PDS,
            0x15 => SegmentType::ODS,
            0x16 => SegmentType::PCS,
            0x17 => SegmentType::WDS,
            0x80 => SegmentType::END,
            byte => panic!("invalid segment type {byte:?}"),
        };

        let segment_size = c.read_u16::<BigEndian>().unwrap();

        let mut seg_data = vec![0u8; segment_size as usize];
        c.read_exact(&mut seg_data).unwrap();
        assert_eq!(segment_size as usize, seg_data.len());

        let segment = match seg_type {
            SegmentType::PCS => {
                let seg = decode::pcs(seg_data);
                println!("PCS {seg:#?}");
                Segment::Pcs(pts, seg)
            }
            SegmentType::WDS => {
                let seg = decode::wds(&seg_data);
                println!("WDS: {} windows", seg.len());
                Segment::Wds(pts, seg)
            }
            SegmentType::PDS => {
                let seg = decode::pds(&seg_data);
                println!("PDS {seg:?}");
                Segment::Pds(pts, seg)
            }
            SegmentType::ODS => {
                let seg = decode::ods(seg_data);
                println!("ODS {seg:?}");
                Segment::Ods(pts, seg)
            }
            SegmentType::END => {
                println!("END");
                println!();
                Segment::End
            }
        };

        segments.push(segment);
    }

    let segs_len = segments.len();

    let mut running_ds = DisplaySetBuilder::new();
    for segment in segments {
        // println!("{:?}", segment.discriminant());

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
                running_ds = DisplaySetBuilder::new();
                // println!();
            }
        }
    }

    println!("processed {segs_len} segments");
    println!("processed {} display sets", display_sets.len());

    let frames = display_sets
        .into_iter()
        .filter(|x| x.state() == DisplaySetState::Complete)
        .map(|x| x.build())
        .collect::<Vec<_>>();
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

    iced::application("sup-decode", ui::SupViewer::update, ui::SupViewer::view)
        .centered()
        .run_with(|| ui::SupViewer::new(frames))
}
