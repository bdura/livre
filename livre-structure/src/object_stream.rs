use std::collections::HashMap;

use livre_extraction::{Extract, FromDictRef};
use livre_utilities::{space, take_whitespace};
use nom::{
    multi::count,
    sequence::{terminated, tuple},
};

use livre_objects::Stream;

#[derive(FromDictRef)]
struct ObjectStreamConfig {
    n: usize,
    first: usize,
    // extends: Option<Reference>,
}

pub struct ObjectStream {
    pub objects: HashMap<usize, usize>,
    pub decoded: Vec<u8>,
}

impl<'input> Extract<'input> for ObjectStream {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, stream) = Stream::<'input, ObjectStreamConfig>::extract(input)?;

        let decoded = stream.decode().unwrap();

        let Stream {
            structured: ObjectStreamConfig { n, first },
            ..
        } = stream;

        let (_, refs) = count(
            tuple((
                terminated(usize::extract, space),
                terminated(usize::extract, take_whitespace),
            )),
            n,
        )(&decoded)
        .unwrap();

        let objects = refs.into_iter().collect();

        let objstm = Self {
            objects,
            decoded: decoded[first..].to_vec(),
        };

        Ok((input, objstm))
    }
}

#[cfg(test)]
mod tests {

    use livre_objects::Object;

    use super::*;

    #[test]
    fn object_stream() {
        let input = include_bytes!("../../tests/objects/stream.bin");

        let (_, objstm) = ObjectStream::extract(input).unwrap();

        let &offset = objstm.objects.get(&90825).unwrap();
        println!("{}", String::from_utf8_lossy(&objstm.decoded[offset..]));

        for (k, &offset) in &objstm.objects {
            let slice = &objstm.decoded[offset..];
            println!(
                "Object {k}: {:?}",
                String::from_utf8_lossy(&slice[..slice.len().min(100)])
            );
            println!("{:?}", Object::extract(slice).unwrap().1);
            println!();
        }
    }
}
