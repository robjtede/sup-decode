use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    iter,
};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt as _};
use winnow::{
    Bytes, ModalResult, Parser,
    binary::{
        be_u8,
        bits::{self, Bits, bits, bool, take},
    },
    combinator::{alt, eof, opt, peek, repeat},
    error::{ContextError, ErrMode, StrContext, StrContextValue},
    stream::{Stream, StreamIsPartial},
    token::{literal, rest},
};

// The Run-length encoding method is defined in the US 7912305 B1 patent.
// Here’s a quick and dirty definition to this method:
// Code                                   Meaning
// CCCCCCCC                               One pixel in color C
// 00000000 00LLLLLL                      L pixels in color 0 (L between 1 and 63)
// 00000000 01LLLLLL LLLLLLLL             L pixels in color 0 (L between 64 and 16383)
// 00000000 10LLLLLL CCCCCCCC             L pixels in color C (L between 3 and 63)
// 00000000 11LLLLLL LLLLLLLL CCCCCCCC    L pixels in color C (L between 64 and 16383)
// 00000000 00000000                      End of line

const COLOR_BLACK: u8 = 0;

/// Decodes `00000000 00000000` form.
fn decode_eol(input: &mut &Bytes) -> winnow::Result<()> {
    literal(&[0, 0]).void().parse_next(input)
}

/// Decodes `CCCCCCCC` form.
fn decode_pixel(input: &mut &Bytes) -> winnow::Result<u8> {
    be_u8.verify(|&value| value != 0).parse_next(input)
}

/// Decodes `11LLLLLL LLLLLLLL CCCCCCCC` form.
fn decode_color_pixels_long(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    bits::<_, _, ContextError, _, _>((bool, bool, take(14_usize), take(8_usize)))
        .verify_map(|(is_color, is_long, len, color)| {
            (is_color && is_long).then(|| vec![color; len])
        })
        .parse_next(input)
}

/// Decodes `10LLLLLL CCCCCCCC` form.
fn decode_color_pixels_short(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    bits::<_, _, ContextError, _, _>((bool, bool, take(6_usize), take(8_usize)))
        .verify_map(|(is_color, is_long, len, color)| {
            (is_color && !is_long).then(|| vec![color; len])
        })
        .parse_next(input)
}

/// Decodes `01LLLLLL LLLLLLLL` form.
fn decode_black_pixels_long(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    bits::<_, _, ContextError, _, _>((bool, bool, take(14_usize)))
        .verify_map(|(is_color, is_long, len)| {
            (!is_color && is_long).then(|| vec![COLOR_BLACK; len])
        })
        .parse_next(input)
}

/// Decodes `00LLLLLL` form.
fn decode_black_pixels_short(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    bits::<_, _, ContextError, _, _>((bool, bool, take(6_usize)))
        .verify_map(|(is_color, is_long, len)| {
            (!is_color && !is_long).then(|| vec![COLOR_BLACK; len])
        })
        .parse_next(input)
}

fn decode_pixels(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    let _zero_bits = be_u8.verify(|&value| value == 0).parse_next(input)?;

    alt((
        decode_color_pixels_long,
        decode_color_pixels_short,
        decode_black_pixels_long,
        decode_black_pixels_short,
    ))
    .parse_next(input)
}

pub fn decode_rle(input: &mut &Bytes) -> winnow::Result<Vec<u8>> {
    (
        alt((
            decode_eol
                .context(StrContext::Label("RLE EoL"))
                .map(|()| Vec::new()),
            decode_pixels.context(StrContext::Label("RLE Pixels")),
            decode_pixel
                .context(StrContext::Label("RLE Pixel"))
                .map(|pixel| vec![pixel]),
        )),
        eof.context(StrContext::Expected(StrContextValue::Description(
            "end of input",
        ))),
    )
        .map(|(result, _)| result)
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eol() {
        assert_eq!(
            Ok((Bytes::new(&[]), ())),
            decode_eol.parse_peek(Bytes::new(&[0, 0])),
        );
        assert_eq!(
            Ok((Bytes::new(&[0]), ())),
            decode_eol.parse_peek(Bytes::new(&[0, 0, 0])),
        );
        assert_eq!(
            Ok((Bytes::new(&[]), Vec::new())),
            decode_rle.parse_peek(Bytes::new(&[0, 0])),
        );

        decode_eol.parse_peek(Bytes::new(&[0, 1])).unwrap_err();
        decode_eol.parse_peek(Bytes::new(&[1])).unwrap_err();
        decode_eol.parse_peek(Bytes::new(&[0, 1])).unwrap_err();
    }

    #[test]
    fn single_pixel() {
        assert_eq!(
            Ok((Bytes::new(&[]), 1)),
            decode_pixel.parse_peek(Bytes::new(&[0b0000_0001])),
        );
    }

    #[test]
    fn short_black_pixels() {
        assert_eq!(
            Ok((Bytes::new(&[]), vec![0; 5])),
            decode_pixels.parse_peek(Bytes::new(&[0, 0b0000_0101])),
        );
        assert_eq!(
            Ok((Bytes::new(&[]), vec![0; 5])),
            decode_rle.parse_peek(Bytes::new(&[0, 0b0000_0101])),
        );
    }

    #[test]
    fn long_black_pixels() {
        assert_eq!(
            Ok((Bytes::new(&[]), vec![0u8; 32])),
            decode_rle.parse_peek(Bytes::new(&[0, 0b0100_0000, 0b0010_0000])),
        );
    }

    #[test]
    fn short_color_pixels() {
        assert_eq!(
            Ok((Bytes::new(&[]), vec![0b0000_0001; 5])),
            decode_rle.parse_peek(Bytes::new(&[0, 0b1000_0101, 0b0000_0001])),
        );
    }

    #[test]
    fn long_color_pixels() {
        assert_eq!(
            Ok((Bytes::new(&[]), vec![1u8; 32])),
            decode_rle.parse_peek(Bytes::new(&[0, 0b1100_0000, 0b0010_0000, 0b0000_0001])),
        );
    }
}
