use livre_extraction::{Extract, FromDictRef, Reference};

#[derive(Debug, PartialEq, Eq, Clone, FromDictRef, Extract)]
pub struct Node {
    pub parent: Option<Reference>,
    pub kids: Vec<Reference>,
    // Redundant.
    // pub count: usize,
}

#[cfg(test)]
mod tests {

    use indoc::indoc;

    use super::*;

    #[test]
    fn node() {
        let input = indoc! {b"
            <</Type /Pages
                /Kids [4 0 R
                    10 0 R
                    24 0 R
                ] /Count 3
            >>
        "};

        let (_, node) = Node::extract(input).unwrap();
        assert_eq!(
            node.kids,
            vec![
                Reference::new(4, 0),
                Reference::new(10, 0),
                Reference::new(24, 0)
            ]
        );
    }
}
