use livre_extraction::{Extract, FromDictRef, Map};
use livre_objects::Reference;

#[derive(Debug, PartialEq, FromDictRef, Extract, Clone)]
pub struct Resources {
    // pub ext_g_state: Option<&'i [u8]>,
    // pub color_space: Option<&'i [u8]>,
    // pub pattern: Option<&'i [u8]>,
    // pub shading: Option<&'i [u8]>,
    // pub x_object: Option<&'i [u8]>,
    pub font: Map<Reference>,
    // pub properties: Option<&'i [u8]>,
}

#[cfg(test)]
mod tests {

    // <</ProcSet [/PDF /Text /ImageB /ImageC /ImageI]
    // /ExtGState <</G3 3 0 R>>
    // /Font <</F4 4 0 R>>>>
}
