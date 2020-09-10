//! Client traits.

use std::{fmt::Debug, io::Result};

// re-export
pub use tokio_util::codec::{Decoder, Encoder};

use async_trait::async_trait;

use crate::{builder::ClientHandle, handle::Handle};

use net3_msg::traits::Message;

/// Connection handler builder trait.
///
/// When implementing your own handler builder for a server,
/// remember that you have to take ownership of a [`Handle`].
/// Otherwise the connection will be dropped immediately.
///
/// [`Handle`]: ../../handle/struct.Handle.html
#[async_trait]
pub trait HandlerBuilder {
    /// Handler type.
    type Handler: Handler;

    /// Creates a new handler for client.
    async fn build_handler(&mut self, handle: &ClientHandle<Self::Handler>) -> Self::Handler;
}

/// Connection initializer.
#[async_trait]
pub trait Initializer<M: Message, U = ()> {
    /// Initializes a connection.
    ///
    /// It can return an error to immediately close the connection.
    async fn init(&mut self, handle: &Handle<M, U>) -> std::io::Result<()>;
}

/// Network [`Channel`] message handler trait.
///
/// [`Channel`]: ../../channel/type.Channel.html
#[async_trait]
pub trait Handler {
    /// Internal event type.
    type Event: Sized + Send + Sync + Clone + Debug;

    /// Message type.
    type Message: Message;

    /// Handles event message.
    async fn handle_notification(&mut self, _message: Self::Message) -> Result<Vec<Self::Message>> {
        Ok(vec![])
    }

    /// Handles request message.
    async fn handle_request(&mut self, _message: Self::Message) -> Result<Vec<Self::Message>> {
        Ok(vec![])
    }

    /// Handles internal event.
    async fn handle_internal_event(&mut self, _event: Self::Event) -> Result<Vec<Self::Message>> {
        Ok(vec![])
    }
}
