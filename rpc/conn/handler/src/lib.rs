//! net3 channel connection loop message handler

use std::{fmt::Debug, io::Result};

/// Network [`Channel`] message handler trait.
///
/// [`Channel`]: ../../../net3_channel/type.Channel.html
#[async_trait::async_trait]
pub trait LoopHandler {
    type InternalEvent: Sized + Send + Sync + Clone + Debug;
    type RemoteMessage;

    async fn handle_remote_message(
        &mut self,
        _message: Self::RemoteMessage,
    ) -> Result<Vec<Self::RemoteMessage>>;

    async fn handle_internal_event(
        &mut self,
        _event: Self::InternalEvent,
    ) -> Result<Vec<Self::RemoteMessage>>;
}
