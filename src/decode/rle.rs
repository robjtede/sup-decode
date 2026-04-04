use winnow::{Bytes, ModalResult, binary::be_u8, error::StrContext, prelude::*};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RleChunk {
    Eol,
    Run { len: usize, color: u8 },
    Pixel(u8),
}

impl RleChunk {
    fn append_to(self, output: &mut Vec<u8>) {
        match self {
            Self::Eol => {}
            Self::Run { len, color } => output.resize(output.len() + len, color),
            Self::Pixel(pixel) => output.push(pixel),
        }
    }

    fn into_vec(self) -> Vec<u8> {
        let mut output = Vec::new();
        self.append_to(&mut output);
        output
    }
}

fn decode_rle_chunk(input: &mut &Bytes) -> ModalResult<RleChunk> {
    let first = be_u8.parse_next(input)?;

    if first != 0 {
        return Ok(RleChunk::Pixel(first));
    }

    let info = be_u8.parse_next(input)?;

    if info == 0 {
        return Ok(RleChunk::Eol);
    }

    let is_color = info & 0b1000_0000 != 0;
    let is_long = info & 0b0100_0000 != 0;
    let len_hi = usize::from(info & 0b0011_1111);

    let len = if is_long {
        (len_hi << 8) | usize::from(be_u8.parse_next(input)?)
    } else {
        len_hi
    };

    let color = if is_color {
        be_u8.parse_next(input)?
    } else {
        COLOR_BLACK
    };

    Ok(RleChunk::Run { len, color })
}

#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn decode_rle(input: &mut &Bytes) -> ModalResult<Vec<u8>> {
    decode_rle_chunk
        .context(StrContext::Label("RLE chunk"))
        .map(RleChunk::into_vec)
        .parse_next(input)
}

pub(crate) fn decode_rle_stream(
    input: &mut &Bytes,
    expected_pixels: usize,
) -> ModalResult<Vec<u8>> {
    let mut output = Vec::with_capacity(expected_pixels);

    while !input.is_empty() {
        decode_rle_chunk.parse_next(input)?.append_to(&mut output);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eol() {
        assert_eq!(
            Ok((Bytes::new(&[]), RleChunk::Eol)),
            decode_rle_chunk.parse_peek(Bytes::new(&[0, 0])),
        );
        assert_eq!(
            Ok((Bytes::new(&[0]), RleChunk::Eol)),
            decode_rle_chunk.parse_peek(Bytes::new(&[0, 0, 0])),
        );
        assert_eq!(
            Ok((Bytes::new(&[]), Vec::new())),
            decode_rle.parse_peek(Bytes::new(&[0, 0])),
        );

        decode_rle.parse_peek(Bytes::new(&[0])).unwrap_err();
    }

    #[test]
    fn single_pixel() {
        assert_eq!(
            Ok((Bytes::new(&[]), RleChunk::Pixel(1))),
            decode_rle_chunk.parse_peek(Bytes::new(&[0b0000_0001])),
        );
    }

    #[test]
    fn short_black_pixels() {
        assert_eq!(
            Ok((
                Bytes::new(&[]),
                RleChunk::Run {
                    len: 5,
                    color: COLOR_BLACK,
                },
            )),
            decode_rle_chunk.parse_peek(Bytes::new(&[0, 0b0000_0101])),
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

    #[test]
    fn rejects_truncated_sequences() {
        decode_rle.parse_peek(Bytes::new(&[0])).unwrap_err();
        decode_rle
            .parse_peek(Bytes::new(&[0, 0b0100_0000]))
            .unwrap_err();
        decode_rle
            .parse_peek(Bytes::new(&[0, 0b1000_0101]))
            .unwrap_err();
        decode_rle
            .parse_peek(Bytes::new(&[0, 0b1100_0000, 0b0010_0000]))
            .unwrap_err();
    }
}
