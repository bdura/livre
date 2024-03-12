use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

fn main() {
    let file = File::open("resource/ISO_32000-2-2020_sponsored.pdf").unwrap();
    let mut reader = BufReader::new(file);

    reader.seek(std::io::SeekFrom::Start(15522695)).unwrap();

    let mut buf: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buf).ok();

    let mut file = File::create("test.bin").unwrap();
    file.write_all(&buf).ok();

    // let (_, Trailer { dict, refs }) = extract(&buf[15522695..]).unwrap();
    // println!("{refs:?}");
}
