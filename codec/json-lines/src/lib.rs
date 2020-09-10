//! JSON lines channel message encoder and decoder implementation.
//!
//! Serialization and deserialization is done with [`serde_json`].
//! Messages are decoded and encoded with a new line separator.
//!
//! [`serde_json`]: https://docs.rs/serde_json/1/serde_json/

use std::io::{Error, ErrorKind};

use bytes::{
    buf::{ext::BufMutExt, BufMut},
    BytesMut,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio_util::codec::{Decoder, Encoder, LinesCodec};

/// JSON lines channel message codec.
///
/// Implements [`Encoder`] and [`Decoder`] traits for messages that implement
/// [`Serialize`] and [`DeserializeOwned`] respectively.
///
/// [`Encoder`]: https://docs.rs/tokio-util/0.3.1/tokio_util/codec/trait.Encoder.html
/// [`Decoder`]: https://docs.rs/tokio-util/0.3.1/tokio_util/codec/trait.Decoder.html
/// [`Serialize`]: https://docs.rs/serde/1/serde/ser/trait.Serialize.html
/// [`DeserializeOwned`]: https://docs.rs/serde/1/serde/de/trait.DeserializeOwned.html
#[derive(Default)]
pub struct Codec<T> {
    inner: LinesCodec,
    phantom: std::marker::PhantomData<T>,
}

/// Deserializes the message using [`serde_json::from_str`].
///
/// [`serde_json::from_str`]: https://docs.rs/serde_json/1/serde_json/fn.from_str.html
impl<T: DeserializeOwned + Sized> Decoder for Codec<T> {
    type Item = T;
    type Error = Error;

    #[inline(always)]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self
            .inner
            .decode(src)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err))?
        {
            Some(msg) => {
                if cfg!(debug_assertions) {
                    log::trace!("JSON deserialize body={}", msg);
                }
                Ok(serde_json::from_str(&msg)
                    .map_err(|err| Error::new(ErrorKind::InvalidData, err))?)
            }
            None => Ok(None),
        }
    }
}

impl<T: Serialize + Sized> Encoder<T> for Codec<T> {
    type Error = Error;

    #[inline(always)]
    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if cfg!(debug_assertions) {
            let body = serde_json::to_string(&item)
                .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
            log::trace!("JSON codec body={}", body);
            dst.put_slice(body.as_bytes());
        } else {
            serde_json::to_writer(dst.writer(), &item)
                .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
        }
        dst.put_slice(b"\n");
        Ok(())
    }
}
