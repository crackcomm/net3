//! Common client utilities.

use std::{
    io::Result,
    ops::{Deref, DerefMut},
};

use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{handle::Handle, traits::Handler};

use net3_msg::traits::Message;

/// Notifications receiver.
pub struct Notifications<M: Message, T = M, U = ()> {
    /// Client handle.
    pub handle: Handle<M, U>,
    /// Notifications receiver.
    pub receiver: UnboundedReceiver<T>,
}

impl<M: Message, T> Deref for Notifications<M, T> {
    type Target = UnboundedReceiver<T>;

    fn deref(&self) -> &Self::Target {
        &self.receiver
    }
}

impl<M: Message, T> DerefMut for Notifications<M, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.receiver
    }
}

/// Notification handler for messages.
pub struct NotificationHandler<M, T = M>(pub UnboundedSender<T>, pub std::marker::PhantomData<M>);

#[async_trait]
impl<M, T> Handler for NotificationHandler<M, T>
where
    M: Message,
    T: From<M> + Send + Sync + Clone,
{
    type Event = ();
    type Message = M;

    async fn handle_notification(&mut self, message: Self::Message) -> Result<Vec<Self::Message>> {
        self.0
            .send(T::from(message))
            .map(|_| vec![])
            .map_err(|_| std::io::ErrorKind::BrokenPipe.into())
    }

    async fn handle_request(&mut self, _message: Self::Message) -> Result<Vec<Self::Message>> {
        log::trace!("request on notification handler");
        Ok(vec![])
    }

    #[allow(clippy::unit_arg)]
    #[inline]
    async fn handle_internal_event(&mut self, _event: Self::Event) -> Result<Vec<Self::Message>> {
        Ok(vec![])
    }
}

impl<M, T> From<UnboundedSender<T>> for NotificationHandler<M, T>
where
    T: From<M>,
{
    fn from(sink: UnboundedSender<T>) -> Self {
        NotificationHandler(sink, std::marker::PhantomData)
    }
}

impl<M, T> Clone for NotificationHandler<M, T> {
    fn clone(&self) -> Self {
        NotificationHandler(self.0.clone(), std::marker::PhantomData)
    }
}
