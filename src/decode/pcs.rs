use std::fmt;

use winnow::{
    Bytes,
    binary::{be_u8, be_u16, length_repeat},
    error::{StrContext, StrContextValue},
    prelude::*,
};

// The Presentation Composition Segment is used for composing a sub picture. It is made of the following fields:
// Name                             Bytes    Description
// Width                            2        Video width in pixels (ex. 0x780 = 1920)
// Height                           2        Video height in pixels (ex. 0x438 = 1080)
// Frame Rate                       1        Always 0x10. Can be ignored.
// Composition Number               2        Number of this specific composition. It is incremented by one every time a graphics update occurs.
// Composition State                1        Type of this composition. Allowed values are:
//                                           0x00: Normal
//                                           0x40: Acquisition Point
//                                           0x80: Epoch Start
// Palette Update Flag              1        Indicates if this PCS describes a Palette only Display Update. Allowed values are:
//                                           0x00: False
//                                           0x80: True
// Palette ID                       1        ID of the palette to be used in the Palette only Display Update
// Number of Composition Objects    1        Number of composition objects defined in this segment

// The composition objects, also known as window information objects, define the position on the screen of every image that will be shown. They have the following structure:
// Name                                   Bytes    Description
// Object ID                              2        ID of the ODS segment that defines the image to be shown
// Window ID                              1        Id of the WDS segment to which the image is allocated in the PCS. Up to two images may be assigned to one window
// Object Cropped Flag                    1        0x40: Force display of the cropped image object
//                                                 0x00: Off
// Object Horizontal Position             2        X offset from the top left pixel of the image on the screen
// Object Vertical Position               2        Y offset from the top left pixel of the image on the screen
// Object Cropping Horizontal Position    2        X offset from the top left pixel of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
// Object Cropping Vertical Position      2        Y offset from the top left pixel of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
// Object Cropping Width                  2        Width of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
// Object Cropping Height Position        2        Height of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.

// When the Object Cropped Flag is set to true (or actually 0x40), then the sub picture is cropped to show only a portion of it. This is used for example when you don’t want to show the whole subtitle at first, but just a few words first, and then the rest.

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CompositionState {
    /// This defines a new display. The Epoch Start contains all functional segments needed to
    /// display a new composition on the screen.
    EpochStart,

    /// This defines a display refresh. This is used to compose in the middle of the Epoch. It
    /// includes functional segments with new objects to be used in a new composition, replacing old
    /// objects with the same Object ID.
    AcquisitionPoint,

    /// This defines a display update, and contains only functional segments with elements that are
    /// different from the preceding composition. It’s mostly used to stop displaying objects on the
    /// screen by defining a composition with no composition objects (a value of zero in the Number
    /// of Composition Objects flag) but also used to define a new composition with new objects and
    /// objects defined since the Epoch Start.
    Normal,
}

/// Presentation Composition Segment
#[derive(Clone)]
pub(crate) struct PresentationComposition {
    pub(crate) comp_no: u16,
    pub(crate) comp_state: CompositionState,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) palette_id: u8,
    pub(crate) palette_update: bool,
    pub(crate) composition_objects: Vec<CompositionObject>,
}

impl PresentationComposition {
    pub(crate) fn find_object_by_id(&self, id: u16) -> Option<&CompositionObject> {
        self.composition_objects.iter().find(|obj| obj.id == id)
    }
}

impl fmt::Debug for PresentationComposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no={} state={:?}, size={}x{}, palette_id={}, palette_update={} ({} objects)",
            self.comp_no,
            self.comp_state,
            self.width,
            self.height,
            self.palette_id,
            self.palette_update,
            self.composition_objects.len(),
        )?;

        if f.alternate() && !self.composition_objects.is_empty() {
            writeln!(f)?;

            for obj in &self.composition_objects {
                write!(f, "  {obj:?}")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct CompositionObject {
    pub(crate) id: u16,
    pub(crate) window_id: u8,
    pub(crate) cropped: bool,
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) crop_x: Option<u16>,
    pub(crate) crop_y: Option<u16>,
    pub(crate) crop_width: Option<u16>,
    pub(crate) crop_height: Option<u16>,
}

impl fmt::Debug for CompositionObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "id={} window={}, x={}, y={}",
            self.id, self.window_id, self.x, self.y,
        )?;

        if self.cropped
            // some real PGS files set cropped=true but don't include the data
            && let Some(crop_x) = self.crop_x
            && let Some(crop_y) = self.crop_y
            && let Some(crop_width) = self.crop_width
            && let Some(crop_height) = self.crop_height
        {
            write!(
                f,
                ", crop_x={crop_x}, crop_y={crop_y}, crop_width={crop_width}, crop_height={crop_height}",
            )?;
        } else {
            write!(f, ", cropped={}", self.cropped)?;
        }

        Ok(())
    }
}

fn parse_comp_state(input: &mut &Bytes) -> winnow::Result<CompositionState> {
    be_u8
        .verify_map(|byte| {
            Some(match byte {
                0x80 => CompositionState::EpochStart,
                0x40 => CompositionState::AcquisitionPoint,
                0x00 => CompositionState::Normal,
                _ => return None,
            })
        })
        .context(StrContext::Label("PCS composition state"))
        .context(StrContext::Expected(StrContextValue::Description(
            "0x00, 0x40, or 0x80",
        )))
        .parse_next(input)
}

fn parse_palette_update(input: &mut &Bytes) -> winnow::Result<bool> {
    be_u8
        .verify_map(|byte| {
            Some(match byte {
                0x00 => false,
                0x80 => true,
                _ => return None,
            })
        })
        .context(StrContext::Label("PCS palette update flag"))
        .context(StrContext::Expected(StrContextValue::Description(
            "0x00 or 0x80",
        )))
        .parse_next(input)
}

fn parse_object_cropped_flag(input: &mut &Bytes) -> winnow::Result<bool> {
    be_u8
        .verify_map(|byte| {
            Some(match byte {
                0x40 => true,
                0x00 => false,
                _ => return None,
            })
        })
        .context(StrContext::Label("PCS composition object cropped flag"))
        .context(StrContext::Expected(StrContextValue::Description(
            "0x00 or 0x40",
        )))
        .parse_next(input)
}

fn parse_crop_rect(input: &mut &Bytes) -> winnow::Result<(u16, u16, u16, u16)> {
    (
        be_u16.context(StrContext::Label("PCS crop x")),
        be_u16.context(StrContext::Label("PCS crop y")),
        be_u16.context(StrContext::Label("PCS crop width")),
        be_u16.context(StrContext::Label("PCS crop height")),
    )
        .context(StrContext::Label("PCS composition object crop"))
        .parse_next(input)
}

fn parse_composition_object(input: &mut &Bytes) -> winnow::Result<CompositionObject> {
    let (id, window_id, cropped, x, y) = (
        be_u16.context(StrContext::Label("PCS composition object id")),
        be_u8.context(StrContext::Label("PCS composition object window id")),
        parse_object_cropped_flag,
        be_u16.context(StrContext::Label("PCS composition object x")),
        be_u16.context(StrContext::Label("PCS composition object y")),
    )
        .context(StrContext::Label("PCS composition object"))
        .parse_next(input)?;

    // Some real PGS files set cropped=true but omit the crop rectangle entirely.
    let (crop_x, crop_y, crop_width, crop_height) = if cropped && !input.is_empty() {
        let (crop_x, crop_y, crop_width, crop_height) = parse_crop_rect
            .context(StrContext::Label("PCS composition object"))
            .parse_next(input)?;
        (
            Some(crop_x),
            Some(crop_y),
            Some(crop_width),
            Some(crop_height),
        )
    } else {
        (None, None, None, None)
    };

    Ok(CompositionObject {
        id,
        window_id,
        cropped,
        x,
        y,
        crop_x,
        crop_y,
        crop_width,
        crop_height,
    })
}

pub(crate) fn decode_pcs(input: &mut &Bytes) -> winnow::Result<PresentationComposition> {
    (
        be_u16.context(StrContext::Label("PCS width")),
        be_u16.context(StrContext::Label("PCS height")),
        be_u8.void().context(StrContext::Label("PCS frame rate")),
        be_u16.context(StrContext::Label("PCS composition number")),
        parse_comp_state,
        parse_palette_update,
        be_u8.context(StrContext::Label("PCS palette id")),
        length_repeat::<_, _, Vec<_>, _, _, _, _>(
            be_u8.context(StrContext::Label("PCS composition object count")),
            parse_composition_object,
        )
        .context(StrContext::Label("PCS composition objects")),
    )
        .context(StrContext::Label("PCS"))
        .map(
            |(
                width,
                height,
                _,
                comp_no,
                comp_state,
                palette_update,
                palette_id,
                composition_objects,
            )| {
                PresentationComposition {
                    comp_no,
                    comp_state,
                    width,
                    height,
                    palette_id,
                    palette_update,
                    composition_objects,
                }
            },
        )
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn pcs() {
        let data = hex! {"
            07 80 04 38  10 04 42 80
            00 00 01 00  00 00 40 02
            4c 03 64
        "};
        let pcs = decode_pcs(&mut Bytes::new(&data)).unwrap();

        assert_eq!(1090, pcs.comp_no);
        assert_eq!(pcs.comp_state, CompositionState::EpochStart);
        assert_eq!(1, pcs.composition_objects.len());

        assert_eq!(1920, pcs.width);
        assert_eq!(1080, pcs.height);

        assert_eq!(0, pcs.palette_id);
        assert!(!pcs.palette_update);

        let obj = &pcs.composition_objects[0];
        assert_eq!(0, obj.id);
        assert_eq!(0, obj.window_id);
        assert!(obj.cropped);
        assert_eq!(588, obj.x);
        assert_eq!(868, obj.y);
        assert_eq!(None, obj.crop_x);
        assert_eq!(None, obj.crop_y);
        assert_eq!(None, obj.crop_width);
        assert_eq!(None, obj.crop_height);
    }

    #[test]
    fn composition_object_allows_missing_crop_rect() {
        let data = hex!("00 00 00 40 02 4c 03 64");
        let obj = parse_composition_object(&mut Bytes::new(&data)).unwrap();

        assert_eq!(0, obj.id);
        assert_eq!(0, obj.window_id);
        assert!(obj.cropped);
        assert_eq!(588, obj.x);
        assert_eq!(868, obj.y);
        assert_eq!(None, obj.crop_x);
        assert_eq!(None, obj.crop_y);
        assert_eq!(None, obj.crop_width);
        assert_eq!(None, obj.crop_height);
    }
}
