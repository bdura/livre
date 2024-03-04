#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Ref {
    pub offset: usize,
    pub compressed: bool,
}
