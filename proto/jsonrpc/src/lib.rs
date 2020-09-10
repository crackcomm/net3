#[macro_use]
extern crate serde_derive;

pub mod code;
pub mod error;
pub mod message;
pub mod params;
pub mod version;

pub use self::code::ErrorCode;
pub use self::error::Error;
pub use self::message::*;
pub use self::params::{Params, RawValue};
pub use self::version::Version;

#[cfg(test)]
mod tests;
