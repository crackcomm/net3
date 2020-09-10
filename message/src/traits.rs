use std::{
    fmt::Debug,
    io::{self, ErrorKind, Result},
};

pub use serde::{de::DeserializeOwned, ser::Serialize};

use crate::{builder::MessageBuilderExt, types};

/// Message ID trait.
pub trait Id {
    fn id(&self) -> &types::Id;
}

/// Message method trait.
pub trait Method {
    fn method(&self) -> Option<&str>;
}

/// Message kind trait.
pub trait Kind {
    fn kind(&self) -> types::MessageKind;
}

/// Message read trait.
pub trait Read {
    fn read<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        self.read_optional::<T>()?
            .ok_or_else(|| io::Error::from(ErrorKind::InvalidInput))
    }
    fn read_optional<T: serde::de::DeserializeOwned>(&self) -> Result<Option<T>>;
}

/// Error message trait.
pub trait Error {
    fn error_kind(&self) -> Option<&types::ErrorKind>;
    fn description(&self) -> Option<&str>;
    fn into_error(self) -> Option<types::Error>;
}

pub trait Message:
    Id
    + Method
    + Kind
    + Read
    + Error
    + MessageBuilderExt
    + DeserializeOwned
    + Serialize
    + Send
    + Sync
    + Clone
    + Debug
{
}
