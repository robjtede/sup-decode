pub fn segment_on<'a>(bytes: &'a [u8], segmark: &'_ [u8]) -> Vec<&'a [u8]> {
    let split_len = segmark.len();
    let bytes_len = bytes.len();

    if split_len == 0 {
        panic!("segment marker shouldnt be empty");
    }

    let mut segments = vec![];
    let mut seg_start = 0;

    for i in 0..(bytes_len - split_len) {
        if &bytes[i..(i + split_len)] == segmark {
            if i != 0 {
                segments.push(&bytes[seg_start..i]);
            }

            seg_start = i;
        }
    }

    // push final segment
    segments.push(&bytes[seg_start..bytes_len]);

    segments
}

// TODO: edge cases
// empty split
// empty bytes
// data before first segmark
// segmark at end
// no segmark

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_marker_segment() {
        let bytes = vec![0xff, 0x0, 0xff, 0x1, 0x2, 0xff, 0x3, 0x4, 0xff, 0x5];
        let segments: Vec<&[u8]> = vec![
            &[0xff, 0x0],
            &[0xff, 0x1, 0x2],
            &[0xff, 0x3, 0x4],
            &[0xff, 0x5],
        ];

        assert_eq!(segment_on(&bytes, &[0xff]), segments)
    }

    #[test]
    fn multi_marker_segment() {
        let bytes = vec![0x42, 0xff, 0x0, 0x42, 0xff, 0x1, 0x2, 0x42, 0xff, 0x3];
        let segments: Vec<&[u8]> = vec![
            &[0x42, 0xff, 0x0],
            &[0x42, 0xff, 0x1, 0x2],
            &[0x42, 0xff, 0x3],
        ];

        assert_eq!(segment_on(&bytes, &[0x42, 0xff]), segments)
    }
}
