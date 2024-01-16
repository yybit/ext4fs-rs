use std::io::Read;

use bincode::Options;
use serde::de::DeserializeOwned;

use super::errors::ExtfsError;

pub trait Decoder: DeserializeOwned {
    #[inline]
    fn decode_from(mut reader: impl Read) -> Result<Self, ExtfsError> {
        let codec = bincode::options()
            .with_little_endian()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let obj: Self = codec.deserialize_from(&mut reader)?;
        Ok(obj)
    }
}

impl<T: DeserializeOwned> Decoder for T {}
