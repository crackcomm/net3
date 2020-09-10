//! Network channel message handler loop module.
//!
//! This library is intended for low-level usage.
//! It is wrapped by [`client`] and [`server`] to provide higher level functionality
//!
//! [`client`]: ../client/index.html
//! [`server`]: ../server/index.html
#![recursion_limit = "512"]

use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    marker::Unpin,
};

use futures::{
    select,
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt},
};
use futures_option::OptionExt as _;

pub use net3_rpc_conn_handler::LoopHandler;

use net3_msg::traits::Message;

/// Starts channel message handler loop.
///
/// Loop will return on connection or [`LoopHandler`] error.
///
/// [`LoopHandler`]: trait.LoopHandler.html
#[inline]
pub async fn start_loop<C, H, M, E>(channel: C, handler: H, events: Option<E>) -> Result<()>
where
    M: Message + 'static,
    C: Sink<M, Error = Error> + Stream<Item = Result<M>> + Unpin,
    E: Stream<Item = <H as LoopHandler>::InternalEvent> + Unpin,
    H: LoopHandler<RemoteMessage = M> + Stream<Item = Result<M>> + Unpin + 'static,
    H: Send,
    <H as LoopHandler>::InternalEvent: Sized + Send + Sync + Clone + Debug,
{
    let mut handler = handler.fuse();
    let mut channel = channel.fuse();
    let mut events = events.map(|stream| stream.fuse());

    loop {
        select! {
            message = handler.next() => match message {
                Some(Ok(message)) => {
                    channel
                        .get_mut()
                        .send(message)
                        .await?;
                },
                Some(Err(err)) => return Err(err),
                None => {
                    log::trace!("Connection aborted because handler sender is dropped.");
                    return Err(ErrorKind::ConnectionAborted.into())
                },
            },
            message = channel.next() => match message {
                Some(Ok(message)) => {
                    // Let the `handler` handle the message.
                    let messages = handler.get_mut().handle_remote_message(message).await?;
                    for message in messages {
                        channel
                            .get_mut()
                            .send(message)
                            .await?;
                    }
                },
                Some(Err(err)) => return Err(err),
                None => {
                    log::trace!("Channel stream was closed.");
                    return Err(ErrorKind::ConnectionReset.into())
                },
            },
            event = events.next() => match event {
                Some(event) => {
                    let messages = handler.get_mut().handle_internal_event(event).await?;
                    for message in messages {
                        channel
                            .get_mut()
                            .send(message)
                            .await?;
                    }
                }
                None => {
                    log::trace!("Connection aborted because event sender is dropped.");
                    return Err(ErrorKind::ConnectionAborted.into())
                },
            },
            complete => break,
        }
    }
    Ok(())
}
