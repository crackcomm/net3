//! Msgpack message channel encoder and decoder implementation.

use std::io::{Error, ErrorKind, Result};

use bytes::{buf::Buf, BytesMut};
use serde::{de::DeserializeOwned, Serialize};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

/// Msgpack message channel codec.
#[derive(Default)]
pub struct Codec<T> {
    inner: LengthDelimitedCodec,
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned + Sized> Decoder for Codec<T> {
    type Item = T;
    type Error = Error;

    #[inline]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if let Some(msg) = self.inner.decode(src)? {
            rmp_serde::from_read(msg.reader())
                .map_err(|err| Error::new(ErrorKind::InvalidData, err))
        } else {
            Ok(None)
        }
    }
}

impl<T: Serialize + Sized> Encoder<T> for Codec<T> {
    type Error = Error;

    #[inline]
    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<()> {
        let body =
            rmp_serde::to_vec(&item).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
        Ok(self.inner.encode(body.into(), dst)?)
    }
}
