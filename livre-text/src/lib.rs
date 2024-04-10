pub mod operators;

mod object;
pub use object::TextState;

trait Operator {
    fn apply(self, obj: &mut TextState);
}
