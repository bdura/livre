use crate::{
    content::operators::{FromArgs, OperatorError},
    extraction::{Name, Object},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingMode {
    /// Fill text.
    Fill,
    /// Stroke text.
    Stroke,
    /// Fill, then stroke text.
    FillThenStroke,
    /// Neither fill nor stroke text (invisible).
    Invisible,
    /// Fill text and add to path for clipping.
    FillAndClip,
    /// Stroke text and add to path for clipping.
    StrokeAndClip,
    /// Fill, then stroke text and add to path for clipping.
    FillThenStrokeAndClip,
    /// Add text to path for clipping.
    AddTextAndClip,
}

/// `Tc` operator.
/// Set the caracter spacing, $T_c$, to a number expressed in unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetCharacterSpacing(pub(crate) f32);

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetWordSpacing(pub(crate) f32);

/// `Tz` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetHorizontalScaling(pub(crate) f32);

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextLeading(pub(crate) f32);

/// `Tf` operator.
#[derive(Debug, Clone, PartialEq)]
pub struct SetFontAndFontSize(pub(crate) Name, pub(crate) f32);

/// `Tr` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextRenderingMode(pub(crate) RenderingMode);

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextRise(pub(crate) f32);

impl FromArgs for SetTextRenderingMode {
    fn from_args(arguments: &mut Vec<Object>) -> Result<Self, OperatorError> {
        let value = arguments.pop().ok_or(OperatorError::MissingOperand)?;
        let value: i32 = value.try_into()?;
        let value = match value {
            0 => RenderingMode::Fill,
            1 => RenderingMode::Stroke,
            2 => RenderingMode::FillThenStroke,
            3 => RenderingMode::Invisible,
            4 => RenderingMode::FillAndClip,
            5 => RenderingMode::StrokeAndClip,
            6 => RenderingMode::FillThenStrokeAndClip,
            7 => RenderingMode::AddTextAndClip,
            _ => return Err(OperatorError::InvalidObject),
        };
        Ok(Self(value))
    }
}

macro_rules! impl_from_args {
    (1; $($t:ty),+) => {
        $(
            impl FromArgs for $t {
                fn from_args(arguments: &mut Vec<Object>) -> Result<Self, OperatorError> {
                    let value = arguments.pop().ok_or(OperatorError::MissingOperand)?;
                    let value = value.try_into()?;
                    Ok(Self(value))
                }
            }
        )+
    };
    (2; $($t:ty),+) => {
        $(
            impl FromArgs for $t {
                fn from_args(arguments: &mut Vec<Object>) -> Result<Self, OperatorError> {
                    let value = arguments.pop().ok_or(OperatorError::MissingOperand)?;
                    let value2 = value.try_into()?;

                    let value = arguments.pop().ok_or(OperatorError::MissingOperand)?;
                    let value1 = value.try_into()?;

                    Ok(Self(value1, value2))
                }
            }
        )+
    };
}

impl_from_args!(1;
    SetCharacterSpacing,
    SetWordSpacing,
    SetTextLeading,
    SetTextRise,
    SetHorizontalScaling
);
impl_from_args!(2;
    SetFontAndFontSize
);
