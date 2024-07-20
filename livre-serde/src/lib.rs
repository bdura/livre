mod de;
mod error;

use error::{Error, Result};

pub use de::{from_bytes, from_bytes_prefix, from_str, Deserializer};
