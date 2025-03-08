use crate::content::state::{TextObject, TextStateParameters};

pub trait TextOperation: Sized {
    fn apply_partial(self, _: &mut (f32, f32), _: &mut TextStateParameters) {}
    fn apply(self, text_object: &mut TextObject) {
        self.apply_partial(&mut text_object.position, &mut text_object.parameters);
    }
}
