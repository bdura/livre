use enum_dispatch::enum_dispatch;

use crate::{
    content::{
        error::{ContentError, Result},
        state::{FontSize, RenderingMode, SelectedFont, TextStateParameters},
        Font, Fonts,
    },
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
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, _: &'f Fonts) -> Result<()> {
        parameters.set_character_spacing(self.0);
        Ok(())
    }
}

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetWordSpacing(pub(crate) f32);

impl TextStateOperation for SetWordSpacing {
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, _: &'f Fonts) -> Result<()> {
        parameters.set_word_spacing(self.0);
        Ok(())
    }
}

/// `Tz` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetHorizontalScaling(f32);

impl TextStateOperation for SetHorizontalScaling {
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, _: &'f Fonts) -> Result<()> {
        parameters.set_horizontal_scaling(self.0);
        Ok(())
    }
}

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextLeading(f32);

impl TextStateOperation for SetTextLeading {
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, _: &'f Fonts) -> Result<()> {
        parameters.set_leading(self.0);
        Ok(())
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
    /// Select the font named by the operator in `fonts`.
    ///
    /// A composite (`Type0`) font is selected with a warning: its widths are not read, so its
    /// glyphs advance by the character spacing only.
    ///
    /// # Errors
    ///
    /// [`ContentError::UnknownFont`] if `fonts` has no font under that name.
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, fonts: &'f Fonts) -> Result<()> {
        let SetFontAndFontSize(name, size) = self;
        let font = fonts
            .get(&name)
            .ok_or_else(|| ContentError::UnknownFont(name.clone()))?;
        if let Font::Composite(composite) = font {
            tracing::warn!(
                "Font {name:?} ({:?}) is composite: its glyphs are given zero width",
                composite.base_font
            );
        }
        parameters.set_font(SelectedFont {
            name,
            font,
            size: FontSize(size),
        });
        Ok(())
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
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, _: &'f Fonts) -> Result<()> {
        parameters.set_rendering_mode(self.0);
        Ok(())
    }
}

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRise(f32);

impl TextStateOperation for SetTextRise {
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, _: &'f Fonts) -> Result<()> {
        parameters.set_rise(self.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use winnow::{ascii::multispace0, combinator::preceded, BStr};

    use super::*;
    use crate::content::{
        font::tests::{fonts, HELVETICA},
        operators::{text::TextOperator, Operator},
    };

    #[test]
    fn state_operators_apply_to_bare_parameters() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        let mut input = BStr::new(b"/F1 12 Tf 2 Tc".as_slice());
        let mut it =
            winnow::combinator::iterator(&mut input, preceded(multispace0, Operator::extract));
        let mut parameters = TextStateParameters::default();

        for operator in &mut it {
            let Operator::Text(TextOperator::TextStateOperator(op)) = operator else {
                panic!("not a text state operator: {operator:?}");
            };
            op.apply(&mut parameters, &fonts).unwrap();
        }
        it.finish().unwrap();

        assert_eq!(
            parameters,
            TextStateParameters {
                font: Some(SelectedFont {
                    name: Name(b"F1".to_vec()),
                    font: fonts.get(&Name(b"F1".to_vec())).unwrap(),
                    size: FontSize(12.0),
                }),
                character_spacing: 2.0,
                ..TextStateParameters::default()
            }
        );
    }

    #[test]
    fn tf_with_an_unknown_font_is_an_error() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        let mut parameters = TextStateParameters::default();
        let result = SetFontAndFontSize(Name(b"F9".to_vec()), 12.0).apply(&mut parameters, &fonts);

        assert!(matches!(result, Err(ContentError::UnknownFont(Name(name))) if name == b"F9"));
        assert_eq!(parameters.font, None);
    }
}
