pub mod operators;

mod object;
pub use object::TextObject;

trait Operator {
    fn apply(self, obj: &mut TextObject);
}
