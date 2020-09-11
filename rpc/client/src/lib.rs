//! Network channel client implementing request-reply and notifications.
//!
//! Client implements request to reply mapping using only string IDs at the moment.
//! String IDs are most frequently the only supported way of calling methods.
//!
//! Clients are created using [`Builder`] and can be spawned in the background using [`spawn`] method.
//!
//! Interface to the client is provided by [`Handle`] structure.
//!
//! Client [`Handle`] sends requests and notifications to [`UnboundedSender`] channel.
//! Requests IDs are registered and received responses are send to requestee using oneshot [`Sender`].
//!
//! This library is intended to provide only a low-level access to network channel service.
//!
//! All APIs in this library are highly experimental and are subject to change.
//!
//! [`Handle`]: handle/struct.Handle.html
//! [`spawn`]: builder/struct.Builder.html#method.spawn
//! [`Channel`]: ../channel/struct.Channel.html
//! [`UnboundedSender`]: https://docs.rs/tokio/0.2/tokio/sync/mpsc/struct.UnboundedSender.html
//! [`Sender`]: https://docs.rs/tokio/0.2/tokio/sync/oneshot/struct.Sender.html

pub mod builder;
pub mod common;
pub mod handle;
pub(crate) mod handler;
pub mod notifications;
pub mod traits;

pub use self::builder::Builder as ClientBuilder;
pub use self::builder::*;
pub use self::handle::*;
pub use self::notifications::*;
pub use self::traits::*;

pub use net3_rpc_error::*;
