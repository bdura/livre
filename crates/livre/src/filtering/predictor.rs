//! Undo the `/Predictor` step of `FlateDecode` and `LZWDecode` streams
//! (ISO 32000-2, §7.4.4.4).

use super::FlateParams;

/// A validated `/Predictor`, with the row geometry it needs.
///
/// Built once from a `/DecodeParms` dictionary with [`Predictor::try_from`],
/// then applied to inflated bytes with [`Predictor::undo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Predictor {
    /// Predictor 1: no prediction.
    #[default]
    None,
    /// TIFF Predictor 2, at 8 bits per component.
    Tiff {
        /// Interleaved colour components per sample: how many bytes back
        /// each difference reaches.
        colors: usize,
        /// Bytes per row.
        row_len: usize,
    },
    /// PNG predictors, `/Predictor` `10..=15`. The value is only a hint,
    /// since every row carries a leading tag byte (0 None, 1 Sub, 2 Up,
    /// 3 Average, 4 Paeth) that decides how that row is decoded.
    Png {
        /// Bytes per complete pixel, at least 1.
        bpp: usize,
        /// Bytes per row, tag byte excluded.
        row_len: usize,
    },
}

impl TryFrom<FlateParams> for Predictor {
    type Error = ();

    /// Validate the `/DecodeParms` entries that describe the predictor.
    ///
    /// Predictor `1` gives [`Predictor::None`] without validating the other
    /// entries.
    ///
    /// Returns `Err(())` when `predictor` is not in `{1, 2, 10..=15}`,
    /// `colors` or `columns` is zero, `bits_per_component` is not in
    /// `{1, 2, 4, 8, 16}`, TIFF is asked for anything but 8 bits per
    /// component, or the row length overflows `usize`.
    fn try_from(params: FlateParams) -> Result<Self, ()> {
        let FlateParams {
            predictor,
            colors,
            bits_per_component,
            columns,
        } = params;

        if predictor == 1 {
            return Ok(Self::None);
        }

        if colors == 0 || columns == 0 || !matches!(bits_per_component, 1 | 2 | 4 | 8 | 16) {
            return Err(());
        }

        let bits_per_pixel = colors.checked_mul(bits_per_component).ok_or(())?;
        let bpp = (bits_per_pixel / 8).max(1);
        let row_len = bits_per_pixel.checked_mul(columns).ok_or(())?.div_ceil(8);

        match predictor {
            2 if bits_per_component == 8 => Ok(Self::Tiff { colors, row_len }),
            10..=15 => Ok(Self::Png { bpp, row_len }),
            _ => Err(()),
        }
    }
}

impl Predictor {
    /// Turn inflated, predicted bytes back into raw rows.
    ///
    /// [`Predictor::None`] returns `data` untouched. PNG tag bytes are
    /// stripped. If `data` does not divide into whole rows, the final partial
    /// row is decoded as far as it goes. Empty input decodes to empty output.
    ///
    /// Returns `Err(())` when a PNG row tag is above 4.
    pub(crate) fn undo(self, data: Vec<u8>) -> Result<Vec<u8>, ()> {
        match self {
            Self::None => Ok(data),
            Self::Tiff { colors, row_len } => Ok(undo_tiff(data, colors, row_len)),
            Self::Png { bpp, row_len } => undo_png(&data, bpp, row_len),
        }
    }
}

/// TIFF Predictor 2 at 8 bits per component: each byte past the first
/// `colors` of a row is a wrapping difference from the byte `colors` earlier.
fn undo_tiff(mut data: Vec<u8>, colors: usize, row_len: usize) -> Vec<u8> {
    for row in data.chunks_mut(row_len) {
        for i in colors..row.len() {
            row[i] = row[i].wrapping_add(row[i - colors]);
        }
    }
    data
}

/// PNG predictors: each row is a tag byte followed by up to `row_len` bytes.
fn undo_png(data: &[u8], bpp: usize, row_len: usize) -> Result<Vec<u8>, ()> {
    let mut out = Vec::with_capacity(data.len() / (row_len + 1) * row_len + row_len);
    // The row above the first one is all zeros.
    let mut prev = vec![0u8; row_len];

    for chunk in data.chunks(row_len + 1) {
        let (&tag, row) = chunk.split_first().ok_or(())?;
        // Checked before decoding: a short last row may have no byte to decode.
        if tag > 4 {
            return Err(());
        }
        let start = out.len();

        for (i, &byte) in row.iter().enumerate() {
            let left = if i >= bpp { out[start + i - bpp] } else { 0 };
            let up = prev[i];
            let up_left = if i >= bpp { prev[i - bpp] } else { 0 };

            let predicted = match tag {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                _ => paeth(left, up, up_left),
            };
            out.push(byte.wrapping_add(predicted));
        }

        prev[..row.len()].copy_from_slice(&out[start..]);
    }

    Ok(out)
}

/// The Paeth predictor from the PNG specification, §9.4.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let (a, b, c) = (i16::from(left), i16::from(up), i16::from(up_left));
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());

    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        up_left
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// Build a [`Predictor`] from loose `/DecodeParms` values and apply it.
    fn undo(
        data: Vec<u8>,
        predictor: u8,
        colors: usize,
        bits_per_component: usize,
        columns: usize,
    ) -> Result<Vec<u8>, ()> {
        Predictor::try_from(FlateParams {
            predictor,
            colors,
            bits_per_component,
            columns,
        })?
        .undo(data)
    }

    #[rstest]
    #[case(vec![1, 2, 3], 1, 1, 8, 0)]
    #[case(vec![], 1, 0, 3, 0)]
    #[case(vec![9, 9, 9, 9], 1, 1, 8, 4)]
    fn predictor_1_is_identity(
        #[case] data: Vec<u8>,
        #[case] predictor: u8,
        #[case] colors: usize,
        #[case] bits_per_component: usize,
        #[case] columns: usize,
    ) {
        let expected = data.clone();
        assert_eq!(
            undo(data, predictor, colors, bits_per_component, columns),
            Ok(expected)
        );
    }

    #[rstest]
    // None
    #[case(vec![0, 1, 2, 3, 4], vec![1, 2, 3, 4])]
    // Sub: each byte adds its left neighbour
    #[case(vec![1, 1, 1, 1, 1], vec![1, 2, 3, 4])]
    // Up: the row above the first is zeros
    #[case(vec![2, 1, 2, 3, 4], vec![1, 2, 3, 4])]
    // Average: 2, 2+2/2, 2+3/2, 4+3/2
    #[case(vec![3, 2, 2, 2, 4], vec![2, 3, 3, 5])]
    // Paeth against a zero row always picks the left neighbour
    #[case(vec![4, 1, 1, 1, 1], vec![1, 2, 3, 4])]
    // Sub wraps
    #[case(vec![1, 200, 100, 0, 0], vec![200, 44, 44, 44])]
    fn png_single_row(#[case] data: Vec<u8>, #[case] expected: Vec<u8>) {
        assert_eq!(undo(data, 15, 1, 8, 4), Ok(expected));
    }

    #[test]
    fn png_rows_predict_from_the_decoded_row_above() {
        let data = vec![
            0, 10, 20, 30, // None
            2, 1, 2, 3, // Up against [10, 20, 30]
            4, 5, 255, 1, // Paeth against [11, 22, 33]
        ];
        // Row 3, prev = [11, 22, 33]:
        // i=0: a=0,  b=11, c=0  -> p=11, pa=11, pb=0,  pc=11 -> b=11, 11+5   = 16
        // i=1: a=16, b=22, c=11 -> p=27, pa=11, pb=5,  pc=16 -> b=22, 22+255 = 21
        // i=2: a=21, b=33, c=22 -> p=32, pa=11, pb=1,  pc=10 -> b=33, 33+1   = 34
        // Predicting from the raw row [1, 2, 3] instead would give [6, 5, 6].
        assert_eq!(
            undo(data, 12, 1, 8, 3),
            Ok(vec![10, 20, 30, 11, 22, 33, 16, 21, 34])
        );
    }

    #[rstest]
    // i=1: a=10, b=20, c=15 -> p=15, pa=5, pb=5, pc=0 -> picks up-left
    // i=2: a=15, b=0, c=20 -> p=-5, pa=20, pb=5, pc=25 -> picks up
    #[case(vec![0, 15, 20, 0, 4, 251, 0, 7], vec![15, 20, 0, 10, 15, 7])]
    // i=1: a=30, b=5, c=7 -> p=28, pa=2, pb=23, pc=21 -> picks left
    // i=2: a=30, b=5, c=5 -> p=30, pa=0 -> picks left
    #[case(vec![0, 7, 5, 5, 4, 23, 0, 0], vec![7, 5, 5, 30, 30, 30])]
    fn png_paeth_picks(#[case] data: Vec<u8>, #[case] expected: Vec<u8>) {
        // Row 2, i=0: a=0, c=0 -> picks up.
        assert_eq!(undo(data, 14, 1, 8, 3), Ok(expected));
    }

    #[test]
    fn png_sub_reaches_one_pixel_back() {
        // colors = 3 -> bpp = 3, row_len = 6
        let data = vec![1, 1, 2, 3, 1, 1, 1];
        assert_eq!(undo(data, 11, 3, 8, 2), Ok(vec![1, 2, 3, 2, 3, 4]));
    }

    #[test]
    fn png_sub_byte_components() {
        // 1 bit, 10 columns -> row_len = 2, bpp = 1
        let data = vec![1, 0b1010_0000, 0b0000_0001, 2, 0b0101_1111, 0b1000_0000];
        assert_eq!(
            undo(data, 15, 1, 1, 10),
            Ok(vec![0b1010_0000, 0b1010_0001, 0b1111_1111, 0b0010_0001])
        );
    }

    #[test]
    fn png_xref_stream_rows() {
        // /W [1 2 1], /Columns 4, /Predictor 12
        let data = vec![
            2, 1, 0, 15, 0, // [1, 0, 15, 0]
            2, 0, 1, 0xF5, 0, // Up: 15 + 0xF5 wraps to 4
        ];
        assert_eq!(undo(data, 12, 1, 8, 4), Ok(vec![1, 0, 15, 0, 1, 1, 4, 0]));
    }

    #[rstest]
    // tag plus two of four bytes
    #[case(vec![0, 7, 8], vec![7, 8])]
    #[case(vec![0, 1, 2, 3, 4, 2, 1, 1], vec![1, 2, 3, 4, 2, 3])]
    // a lone tag byte
    #[case(vec![0, 1, 2, 3, 4, 2], vec![1, 2, 3, 4])]
    fn png_short_last_row(#[case] data: Vec<u8>, #[case] expected: Vec<u8>) {
        assert_eq!(undo(data, 12, 1, 8, 4), Ok(expected));
    }

    #[rstest]
    #[case(1, 4, vec![1, 1, 1, 1], vec![1, 2, 3, 4])]
    // each row restarts from its own first byte
    #[case(1, 4, vec![1, 1, 1, 1, 5, 1, 1, 1], vec![1, 2, 3, 4, 5, 6, 7, 8])]
    #[case(1, 2, vec![200, 100], vec![200, 44])]
    #[case(3, 2, vec![1, 2, 3, 1, 1, 1], vec![1, 2, 3, 2, 3, 4])]
    // short last row
    #[case(1, 4, vec![1, 1, 1, 1, 5, 1], vec![1, 2, 3, 4, 5, 6])]
    fn tiff_8_bits(
        #[case] colors: usize,
        #[case] columns: usize,
        #[case] data: Vec<u8>,
        #[case] expected: Vec<u8>,
    ) {
        assert_eq!(undo(data, 2, colors, 8, columns), Ok(expected));
    }

    #[rstest]
    #[case(2)]
    #[case(12)]
    fn empty_input(#[case] predictor: u8) {
        assert_eq!(undo(vec![], predictor, 1, 8, 4), Ok(vec![]));
    }

    #[rstest]
    #[case::predictor_3(vec![0, 1], 3, 1, 8, 1)]
    #[case::predictor_9(vec![0, 1], 9, 1, 8, 1)]
    #[case::predictor_16(vec![0, 1], 16, 1, 8, 1)]
    #[case::png_tag_5(vec![5, 1], 12, 1, 8, 1)]
    #[case::png_tag_5_lone(vec![0, 1, 5], 12, 1, 8, 1)]
    #[case::colors_0(vec![0, 1], 12, 0, 8, 1)]
    #[case::columns_0(vec![0, 1], 12, 1, 8, 0)]
    #[case::bits_3(vec![0, 1], 12, 1, 3, 1)]
    #[case::tiff_bits_16(vec![0, 1], 2, 1, 16, 1)]
    #[case::tiff_bits_4(vec![0, 1], 2, 1, 4, 1)]
    fn rejected(
        #[case] data: Vec<u8>,
        #[case] predictor: u8,
        #[case] colors: usize,
        #[case] bits_per_component: usize,
        #[case] columns: usize,
    ) {
        assert_eq!(
            undo(data, predictor, colors, bits_per_component, columns),
            Err(())
        );
    }

    #[rstest]
    #[case::predictor_3(3, 1, 8, 1)]
    #[case::predictor_9(9, 1, 8, 1)]
    #[case::predictor_16(16, 1, 8, 1)]
    #[case::colors_0(12, 0, 8, 1)]
    #[case::columns_0(12, 1, 8, 0)]
    #[case::bits_3(12, 1, 3, 1)]
    #[case::tiff_bits_16(2, 1, 16, 1)]
    #[case::tiff_bits_4(2, 1, 4, 1)]
    fn rejected_when_built(
        #[case] predictor: u8,
        #[case] colors: usize,
        #[case] bits_per_component: usize,
        #[case] columns: usize,
    ) {
        let params = FlateParams {
            predictor,
            colors,
            bits_per_component,
            columns,
        };
        assert_eq!(Predictor::try_from(params), Err(()));
    }

    #[rstest]
    #[case(vec![5, 1])]
    #[case(vec![0, 1, 5])]
    fn png_tag_is_rejected_by_undo(#[case] data: Vec<u8>) {
        let params = FlateParams {
            predictor: 12,
            colors: 1,
            bits_per_component: 8,
            columns: 1,
        };
        let predictor = Predictor::try_from(params).expect("valid PNG parameters");
        assert_eq!(predictor.undo(data), Err(()));
    }

    #[test]
    fn predictor_1_ignores_the_geometry() {
        let params = FlateParams {
            predictor: 1,
            colors: 0,
            bits_per_component: 3,
            columns: 0,
        };
        assert_eq!(Predictor::try_from(params), Ok(Predictor::None));
    }
}
