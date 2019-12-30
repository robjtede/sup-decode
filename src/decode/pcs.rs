use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

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
// Object Cropping Height Position        2        Heightl of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.

// When the Object Cropped Flag is set to true (or actually 0x40), then the sub picture is cropped to show only a portion of it. This is used for example when you don’t want to show the whole subtitle at first, but just a few words first, and then the rest.

/// Presentation Composition Segment
#[derive(Debug, Clone)]
pub struct PresentationComposition {
    pub comp_no: u16,
    pub comp_state: CompositionState,
    pub num_comp_objects: u8,
    pub width: u16,
    pub height: u16,
    pub palette_id: u8,
    pub palette_update: bool,
    pub composition_objects: Vec<CompositionObject>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompositionState {
    /// This defines a new display. The Epoch Start contains all functional segments needed to display a new composition on the screen.
    EpochStart,

    /// This defines a display refresh. This is used to compose in the middle of the Epoch. It includes functional segments with new objects to be used in a new composition, replacing old objects with the same Object ID.
    AcquisitionPoint,

    /// This defines a display update, and contains only functional segments with elements that are different from the preceding composition. It’s mostly used to stop displaying objects on the screen by defining a composition with no composition objects (a value of zero in the Number of Composition Objects flag) but also used to define a new composition with new objects and objects defined since the Epoch Start.
    Normal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionObject {
    id: u16,
    window_id: u8,
    x: u16,
    y: u16,
    cropped: bool,
    crop_x: Option<u16>,
    crop_y: Option<u16>,
    crop_width: Option<u16>,
    crop_height: Option<u16>,
}

pub fn decode_pcs<T: AsRef<[u8]>>(data: T) -> PresentationComposition {
    let data = data.as_ref();
    let mut c = Cursor::new(data);

    let width = c.read_u16::<BigEndian>().unwrap();
    let height = c.read_u16::<BigEndian>().unwrap();

    // skip "frame rate" useless value
    c.seek(SeekFrom::Current(1)).unwrap();

    let comp_no = c.read_u16::<BigEndian>().unwrap();

    let comp_state = match c.read_u8().unwrap() {
        0x80 => CompositionState::EpochStart,
        0x40 => CompositionState::AcquisitionPoint,
        0x00 => CompositionState::Normal,
        x => panic!("unknown composition state: {}", x),
    };

    let palette_update = match c.read_u8().unwrap() {
        0x00 => false,
        0x80 => true,
        x => panic!("unknown pallet update flag: {}", x),
    };

    let palette_id = c.read_u8().unwrap();
    let num_comp_objects = c.read_u8().unwrap();

    PresentationComposition {
        comp_no,
        comp_state,
        num_comp_objects,
        width,
        height,
        palette_id,
        palette_update,
        // TODO: decode comp objects
        composition_objects: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn pcs() {
        let data = hex!(
            "
        07 80 04 38 10 04 42 80 00 00 01
        00 00 00 40 02 4c 03 64"
        );
        let pcs = decode_pcs(data.to_vec());

        assert_eq!(pcs.comp_no, 1090);
        assert_eq!(pcs.comp_state, CompositionState::EpochStart);
        assert_eq!(pcs.num_comp_objects, 1);

        assert_eq!(pcs.width, 1920);
        assert_eq!(pcs.height, 1080);

        assert_eq!(pcs.palette_id, 0);
        assert_eq!(pcs.palette_update, false);
    }
}
