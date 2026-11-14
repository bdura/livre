use enum_dispatch::enum_dispatch;

use crate::{
    content::state::{Font, RenderingMode, TextStateParameters},
    extraction::{Extract, Name},
};

use super::TextStateOperation;

#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(TextStateOperation)]
pub enum TextStateOperator {
    SetCharacterSpacing(SetCharacterSpacing),
    SetWordSpacing(SetWordSpacing),
    SetHorizontalScaling(SetHorizontalScaling),
    SetTextLeading(SetTextLeading),
    SetFontAndFontSize(SetFontAndFontSize),
    SetTextRenderingMode(SetTextRenderingMode),
    SetTextRise(SetTextRise),
}

/// `Tc` operator.
/// Set the caracter spacing, $T_c$, to a number expressed in unscaled text space units.
///
/// ```raw
/// -0.024 Tc
/// 0.03 Tc
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetCharacterSpacing(pub(crate) f32);

impl TextStateOperation for SetCharacterSpacing {
    fn apply(self, parameters: &mut TextStateParameters) {
        parameters.set_character_spacing(self.0);
    }
}

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetWordSpacing(pub(crate) f32);

impl TextStateOperation for SetWordSpacing {
    fn apply(self, parameters: &mut TextStateParameters) {
        parameters.set_word_spacing(self.0);
    }
}

/// `Tz` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetHorizontalScaling(f32);

impl TextStateOperation for SetHorizontalScaling {
    fn apply(self, parameters: &mut TextStateParameters) {
        parameters.set_horizontal_scaling(self.0);
    }
}

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextLeading(f32);

impl TextStateOperation for SetTextLeading {
    fn apply(self, parameters: &mut TextStateParameters) {
        parameters.set_leading(self.0);
    }
}

/// `Tf` operator.
///
/// Selects the font and font size. May appear any number of times within a text object;
/// each occurrence replaces the previous selection.
///
/// ```raw
/// /F6 9 Tf
/// /F4 14.666667 Tf
/// ```
#[derive(Debug, Clone, PartialEq, Extract)]
pub struct SetFontAndFontSize(pub(crate) Name, pub(crate) f32);

impl TextStateOperation for SetFontAndFontSize {
    fn apply(self, parameters: &mut TextStateParameters) {
        let SetFontAndFontSize(name, size) = self;
        parameters.set_font(Font { name, size });
    }
}

/// `Tr` operator.
///
/// ```raw
/// 2 Tr
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRenderingMode(RenderingMode);

impl TextStateOperation for SetTextRenderingMode {
    fn apply(self, parameters: &mut TextStateParameters) {
        parameters.set_rendering_mode(self.0);
    }
}

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRise(f32);

impl TextStateOperation for SetTextRise {
    fn apply(self, parameters: &mut TextStateParameters) {
        parameters.set_rise(self.0);
    }
}

#[cfg(test)]
mod tests {
    use winnow::{ascii::multispace0, combinator::preceded, BStr};

    use super::*;
    use crate::content::operators::{text::TextOperator, Operator};

    #[test]
    fn state_operators_apply_to_bare_parameters() {
        let mut input = BStr::new(b"/F1 12 Tf 2 Tc".as_slice());
        let mut it =
            winnow::combinator::iterator(&mut input, preceded(multispace0, Operator::extract));
        let mut parameters = TextStateParameters::default();

        for operator in &mut it {
            let Operator::Text(TextOperator::TextStateOperator(op)) = operator else {
                panic!("not a text state operator: {operator:?}");
            };
            op.apply(&mut parameters);
        }
        it.finish().unwrap();

        assert_eq!(
            parameters,
            TextStateParameters {
                font: Some(Font {
                    name: Name(b"F1".to_vec()),
                    size: 12.0
                }),
                character_spacing: 2.0,
                ..TextStateParameters::default()
            }
        );
    }
}
