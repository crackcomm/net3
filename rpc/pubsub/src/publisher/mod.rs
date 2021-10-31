//! Internal job channel server.

use std::io::{Error, ErrorKind};

use serde::ser::Serialize;

use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};

use net3_msg::{builder, traits::Message};
use net3_rpc_client::{common::InitFunc, Handle};

/// Publisher structure.
#[derive(Clone)]
pub struct Publisher<M: Message> {
    sender: mpsc::UnboundedSender<M>,
}

impl<M> Publisher<M>
where
    M: Message + 'static,
    M: builder::MessageBuilder<M>,
    M: builder::MessageBuilderExt<Builder = M>,
{
    /// Publishes a job.
    pub fn publish<T: Serialize>(&self, channel: &str, data: Option<&T>) -> std::io::Result<()> {
        self.sender
            .send(builder::new_event::<M, T>(channel, data)?)
            .map_err(|err| Error::new(ErrorKind::ConnectionReset, err))
    }
}

/// Publisher builder.
pub struct Builder<M: Message, U = ()> {
    // Communication with the server
    hnd_sender: mpsc::UnboundedSender<Handle<M, U>>,
    hnd_receiver: mpsc::UnboundedReceiver<Handle<M, U>>,
    // Communication with publishers
    msg_sender: mpsc::UnboundedSender<M>,
    msg_receiver: mpsc::UnboundedReceiver<M>,
}

impl<M: Message + 'static, U: 'static + Send> Builder<M, U> {
    /// Creates a publisher
    pub fn publisher(&self) -> Publisher<M> {
        Publisher {
            sender: self.msg_sender.clone(),
        }
    }

    /// Spawns publisher in the background.
    pub fn background(self) -> tokio::task::JoinHandle<std::io::Result<()>> {
        tokio::spawn(async move { self.start().await })
    }

    /// Binds a publisher to a TCP address and starts publisher loop.
    pub async fn start(self) -> std::io::Result<()> {
        let mut handles = Vec::new();
        let mut msg_receiver = UnboundedReceiverStream::new(self.msg_receiver);
        let mut hnd_receiver = UnboundedReceiverStream::new(self.hnd_receiver);
        loop {
            tokio::select! {
                message = msg_receiver.next() => match message {
                    Some(message) => {
                        handles = handles
                            .into_iter()
                            .filter(|handle: &Handle<M, U> | {
                                handle.send(message.clone()).is_ok()
                            })
                            .collect();
                    },
                    None => return Ok(()),
                },
                handle = hnd_receiver.next() => match handle {
                    Some(handle) => handles.push(handle),
                    None => return Ok(()),
                }
            }
        }
    }

    /// Returns handle register channel.
    #[inline]
    pub fn registration(&self) -> mpsc::UnboundedSender<Handle<M, U>> {
        self.hnd_sender.clone()
    }

    /// Returns server initialization function.
    pub fn server_init_fn(&self) -> InitFunc<M, U> {
        let sender = self.registration();
        Box::new(move |handle| {
            let _ = sender.send(handle.clone());
        })
    }
}

impl<M: Message, U> Default for Builder<M, U> {
    fn default() -> Self {
        let (hnd_sender, hnd_receiver) = mpsc::unbounded_channel();
        let (msg_sender, msg_receiver) = mpsc::unbounded_channel();
        Builder {
            hnd_sender,
            hnd_receiver,
            msg_sender,
            msg_receiver,
        }
    }
}
