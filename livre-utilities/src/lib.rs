//! Utilities for parsing PDFs.

mod whitespace;
pub use whitespace::{
    is_space_or_newline, space, take_eol, take_eol_no_r, take_whitespace, take_whitespace1,
};

mod delimited;
pub use delimited::{parse_dict_body, take_within_balanced};

mod digits;
pub use digits::{
    parse_digits, parse_hexadecimal_bigram, parse_integer, parse_number, parse_octal, parse_real,
    parse_sign, parse_unsigned_integer,
};

mod misc;
pub use misc::{parse_comment, parse_string_with_escapes};
