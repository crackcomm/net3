#[macro_use]
extern crate serde_derive;

pub mod builder;
pub mod compact;
pub mod traits;
pub mod types;

pub mod prelude {
    pub use crate::{
        builder::{self, MessageBuilder, MessageBuilderExt},
        traits::{
            self, DeserializeOwned, Error as _, Id as _, Kind as _, Message as _, Method as _,
            Read as _, Serialize,
        },
        types::{self, ErrorKind, Id, MessageKind},
    };
}

pub type Result<T> = std::result::Result<T, types::Error>;
