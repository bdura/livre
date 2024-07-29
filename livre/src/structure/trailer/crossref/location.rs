#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RefLocation {
    Uncompressed(usize),
    Compressed(usize),
}

impl RefLocation {
    pub fn from_offset_and_flag(offset: usize, compressed: bool) -> Self {
        if compressed {
            Self::Compressed(offset)
        } else {
            Self::Uncompressed(offset)
        }
    }
}
