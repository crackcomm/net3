//! Network channel client handle.

use std::{
    io::{Error, ErrorKind},
    ops::{Deref, Drop},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::{de::DeserializeOwned, ser::Serialize};
use tokio::{
    sync::{mpsc::UnboundedSender, oneshot::channel},
    time::timeout,
};

use tracing_attributes::instrument;

use crate::handler::internal::{ClientMessage, ResponseReceiver};

use net3_msg::{
    builder::{self, MessageBuilder},
    traits::Message,
    types::Id,
};
use net3_rpc_error::{Error as CallError, Result};

/// Handle reference.
///
/// It contains a clone of [`Handle`] but is not _owned_.
/// Contrary to a [`Handle`] client can disconnect even if there
/// are existing instances of `HandleRef`.
///
/// [`Handle`]: struct.Handle.html
pub struct HandleRef<M: Message, U> {
    inner: Handle<M, U>,
}

impl<M: Message, U> From<Handle<M, U>> for HandleRef<M, U> {
    fn from(handle: Handle<M, U>) -> Self {
        let inner = handle.unowned();
        HandleRef { inner }
    }
}

impl<M: Message, U> Deref for HandleRef<M, U> {
    type Target = Handle<M, U>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<M: Message, U> Clone for HandleRef<M, U> {
    fn clone(&self) -> Self {
        let inner = self.inner.clone().unowned();
        HandleRef { inner }
    }
}

/// Inner handle data representation.
pub(crate) struct InnerHandle<M: Message, U = ()> {
    /// Client ID.
    pub(crate) client_id: Option<u64>,
    /// Internal event sender.
    pub(crate) events: UnboundedSender<U>,
    /// Sender of messages forwarded to network.
    pub(crate) sender: UnboundedSender<ClientMessage<M>>,
    /// Atomic request counter for message ID.
    pub(crate) requests: Arc<AtomicU64>,
    /// Default request timeout used on a [`request`] call.
    ///
    /// [`request`]: struct.Handle.html#method.request
    pub(crate) request_timeout: Duration,
    /// Owned handle reference counter.
    /// It is decremented on a clone in `HandleRef`.
    pub(crate) instances: Arc<AtomicU64>,
}

impl<M: Message, U> Clone for InnerHandle<M, U> {
    fn clone(&self) -> Self {
        InnerHandle {
            client_id: self.client_id,
            events: self.events.clone(),
            sender: self.sender.clone(),
            requests: self.requests.clone(),
            request_timeout: self.request_timeout,
            instances: self.instances.clone(),
        }
    }
}

/// Network channel service client handle based on an [`UnboundedSender`].
///
/// Designed as a base for strongly typed API clients.
///
/// [`UnboundedSender`]: https://docs.rs/tokio/0.2/tokio/sync/mpsc/struct.UnboundedSender.html
pub struct Handle<M: Message, U = ()> {
    /// Shared handle data.
    pub(crate) inner: Arc<InnerHandle<M, U>>,
    /// Is the handle owned or not.
    /// Should the `Drop` decrement `instances`.
    pub(crate) is_owned: bool,
}

impl<M: Message, U> Handle<M, U> {
    /// Returns client ID if set in Builder.
    pub fn client_id(&self) -> Option<u64> {
        self.inner.client_id
    }

    /// Emits internal event.
    pub fn emit_internal(&self, event: U) -> std::io::Result<()> {
        self.inner
            .events
            .send(event)
            .map_err(|_err| Error::from(ErrorKind::ConnectionReset))
    }

    /// Sends a method call request to a network channel.
    /// Receives response from oneshot sender asynchronously.
    ///
    /// Default timeout duration is used to await for the response.
    #[instrument(skip(self, params))]
    pub async fn request<V: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<&V>,
    ) -> Result<R> {
        match self.request_opt(method, params).await? {
            Some(res) => Ok(res),
            None => Err(Error::from(ErrorKind::InvalidData).into()),
        }
    }

    /// Sends a method call request to a network channel.
    /// Receives response from oneshot sender asynchronously.
    ///
    /// Default timeout duration is used to await for the response.
    #[instrument(skip(self, params))]
    pub async fn request_opt<V: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<&V>,
    ) -> Result<Option<R>> {
        self.request_timeout(method, params, self.inner.request_timeout)
            .await
    }

    /// Helper for sending requests with no parameters.
    ///
    /// Otherwise one would have to use confusing `None as Option<&()>`.
    #[instrument(skip(self))]
    pub async fn request_no_params<R: DeserializeOwned>(&self, method: &str) -> Result<Option<R>> {
        self.request(method, None as Option<&()>).await
    }

    /// Sends a method call request to a network channel.
    ///
    /// Receives response from oneshot sender asynchronously.
    /// Timeouts with a default timeout set with [`with_timeout`].
    ///
    /// [`with_timeout`]: ../builder/struct.Builder.html#method.with_timeout
    #[instrument(skip(self, params))]
    pub async fn request_timeout<V: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<&V>,
        request_timeout: Duration,
    ) -> Result<Option<R>> {
        let (msg_id, receiver) = self.send_request(method, params)?;
        match timeout(request_timeout, receiver).await {
            Ok(response) => {
                match response.map_err(|_err| Error::from(ErrorKind::ConnectionReset))? {
                    Ok(message) => Ok(message.read_optional()?),
                    Err(err) => Err(CallError::Rpc(err)),
                }
            }
            Err(_) => {
                // Send cancelation requests to channel.
                self.inner
                    .sender
                    .send(ClientMessage::Cancel(msg_id))
                    .map_err(|_err| Error::from(ErrorKind::ConnectionReset))?;
                Err(CallError::from(ErrorKind::TimedOut))
            }
        }
    }

    /// Sends an event message to a network channel.
    #[instrument(skip(self, params))]
    pub fn send_notification<T: Serialize>(
        &self,
        method: &str,
        params: Option<&T>,
    ) -> std::io::Result<()> {
        // let message = Message::new_event(method, params)?;
        let message = builder::new_event::<M, T>(method, params)?.build();
        tracing::info!("sending event");
        self.inner
            .sender
            .send(ClientMessage::Request(
                message,
                None,
                tracing::info_span!("send_notification"),
            ))
            .map_err(|_err| Error::from(ErrorKind::ConnectionReset))?;
        Ok(())
    }

    /// Sends a method call request to a network channel.
    /// Does not await for response, instead returns a receiver handle.
    /// Includes `u64` request ID to provide ability to cancel requests.
    #[instrument(skip(self, params))]
    fn send_request<T: Serialize>(
        &self,
        method: &str,
        params: Option<&T>,
    ) -> Result<(Id, ResponseReceiver<M>)> {
        // Create a oneshot response channel.
        let (sender, receiver) = channel();
        // Increment request ID and get previous value.
        let msg_id = self.inner.requests.fetch_add(1, Ordering::SeqCst);
        // Convert message ID to a string.
        let msg_id = Id::Str(msg_id.to_string());
        // Create a protocol message.
        let message = builder::new_request::<M, T>(msg_id.clone(), method, params)?.build();
        tracing::info!("sending to channel");
        // Send message to a client channel.
        self.inner
            .sender
            .send(ClientMessage::Request(
                message,
                Some(sender),
                tracing::info_span!("send"),
            ))
            .map_err(|_| Error::from(ErrorKind::ConnectionReset))?;
        Ok((msg_id, receiver))
    }

    /// Sends a protocol message to a channel.
    pub fn send(&self, message: M) -> std::io::Result<()> {
        Ok(self
            .inner
            .sender
            .send(ClientMessage::Request(
                message,
                None,
                tracing::Span::current(),
            ))
            .map_err(|_err| Error::from(ErrorKind::ConnectionReset))?)
    }

    /// Closes the connection.
    pub(crate) fn close(&self) -> std::io::Result<()> {
        Ok(self
            .inner
            .sender
            .send(ClientMessage::Close)
            .map_err(|_err| Error::from(ErrorKind::ConnectionReset))?)
    }

    fn unowned(mut self) -> Self {
        if self.is_owned {
            let _ = self.inner.instances.fetch_sub(1, Ordering::SeqCst);
            self.is_owned = false;
        }
        self
    }
}

impl<M: Message, U> Drop for Handle<M, U> {
    fn drop(&mut self) {
        if self.is_owned {
            let instances = self.inner.instances.fetch_sub(1, Ordering::SeqCst);
            log::trace!("handle dropped; left={}", instances - 1);
        }
    }
}

impl<M: Message, U> Clone for Handle<M, U> {
    fn clone(&self) -> Self {
        if self.is_owned {
            let instances = self.inner.instances.fetch_add(1, Ordering::SeqCst);
            log::trace!("handle cloned; total={}", instances + 1);
        }
        Handle {
            inner: self.inner.clone(),
            is_owned: self.is_owned,
        }
    }
}
