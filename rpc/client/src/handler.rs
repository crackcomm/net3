//! Network channel client handler implementation.

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Debug},
    io::{Error, ErrorKind, Result},
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures::stream::Stream;
use pin_project::pin_project;
use tokio::sync::mpsc::UnboundedReceiver;

use net3_msg::{
    prelude::*,
    types::{Id, MessageKind},
};
use net3_rpc_conn::LoopHandler;

use crate::{handle::HandleRef, traits::Handler};

pub(crate) mod internal {
    use net3_msg::types::{Error, Id};
    use tokio::sync::oneshot::{Receiver, Sender};

    /// Network channel client response `Result` type.
    pub type Response<M> = Result<M, Error>;

    /// Response result oneshot sender type.
    pub type ResponseSender<M> = Sender<Response<M>>;

    /// Response result oneshot receiver type.
    pub type ResponseReceiver<M> = Receiver<Response<M>>;

    /// Client message with optional response sender and span.
    pub enum ClientMessage<M> {
        /// Close request.
        Close,

        /// Cancellation of a request ID.
        /// Sent by [`Handle.request_timeout`] after time-out fires.
        Cancel(Id),

        /// Request to send a message to the channel.
        /// Includes optional [`ResponseSender`] and `u64` request ID.
        Request(M, Option<ResponseSender<M>>, tracing::Span),
    }
}

use self::internal::{ClientMessage, ResponseSender};

/// Response oneshot sender with span.
type SpannedSender<M> = (ResponseSender<M>, tracing::Span);

/// Network channel client service handler.
#[pin_project]
pub(crate) struct ClientHandler<H: Handler> {
    #[pin]
    receiver: ClonedReceiver<ClientMessage<<H as Handler>::Message>>,
    handler: H,
    handle: HandleRef<<H as Handler>::Message, <H as Handler>::Event>,
    requests: HashMap<Id, SpannedSender<<H as Handler>::Message>>,
    pending_requests: usize,
    /// Counter of client instances.
    client_handles: Arc<AtomicU64>,
}

impl<H: Handler> ClientHandler<H> {
    pub fn new(
        rx: ClonedReceiver<ClientMessage<<H as Handler>::Message>>,
        handler: H,
        handle: HandleRef<<H as Handler>::Message, <H as Handler>::Event>,
        client_handles: Arc<AtomicU64>,
    ) -> Self {
        ClientHandler {
            receiver: rx,
            requests: HashMap::default(),
            handler,
            handle,
            pending_requests: 0,
            client_handles,
        }
    }
}

#[async_trait]
impl<H> LoopHandler for ClientHandler<H>
where
    H: Handler + Send,
{
    type InternalEvent = <H as Handler>::Event;
    type RemoteMessage = <H as Handler>::Message;

    /// Handles internal event.
    async fn handle_internal_event(
        &mut self,
        event: Self::InternalEvent,
    ) -> Result<Vec<Self::RemoteMessage>> {
        self.handler.handle_internal_event(event).await
    }

    /// Handles event message.
    #[inline]
    async fn handle_remote_message(
        &mut self,
        message: Self::RemoteMessage,
    ) -> Result<Vec<Self::RemoteMessage>> {
        match message.kind() {
            MessageKind::Undefined => {
                Err(Error::new(ErrorKind::InvalidData, "undefined message kind"))
            }
            MessageKind::Event => self.handler.handle_notification(message).await,
            MessageKind::Request => self.handler.handle_request(message).await,
            MessageKind::Response => {
                tracing::info!("handle_call_response");
                if let Some((sender, span)) = self.requests.remove(message.id()) {
                    let span = tracing::info_span!(parent: &span, "response");
                    let _enter = span.enter();
                    tracing::info!("received response");
                    if sender.send(Ok(message)).is_err() {
                        tracing::error!("could not send message");
                    }
                } else {
                    tracing::warn!("response handler not found");
                }
                Ok(vec![])
            }
            MessageKind::ErrorResponse => {
                tracing::info!("handle_call_error");
                if let Some((sender, span)) = self.requests.remove(message.id()) {
                    let span = tracing::info_span!(parent: &span, "response");
                    let _enter = span.enter();
                    tracing::info!("received error");
                    self.pending_requests -= 1;
                    let err = message.into_error().expect("error");
                    if sender.send(Err(err)).is_err() {
                        tracing::error!("could not send value");
                    }
                } else {
                    tracing::warn!("response handler not found (possible receive after timeout)");
                }
                Ok(vec![])
            }
        }
    }
}

impl<H: Handler + 'static> Stream for ClientHandler<H> {
    type Item = Result<<H as Handler>::Message>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let project = self.project();
        match project.receiver.poll_next(cx) {
            Poll::Ready(Some(ClientMessage::Close)) => Poll::Ready(Some(Err(Error::new(
                ErrorKind::ConnectionAborted,
                "requested close",
            )))),
            Poll::Ready(Some(ClientMessage::Cancel(request))) => {
                // Remove pending request
                project.requests.remove(&request);
                // continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(ClientMessage::Request(message, sender, span))) => {
                let _enter = span.enter();
                if let Some(sender) = sender {
                    // Insert response handler to requests map.
                    project.requests.insert(
                        message.id().clone(),
                        (sender, tracing::info_span!(parent: &span, "request")),
                    );
                    *project.pending_requests += 1;
                    tracing::info!("sending request");
                } else {
                    tracing::info!("sending notification");
                }
                Poll::Ready(Some(Ok(message)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                if project.client_handles.load(Ordering::SeqCst) == 0 {
                    log::trace!("client handles are all dropped");
                    Poll::Ready(None)
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
    }
}

impl<H: Handler> Debug for ClientHandler<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientHandler")
            .field("pending_requests", &self.pending_requests)
            .finish()
    }
}

/// Network message receiver that can be cloned.
///
/// It is used in [`start_loop_reconnect`] to re-use [`Handle`] senders.
/// Do **NOT** use this receiver concurrently.
///
/// [`start_loop_reconnect`]: ../struct.Builder.html#method.start_loop_reconnect
/// [`Handle`]: ../handle/struct.Handle.html
#[pin_project]
pub(crate) struct ClonedReceiver<M> {
    #[pin]
    inner: Arc<RefCell<UnboundedReceiver<M>>>,
}

unsafe impl<M> Send for ClonedReceiver<M> {}
unsafe impl<M> Sync for ClonedReceiver<M> {}

impl<M> Stream for ClonedReceiver<M> {
    type Item = M;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.borrow_mut().poll_recv(cx)
    }
}

impl<M> Clone for ClonedReceiver<M> {
    fn clone(&self) -> Self {
        ClonedReceiver {
            inner: self.inner.clone(),
        }
    }
}

impl<M> From<UnboundedReceiver<M>> for ClonedReceiver<M> {
    fn from(inner: UnboundedReceiver<M>) -> Self {
        ClonedReceiver {
            inner: Arc::new(RefCell::new(inner)),
        }
    }
}
