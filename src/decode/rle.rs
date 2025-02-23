use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt as _};

// The Run-length encoding method is defined in the US 7912305 B1 patent.
// Here’s a quick and dirty definition to this method:
// Code                                   Meaning
// CCCCCCCC                               One pixel in color C
// 00000000 00LLLLLL                      L pixels in color 0 (L between 1 and 63)
// 00000000 01LLLLLL LLLLLLLL             L pixels in color 0 (L between 64 and 16383)
// 00000000 10LLLLLL CCCCCCCC             L pixels in color C (L between 3 and 63)
// 00000000 11LLLLLL LLLLLLLL CCCCCCCC    L pixels in color C (L between 64 and 16383)
// 00000000 00000000                      End of line

fn is_color(byte: u8) -> bool {
    byte >> 7 == 1
}

fn is_long(byte: u8) -> bool {
    (byte & 0b0100_0000) >> 6 == 1
}

pub fn decode_rle<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
    let data = data.as_ref();
    let data_len = data.len() as u64;
    let mut c = Cursor::new(data);
    let mut output = vec![];

    loop {
        if c.position() >= data_len {
            break;
        }

        // check first byte color
        match c.read_u8().unwrap() {
            0x00 => {}
            _ => {
                output.push(1);
                continue;
            }
        };

        // check second byte for length
        let info = match c.read_u8().unwrap() {
            0 => continue,
            x => x,
        };

        let is_color = is_color(info);
        let big_len = is_long(info);

        let len_u8 = info & 0b0011_1111;
        assert_eq!(len_u8 >> 6, 0);

        // println!("big len: {}", big_len);
        // println!("high len: {}", len_u8);

        let len = if big_len {
            let len2_u8 = c.read_u8().unwrap();
            // println!("low len: {}", len2_u8);
            let buf = [len_u8, len2_u8];
            BigEndian::read_u16(&buf)
        } else {
            len_u8 as u16
        };

        let color = if is_color {
            c.read_u8().unwrap()
        } else {
            // use preferred color
            0
        };

        // println!("{} colored {}", len, color);
        for x in 0..len {
            output.push(color);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn long_indicator() {
        assert!(is_long(0b1100_0000));
        assert!(is_long(0b1111_1111));
        assert!(is_long(0b0100_0000));
        assert!(is_long(0b0111_1111));

        assert!(!is_long(0b1000_0000));
        assert!(!is_long(0b1011_1111));
        assert!(!is_long(0b0000_0000));
        assert!(!is_long(0b0011_1111));
    }

    #[test]
    fn color_indicator() {
        assert!(is_color(0b1000_0000));
        assert!(is_color(0b1111_1111));
        assert!(!is_color(0b0000_0000));
        assert!(!is_color(0b0111_1111));
    }

    #[test]
    fn single_pixel() {
        assert_eq!(decode_rle(vec![1]), [1]);
    }

    #[test]
    fn short_black_pixels() {
        assert_eq!(decode_rle([0, 0b0000_0101]), [0, 0, 0, 0, 0]);
    }

    #[test]
    fn long_black_pixels() {
        assert_eq!(decode_rle([0, 0b0100_0000, 0b0010_0000]), [0u8; 32]);
    }

    #[test]
    fn short_color_pixels() {
        assert_eq!(decode_rle([0, 0b1000_0101, 0b0000_0001]), [0b0000_0001; 5]);
    }

    #[test]
    fn long_color_pixels() {
        assert_eq!(
            decode_rle([0, 0b1100_0000, 0b0010_0000, 0b0000_0001]),
            [1u8; 32],
        );
    }
}
