pub mod operators;

mod object;
pub use object::TextObject;

trait Operator {
    fn operate(self, obj: &mut TextObject);
}
