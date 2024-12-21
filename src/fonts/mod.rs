mod descriptor;
mod simple_font;

use enum_dispatch::enum_dispatch;

use crate::extraction::LiteralString;

#[enum_dispatch]
pub trait Font {
    fn process(&self, LiteralString(string): LiteralString) -> Vec<(char, f32, bool)> {
        string
            .iter()
            .copied()
            .map(|b| (char::from(b), 0.5, b == b' '))
            .collect()
    }
    fn width(&self, code: u8) -> f32;
    fn ascent(&self) -> f32;
    fn descent(&self) -> f32;
    fn name(&self) -> &str;
}
